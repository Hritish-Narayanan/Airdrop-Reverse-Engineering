use crate::cpio::{extract_cpio_to_directory, parse_cpio_archive};
use crate::protocol::{AskRequest, AskResponse, DiscoverResponse, UploadResponse};
use crate::tls::{create_server_config, generate_airdrop_certificate};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tracing::{error, info, warn};

/// AirDrop receiver server configuration.
pub struct AirDropServerConfig {
    pub bind_addr: SocketAddr,
    pub computer_name: String,
    pub download_dir: PathBuf,
    pub auto_accept: bool,
}

/// Runs the AirDrop HTTPS receiver daemon.
pub async fn run_airdrop_server(
    config: AirDropServerConfig,
    shutdown_flag: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cert = generate_airdrop_certificate(&config.computer_name)?;
    let tls_config = create_server_config(&cert)?;
    let acceptor = TlsAcceptor::from(tls_config);

    let listener = TcpListener::bind(config.bind_addr).await?;
    let local_addr = listener.local_addr()?;
    info!("AirDrop receiver listening on https://{}", local_addr);

    let download_dir = Arc::new(config.download_dir);
    let auto_accept = config.auto_accept;

    while !shutdown_flag.load(Ordering::Relaxed) {
        tokio::select! {
            accept_res = listener.accept() => {
                match accept_res {
                    Ok((stream, peer_addr)) => {
                        let acceptor = acceptor.clone();
                        let dl_dir = download_dir.clone();

                        tokio::spawn(async move {
                            match acceptor.accept(stream).await {
                                Ok(tls_stream) => {
                                    if let Err(e) = handle_connection(tls_stream, dl_dir, auto_accept).await {
                                        warn!("Error handling AirDrop peer {}: {}", peer_addr, e);
                                    }
                                }
                                Err(e) => {
                                    warn!("TLS handshake failed with {}: {}", peer_addr, e);
                                }
                            }
                        });
                    }
                    Err(e) => {
                        error!("Accept error: {}", e);
                    }
                }
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)) => {
                if shutdown_flag.load(Ordering::Relaxed) {
                    break;
                }
            }
        }
    }

    info!("AirDrop receiver stopped.");
    Ok(())
}

async fn handle_connection<S>(
    mut stream: S,
    download_dir: Arc<PathBuf>,
    auto_accept: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let mut buffer = Vec::with_capacity(8192);
    let mut temp = [0u8; 4096];

    // Read HTTP header until \r\n\r\n
    let header_end;
    loop {
        let n = stream.read(&mut temp).await?;
        if n == 0 {
            return Ok(());
        }
        buffer.extend_from_slice(&temp[..n]);

        if let Some(pos) = find_subsequence(&buffer, b"\r\n\r\n") {
            header_end = pos + 4;
            break;
        }
    }

    let header_str = String::from_utf8_lossy(&buffer[..header_end]);
    let first_line = header_str.lines().next().unwrap_or_default();
    let parts: Vec<&str> = first_line.split_whitespace().collect();

    if parts.len() < 2 {
        return Ok(());
    }

    let method = parts[0];
    let path = parts[1];

    // Extract Content-Length
    let content_length = extract_content_length(&header_str).unwrap_or(0);

    // Read remaining body bytes
    let mut body = buffer[header_end..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut temp).await?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&temp[..n]);
    }

    info!("AirDrop Request: {} {}", method, path);

    match path {
        "/Discover" => {
            let resp = DiscoverResponse {
                status: "success".to_string(),
                device_support_flags: Some(1),
                protocol_version: Some("1.0".to_string()),
            };
            send_json_response(&mut stream, 200, &resp).await?;
        }
        "/Ask" => {
            let ask_req: Result<AskRequest, _> = serde_json::from_slice(&body);
            let accepted = match ask_req {
                Ok(req) => {
                    info!("Incoming AirDrop from '{}' with {} files", req.sender_computer_name, req.files.len());
                    for f in &req.files {
                        info!("  File: {} ({} bytes)", f.file_name, f.file_size);
                    }
                    auto_accept
                }
                Err(e) => {
                    warn!("Failed to parse /Ask body: {}", e);
                    true
                }
            };

            let resp = AskResponse {
                status: if accepted { "Accepted" } else { "Declined" }.to_string(),
                next_attempt_delay_ms: None,
            };
            send_json_response(&mut stream, 200, &resp).await?;
        }
        "/Upload" => {
            info!("Received /Upload payload of {} bytes", body.len());
            match parse_cpio_archive(&body) {
                Ok(entries) => {
                    info!("Unpacking {} CPIO entries into {:?}", entries.len(), download_dir);
                    match extract_cpio_to_directory(&entries, &download_dir) {
                        Ok(saved) => {
                            for s in saved {
                                info!("Saved AirDrop file: {:?}", s);
                            }
                            let resp = UploadResponse {
                                status: "Success".to_string(),
                            };
                            send_json_response(&mut stream, 200, &resp).await?;
                        }
                        Err(e) => {
                            error!("Failed to write files to directory: {}", e);
                            send_error_response(&mut stream, 500, "Failed to save files").await?;
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to parse CPIO archive: {}", e);
                    // If it's a single raw file uploaded directly
                    let file_path = download_dir.join("airdrop_received_file.dat");
                    if let Ok(_) = std::fs::write(&file_path, &body) {
                        info!("Saved raw payload to {:?}", file_path);
                        let resp = UploadResponse {
                            status: "Success".to_string(),
                        };
                        send_json_response(&mut stream, 200, &resp).await?;
                    } else {
                        send_error_response(&mut stream, 400, "Invalid archive format").await?;
                    }
                }
            }
        }
        _ => {
            send_error_response(&mut stream, 404, "Not Found").await?;
        }
    }

    stream.flush().await?;
    Ok(())
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

fn extract_content_length(headers: &str) -> Option<usize> {
    for line in headers.lines() {
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                return v.trim().parse::<usize>().ok();
            }
        }
    }
    None
}

async fn send_json_response<S, T>(stream: &mut S, status: u16, data: &T) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: tokio::io::AsyncWrite + Unpin,
    T: serde::Serialize,
{
    let body = serde_json::to_vec(data)?;
    let header = format!(
        "HTTP/1.1 {} OK\r\nServer: AirDrop/1.0\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status,
        body.len()
    );
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(&body).await?;
    Ok(())
}

async fn send_error_response<S>(stream: &mut S, status: u16, msg: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: tokio::io::AsyncWrite + Unpin,
{
    let header = format!(
        "HTTP/1.1 {} Error\r\nServer: AirDrop/1.0\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        msg.len(),
        msg
    );
    stream.write_all(header.as_bytes()).await?;
    Ok(())
}
