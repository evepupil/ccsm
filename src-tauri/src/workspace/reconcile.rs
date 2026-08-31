use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, SecondsFormat, Utc};

use crate::{
    models::{OpenSessionSummary, SessionCatalog, SessionProvider, SessionSummary},
    terminal::{self, ActiveTerminal},
};

use super::state::{WorkspaceSnapshot, WorkspaceState};

const SESSION_START_MATCH_WINDOW_MS: i64 = 15 * 60 * 1_000;
const SESSION_START_CLOCK_SKEW_MS: i64 = 30 * 1_000;

#[derive(Debug, Clone)]
pub struct CaptureResult {
    pub saved_sessions: Vec<OpenSessionSummary>,
    pub live_sessions: Vec<OpenSessionSummary>,
    pub unresolved_terminals: Vec<ActiveTerminal>,
}

pub fn capture_state(
    state: &mut WorkspaceState,
    catalog: &SessionCatalog,
    active_terminals: &[ActiveTerminal],
) -> CaptureResult {
    let session_map = catalog
        .projects
        .iter()
        .flat_map(|project| project.sessions.iter())
        .map(|session| (session_key(session.provider, &session.id), session))
        .collect::<BTreeMap<_, _>>();
    let active_process_keys = active_terminals
        .iter()
        .filter_map(process_key)
        .collect::<BTreeSet<_>>();
    state.process_bindings.retain(|process, session| {
        active_process_keys.contains(process) && session_map.contains_key(session)
    });

    let mut assigned_sessions = BTreeSet::new();
    let mut unresolved = Vec::new();
    for terminal in active_terminals {
        let (Some(provider), Some(process)) = (terminal.provider, process_key(terminal)) else {
            continue;
        };
        let explicit_session = terminal
            .session_id
            .as_deref()
            .map(|session_id| session_key(provider, session_id))
            .filter(|session| session_map.contains_key(session));
        let bound_session = state
            .process_bindings
            .get(&process)
            .filter(|session| session_map.contains_key(*session))
            .cloned();
        if let Some(session) = explicit_session.or(bound_session) {
            state.process_bindings.insert(process, session.clone());
            assigned_sessions.insert(session);
        } else {
            unresolved.push(terminal.clone());
        }
    }

    bind_new_sessions(state, &session_map, &mut assigned_sessions, &mut unresolved);

    let mut live_sessions = Vec::new();
    let mut still_unresolved = Vec::new();
    for terminal in active_terminals {
        let Some(process) = process_key(terminal) else {
            continue;
        };
        let Some(session_key) = state.process_bindings.get(&process) else {
            if terminal.provider.is_some() {
                still_unresolved.push(terminal.clone());
            }
            continue;
        };
        let Some(session) = session_map.get(session_key) else {
            continue;
        };
        live_sessions.push(open_session(session, terminal));
    }
    sort_and_deduplicate(&mut live_sessions);

    update_snapshots(
        state,
        catalog,
        active_terminals,
        &live_sessions,
        &still_unresolved,
    );
    state.known_sessions = session_map.keys().cloned().collect();

    CaptureResult {
        saved_sessions: all_saved_sessions(state),
        live_sessions,
        unresolved_terminals: still_unresolved,
    }
}

pub fn all_saved_sessions(state: &WorkspaceState) -> Vec<OpenSessionSummary> {
    let mut sessions = state
        .snapshots
        .values()
        .flat_map(|snapshot| snapshot.sessions.iter().cloned())
        .collect::<Vec<_>>();
    sort_and_deduplicate(&mut sessions);
    sessions
}

pub fn managed_window_for_path(state: &WorkspaceState, project_path: &str) -> Option<String> {
    state
        .snapshots
        .values()
        .filter_map(|snapshot| {
            let window_name = snapshot.window_name.as_ref()?;
            terminal::is_path_within(&snapshot.project_path, project_path).then_some((
                terminal::normalize_path(&snapshot.project_path).len(),
                window_name,
            ))
        })
        .max_by_key(|(path_length, _)| *path_length)
        .map(|(_, window_name)| window_name.clone())
}

