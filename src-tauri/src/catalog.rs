use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use chrono::{SecondsFormat, Utc};
use thiserror::Error;

use crate::{
    models::{
        ProjectSummary, SessionCatalog, SessionProvider, SessionSummary, TEMPORARY_PROJECT_ID,
        TEMPORARY_PROJECT_NAME,
    },
    providers::{self, ProviderScan},
};

#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("无法确定当前 Windows 用户目录")]
    HomeUnavailable,
}

struct ProjectAccumulator {
    path: String,
    sessions: Vec<SessionSummary>,
}

pub fn scan_sessions() -> Result<SessionCatalog, CatalogError> {
    let home_dir = dirs::home_dir().ok_or(CatalogError::HomeUnavailable)?;
    let scans = [
        providers::claude::scan_sessions(&home_dir),
        providers::codex::scan_sessions(&home_dir),
    ];
    let temporary_root = temporary_sessions_root(&home_dir);

    Ok(build_catalog(scans, &temporary_root))
}

pub fn temporary_sessions_root(home_dir: &Path) -> PathBuf {
    dirs::document_dir()
        .unwrap_or_else(|| home_dir.join("Documents"))
        .join("CCSM")
}

fn build_catalog(
    scans: impl IntoIterator<Item = ProviderScan>,
    temporary_root: &Path,
) -> SessionCatalog {
    let mut sources = Vec::new();
    let mut warnings = Vec::new();
    let mut sessions = Vec::new();

    for scan in scans {
        sources.push(scan.source);
        warnings.extend(scan.warnings);
        sessions.extend(scan.sessions);
    }

    let projects = group_projects(sessions, temporary_root);
    SessionCatalog {
        projects,
        scanned_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        sources,
        warnings,
    }
}

fn group_projects(sessions: Vec<SessionSummary>, temporary_root: &Path) -> Vec<ProjectSummary> {
    let mut projects: BTreeMap<String, ProjectAccumulator> = BTreeMap::new();
    let mut temporary_sessions = Vec::new();

    for session in sessions {
        let project_path = session.project_path.clone();
        if is_temporary_path(&project_path, temporary_root) {
            temporary_sessions.push(session);
            continue;
        }

        let key = normalize_project_key(&project_path);
        let project = projects.entry(key).or_insert_with(|| ProjectAccumulator {
            path: project_path,
            sessions: Vec::new(),
        });
        project.sessions.push(session);
    }

    let mut summaries: Vec<ProjectSummary> = projects
        .into_values()
        .map(|project| {
            summarize_project(
                normalize_project_key(&project.path),
                project_name(&project.path),
                project.path,
                project.sessions,
                false,
            )
        })
        .collect();

    summaries.push(summarize_project(
        TEMPORARY_PROJECT_ID.to_owned(),
        TEMPORARY_PROJECT_NAME.to_owned(),
        temporary_root.display().to_string(),
        temporary_sessions,
        true,
    ));

    summaries.sort_by(|left, right| {
        right
            .last_activity
            .cmp(&left.last_activity)
            .then_with(|| left.name.cmp(&right.name))
    });
    summaries
}

fn summarize_project(
    id: String,
    name: String,
    path: String,
    mut sessions: Vec<SessionSummary>,
    is_temporary: bool,
) -> ProjectSummary {
    sessions.sort_by(|left, right| {
        right
            .last_activity
            .cmp(&left.last_activity)
            .then_with(|| left.title.cmp(&right.title))
            .then_with(|| left.provider.cmp(&right.provider))
    });
    let last_activity = sessions
        .first()
        .map(|session| session.last_activity.clone())
        .unwrap_or_default();
    let total_size = sessions.iter().map(|session| session.file_size).sum();
    let providers = if is_temporary {
        vec![SessionProvider::Claude, SessionProvider::Codex]
    } else {
        sessions
            .iter()
            .map(|session| session.provider)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    };

    ProjectSummary {
        id,
        name,
        path,
        is_temporary,
        last_activity,
        session_count: sessions.len(),
        total_size,
        providers,
        sessions,
    }
}

fn is_temporary_path(path: &str, temporary_root: &Path) -> bool {
    let path = normalize_project_key(path);
    let root = normalize_project_key(&temporary_root.display().to_string());
    path == root || path.starts_with(&(root + "\\"))
}

fn normalize_project_key(path: &str) -> String {
    path.trim()
        .trim_end_matches(['\\', '/'])
        .replace('/', "\\")
        .to_lowercase()
}

fn project_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or(path)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: &str, provider: SessionProvider, project_path: &str) -> SessionSummary {
        SessionSummary {
            id: id.to_owned(),
            provider,
            title: format!("{provider} session"),
            title_source: "fixture".to_owned(),
            project_path: project_path.to_owned(),
            source_path: format!("{id}.jsonl"),
            source_detail: None,
            created_at: Some("2026-07-27T05:00:00Z".to_owned()),
            last_activity: "2026-07-27T06:00:00Z".to_owned(),
            message_count: None,
            tokens_used: None,
            branch: None,
            model: None,
            cli_version: None,
            file_size: 100,
            is_archived: false,
            can_resume: true,
        }
    }

    #[test]
    fn merges_provider_sessions_by_normalized_project_path() {
        let scans = [
            ProviderScan::available(
                SessionProvider::Claude,
                "claude".to_owned(),
                vec![session(
                    "claude-id",
                    SessionProvider::Claude,
                    "C:\\code\\CCSM",
                )],
                Vec::new(),
            ),
            ProviderScan::available(
                SessionProvider::Codex,
                "codex".to_owned(),
                vec![session("codex-id", SessionProvider::Codex, "c:/code/ccsm/")],
                Vec::new(),
            ),
        ];

        let catalog = build_catalog(scans, Path::new("C:\\Users\\tester\\Documents\\CCSM"));

        assert_eq!(catalog.projects.len(), 2);
        let project = catalog
            .projects
            .iter()
            .find(|project| project.id == "c:\\code\\ccsm")
            .expect("merged project");
        assert_eq!(project.session_count, 2);
        assert_eq!(
            project.providers,
            vec![SessionProvider::Claude, SessionProvider::Codex]
        );
    }

    #[test]
    fn keeps_temporary_sessions_in_one_virtual_project() {
        let temporary_root = Path::new("C:\\Users\\tester\\Documents\\CCSM");
        let scans = [ProviderScan::available(
            SessionProvider::Claude,
            "claude".to_owned(),
            vec![session(
                "temporary-id",
                SessionProvider::Claude,
                "C:\\Users\\tester\\Documents\\CCSM\\2026-08-18\\new-chat",
            )],
            Vec::new(),
        )];

        let projects = group_projects(
            scans.into_iter().flat_map(|scan| scan.sessions).collect(),
            temporary_root,
        );

        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].id, TEMPORARY_PROJECT_ID);
        assert_eq!(projects[0].name, TEMPORARY_PROJECT_NAME);
        assert!(projects[0].is_temporary);
        assert_eq!(projects[0].session_count, 1);
        assert_eq!(
            projects[0].sessions[0].project_path,
            "C:\\Users\\tester\\Documents\\CCSM\\2026-08-18\\new-chat"
        );
    }

    #[test]
    fn exposes_empty_temporary_project_for_new_sessions() {
        let projects = group_projects(Vec::new(), Path::new("C:\\Users\\tester\\Documents\\CCSM"));

        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].id, TEMPORARY_PROJECT_ID);
        assert!(projects[0].is_temporary);
        assert_eq!(
            projects[0].providers,
            vec![SessionProvider::Claude, SessionProvider::Codex]
        );
        assert!(projects[0].sessions.is_empty());
    }
}
