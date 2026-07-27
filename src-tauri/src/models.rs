use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCatalog {
    pub projects: Vec<ProjectSummary>,
    pub scanned_at: String,
    pub sessions_root: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub path: String,
    pub encoded_directory: String,
    pub last_activity: String,
    pub session_count: usize,
    pub total_size: u64,
    pub sessions: Vec<SessionSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub id: String,
    pub title: String,
    pub title_source: String,
    pub project_path: String,
    pub file_path: String,
    pub created_at: Option<String>,
    pub last_activity: String,
    pub message_count: u32,
    pub branch: Option<String>,
    pub model: Option<String>,
    pub claude_version: Option<String>,
    pub file_size: u64,
    pub is_archived: bool,
    pub can_resume: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliStatus {
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
    pub terminal: String,
    pub forked: bool,
}
