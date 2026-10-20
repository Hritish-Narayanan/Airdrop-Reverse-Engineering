use crate::cpio::{build_cpio_archive, CpioEntry};
use crate::protocol::{infer_uti, AskFile, AskRequest, AskResponse, DiscoverRequest, DiscoverResponse, UploadResponse};
use reqwest::header::{CONTENT_TYPE, USER_AGENT};
use std::path::Path;
use tracing::{info, warn};
use uuid::Uuid;
use base64::prelude::*;

#[derive(Debug, thiserror::Error)]
pub enum AirDropClientError {
    #[error("Network HTTP error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Transfer was declined by receiver")]
    Declined,
    #[error("Handshake failed: {0}")]
    Handshake(String),
}

/// AirDrop client capable of initiating transfers to an Apple device.
pub struct AirDropClient {
    client: reqwest::Client,
    computer_name: String,
    model_name: String,
    sender_id: String,
}

impl AirDropClient {
    pub fn new(computer_name: String, model_name: String) -> Self {
        // Build reqwest client accepting self-signed AirDrop peer certificates
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .expect("Failed to build reqwest client");

        Self {
            client,
            computer_name,
            model_name,
            sender_id: Uuid::new_v4().to_string(),
        }
    }

    /// Performs the initial `/Discover` handshake with the target AirDrop peer.
    pub async fn discover(&self, target_base_url: &str) -> Result<DiscoverResponse, AirDropClientError> {
        let url = format!("{}/Discover", target_base_url.trim_end_matches('/'));
        info!("Sending /Discover to {}", url);

        let req = DiscoverRequest {
            sender_record_data: None,
            receiver_record_data: None,
        };

        let res = self.client.post(&url)
            .header(USER_AGENT, "AirDrop/1.0")
            .header(CONTENT_TYPE, "application/json")
            .json(&req)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            warn!("Discover rejected by target: status={}, body={}", status, body);
            // Even if Discover returns 404 or empty on some devices, /Ask can proceed
            return Ok(DiscoverResponse {
                status: "success".to_string(),
                device_support_flags: Some(1),
                protocol_version: Some("1.0".to_string()),
            });
        }

        let resp = res.json::<DiscoverResponse>().await.unwrap_or(DiscoverResponse {
            status: "success".to_string(),
            device_support_flags: Some(1),
            protocol_version: Some("1.0".to_string()),
        });

        Ok(resp)
    }

    /// Asks the recipient device for permission to send the files (`POST /Ask`).
    pub async fn ask(
        &self,
        target_base_url: &str,
        ask_request: &AskRequest,
    ) -> Result<AskResponse, AirDropClientError> {
        let url = format!("{}/Ask", target_base_url.trim_end_matches('/'));
        info!("Sending /Ask to {} for {} files", url, ask_request.files.len());

        let res = self.client.post(&url)
            .header(USER_AGENT, "AirDrop/1.0")
            .header(CONTENT_TYPE, "application/json")
            .json(ask_request)
            .send()
            .await?;

        let status_code = res.status();
        let body_text = res.text().await?;
        info!("/Ask reply status: {}, body: {}", status_code, body_text);

        if !status_code.is_success() {
            return Err(AirDropClientError::Declined);
        }

        let resp: AskResponse = serde_json::from_str(&body_text)
            .unwrap_or_else(|_| AskResponse {
                status: if body_text.to_lowercase().contains("accept") {
                    "Accepted".to_string()
                } else {
                    "Declined".to_string()
                },
                next_attempt_delay_ms: None,
            });

        if resp.status.to_lowercase() != "accepted" {
            return Err(AirDropClientError::Declined);
        }

        Ok(resp)
    }

    /// Uploads the CPIO payload to `POST /Upload`.
    pub async fn upload(
        &self,
        target_base_url: &str,
        cpio_payload: Vec<u8>,
    ) -> Result<UploadResponse, AirDropClientError> {
        let url = format!("{}/Upload", target_base_url.trim_end_matches('/'));
        info!("Uploading {} bytes to {}", cpio_payload.len(), url);

        let res = self.client.post(&url)
            .header(USER_AGENT, "AirDrop/1.0")
            .header(CONTENT_TYPE, "application/x-cpio")
            .header("X-AirDrop-Format", "cpio")
            .body(cpio_payload)
            .send()
            .await?;

        let status_code = res.status();
        let body = res.text().await.unwrap_or_default();
        info!("/Upload response: {}, body: {}", status_code, body);

        if !status_code.is_success() {
            return Err(AirDropClientError::Handshake(format!("Upload failed with status {}", status_code)));
        }

        Ok(UploadResponse {
            status: "Success".to_string(),
        })
    }

    /// High-level function to send one or more local files to an AirDrop target URL.
    pub async fn send_files<P: AsRef<Path>>(
        &self,
        target_base_url: &str,
        file_paths: &[P],
    ) -> Result<(), AirDropClientError> {
        // Step 1: Read metadata first without buffering full payload in memory
        let mut ask_files = Vec::new();
        let mut preview_icon_data: Option<String> = None;

        for p in file_paths {
            let path = p.as_ref();
            let file_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file.dat")
                .to_string();

            let metadata = std::fs::metadata(path)?;
            let size = metadata.len();
            let file_type = infer_uti(&file_name).to_string();

            // If an image and small preview not generated yet, extract thumbnail
            if preview_icon_data.is_none() && (
                file_name.ends_with(".png") || file_name.ends_with(".jpg") || file_name.ends_with(".jpeg") || file_name.ends_with(".webp")
            ) {
                if size <= 500_000 {
                    if let Ok(data) = std::fs::read(path) {
                        preview_icon_data = Some(BASE64_STANDARD.encode(&data));
                    }
                }
            }

            ask_files.push(AskFile {
                file_name: file_name.clone(),
                file_type,
                file_bom_path: format!("./{}", file_name),
                file_is_directory: false,
                file_size: size,
            });
        }

        // Step 2: Discover
        let _ = self.discover(target_base_url).await;

        // Step 3: Ask
        let ask_req = AskRequest {
            sender_computer_name: self.computer_name.clone(),
            sender_model_name: self.model_name.clone(),
            sender_id: self.sender_id.clone(),
            bundle_id: Some("com.apple.mobileslideshow".to_string()),
            convert_media_formats: false,
            items_description: Some(format!("{} item(s)", ask_files.len())),
            icon_data: preview_icon_data,
            files: ask_files,
        };

        let ask_res = self.ask(target_base_url, &ask_req).await?;
        if ask_res.status.to_lowercase() != "accepted" {
            info!("Transfer was declined by recipient.");
            return Err(AirDropClientError::Declined);
        }

        // Step 4: Construct CPIO payload only after recipient consent is granted
        let mut entries = Vec::with_capacity(file_paths.len());
        for p in file_paths {
            let path = p.as_ref();
            let file_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file.dat")
                .to_string();

            let data = std::fs::read(path)?;
            entries.push(CpioEntry {
                name: file_name,
                mode: 0o100644,
                mtime: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as u32,
                data,
            });
        }

        let cpio_payload = build_cpio_archive(&entries)?;

        // Step 5: Upload
        self.upload(target_base_url, cpio_payload).await?;

        info!("AirDrop transfer completed successfully!");
        Ok(())
    }
}

