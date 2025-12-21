// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod models;

use commands::{scan_drive, get_scan_history, compare_snapshots, delete_snapshot, get_data_directory, open_data_directory, get_available_drives};

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            scan_drive,
            get_scan_history,
            compare_snapshots,
            delete_snapshot,
            get_data_directory,
            open_data_directory,
            get_available_drives
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
