use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppMode {
    Real,
    Fake,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartitionInfo {
    pub path: String,
    pub mountpoint: Option<String>,
    pub fstype: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskDevice {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    pub size_human: String,
    pub model: Option<String>,
    pub serial: Option<String>,
    pub removable: bool,
    pub transport: Option<String>,
    pub mounted_partitions: Vec<PartitionInfo>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloneConfig {
    pub source_path: String,
    pub target_path: String,
    pub total_bytes: u64,
    pub block_size: String,
    pub verify_after_clone: bool,
    pub command_preview: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunPhase {
    Clone,
    Verify,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarningSeverity {
    Info,
    Warning,
    Danger,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarningItem {
    pub severity: WarningSeverity,
    pub code: &'static str,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloneProgress {
    pub phase: RunPhase,
    pub bytes_copied: u64,
    pub total_bytes: u64,
    pub percent: f32,
    pub speed_bytes_per_sec: Option<u64>,
    pub elapsed_secs: u64,
    pub eta_secs: Option<u64>,
    pub raw_output: Vec<String>,
}

impl CloneProgress {
    pub fn new(phase: RunPhase, total_bytes: u64) -> Self {
        Self {
            phase,
            bytes_copied: 0,
            total_bytes,
            percent: 0.0,
            speed_bytes_per_sec: None,
            elapsed_secs: 0,
            eta_secs: None,
            raw_output: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloneResult {
    pub phase: RunPhase,
    pub success: bool,
    pub bytes_copied: u64,
    pub elapsed_secs: u64,
    pub average_speed: Option<u64>,
    pub final_message: String,
    pub verify_requested: bool,
    pub verify_completed: bool,
    pub raw_output: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeStatus {
    pub mode: AppMode,
    pub is_root: bool,
    pub has_lsblk: bool,
    pub has_dd: bool,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartCloneResponse {
    pub run_id: String,
    pub command_preview: String,
    pub total_bytes: u64,
    pub verify_after_clone: bool,
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];

    if bytes == 0 {
        return "0 B".to_owned();
    }

    let mut value = bytes as f64;
    let mut idx = 0;
    while value >= 1024.0 && idx < UNITS.len() - 1 {
        value /= 1024.0;
        idx += 1;
    }

    if idx == 0 {
        format!("{bytes} {}", UNITS[idx])
    } else {
        format!("{value:.1} {}", UNITS[idx])
    }
}
