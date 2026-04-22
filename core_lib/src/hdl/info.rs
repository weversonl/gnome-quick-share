use std::fs::File;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::TextPayloadType;
use crate::utils::RemoteDeviceInfo;

#[derive(Debug)]
pub struct InternalFileInfo {
    pub payload_id: i64,
    pub file_url: PathBuf,
    pub bytes_transferred: i64,
    pub total_size: i64,
    pub file: Option<File>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TransferRiskLevel {
    #[default]
    None,
    Extension,
    High,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TransferMetadata {
    pub id: String,
    pub source: Option<RemoteDeviceInfo>,
    pub pin_code: Option<String>,

    pub destination: Option<String>,
    pub files: Option<Vec<String>>,

    pub text_type: Option<TextPayloadType>,
    pub text_description: Option<String>,
    pub text_payload: Option<String>,
    pub contains_dangerous_files: bool,
    pub risk_level: TransferRiskLevel,
    pub detected_content_label: Option<String>,
    pub detected_content_description: Option<String>,
    pub suspicious_file_name: Option<String>,

    pub total_bytes: u64,
    pub ack_bytes: u64,
}
