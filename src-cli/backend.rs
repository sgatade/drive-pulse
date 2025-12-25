use drive_pulse_lib::{FileEntry, Snapshot, SnapshotSummary, FileDiff, ComparisonResult};
use std::path::PathBuf;
use std::fs;

/// Get the data directory for storing snapshots
pub fn get_data_directory() -> Result<PathBuf, String> {
    let data_dir = drive_pulse_lib::get_data_dir()?;
    let snapshots_dir = data_dir.join("snapshots");
    
    if !snapshots_dir.exists() {
        fs::create_dir_all(&snapshots_dir)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;
    }
    
    Ok(snapshots_dir)
}

/// Scan a drive and create a snapshot
pub fn scan_drive(drive_path: String) -> Result<Snapshot, String> {
    let pb = indicatif::ProgressBar::new_spinner();
    pb.set_style(indicatif::ProgressStyle::default_spinner().template("{spinner:.cyan} [{elapsed_precise}] {pos} files | {wide_msg}").unwrap());
    let snapshot = drive_pulse_lib::scan_drive(drive_path, |count, path| {
        pb.set_position(count as u64);
        pb.set_message(path);
    })?;
    pb.finish_with_message("Scan complete");
    drive_pulse_lib::save_snapshot(&snapshot, false, None)?;
    drive_pulse_lib::save_snapshot_metadata(&snapshot)?;
    Ok(snapshot)
}

/// Get all saved snapshots
pub fn get_scan_history() -> Result<Vec<SnapshotSummary>, String> {
    drive_pulse_lib::get_scan_history()
}

/// Load a specific snapshot by ID
pub fn load_snapshot(snapshot_id: &str) -> Result<Snapshot, String> {
    drive_pulse_lib::load_snapshot(snapshot_id, None)
}

/// Compare two snapshots
pub fn compare_snapshots(snapshot1_id: &str, snapshot2_id: &str) -> Result<ComparisonResult, String> {
    let snapshot1 = load_snapshot(snapshot1_id)?;
    let snapshot2 = load_snapshot(snapshot2_id)?;
    Ok(drive_pulse_lib::compare_snapshots(&snapshot1, &snapshot2))
}
