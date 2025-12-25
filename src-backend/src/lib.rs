use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use walkdir::WalkDir;
use sha2::{Sha256, Digest};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand;
use std::io::{Read, Write};
use bincode;
use serde_json;
use indicatif;
use std::time;

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
    pub scan_duration: u64,
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotSummary {
    pub id: String,
    pub drive_path: String,
    pub timestamp: i64,
    pub total_files: usize,
    pub total_size: u64,
    pub scan_duration: u64,
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
    pub snapshot1: SnapshotSummary,
    pub snapshot2: SnapshotSummary,
    pub diffs: Vec<FileDiff>,
    pub added_count: usize,
    pub deleted_count: usize,
    pub modified_count: usize,
}

pub fn get_data_dir() -> Result<std::path::PathBuf, String> {
    let data_dir = dirs::data_local_dir()
        .ok_or("Could not find local app data directory")?
        .join("com.pifrost.drivepulse");
    Ok(data_dir)
}

pub fn derive_key(password: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

pub fn save_snapshot(snapshot: &Snapshot, encrypt: bool, password: Option<&str>) -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let snapshots_dir = data_dir.join("snapshots");
    fs::create_dir_all(&snapshots_dir).map_err(|e| e.to_string())?;
    let file_ext = if encrypt { "bin" } else { "json" };
    let snapshot_path = snapshots_dir.join(format!("{}.{}", snapshot.id, file_ext));
    let data_to_write = if encrypt {
        let password = password.ok_or("Password required for encryption")?;
        let serialized = bincode::serialize(snapshot).map_err(|e| format!("Failed to serialize: {}", e))?;
        let key = derive_key(password);
        let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| format!("Failed to create cipher: {}", e))?;
        let nonce_bytes: [u8; 12] = rand::random();
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher.encrypt(nonce, serialized.as_ref()).map_err(|e| format!("Encryption failed: {}", e))?;
        let mut encrypted_data = nonce_bytes.to_vec();
        encrypted_data.extend_from_slice(&ciphertext);
        encrypted_data
    } else {
        let serialized = serde_json::to_string_pretty(snapshot).map_err(|e| format!("Failed to serialize: {}", e))?;
        serialized.into_bytes()
    };
    let mut file = fs::File::create(&snapshot_path).map_err(|e| format!("Failed to create file: {}", e))?;
    file.write_all(&data_to_write).map_err(|e| format!("Failed to write file: {}", e))?;
    Ok(())
}

pub fn save_snapshot_metadata(snapshot: &Snapshot) -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let metadata_dir = data_dir.join("metadata");
    fs::create_dir_all(&metadata_dir).map_err(|e| e.to_string())?;
    let summary = SnapshotSummary {
        id: snapshot.id.clone(),
        drive_path: snapshot.drive_path.clone(),
        timestamp: snapshot.timestamp,
        total_files: snapshot.total_files,
        total_size: snapshot.total_size,
        scan_duration: snapshot.scan_duration,
    };
    let metadata_path = metadata_dir.join(format!("{}.json", snapshot.id));
    let json = serde_json::to_string(&summary).map_err(|e| format!("Failed to serialize metadata: {}", e))?;
    fs::write(&metadata_path, json).map_err(|e| format!("Failed to write metadata: {}", e))?;
    Ok(())
}

pub fn load_snapshot(snapshot_id: &str, password: Option<&str>) -> Result<Snapshot, String> {
    match load_snapshot_binary(snapshot_id, password) {
        Ok(snapshot) => Ok(snapshot),
        Err(_) => load_snapshot_json(snapshot_id),
    }
}

fn load_snapshot_binary(snapshot_id: &str, password: Option<&str>) -> Result<Snapshot, String> {
    let data_dir = get_data_dir()?;
    let snapshot_path = data_dir.join("snapshots").join(format!("{}.bin", snapshot_id));
    let mut file = fs::File::open(&snapshot_path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut data = Vec::new();
    file.read_to_end(&mut data).map_err(|e| format!("Failed to read file: {}", e))?;
    if data.len() < 12 {
        return Err("Invalid encrypted file".to_string());
    }
    let nonce_bytes = &data[..12];
    let ciphertext = &data[12..];
    let password = password.ok_or("Password required for decryption")?;
    let key = derive_key(password);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| format!("Failed to create cipher: {}", e))?;
    let nonce = Nonce::from_slice(nonce_bytes);
    let decrypted = cipher.decrypt(nonce, ciphertext).map_err(|e| format!("Decryption failed: {}", e))?;
    let snapshot: Snapshot = bincode::deserialize(&decrypted).map_err(|e| format!("Failed to deserialize: {}", e))?;
    Ok(snapshot)
}

fn load_snapshot_json(snapshot_id: &str) -> Result<Snapshot, String> {
    let data_dir = get_data_dir()?;
    let snapshot_path = data_dir.join("snapshots").join(format!("{}.json", snapshot_id));
    let content = fs::read_to_string(&snapshot_path).map_err(|e| format!("Failed to read file: {}", e))?;
    let snapshot: Snapshot = serde_json::from_str(&content).map_err(|e| format!("Failed to parse: {}", e))?;
    Ok(snapshot)
}

