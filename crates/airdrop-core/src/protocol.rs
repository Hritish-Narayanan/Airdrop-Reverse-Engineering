use serde::{Deserialize, Serialize};

/// Payload sent in POST /Discover
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverRequest {
    #[serde(rename = "senderRecordData", skip_serializing_if = "Option::is_none")]
    pub sender_record_data: Option<String>,
    #[serde(rename = "receiverRecordData", skip_serializing_if = "Option::is_none")]
    pub receiver_record_data: Option<String>,
}

/// Response returned from POST /Discover
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverResponse {
    pub status: String,
    #[serde(rename = "deviceSupportFlags", skip_serializing_if = "Option::is_none")]
    pub device_support_flags: Option<u64>,
    #[serde(rename = "protocolVersion", skip_serializing_if = "Option::is_none")]
    pub protocol_version: Option<String>,
}

/// Description of a single file in POST /Ask
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskFile {
    #[serde(rename = "FileName")]
    pub file_name: String,
    #[serde(rename = "FileType")]
    pub file_type: String,
    #[serde(rename = "FileBomPath")]
    pub file_bom_path: String,
    #[serde(rename = "FileIsDirectory")]
    pub file_is_directory: bool,
    #[serde(rename = "FileSize")]
    pub file_size: u64,
}

/// Payload sent in POST /Ask requesting permission to send files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskRequest {
    #[serde(rename = "SenderComputerName")]
    pub sender_computer_name: String,
    #[serde(rename = "SenderModelName")]
    pub sender_model_name: String,
    #[serde(rename = "SenderID")]
    pub sender_id: String,
    #[serde(rename = "BundleID", skip_serializing_if = "Option::is_none")]
    pub bundle_id: Option<String>,
    #[serde(rename = "ConvertMediaFormats", default)]
    pub convert_media_formats: bool,
    #[serde(rename = "Files")]
    pub files: Vec<AskFile>,
    #[serde(rename = "itemsDescription", skip_serializing_if = "Option::is_none")]
    pub items_description: Option<String>,
}

/// Response returned from POST /Ask
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskResponse {
    pub status: String, // "Accepted" or "Declined"
    #[serde(rename = "nextAttemptDelayMS", skip_serializing_if = "Option::is_none")]
    pub next_attempt_delay_ms: Option<u64>,
}

/// Response returned from POST /Upload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResponse {
    pub status: String,
}

/// Infers the Apple UTI (Uniform Type Identifier) from a file extension.
pub fn infer_uti(filename: &str) -> &'static str {
    let lower = filename.to_lowercase();
    if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "public.jpeg"
    } else if lower.ends_with(".png") {
        "public.png"
    } else if lower.ends_with(".heic") {
        "public.heic"
    } else if lower.ends_with(".pdf") {
        "com.adobe.pdf"
    } else if lower.ends_with(".txt") {
        "public.plain-text"
    } else if lower.ends_with(".mp4") {
        "public.mpeg-4"
    } else if lower.ends_with(".mov") {
        "com.apple.quicktime-movie"
    } else if lower.ends_with(".mp3") {
        "public.mp3"
    } else if lower.ends_with(".zip") {
        "public.zip-archive"
    } else {
        "public.data"
    }
}
