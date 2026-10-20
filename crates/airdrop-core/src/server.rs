use crate::cpio::{extract_cpio_to_directory, parse_archive_auto};
use crate::protocol::{AskRequest, AskResponse, DiscoverResponse, UploadResponse};
use crate::tls::{create_server_config, generate_airdrop_certificate};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_rustls::TlsAcceptor;
use tracing::{error, info, warn};

/// Details of an incoming AirDrop transfer requiring user approval.
#[derive(Debug, Clone)]
pub struct ConsentDetails {
    pub transfer_id: String,
    pub sender_name: String,
    pub sender_model: String,
    pub file_count: usize,
    pub total_bytes: u64,
    pub icon_data: Option<String>,
}

static PENDING_CONSENT: Mutex<Option<HashMap<String, oneshot::Sender<bool>>>> = Mutex::new(None);
static CONSENT_LISTENER: Mutex<Option<Box<dyn Fn(ConsentDetails) + Send + Sync>>> = Mutex::new(None);

/// Registers a listener invoked whenever an incoming transfer requires user approval.
pub fn set_consent_listener<F>(listener: F)
where
    F: Fn(ConsentDetails) + Send + Sync + 'static,
{
    let mut guard = CONSENT_LISTENER.lock().unwrap();
    *guard = Some(Box::new(listener));
}