pub fn get_scan_history() -> Result<Vec<SnapshotSummary>, String> {
    let data_dir = get_data_dir()?;
    let metadata_dir = data_dir.join("metadata");
    if metadata_dir.exists() {
        let mut summaries = Vec::new();
        for entry in fs::read_dir(&metadata_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match fs::read_to_string(&path) {
                    Ok(content) => {
                        match serde_json::from_str::<SnapshotSummary>(&content) {
                            Ok(summary) => summaries.push(summary),
                            Err(_) => continue,
                        }
                    }
                    Err(_) => continue,
                }
            }
        }
        summaries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(summaries)
    } else {
        let snapshots_dir = data_dir.join("snapshots");
        let mut summaries = Vec::new();
        if !snapshots_dir.exists() {
            return Ok(summaries);
        }
        for entry in fs::read_dir(&snapshots_dir).map_err(|e| format!("Failed to read data directory: {}", e))? {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path).map_err(|e| format!("Failed to read snapshot file: {}", e))?;
                let snapshot: Snapshot = serde_json::from_str(&content).map_err(|e| format!("Failed to parse snapshot: {}", e))?;
                summaries.push(SnapshotSummary {
                    id: snapshot.id,
                    drive_path: snapshot.drive_path,
                    timestamp: snapshot.timestamp,
                    total_files: snapshot.total_files,
                    total_size: snapshot.total_size,
                    scan_duration: snapshot.scan_duration,
                });
            }
        }
        summaries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(summaries)
    }
}

pub fn scan_drive<F>(drive_path: String, mut progress_callback: F) -> Result<Snapshot, String>
where
    F: FnMut(usize, String),
{
    let scan_start = time::Instant::now();
    let mut files = Vec::new();
    let mut total_size: u64 = 0;
    for entry in WalkDir::new(&drive_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if let Ok(metadata) = entry.metadata() {
            let file_size = metadata.len();
            total_size += file_size;
            let modified = metadata.modified().unwrap_or(time::SystemTime::UNIX_EPOCH).duration_since(time::SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
            files.push(FileEntry {
                path: path.to_string_lossy().to_string(),
                size: file_size,
                modified,
                is_dir: metadata.is_dir(),
            });
            progress_callback(files.len(), path.to_string_lossy().to_string());
        }
    }
    let scan_duration = scan_start.elapsed().as_secs();
    let mut hasher = Sha256::new();
    hasher.update(drive_path.as_bytes());
    hasher.update(scan_start.elapsed().as_nanos().to_string().as_bytes());
    let snapshot_id = format!("{:x}", hasher.finalize())[..16].to_string();
    let snapshot = Snapshot {
        id: snapshot_id,
        drive_path,
        timestamp: time::SystemTime::now().duration_since(time::SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64,
        total_files: files.len(),
        total_size,
        scan_duration,
        files,
    };
    Ok(snapshot)
}

pub fn compare_snapshots(snapshot1: &Snapshot, snapshot2: &Snapshot) -> ComparisonResult {
    let mut map1: HashMap<String, &FileEntry> = HashMap::new();
    for file in &snapshot1.files {
        map1.insert(file.path.clone(), file);
    }
    let mut map2: HashMap<String, &FileEntry> = HashMap::new();
    for file in &snapshot2.files {
        map2.insert(file.path.clone(), file);
    }
    let mut added = Vec::new();
    let mut deleted = Vec::new();
    let mut modified = Vec::new();
    for (path, file2) in &map2 {
        if let Some(file1) = map1.get(path) {
            if file1.size != file2.size || file1.modified != file2.modified {
                modified.push(FileDiff {
                    path: path.clone(),
                    status: DiffStatus::Modified,
                    old_size: Some(file1.size),
                    new_size: Some(file2.size),
                    old_modified: Some(file1.modified),
                    new_modified: Some(file2.modified),
                });
            }
        } else {
            added.push(FileDiff {
                path: path.clone(),
                status: DiffStatus::Added,
                old_size: None,
                new_size: Some(file2.size),
                old_modified: None,
                new_modified: Some(file2.modified),
            });
        }
    }
    for (path, file1) in &map1 {
        if !map2.contains_key(path) {
            deleted.push(FileDiff {
                path: path.clone(),
                status: DiffStatus::Deleted,
                old_size: Some(file1.size),
                new_size: None,
                old_modified: Some(file1.modified),
                new_modified: None,
            });
        }
    }
    let added_count = added.len();
    let deleted_count = deleted.len();
    let modified_count = modified.len();

    ComparisonResult {
        snapshot1: SnapshotSummary {
            id: snapshot1.id.clone(),
            drive_path: snapshot1.drive_path.clone(),
            timestamp: snapshot1.timestamp,
            total_files: snapshot1.total_files,
            total_size: snapshot1.total_size,
            scan_duration: snapshot1.scan_duration,
        },
        snapshot2: SnapshotSummary {
            id: snapshot2.id.clone(),
            drive_path: snapshot2.drive_path.clone(),
            timestamp: snapshot2.timestamp,
            total_files: snapshot2.total_files,
            total_size: snapshot2.total_size,
            scan_duration: snapshot2.scan_duration,
        },
        diffs: added.into_iter().chain(deleted.into_iter()).chain(modified.into_iter()).collect(),
        added_count,
        deleted_count,
        modified_count,
    }
}