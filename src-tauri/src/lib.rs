mod catalog;
mod launcher;
mod message_previews;
mod models;
mod providers;

use std::path::Path;

use models::{CliStatus, LaunchResult, NewSessionResult, SessionCatalog, SessionProvider};

#[tauri::command]
fn scan_session_catalog() -> Result<SessionCatalog, String> {
    catalog::scan_sessions().map_err(|error| error.to_string())
}

#[tauri::command]
fn get_cli_statuses() -> Vec<CliStatus> {
    launcher::cli_statuses()
}

#[tauri::command]
fn get_user_message_previews(
    provider: SessionProvider,
    source_path: String,
    limit: usize,
) -> Result<Vec<String>, String> {
    message_previews::read_user_message_previews(provider, Path::new(&source_path), limit)
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

#[tauri::command]
fn start_new_session(
    provider: SessionProvider,
    project_id: String,
    highest_permissions: bool,
) -> Result<NewSessionResult, String> {
    launcher::start_new_session(provider, &project_id, highest_permissions)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            scan_session_catalog,
            get_cli_statuses,
            get_user_message_previews,
            resume_session,
            start_new_session
        ])
        .run(tauri::generate_context!())
        .expect("error while running CCSM");
}
