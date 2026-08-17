use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use chrono::{DateTime, SecondsFormat, Utc};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use super::ProviderScan;
use crate::models::{SessionProvider, SessionSummary};

const MAX_TITLE_CHARS: usize = 96;

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
    custom_title: Option<String>,
    summary: Option<String>,
    first_user_message: Option<String>,
}

pub fn scan_sessions(home_dir: &Path) -> ProviderScan {
    let sessions_root = home_dir.join(".claude").join("projects");
    scan_sessions_at(&sessions_root)
}

fn scan_sessions_at(sessions_root: &Path) -> ProviderScan {
    let location = sessions_root.display().to_string();
    if !sessions_root.is_dir() {
        return ProviderScan::unavailable(
            SessionProvider::Claude,
            location,
            "Claude 会话目录不存在".to_owned(),
        );
    }

    let mut warnings = Vec::new();
    let desktop_sessions = load_desktop_sessions(&mut warnings);
    let Ok(project_directories) = fs::read_dir(sessions_root) else {
        return ProviderScan::unavailable(
            SessionProvider::Claude,
            location,
            "Claude 会话目录无法读取".to_owned(),
        );
    };

    let mut sessions = Vec::new();
    let mut skipped_files = 0_u32;

    for project_entry in project_directories.flatten() {
        let project_dir = project_entry.path();
        if !project_dir.is_dir() {
            continue;
        }

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
                Ok(session) => sessions.push(session),
                Err(_) => skipped_files += 1,
            }
        }
    }

    if skipped_files > 0 {
        warnings.push(format!("有 {skipped_files} 个会话文件无法解析，已跳过"));
    }

    ProviderScan::available(SessionProvider::Claude, location, sessions, warnings)
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
        provider: SessionProvider::Claude,
        title,
        title_source,
        project_path,
        source_path: path.display().to_string(),
        source_detail: Some("Claude Code JSONL".to_owned()),
        created_at,
        last_activity: format_timestamp(last_activity),
        message_count: Some(u64::from(parsed.message_count)),
        tokens_used: None,
        branch: parsed.branch,
        model: desktop
            .and_then(|record| record.model.clone())
            .or(parsed.model),
        cli_version: parsed.claude_version,
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
        Some("custom-title") => {
            if let Some(custom_title) = value.get("customTitle").and_then(Value::as_str) {
                parsed.custom_title = clean_title(custom_title);
            }
        }
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
    if let Some(custom_title) = parsed.custom_title.clone() {
        return (custom_title, "customTitle".to_owned());
    }

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
    fn latest_custom_title_has_priority() {
        let desktop = DesktopSession {
            cli_session_id: Uuid::new_v4().to_string(),
            title: Some("Desktop 标题".to_owned()),
            title_source: Some("user".to_owned()),
            ..DesktopSession::default()
        };
        let mut parsed = ParsedSession {
            summary: Some("会话摘要".to_owned()),
            first_user_message: Some("第一条消息".to_owned()),
            ..ParsedSession::default()
        };

        absorb_record(
            &mut parsed,
            &serde_json::json!({
                "type": "custom-title",
                "customTitle": "旧的用户标题"
            }),
        );
        absorb_record(
            &mut parsed,
            &serde_json::json!({
                "type": "custom-title",
                "customTitle": "新的用户标题"
            }),
        );

        assert_eq!(
            choose_title(Some(&desktop), &parsed),
            ("新的用户标题".to_owned(), "customTitle".to_owned())
        );
    }

    #[test]
    fn scans_a_fixture_catalog_end_to_end() {
        let root = env::temp_dir().join(format!("ccsm-{}", Uuid::new_v4()));
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

        let scan = scan_sessions_at(&root);

        assert!(scan.source.available);
        assert_eq!(scan.sessions.len(), 1);
        let session = &scan.sessions[0];
        assert_eq!(session.provider, SessionProvider::Claude);
        assert_eq!(session.title, "检查临时会话目录");
        assert_eq!(session.message_count, Some(2));
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