fn bind_new_sessions(
    state: &mut WorkspaceState,
    session_map: &BTreeMap<String, &SessionSummary>,
    assigned_sessions: &mut BTreeSet<String>,
    unresolved: &mut Vec<ActiveTerminal>,
) {
    let baseline_is_empty = state.known_sessions.is_empty();
    let mut terminal_groups = BTreeMap::<(SessionProvider, String), Vec<ActiveTerminal>>::new();
    for terminal in unresolved.iter() {
        let (Some(provider), Some(working_directory), Some(_)) = (
            terminal.provider,
            terminal.working_directory.as_deref(),
            terminal.started_at_ms,
        ) else {
            continue;
        };
        terminal_groups
            .entry((provider, terminal::normalize_path(working_directory)))
            .or_default()
            .push(terminal.clone());
    }

    for ((provider, working_directory), mut terminals) in terminal_groups {
        let mut candidates = session_map
            .iter()
            .filter(|(key, session)| {
                session.provider == provider
                    && terminal::normalize_path(&session.project_path) == working_directory
                    && !assigned_sessions.contains(*key)
                    && (baseline_is_empty || !state.known_sessions.contains(*key))
                    && terminals
                        .iter()
                        .any(|terminal| session_matches_process_start(session, terminal))
            })
            .map(|(key, session)| (key.clone(), *session))
            .collect::<Vec<_>>();
        if candidates.is_empty() || candidates.len() != terminals.len() {
            continue;
        }

        terminals.sort_by_key(|terminal| terminal.started_at_ms.unwrap_or(i64::MAX));
        candidates.sort_by_key(|(_, session)| session_created_at_ms(session).unwrap_or(i64::MAX));
        for (terminal, (session_key, _)) in terminals.into_iter().zip(candidates) {
            if let Some(process) = process_key(&terminal) {
                state.process_bindings.insert(process, session_key.clone());
                assigned_sessions.insert(session_key);
            }
        }
    }

    unresolved.retain(|terminal| {
        process_key(terminal).is_none_or(|process| !state.process_bindings.contains_key(&process))
    });
}

fn session_matches_process_start(session: &SessionSummary, terminal: &ActiveTerminal) -> bool {
    let (Some(created_at), Some(started_at)) =
        (session_created_at_ms(session), terminal.started_at_ms)
    else {
        return false;
    };
    created_at >= started_at - SESSION_START_CLOCK_SKEW_MS
        && created_at <= started_at + SESSION_START_MATCH_WINDOW_MS
}

fn update_snapshots(
    state: &mut WorkspaceState,
    catalog: &SessionCatalog,
    active_terminals: &[ActiveTerminal],
    live_sessions: &[OpenSessionSummary],
    unresolved: &[ActiveTerminal],
) {
    let captured_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    for project in catalog
        .projects
        .iter()
        .filter(|project| !project.is_temporary)
    {
        let live_for_project = live_sessions
            .iter()
            .filter(|session| terminal::is_path_within(&project.path, &session.project_path))
            .cloned()
            .collect::<Vec<_>>();
        let active_count = active_terminals
            .iter()
            .filter(|terminal| {
                terminal.provider.is_some()
                    && terminal_in_project(terminal, &project.path, live_sessions)
            })
            .count();
        if active_count == 0 {
            if let Some(snapshot) = state.snapshots.get_mut(&project.id) {
                snapshot.pending_sessions.clear();
                snapshot.pending_observations = 0;
            }
            continue;
        }
        let unresolved_count = unresolved
            .iter()
            .filter(|terminal| terminal_in_project(terminal, &project.path, live_sessions))
            .count();
        let snapshot = state
            .snapshots
            .entry(project.id.clone())
            .or_insert_with(|| WorkspaceSnapshot {
                project_path: project.path.clone(),
                ..Default::default()
            });
        snapshot.project_path = project.path.clone();
        snapshot.captured_at = captured_at.clone();
        if unresolved_count == 0 {
            merge_additions(&mut snapshot.sessions, &live_for_project);
            if snapshot.sessions == live_for_project {
                snapshot.pending_sessions.clear();
                snapshot.pending_observations = 0;
            } else if snapshot.pending_sessions == live_for_project {
                snapshot.pending_observations = snapshot.pending_observations.saturating_add(1);
                if snapshot.pending_observations >= 2 {
                    snapshot.sessions = live_for_project;
                    snapshot.pending_sessions.clear();
                    snapshot.pending_observations = 0;
                }
            } else {
                snapshot.pending_sessions = live_for_project;
                snapshot.pending_observations = 1;
            }
        } else {
            merge_additions(&mut snapshot.sessions, &live_for_project);
            snapshot.pending_sessions.clear();
            snapshot.pending_observations = 0;
        }
    }
}