/// Resolves an in-flight consent request by transfer ID.
pub fn respond_to_consent(transfer_id: &str, accept: bool) -> bool {
    let mut guard = PENDING_CONSENT.lock().unwrap();
    if let Some(map) = guard.as_mut() {
        if let Some(tx) = map.remove(transfer_id) {
            let _ = tx.send(accept);
            return true;
        }
    }
    false
}

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
            _ = tokio::time::sleep(Duration::from_millis(250)) => {
                if shutdown_flag.load(Ordering::Relaxed) {
                    break;
                }
            }
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

    // Handle Expect: 100-continue immediately if present
    if has_expect_100_continue(&header_str) {
        stream.write_all(b"HTTP/1.1 100 Continue\r\n\r\n").await?;
        stream.flush().await?;
    }

    let is_chunked_transfer = is_chunked(&header_str);
    let mut body = buffer[header_end..].to_vec();

    if is_chunked_transfer {
        body = read_chunked_body(&mut stream, body).await?;
    } else {
        // Extract Content-Length
        let content_length = extract_content_length(&header_str).unwrap_or(0);

        // Read remaining body bytes
        while body.len() < content_length {
            let n = stream.read(&mut temp).await?;
            if n == 0 {
                break;
            }
            body.extend_from_slice(&temp[..n]);
        }
    }

    info!("AirDrop Request: {} {} (body: {} bytes)", method, path, body.len());

    match path {
        "/Discover" => {
            let resp = DiscoverResponse {
                status: "success".to_string(),
                device_support_flags: Some(1),
                protocol_version: Some("1.0".to_string()),
            };
            send_json_response(&mut stream, 200, &resp).await?;
        }
        "/Exchange" => {
            // Apple AirDrop /Exchange capability handshake
            let resp = serde_json::json!({
                "status": "success",
                "supportedProtocols": ["AirDrop/1.0", "AirDrop/2.0"],
                "supportedPayloadFormats": ["cpio", "dvzip"],
                "capabilities": {
                    "supportsCompression": true
                }
            });
            send_json_response(&mut stream, 200, &resp).await?;
        }
        "/Ask" => {
            let ask_req: Result<AskRequest, _> = parse_airdrop_payload(&body);
            let accepted = match ask_req {
                Ok(req) => {
                    info!("Incoming AirDrop from '{}' with {} files", req.sender_computer_name, req.files.len());
                    for f in &req.files {
                        info!("  File: {} ({} bytes)", f.file_name, f.file_size);
                    }
                    if auto_accept {
                        true
                    } else {
                        let transfer_id = uuid::Uuid::new_v4().to_string();
                        let total_bytes = req.files.iter().map(|f| f.file_size).sum();
                        let (tx, rx) = oneshot::channel();
                        {
                            let mut guard = PENDING_CONSENT.lock().unwrap();
                            let map = guard.get_or_insert_with(HashMap::new);
                            map.insert(transfer_id.clone(), tx);
                        }

                        // Trigger consent listener if registered
                        {
                            let listener_guard = CONSENT_LISTENER.lock().unwrap();
                            if let Some(listener) = listener_guard.as_ref() {
                                listener(ConsentDetails {
                                    transfer_id: transfer_id.clone(),
                                    sender_name: req.sender_computer_name.clone(),
                                    sender_model: req.sender_model_name.clone(),
                                    file_count: req.files.len(),
                                    total_bytes,
                                    icon_data: req.icon_data.clone(),
                                });
                            }
                        }

                        // Wait up to 30s for user response
                        match tokio::time::timeout(Duration::from_secs(30), rx).await {
                            Ok(Ok(user_decision)) => user_decision,
                            _ => {
                                warn!("Consent prompt timed out or cancelled for transfer {}", transfer_id);
                                false
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to parse /Ask body: {}", e);
                    false
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
            match parse_archive_auto(&body) {
                Ok(entries) => {
                    info!("Unpacking {} CPIO/DVZip entries into {:?}", entries.len(), download_dir);
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

fn has_expect_100_continue(headers: &str) -> bool {
    for line in headers.lines() {
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case("expect") && v.to_lowercase().contains("100-continue") {
                return true;
            }
        }
    }
    false
}

fn is_chunked(headers: &str) -> bool {
    for line in headers.lines() {
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case("transfer-encoding") && v.to_lowercase().contains("chunked") {
                return true;
            }
        }
    }
    false
}

/// Decodes an HTTP/1.1 chunked stream from bytes into the unchunked payload.
pub fn decode_chunked_stream(input: &[u8]) -> Result<(Vec<u8>, usize), std::io::Error> {
    let mut decoded = Vec::new();
    let mut cursor = 0;

    loop {
        if cursor >= input.len() {
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Incomplete chunked body"));
        }

        let rest = &input[cursor..];
        let crlf_pos = match rest.windows(2).position(|w| w == b"\r\n") {
            Some(p) => p,
            None => return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Missing CRLF after chunk size")),
        };

        let size_str = String::from_utf8_lossy(&rest[..crlf_pos]);
        let size_hex = size_str.split(';').next().unwrap_or("").trim();
        let chunk_size = usize::from_str_radix(size_hex, 16)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Invalid chunk size '{}': {}", size_hex, e)))?;

        cursor += crlf_pos + 2;

        if chunk_size == 0 {
            // Final chunk
            if cursor + 2 <= input.len() && &input[cursor..cursor + 2] == b"\r\n" {
                cursor += 2;
            }
            break;
        }

        if cursor + chunk_size > input.len() {
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Incomplete chunk data"));
        }

        decoded.extend_from_slice(&input[cursor..cursor + chunk_size]);
        cursor += chunk_size;

        if cursor + 2 <= input.len() && &input[cursor..cursor + 2] == b"\r\n" {
            cursor += 2;
        }
    }

    Ok((decoded, cursor))
}

async fn read_chunked_body<S>(stream: &mut S, mut initial_buffer: Vec<u8>) -> Result<Vec<u8>, std::io::Error>
where
    S: tokio::io::AsyncRead + Unpin,
{
    let mut temp = [0u8; 8192];
    loop {
        if let Ok((decoded, _)) = decode_chunked_stream(&initial_buffer) {
            return Ok(decoded);
        }
        let n = stream.read(&mut temp).await?;
        if n == 0 {
            return decode_chunked_stream(&initial_buffer).map(|(d, _)| d);
        }
        initial_buffer.extend_from_slice(&temp[..n]);
    }
}

/// Parses an incoming payload whether it is encoded as a Binary Plist (bplist00), XML Plist, or JSON.
pub fn parse_airdrop_payload<T: serde::de::DeserializeOwned>(body: &[u8]) -> Result<T, serde_json::Error> {
    if body.is_empty() {
        return serde_json::from_str("{}");
    }

    // 1. Check for Apple binary property list magic "bplist00"
    if body.starts_with(b"bplist00") {
        match plist::from_bytes(body) {
            Ok(val) => return Ok(val),
            Err(e) => {
                warn!("bplist parsing failed: {}", e);
            }
        }
    }

    // 2. Check for XML plist "<?xml" or "<!DOCTYPE plist"
    if body.starts_with(b"<?xml") || body.starts_with(b"<!DOCTYPE plist") {
        match plist::from_bytes(body) {
            Ok(val) => return Ok(val),
            Err(e) => {
                warn!("xml plist parsing failed: {}", e);
            }
        }
    }

    // 3. Fallback to standard JSON
    serde_json::from_slice(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::AskRequest;

    #[test]
    fn test_chunked_decoding() {
        let chunked_data = b"5\r\nHello\r\n6\r\n World\r\n0\r\n\r\n";
        let (decoded, bytes_consumed) = decode_chunked_stream(chunked_data).expect("Failed to decode");
        assert_eq!(decoded, b"Hello World");
        assert_eq!(bytes_consumed, chunked_data.len());
    }

    #[test]
    fn test_expect_100_continue_detection() {
        let headers = "POST /Upload HTTP/1.1\r\nHost: 127.0.0.1\r\nExpect: 100-continue\r\nContent-Length: 100\r\n";
        assert!(has_expect_100_continue(headers));

        let headers_normal = "POST /Upload HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 100\r\n";
        assert!(!has_expect_100_continue(headers_normal));
    }

    #[test]
    fn test_is_chunked_detection() {
        let headers = "POST /Upload HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n";
        assert!(is_chunked(headers));

        let headers_identity = "POST /Upload HTTP/1.1\r\nContent-Length: 50\r\n\r\n";
        assert!(!is_chunked(headers_identity));
    }

    #[test]
    fn test_parse_airdrop_payload_json() {
        let json_body = br#"{"SenderComputerName":"MacBook","SenderModelName":"MacBookAir","SenderID":"1234","Files":[]}"#;
        let req: AskRequest = parse_airdrop_payload(json_body).expect("Failed to parse JSON AskRequest");
        assert_eq!(req.sender_computer_name, "MacBook");
        assert_eq!(req.sender_model_name, "MacBookAir");
    }

    #[test]
    fn test_parse_airdrop_payload_binary_plist() {
        // Create a plist dictionary and serialize to binary plist
        let mut dict = plist::Dictionary::new();
        dict.insert("SenderComputerName".to_string(), plist::Value::String("iPhone 15".to_string()));
        dict.insert("SenderModelName".to_string(), plist::Value::String("iPhone15,2".to_string()));
        dict.insert("SenderID".to_string(), plist::Value::String("uuid-test".to_string()));
        dict.insert("Files".to_string(), plist::Value::Array(vec![]));

        let mut bplist_bytes = Vec::new();
        plist::to_writer_binary(&mut bplist_bytes, &plist::Value::Dictionary(dict)).expect("to binary plist");
        assert!(bplist_bytes.starts_with(b"bplist00"));

        let req: AskRequest = parse_airdrop_payload(&bplist_bytes).expect("Failed to parse bplist AskRequest");
        assert_eq!(req.sender_computer_name, "iPhone 15");
        assert_eq!(req.sender_model_name, "iPhone15,2");
        assert_eq!(req.sender_id, "uuid-test");
    }

    #[test]
    fn test_parse_airdrop_payload_xml_plist() {
        let xml_plist = br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>SenderComputerName</key>
    <string>iPad Pro</string>
    <key>SenderModelName</key>
    <string>iPad13,8</string>
    <key>SenderID</key>
    <string>ipad-uuid</string>
    <key>Files</key>
    <array/>
</dict>
</plist>"#;

        let req: AskRequest = parse_airdrop_payload(xml_plist).expect("Failed to parse XML plist AskRequest");
        assert_eq!(req.sender_computer_name, "iPad Pro");
        assert_eq!(req.sender_model_name, "iPad13,8");
    }
}
