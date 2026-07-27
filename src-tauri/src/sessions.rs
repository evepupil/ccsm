use std::{
    collections::{BTreeMap, HashMap},
    env,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use chrono::{DateTime, SecondsFormat, Utc};
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use crate::models::{ProjectSummary, SessionCatalog, SessionSummary};

const MAX_TITLE_CHARS: usize = 96;

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("无法确定当前 Windows 用户目录")]
    HomeUnavailable,
    #[error("Claude 会话目录不存在：{0}")]
    SessionsMissing(String),
    #[error("无法读取 Claude 会话目录 {path}：{source}")]
    SessionsUnreadable {
        path: String,
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopSession {
    cli_session_id: String,
    cwd: Option<String>,
    created_at: Option<i64>,
    last_activity_at: Option<i64>,
    model: Option<String>,
    title: Option<String>,
    title_source: Option<String>,
    #[serde(default)]
    is_archived: bool,
}

#[derive(Debug, Default)]
struct ParsedSession {
    cwd: Option<String>,
    created_at: Option<DateTime<Utc>>,
    last_activity: Option<DateTime<Utc>>,
    message_count: u32,
    branch: Option<String>,
    model: Option<String>,
    claude_version: Option<String>,
    summary: Option<String>,
    first_user_message: Option<String>,
}

struct ProjectAccumulator {
    path: String,
    encoded_directory: String,
    sessions: Vec<SessionSummary>,
}

pub fn scan_sessions() -> Result<SessionCatalog, ScanError> {
    let sessions_root = dirs::home_dir()
        .ok_or(ScanError::HomeUnavailable)?
        .join(".claude")
        .join("projects");

    scan_sessions_at(&sessions_root)
}

fn scan_sessions_at(sessions_root: &Path) -> Result<SessionCatalog, ScanError> {
    if !sessions_root.is_dir() {
        return Err(ScanError::SessionsMissing(
            sessions_root.display().to_string(),
        ));
    }

    let mut warnings = Vec::new();
    let desktop_sessions = load_desktop_sessions(&mut warnings);
    let project_directories =
        fs::read_dir(sessions_root).map_err(|source| ScanError::SessionsUnreadable {
            path: sessions_root.display().to_string(),
            source,
        })?;

    let mut projects: BTreeMap<String, ProjectAccumulator> = BTreeMap::new();
    let mut skipped_files = 0_u32;

    for project_entry in project_directories.flatten() {
        let project_dir = project_entry.path();
        if !project_dir.is_dir() {
            continue;
        }

        let encoded_directory = project_entry.file_name().to_string_lossy().into_owned();
        let Ok(files) = fs::read_dir(&project_dir) else {
            warnings.push(format!("无法读取项目索引目录：{}", project_dir.display()));
            continue;
        };

        for entry in files.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
                continue;
            }

            let Some(session_id) = path.file_stem().and_then(|value| value.to_str()) else {
                continue;
            };
            if Uuid::parse_str(session_id).is_err() {
                continue;
            }

            let desktop = desktop_sessions.get(session_id);
            match parse_session_file(&path, session_id, desktop) {
                Ok(session) => {
                    let project_path = session.project_path.clone();
                    let key = project_path.to_lowercase();
                    projects
                        .entry(key)
                        .or_insert_with(|| ProjectAccumulator {
                            path: project_path,
                            encoded_directory: encoded_directory.clone(),
                            sessions: Vec::new(),
                        })
                        .sessions
                        .push(session);
                }
                Err(_) => skipped_files += 1,
            }
        }
    }

    if skipped_files > 0 {
        warnings.push(format!("有 {skipped_files} 个会话文件无法解析，已跳过"));
    }

    let mut project_summaries: Vec<ProjectSummary> = projects
        .into_values()
        .map(|mut project| {
            project.sessions.sort_by(|left, right| {
                right
                    .last_activity
                    .cmp(&left.last_activity)
                    .then_with(|| left.title.cmp(&right.title))
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
            let name = project_name(&project.path);

            ProjectSummary {
                id: project.path.to_lowercase(),
                name,
                path: project.path,
                encoded_directory: project.encoded_directory,
                last_activity,
                session_count: project.sessions.len(),
                total_size,
                sessions: project.sessions,
            }
        })
        .collect();

    project_summaries.sort_by(|left, right| {
        right
            .last_activity
            .cmp(&left.last_activity)
            .then_with(|| left.name.cmp(&right.name))
    });

    Ok(SessionCatalog {
        projects: project_summaries,
        scanned_at: format_timestamp(Utc::now()),
        sessions_root: sessions_root.display().to_string(),
        warnings,
    })
}

fn parse_session_file(
    path: &Path,
    session_id: &str,
    desktop: Option<&DesktopSession>,
) -> Result<SessionSummary, std::io::Error> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    let mut parsed = ParsedSession::default();

    for line in BufReader::new(file).lines() {
        let Ok(line) = line else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        absorb_record(&mut parsed, &value);
    }

    let fallback_time: DateTime<Utc> = metadata
        .modified()
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(|_| Utc::now());
    let project_path = desktop
        .and_then(|record| record.cwd.as_deref())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .or(parsed.cwd.clone())
        .unwrap_or_else(|| "未知项目".to_owned());

    let (title, title_source) = choose_title(desktop, &parsed);
    let created_at = desktop
        .and_then(|record| record.created_at)
        .and_then(DateTime::<Utc>::from_timestamp_millis)
        .or(parsed.created_at)
        .map(format_timestamp);
    let last_activity = desktop
        .and_then(|record| record.last_activity_at)
        .and_then(DateTime::<Utc>::from_timestamp_millis)
        .or(parsed.last_activity)
        .unwrap_or(fallback_time);
    let can_resume = Path::new(&project_path).is_dir();

    Ok(SessionSummary {
        id: session_id.to_owned(),
        title,
        title_source,
        project_path,
        file_path: path.display().to_string(),
        created_at,
        last_activity: format_timestamp(last_activity),
        message_count: parsed.message_count,
        branch: parsed.branch,
        model: desktop
            .and_then(|record| record.model.clone())
            .or(parsed.model),
        claude_version: parsed.claude_version,
        file_size: metadata.len(),
        is_archived: desktop.is_some_and(|record| record.is_archived),
        can_resume,
    })
}