fn merge_additions(
    snapshot_sessions: &mut Vec<OpenSessionSummary>,
    live_sessions: &[OpenSessionSummary],
) {
    snapshot_sessions.extend(live_sessions.iter().cloned());
    sort_and_deduplicate(snapshot_sessions);
}

fn terminal_in_project(
    terminal: &ActiveTerminal,
    project_path: &str,
    live_sessions: &[OpenSessionSummary],
) -> bool {
    terminal
        .working_directory
        .as_deref()
        .is_some_and(|path| terminal::is_path_within(project_path, path))
        || terminal.provider.is_some_and(|provider| {
            live_sessions.iter().any(|session| {
                session.provider == provider
                    && session.session_id == terminal.session_id.clone().unwrap_or_default()
                    && terminal::is_path_within(project_path, &session.project_path)
            })
        })
}

fn process_key(terminal: &ActiveTerminal) -> Option<String> {
    let provider = terminal.provider?;
    Some(format!(
        "{provider}:{}:{}",
        terminal.process_id,
        terminal.started_at_ms.unwrap_or_default()
    ))
}

fn session_key(provider: SessionProvider, session_id: &str) -> String {
    format!("{provider}:{session_id}")
}

fn session_created_at_ms(session: &SessionSummary) -> Option<i64> {
    session
        .created_at
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.timestamp_millis())
}

fn open_session(session: &SessionSummary, terminal: &ActiveTerminal) -> OpenSessionSummary {
    OpenSessionSummary {
        session_id: session.id.clone(),
        provider: session.provider,
        project_path: session.project_path.clone(),
        working_directory: terminal
            .working_directory
            .clone()
            .unwrap_or_else(|| session.project_path.clone()),
    }
}

