mod reconcile;
mod state;

use std::sync::Mutex;

use crate::{
    models::{OpenSessionSummary, SessionCatalog},
    terminal::ActiveTerminal,
};

pub use reconcile::CaptureResult;
use reconcile::capture_state;
use state::WorkspaceRepository;

static WORKSPACE_STATE_LOCK: Mutex<()> = Mutex::new(());

pub fn capture(
    catalog: &SessionCatalog,
    active_terminals: &[ActiveTerminal],
) -> Result<CaptureResult, String> {
    let _guard = WORKSPACE_STATE_LOCK
        .lock()
        .map_err(|_| "工作区快照锁已损坏".to_owned())?;
    let repository = WorkspaceRepository::local()?;
    let mut state = repository.load()?;
    let result = capture_state(&mut state, catalog, active_terminals);
    repository.save(&state)?;
    Ok(result)
}

pub fn project_snapshot(project_id: &str) -> Result<Vec<OpenSessionSummary>, String> {
    let _guard = WORKSPACE_STATE_LOCK
        .lock()
        .map_err(|_| "工作区快照锁已损坏".to_owned())?;
    let state = WorkspaceRepository::local()?.load()?;
    Ok(state
        .snapshots
        .get(project_id)
        .map(|snapshot| snapshot.sessions.clone())
        .unwrap_or_default())
}

pub fn managed_window_for_path(project_path: &str) -> Result<Option<String>, String> {
    let _guard = WORKSPACE_STATE_LOCK
        .lock()
        .map_err(|_| "工作区快照锁已损坏".to_owned())?;
    let state = WorkspaceRepository::local()?.load()?;
    Ok(reconcile::managed_window_for_path(&state, project_path))
}

pub fn mark_project_window(
    project_id: &str,
    project_path: &str,
    window_name: &str,
) -> Result<(), String> {
    let _guard = WORKSPACE_STATE_LOCK
        .lock()
        .map_err(|_| "工作区快照锁已损坏".to_owned())?;
    let repository = WorkspaceRepository::local()?;
    let mut state = repository.load()?;
    let snapshot = state.snapshots.entry(project_id.to_owned()).or_default();
    snapshot.project_path = project_path.to_owned();
    snapshot.window_name = Some(window_name.to_owned());
    repository.save(&state)
}