fn absorb_record(parsed: &mut ParsedSession, value: &Value) {
    if let Some(timestamp) = value
        .get("timestamp")
        .and_then(Value::as_str)
        .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
        .map(|timestamp| timestamp.with_timezone(&Utc))
    {
        let is_earliest = parsed
            .created_at
            .as_ref()
            .is_none_or(|current| &timestamp < current);
        let is_latest = parsed
            .last_activity
            .as_ref()
            .is_none_or(|current| &timestamp > current);
        if is_earliest {
            parsed.created_at = Some(timestamp);
        }
        if is_latest {
            parsed.last_activity = Some(timestamp);
        }
    }

    set_first_string(&mut parsed.cwd, value.get("cwd"));
    set_first_string(&mut parsed.branch, value.get("gitBranch"));
    set_first_string(&mut parsed.claude_version, value.get("version"));

    match value.get("type").and_then(Value::as_str) {
        Some("summary") => {
            if let Some(summary) = value.get("summary").and_then(Value::as_str) {
                parsed.summary = clean_title(summary);
            }
        }
        Some("user") => {
            if value.get("isMeta").and_then(Value::as_bool) == Some(true) {
                return;
            }
            parsed.message_count += 1;
            if parsed.first_user_message.is_none() {
                parsed.first_user_message = value
                    .get("message")
                    .and_then(|message| message.get("content"))
                    .and_then(extract_text)
                    .and_then(|text| clean_title(&text));
            }
        }
        Some("assistant") => {
            parsed.message_count += 1;
            if parsed.model.is_none() {
                parsed.model = value
                    .get("message")
                    .and_then(|message| message.get("model"))
                    .and_then(Value::as_str)
                    .map(str::to_owned);
            }
        }
        _ => {}
    }
}

