use crate::models::{FileEntry, Snapshot, SnapshotSummary, FileDiff, DiffStatus, ComparisonResult};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use tauri::{api::path::app_data_dir, Window};
use walkdir::WalkDir;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use sha2::{Sha256, Digest};

#[derive(Clone, serde::Serialize)]
pub struct DriveInfo {
    pub path: String,
    pub label: String,
}

#[derive(Clone, serde::Serialize)]
struct ScanProgress {
    files_scanned: usize,
    current_path: String,
    total_size: u64,
}

#[tauri::command]
pub fn get_available_drives() -> Result<Vec<DriveInfo>, String> {
    let mut drives = Vec::new();
    
    #[cfg(target_os = "windows")]
    {
        // On Windows, check drives A-Z
        for letter in b'A'..=b'Z' {
            let drive_path = format!("{}:\\", letter as char);
            if Path::new(&drive_path).exists() {
                let label = format!("{}: Drive", letter as char);
                drives.push(DriveInfo {
                    path: drive_path,
                    label,
                });
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        // On macOS, list volumes
        let volumes_path = Path::new("/Volumes");
        if volumes_path.exists() {
            if let Ok(entries) = fs::read_dir(volumes_path) {
                for entry in entries.flatten() {
                    if let Ok(name) = entry.file_name().into_string() {
                        let full_path = format!("/Volumes/{}", name);
                        drives.push(DriveInfo {
                            path: full_path.clone(),
                            label: name,
                        });
                    }
                }
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        // On Linux, list common mount points
        drives.push(DriveInfo {
            path: "/".to_string(),
            label: "Root (/)".to_string(),
        });
        
        let media_path = Path::new("/media");
        if media_path.exists() {
            if let Ok(entries) = fs::read_dir(media_path) {
                for entry in entries.flatten() {
                    if let Ok(name) = entry.file_name().into_string() {
                        let full_path = format!("/media/{}", name);
                        drives.push(DriveInfo {
                            path: full_path.clone(),
                            label: format!("Media: {}", name),
                        });
                    }
                }
            }
        }
        
        let mnt_path = Path::new("/mnt");
        if mnt_path.exists() {
            if let Ok(entries) = fs::read_dir(mnt_path) {
                for entry in entries.flatten() {
                    if let Ok(name) = entry.file_name().into_string() {
                        let full_path = format!("/mnt/{}", name);
                        drives.push(DriveInfo {
                            path: full_path.clone(),
                            label: format!("Mount: {}", name),
                        });
                    }
                }
            }
        }
    }
    
    Ok(drives)
}

#[tauri::command]
pub async fn scan_drive(drive_path: String, encrypt: bool, password: Option<String>, window: Window) -> Result<Snapshot, String> {
    // Validate encryption parameters
    if encrypt && password.is_none() {
        return Err("Password required for encryption".to_string());
    }
    
    // Run the blocking scan operation in a separate thread
    let drive_path_clone = drive_path.clone();
    let window_clone = window.clone();
    
    tokio::task::spawn_blocking(move || {
        println!("[RUST] Starting scan of: {}", drive_path_clone);
        let scan_start = std::time::Instant::now();
        let mut files = Vec::new();
        let mut total_size: u64 = 0;
        let mut progress_counter = 0;

        // Walk through the directory
        for entry in WalkDir::new(&drive_path_clone)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            let size = metadata.len();
            let modified = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            let file_entry = FileEntry {
                path: path.to_string_lossy().to_string(),
                size,
                modified,
                is_dir: metadata.is_dir(),
            };

            if !metadata.is_dir() {
                total_size += size;
            }

            files.push(file_entry);
            
            // Emit progress every 100 files to avoid overwhelming the frontend
            progress_counter += 1;
            if progress_counter % 100 == 0 {
                let _ = window_clone.emit("scan-progress", ScanProgress {
                    files_scanned: files.len(),
                    current_path: path.to_string_lossy().to_string(),
                    total_size,
                });
            }
        }
        
        println!("[RUST] Scan completed! Files: {}, Size: {}", files.len(), total_size);
        let scan_duration = scan_start.elapsed().as_secs();

        let timestamp = chrono::Utc::now().timestamp();
        let id = format!("{}_{}", timestamp, drive_path_clone.replace([':', '\\', '/'], "_"));

        let snapshot = Snapshot {
            id: id.clone(),
            drive_path: drive_path_clone.clone(),
            timestamp,
            total_files: files.len(),
            total_size,
            scan_duration,
            files,
        };

        println!("[RUST] Saving snapshot to disk...");
        // Save snapshot to disk with optional encryption
        save_snapshot(&snapshot, encrypt, password.as_deref())?;
        
        // Save metadata separately for fast history loading
        save_snapshot_metadata(&snapshot)?;
        
        println!("[RUST] Snapshot saved successfully!");

        // Return a lightweight summary instead of full snapshot to avoid IPC overflow
        let summary = Snapshot {
            id: snapshot.id,
            drive_path: snapshot.drive_path,
            timestamp: snapshot.timestamp,
            total_files: snapshot.total_files,
            total_size: snapshot.total_size,
            scan_duration: snapshot.scan_duration,
            files: Vec::new(), // Don't send millions of file entries over IPC
        };
        
        println!("[RUST] Returning summary to frontend");
        Ok(summary)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub fn get_scan_history() -> Result<Vec<SnapshotSummary>, String> {
    let data_dir = get_data_dir()?;
    let metadata_dir = data_dir.join("metadata");

    if !metadata_dir.exists() {
        return Ok(Vec::new());
    }

    let mut summaries = Vec::new();

    // Read from fast metadata files instead of full snapshots
    for entry in fs::read_dir(&metadata_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    match serde_json::from_str::<SnapshotSummary>(&content) {
                        Ok(summary) => summaries.push(summary),
                        Err(_) => continue, // Skip invalid metadata files
                    }
                }
                Err(_) => continue, // Skip unreadable files
            }
        }
    }

    // Sort by timestamp descending (newest first)
    summaries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(summaries)
}

#[tauri::command]
pub fn compare_snapshots(snapshot1_id: String, snapshot2_id: String, password: Option<String>) -> Result<ComparisonResult, String> {
    let snapshot1 = load_snapshot(&snapshot1_id, password.as_deref())?;
    let snapshot2 = load_snapshot(&snapshot2_id, password.as_deref())?;

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
    let mut unchanged_count = 0;

    // Find added and modified files
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
            } else {
                unchanged_count += 1;
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

    // Find deleted files
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

    Ok(ComparisonResult {
        snapshot1_id,
        snapshot2_id,
        added,
        deleted,
        modified,
        unchanged_count,
    })
}

#[tauri::command]
pub fn get_data_directory() -> Result<String, String> {
    let data_dir = get_data_dir()?;
    let snapshots_dir = data_dir.join("snapshots");
    Ok(snapshots_dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_data_directory() -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let snapshots_dir = data_dir.join("snapshots");
    
    // Create the directory if it doesn't exist
    fs::create_dir_all(&snapshots_dir).map_err(|e| e.to_string())?;
    
    // Open the folder in the default file explorer
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&snapshots_dir)
            .spawn()
            .map_err(|e| format!("Failed to open explorer: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&snapshots_dir)
            .spawn()
            .map_err(|e| format!("Failed to open finder: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&snapshots_dir)
            .spawn()
            .map_err(|e| format!("Failed to open file manager: {}", e))?;
    }
    
    Ok(())
}

#[tauri::command]
pub fn delete_snapshot(snapshot_id: String) -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let snapshots_dir = data_dir.join("snapshots");
    let metadata_dir = data_dir.join("metadata");
    
    // Try both .json and .bin extensions for snapshot
    let json_path = snapshots_dir.join(format!("{}.json", snapshot_id));
    let bin_path = snapshots_dir.join(format!("{}.bin", snapshot_id));
    
    // Delete metadata file
    let metadata_path = metadata_dir.join(format!("{}.json", snapshot_id));

    if json_path.exists() {
        fs::remove_file(json_path).map_err(|e| e.to_string())?;
    } else if bin_path.exists() {
        fs::remove_file(bin_path).map_err(|e| e.to_string())?;
    }
    
    // Also remove metadata file if it exists
    if metadata_path.exists() {
        fs::remove_file(metadata_path).map_err(|e| e.to_string())?;
    }

    Ok(())
}

// Helper functions

fn get_data_dir() -> Result<std::path::PathBuf, String> {
    let config = tauri::Config::default();
    let data_dir = app_data_dir(&config).ok_or("Failed to get app data directory")?;
    Ok(data_dir)
}

// Derive encryption key from password using SHA-256
fn derive_key(password: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

fn save_snapshot(snapshot: &Snapshot, encrypt: bool, password: Option<&str>) -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let snapshots_dir = data_dir.join("snapshots");

    fs::create_dir_all(&snapshots_dir).map_err(|e| e.to_string())?;

    let snapshot_path = snapshots_dir.join(format!("{}.bin", snapshot.id));
    
    // Serialize using bincode
    let serialized = bincode::serialize(snapshot)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    
    let data_to_write = if encrypt {
        let password = password.ok_or("Password required for encryption")?;
        
        // Derive key from password
        let key = derive_key(password);
        
        // Create cipher
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| format!("Failed to create cipher: {}", e))?;
        
        // Generate random nonce (12 bytes for AES-GCM)
        let nonce_bytes: [u8; 12] = rand::random();
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Encrypt
        let ciphertext = cipher.encrypt(nonce, serialized.as_ref())
            .map_err(|e| format!("Encryption failed: {}", e))?;
        
        // Prepend nonce to ciphertext (we need it for decryption)
        let mut encrypted_data = nonce_bytes.to_vec();
        encrypted_data.extend_from_slice(&ciphertext);
        encrypted_data
    } else {
        serialized
    };
    
    // Write to file
    let mut file = fs::File::create(&snapshot_path)
        .map_err(|e| format!("Failed to create file: {}", e))?;
    file.write_all(&data_to_write)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
}

fn save_snapshot_metadata(snapshot: &Snapshot) -> Result<(), String> {
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
    let json = serde_json::to_string(&summary)
        .map_err(|e| format!("Failed to serialize metadata: {}", e))?;
    
    fs::write(&metadata_path, json)
        .map_err(|e| format!("Failed to write metadata: {}", e))?;
    
    Ok(())
}

fn load_snapshot(snapshot_id: &str, password: Option<&str>) -> Result<Snapshot, String> {
    // Try .bin first, then .json for backward compatibility
    match load_snapshot_binary(snapshot_id, password) {
        Ok(snapshot) => Ok(snapshot),
        Err(_) => load_snapshot_json(snapshot_id),
    }
}

fn load_snapshot_binary(snapshot_id: &str, password: Option<&str>) -> Result<Snapshot, String> {
    let data_dir = get_data_dir()?;
    let snapshot_path = data_dir.join("snapshots").join(format!("{}.bin", snapshot_id));

    let mut file = fs::File::open(&snapshot_path)
        .map_err(|e| format!("Failed to open file: {}", e))?;
    
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    // Try to decrypt if password provided, otherwise try direct deserialization
    let decrypted_data = if let Some(pwd) = password {
        // Extract nonce (first 12 bytes)
        if data.len() < 12 {
            return Err("Invalid encrypted data".to_string());
        }
        
        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        // Derive key from password
        let key = derive_key(pwd);
        
        // Create cipher
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| format!("Failed to create cipher: {}", e))?;
        
        // Decrypt
        cipher.decrypt(nonce, ciphertext)
            .map_err(|e| format!("Decryption failed (wrong password?): {}", e))?
    } else {
        // Try unencrypted first
        match bincode::deserialize::<Snapshot>(&data) {
            Ok(_) => data, // It's unencrypted
            Err(_) => return Err("File appears to be encrypted, password required".to_string()),
        }
    };
    
    // Deserialize using bincode
    bincode::deserialize(&decrypted_data)
        .map_err(|e| format!("Failed to deserialize: {}", e))
}

fn load_snapshot_json(snapshot_id: &str) -> Result<Snapshot, String> {
    let data_dir = get_data_dir()?;
    let snapshot_path = data_dir.join("snapshots").join(format!("{}.json", snapshot_id));

    let content = fs::read_to_string(snapshot_path).map_err(|e| e.to_string())?;
    let snapshot: Snapshot = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    Ok(snapshot)
}