fn sort_and_deduplicate(sessions: &mut Vec<OpenSessionSummary>) {
    sessions.sort_by(|left, right| {
        left.project_path
            .cmp(&right.project_path)
            .then_with(|| left.provider.cmp(&right.provider))
            .then_with(|| left.session_id.cmp(&right.session_id))
    });
    sessions.dedup_by(|left, right| {
        left.provider == right.provider && left.session_id == right.session_id
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{ProjectSummary, SessionCatalog},
        terminal::TerminalHost,
    };

    fn session(id: &str, project_path: &str, created_at: &str) -> SessionSummary {
        SessionSummary {
            id: id.to_owned(),
            provider: SessionProvider::Claude,
            title: id.to_owned(),
            title_source: "fixture".to_owned(),
            project_path: project_path.to_owned(),
            source_path: format!("{id}.jsonl"),
            source_detail: None,
            created_at: Some(created_at.to_owned()),
            last_activity: created_at.to_owned(),
            message_count: None,
            tokens_used: None,
            branch: None,
            model: None,
            cli_version: None,
            file_size: 1,
            is_archived: false,
            can_resume: true,
        }
    }

    fn catalog(sessions: Vec<SessionSummary>) -> SessionCatalog {
        SessionCatalog {
            projects: vec![ProjectSummary {
                id: "c:\\code\\demo".to_owned(),
                name: "demo".to_owned(),
                path: "C:\\code\\demo".to_owned(),
                is_temporary: false,
                last_activity: "2026-08-31T04:00:00Z".to_owned(),
                session_count: sessions.len(),
                total_size: sessions.len() as u64,
                providers: vec![SessionProvider::Claude],
                sessions,
            }],
            open_sessions: Vec::new(),
            scanned_at: "2026-08-31T04:00:00Z".to_owned(),
            sources: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn terminal(id: Option<&str>, started_at_ms: i64) -> ActiveTerminal {
        ActiveTerminal {
            process_id: 42,
            started_at_ms: Some(started_at_ms),
            provider: Some(SessionProvider::Claude),
            session_id: id.map(str::to_owned),
            working_directory: Some("C:\\code\\demo".to_owned()),
            host: TerminalHost::WindowsTerminal,
        }
    }

    #[test]
    fn keeps_the_last_non_empty_snapshot_after_terminals_close() {
        let session_id = "00000000-0000-4000-8000-000000000001";
        let catalog = catalog(vec![session(
            session_id,
            "C:\\code\\demo",
            "2026-08-31T04:00:00Z",
        )]);
        let mut state = WorkspaceState::default();

        capture_state(&mut state, &catalog, &[terminal(Some(session_id), 1)]);
        capture_state(&mut state, &catalog, &[]);

        assert_eq!(state.snapshots["c:\\code\\demo"].sessions.len(), 1);
    }

    #[test]
    fn refuses_to_guess_when_multiple_history_sessions_match_one_process() {
        let catalog = catalog(vec![
            session(
                "00000000-0000-4000-8000-000000000001",
                "C:\\code\\demo",
                "2026-08-31T04:00:00Z",
            ),
            session(
                "00000000-0000-4000-8000-000000000002",
                "C:\\code\\demo",
                "2026-08-31T04:01:00Z",
            ),
        ]);
        let mut state = WorkspaceState::default();
        let result = capture_state(&mut state, &catalog, &[terminal(None, 1_788_148_800_000)]);

        assert!(result.live_sessions.is_empty());
        assert_eq!(result.unresolved_terminals.len(), 1);
    }

    #[test]
    fn binds_one_new_session_to_one_new_process() {
        let catalog = catalog(vec![session(
            "00000000-0000-4000-8000-000000000001",
            "C:\\code\\demo",
            "2026-08-31T04:00:30Z",
        )]);
        let mut state = WorkspaceState::default();
        let result = capture_state(&mut state, &catalog, &[terminal(None, 1_788_148_800_000)]);

        assert_eq!(result.live_sessions.len(), 1);
        assert!(result.unresolved_terminals.is_empty());
    }

    #[test]
    fn replaces_a_snapshot_when_all_running_sessions_are_identified() {
        let first_id = "00000000-0000-4000-8000-000000000001";
        let second_id = "00000000-0000-4000-8000-000000000002";
        let catalog = catalog(vec![
            session(first_id, "C:\\code\\demo", "2026-08-31T04:00:00Z"),
            session(second_id, "C:\\code\\demo", "2026-08-31T04:01:00Z"),
        ]);
        let mut state = WorkspaceState::default();
        let mut first_terminal = terminal(Some(first_id), 1);
        first_terminal.process_id = 1;
        let mut second_terminal = terminal(Some(second_id), 2);
        second_terminal.process_id = 2;

        capture_state(
            &mut state,
            &catalog,
            &[first_terminal.clone(), second_terminal],
        );
        capture_state(&mut state, &catalog, &[first_terminal.clone()]);
        capture_state(&mut state, &catalog, &[first_terminal]);

        assert_eq!(
            state.snapshots["c:\\code\\demo"].sessions,
            vec![OpenSessionSummary {
                session_id: first_id.to_owned(),
                provider: SessionProvider::Claude,
                project_path: "C:\\code\\demo".to_owned(),
                working_directory: "C:\\code\\demo".to_owned(),
            }]
        );
    }

    #[test]
    fn keeps_a_complete_snapshot_when_a_partial_scan_is_followed_by_empty() {
        let first_id = "00000000-0000-4000-8000-000000000001";
        let second_id = "00000000-0000-4000-8000-000000000002";
        let catalog = catalog(vec![
            session(first_id, "C:\\code\\demo", "2026-08-31T04:00:00Z"),
            session(second_id, "C:\\code\\demo", "2026-08-31T04:01:00Z"),
        ]);
        let mut state = WorkspaceState::default();
        let mut first_terminal = terminal(Some(first_id), 1);
        first_terminal.process_id = 1;
        let mut second_terminal = terminal(Some(second_id), 2);
        second_terminal.process_id = 2;

        capture_state(
            &mut state,
            &catalog,
            &[first_terminal.clone(), second_terminal],
        );
        capture_state(&mut state, &catalog, &[first_terminal]);
        capture_state(&mut state, &catalog, &[]);

        assert_eq!(state.snapshots["c:\\code\\demo"].sessions.len(), 2);
    }
}
