use drive_pulse_lib::{FileEntry, Snapshot, SnapshotSummary, FileDiff, DiffStatus, ComparisonResult};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use tauri::{Window};
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
        drive_pulse_lib::save_snapshot(&snapshot, encrypt, password.as_deref())?;
        
        // Save metadata separately for fast history loading
        drive_pulse_lib::save_snapshot_metadata(&snapshot)?;
        
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
    drive_pulse_lib::get_scan_history()
}

#[tauri::command]
pub fn compare_snapshots(snapshot1_id: String, snapshot2_id: String, password: Option<String>) -> Result<ComparisonResult, String> {
    let snapshot1 = drive_pulse_lib::load_snapshot(&snapshot1_id, password.as_deref())?;
    let snapshot2 = drive_pulse_lib::load_snapshot(&snapshot2_id, password.as_deref())?;
    Ok(drive_pulse_lib::compare_snapshots(&snapshot1, &snapshot2))
}

#[tauri::command]
pub fn get_data_directory() -> Result<String, String> {
    let data_dir = drive_pulse_lib::get_data_dir()?;
    let snapshots_dir = data_dir.join("snapshots");
    Ok(snapshots_dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_data_directory() -> Result<(), String> {
    let data_dir = drive_pulse_lib::get_data_dir()?;
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
    let data_dir = drive_pulse_lib::get_data_dir()?;
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