fn choose_title(desktop: Option<&DesktopSession>, parsed: &ParsedSession) -> (String, String) {
    if let Some(record) = desktop
        && let Some(title) = record.title.as_deref().and_then(clean_title)
    {
        return (
            title,
            record
                .title_source
                .clone()
                .unwrap_or_else(|| "desktop".to_owned()),
        );
    }

    if let Some(summary) = parsed.summary.clone() {
        return (summary, "summary".to_owned());
    }
    if let Some(first_user_message) = parsed.first_user_message.clone() {
        return (first_user_message, "firstMessage".to_owned());
    }

    ("未命名会话".to_owned(), "fallback".to_owned())
}

fn extract_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(items) => {
            let text = items
                .iter()
                .filter(|item| item.get("type").and_then(Value::as_str) == Some("text"))
                .filter_map(|item| item.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join(" ");
            (!text.trim().is_empty()).then_some(text)
        }
        _ => None,
    }
}

fn clean_title(value: &str) -> Option<String> {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty()
        || normalized.starts_with("<command-")
        || normalized.starts_with("<local-command-")
        || normalized.starts_with("<system-reminder")
    {
        return None;
    }

    let mut chars = normalized.chars();
    let title: String = chars.by_ref().take(MAX_TITLE_CHARS).collect();
    if chars.next().is_some() {
        Some(format!("{title}..."))
    } else {
        Some(title)
    }
}

fn set_first_string(target: &mut Option<String>, value: Option<&Value>) {
    if target.is_none()
        && let Some(value) = value
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
    {
        *target = Some(value.to_owned());
    }
}

fn load_desktop_sessions(warnings: &mut Vec<String>) -> HashMap<String, DesktopSession> {
    let Some(local_app_data) = env::var_os("LOCALAPPDATA") else {
        return HashMap::new();
    };
    let packages = PathBuf::from(local_app_data).join("Packages");
    let Ok(package_entries) = fs::read_dir(packages) else {
        return HashMap::new();
    };

    let mut json_files = Vec::new();
    for package in package_entries.flatten() {
        let name = package.file_name().to_string_lossy().to_string();
        if !name.starts_with("Claude_") {
            continue;
        }
        let index_root = package
            .path()
            .join("LocalCache")
            .join("Roaming")
            .join("Claude")
            .join("claude-code-sessions");
        collect_json_files(&index_root, 4, &mut json_files);
    }

    let mut sessions: HashMap<String, DesktopSession> = HashMap::new();
    let mut invalid_count = 0_u32;
    for path in json_files {
        let Ok(contents) = fs::read_to_string(&path) else {
            invalid_count += 1;
            continue;
        };
        let Ok(record) = serde_json::from_str::<DesktopSession>(&contents) else {
            invalid_count += 1;
            continue;
        };
        if Uuid::parse_str(&record.cli_session_id).is_err() {
            continue;
        }

        let should_replace = sessions
            .get(&record.cli_session_id)
            .is_none_or(|current| record.last_activity_at > current.last_activity_at);
        if should_replace {
            sessions.insert(record.cli_session_id.clone(), record);
        }
    }

    if invalid_count > 0 {
        warnings.push(format!(
            "Claude Desktop 索引中有 {invalid_count} 个文件无法解析，已忽略"
        ));
    }
    sessions
}

fn collect_json_files(directory: &Path, depth: usize, output: &mut Vec<PathBuf>) {
    if depth == 0 || !directory.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(&path, depth - 1, output);
        } else if path.extension().and_then(|value| value.to_str()) == Some("json") {
            output.push(path);
        }
    }
}

