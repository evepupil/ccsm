use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionProvider {
    Claude,
    Codex,
}

pub const TEMPORARY_PROJECT_ID: &str = "ccsm://temporary";
pub const TEMPORARY_PROJECT_NAME: &str = "\u{4e34}\u{65f6}\u{4f1a}\u{8bdd}";

impl SessionProvider {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }
}

impl fmt::Display for SessionProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCatalog {
    pub projects: Vec<ProjectSummary>,
    pub scanned_at: String,
    pub sources: Vec<SessionSource>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSource {
    pub provider: SessionProvider,
    pub location: String,
    pub available: bool,
    pub session_count: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub path: String,
    pub is_temporary: bool,
    pub last_activity: String,
    pub session_count: usize,
    pub total_size: u64,
    pub providers: Vec<SessionProvider>,
    pub sessions: Vec<SessionSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub id: String,
    pub provider: SessionProvider,
    pub title: String,
    pub title_source: String,
    pub project_path: String,
    pub source_path: String,
    pub source_detail: Option<String>,
    pub created_at: Option<String>,
    pub last_activity: String,
    pub message_count: Option<u64>,
    pub tokens_used: Option<u64>,
    pub branch: Option<String>,
    pub model: Option<String>,
    pub cli_version: Option<String>,
    pub file_size: u64,
    pub is_archived: bool,
    pub can_resume: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliStatus {
    pub provider: SessionProvider,
    pub available: bool,
    pub version: Option<String>,
    pub logged_in: Option<bool>,
    pub auth_method: Option<String>,
    pub api_provider: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchResult {
    pub session_id: String,
    pub provider: SessionProvider,
    pub terminal: String,
    pub forked: bool,
    pub highest_permissions: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewSessionResult {
    pub provider: SessionProvider,
    pub terminal: String,
    pub working_directory: String,
    pub highest_permissions: bool,
}
