mod launcher;
mod models;
mod sessions;

use models::{CliStatus, LaunchResult, SessionCatalog};

#[tauri::command]
fn scan_session_catalog() -> Result<SessionCatalog, String> {
    sessions::scan_sessions().map_err(|error| error.to_string())
}

#[tauri::command]
fn get_cli_status() -> CliStatus {
    launcher::cli_status()
}

#[tauri::command]
fn resume_session(session_id: String, fork: bool) -> Result<LaunchResult, String> {
    launcher::resume_session(&session_id, fork)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            scan_session_catalog,
            get_cli_status,
            resume_session
        ])
        .run(tauri::generate_context!())
        .expect("error while running Claude Session Manager");
}