fn project_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or(path)
        .to_owned()
}

fn format_timestamp(timestamp: DateTime<Utc>) -> String {
    timestamp.to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_text_from_content_blocks() {
        let value = serde_json::json!([
            {"type": "text", "text": "第一段"},
            {"type": "tool_result", "content": "ignored"},
            {"type": "text", "text": "第二段"}
        ]);

        assert_eq!(extract_text(&value).as_deref(), Some("第一段 第二段"));
    }

    #[test]
    fn cleans_and_truncates_titles() {
        let long_title = "a".repeat(MAX_TITLE_CHARS + 8);
        let title = clean_title(&format!("  {long_title}  ")).expect("title");

        assert!(title.ends_with("..."));
        assert_eq!(title.chars().count(), MAX_TITLE_CHARS + 3);
        assert!(clean_title("<command-message>resume</command-message>").is_none());
    }

    #[test]
    fn desktop_title_has_priority() {
        let desktop = DesktopSession {
            cli_session_id: Uuid::new_v4().to_string(),
            title: Some("桌面端标题".to_owned()),
            title_source: Some("user".to_owned()),
            ..DesktopSession::default()
        };
        let parsed = ParsedSession {
            summary: Some("会话摘要".to_owned()),
            first_user_message: Some("第一条消息".to_owned()),
            ..ParsedSession::default()
        };

        assert_eq!(
            choose_title(Some(&desktop), &parsed),
            ("桌面端标题".to_owned(), "user".to_owned())
        );
    }

    #[test]
    fn scans_a_fixture_catalog_end_to_end() {
        let root = env::temp_dir().join(format!("claude-session-manager-{}", Uuid::new_v4()));
        let project_directory = root.join("C--code-demo");
        fs::create_dir_all(&project_directory).expect("create fixture directory");
        let session_id = Uuid::new_v4().to_string();
        let session_path = project_directory.join(format!("{session_id}.jsonl"));
        let user_record = serde_json::json!({
            "type": "user",
            "sessionId": session_id,
            "timestamp": "2026-07-27T05:00:00.000Z",
            "cwd": project_directory.to_string_lossy(),
            "message": {"role": "user", "content": "检查临时会话目录"}
        });
        let assistant_record = serde_json::json!({
            "type": "assistant",
            "sessionId": session_id,
            "timestamp": "2026-07-27T05:01:00.000Z",
            "cwd": project_directory.to_string_lossy(),
            "message": {"role": "assistant", "model": "claude-sonnet", "content": []}
        });
        fs::write(
            &session_path,
            format!("{user_record}\n{assistant_record}\n"),
        )
        .expect("write fixture session");

        let catalog = scan_sessions_at(&root).expect("scan fixture catalog");

        assert_eq!(catalog.projects.len(), 1);
        assert_eq!(catalog.projects[0].sessions.len(), 1);
        let session = &catalog.projects[0].sessions[0];
        assert_eq!(session.title, "检查临时会话目录");
        assert_eq!(session.message_count, 2);
        assert!(session.can_resume);

        fs::remove_dir_all(root).expect("remove fixture directory");
    }

    #[test]
    fn absorbs_session_metadata_without_message_content_leaking() {
        let mut parsed = ParsedSession::default();
        let record = serde_json::json!({
            "type": "user",
            "timestamp": "2026-07-27T05:00:00.000Z",
            "cwd": "C:\\code\\demo",
            "gitBranch": "main",
            "version": "2.1.217",
            "message": {"role": "user", "content": "实现会话管理器"}
        });

        absorb_record(&mut parsed, &record);

        assert_eq!(parsed.cwd.as_deref(), Some("C:\\code\\demo"));
        assert_eq!(parsed.branch.as_deref(), Some("main"));
        assert_eq!(parsed.message_count, 1);
        assert_eq!(parsed.first_user_message.as_deref(), Some("实现会话管理器"));
    }
}
