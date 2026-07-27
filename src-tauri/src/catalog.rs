use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use chrono::{SecondsFormat, Utc};
use thiserror::Error;

use crate::{
    models::{ProjectSummary, SessionCatalog, SessionProvider, SessionSummary},
    providers::{self, ProviderScan},
};

#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("无法确定当前 Windows 用户目录")]
    HomeUnavailable,
}

struct ProjectAccumulator {
    path: String,
    providers: BTreeSet<SessionProvider>,
    sessions: Vec<SessionSummary>,
}

pub fn scan_sessions() -> Result<SessionCatalog, CatalogError> {
    let home_dir = dirs::home_dir().ok_or(CatalogError::HomeUnavailable)?;
    let scans = [
        providers::claude::scan_sessions(&home_dir),
        providers::codex::scan_sessions(&home_dir),
    ];

    Ok(build_catalog(scans))
}

fn build_catalog(scans: impl IntoIterator<Item = ProviderScan>) -> SessionCatalog {
    let mut sources = Vec::new();
    let mut warnings = Vec::new();
    let mut sessions = Vec::new();

    for scan in scans {
        sources.push(scan.source);
        warnings.extend(scan.warnings);
        sessions.extend(scan.sessions);
    }

    let projects = group_projects(sessions);
    SessionCatalog {
        projects,
        scanned_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        sources,
        warnings,
    }
}

fn group_projects(sessions: Vec<SessionSummary>) -> Vec<ProjectSummary> {
    let mut projects: BTreeMap<String, ProjectAccumulator> = BTreeMap::new();

    for session in sessions {
        let project_path = session.project_path.clone();
        let key = normalize_project_key(&project_path);
        let project = projects.entry(key).or_insert_with(|| ProjectAccumulator {
            path: project_path,
            providers: BTreeSet::new(),
            sessions: Vec::new(),
        });
        project.providers.insert(session.provider);
        project.sessions.push(session);
    }

    let mut summaries: Vec<ProjectSummary> = projects
        .into_values()
        .map(|mut project| {
            project.sessions.sort_by(|left, right| {
                right
                    .last_activity
                    .cmp(&left.last_activity)
                    .then_with(|| left.title.cmp(&right.title))
                    .then_with(|| left.provider.cmp(&right.provider))
            });
            let last_activity = project
                .sessions
                .first()
                .map(|session| session.last_activity.clone())
                .unwrap_or_default();
            let total_size = project
                .sessions
                .iter()
                .map(|session| session.file_size)
                .sum();

            ProjectSummary {
                id: normalize_project_key(&project.path),
                name: project_name(&project.path),
                path: project.path,
                last_activity,
                session_count: project.sessions.len(),
                total_size,
                providers: project.providers.into_iter().collect(),
                sessions: project.sessions,
            }
        })
        .collect();

    summaries.sort_by(|left, right| {
        right
            .last_activity
            .cmp(&left.last_activity)
            .then_with(|| left.name.cmp(&right.name))
    });
    summaries
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

        let catalog = build_catalog(scans);

        assert_eq!(catalog.projects.len(), 1);
        assert_eq!(catalog.projects[0].session_count, 2);
        assert_eq!(
            catalog.projects[0].providers,
            vec![SessionProvider::Claude, SessionProvider::Codex]
        );
    }
}
