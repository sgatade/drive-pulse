use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub modified: i64,
    pub is_dir: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Snapshot {
    pub id: String,
    pub drive_path: String,
    pub timestamp: i64,
    pub total_files: usize,
    pub total_size: u64,
    pub scan_duration: u64, // Duration in seconds
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotSummary {
    pub id: String,
    pub drive_path: String,
    pub timestamp: i64,
    pub total_files: usize,
    pub total_size: u64,
    pub scan_duration: u64, // Duration in seconds
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub status: DiffStatus,
    pub old_size: Option<u64>,
    pub new_size: Option<u64>,
    pub old_modified: Option<i64>,
    pub new_modified: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffStatus {
    Added,
    Deleted,
    Modified,
    Unchanged,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub snapshot1_id: String,
    pub snapshot2_id: String,
    pub added: Vec<FileDiff>,
    pub deleted: Vec<FileDiff>,
    pub modified: Vec<FileDiff>,
    pub unchanged_count: usize,
}
