mod catalog;
mod launcher;
mod models;
mod providers;

use models::{CliStatus, LaunchResult, SessionCatalog, SessionProvider};

#[tauri::command]
fn scan_session_catalog() -> Result<SessionCatalog, String> {
    catalog::scan_sessions().map_err(|error| error.to_string())
}

#[tauri::command]
fn get_cli_statuses() -> Vec<CliStatus> {
    launcher::cli_statuses()
}

#[tauri::command]
fn resume_session(
    provider: SessionProvider,
    session_id: String,
    fork: bool,
    highest_permissions: bool,
) -> Result<LaunchResult, String> {
    launcher::resume_session(provider, &session_id, fork, highest_permissions)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            scan_session_catalog,
            get_cli_statuses,
            resume_session
        ])
        .run(tauri::generate_context!())
        .expect("error while running CCSM");
}
