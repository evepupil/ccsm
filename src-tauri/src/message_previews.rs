use std::{
    collections::VecDeque,
    env,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use serde_json::Value;

use crate::models::SessionProvider;

const MAX_PREVIEW_LIMIT: usize = 10;
const MAX_PREVIEW_CHARS: usize = 240;

pub fn read_user_message_previews(
    provider: SessionProvider,
    source_path: &Path,
    limit: usize,
) -> Result<Vec<String>, String> {
    let limit = limit.clamp(1, MAX_PREVIEW_LIMIT);
    let source_path = canonicalize_source_path(provider, source_path)?;

    match provider {
        SessionProvider::Claude => read_claude_file(&source_path, limit),
        SessionProvider::Codex => read_codex_file(&source_path, limit),
    }
}

fn canonicalize_source_path(
    provider: SessionProvider,
    source_path: &Path,
) -> Result<PathBuf, String> {
    let source_path = std::fs::canonicalize(source_path)
        .map_err(|error| format!("无法读取会话消息文件：{error}"))?;
    if source_path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
        return Err("会话消息文件格式不受支持".to_owned());
    }

    let root = std::fs::canonicalize(provider_root(provider)?)
        .map_err(|error| format!("无法确认会话数据目录：{error}"))?;
    if !is_within_root(&root, &source_path) {
        return Err("会话消息文件不在受支持的数据目录中".to_owned());
    }

    Ok(source_path)
}

fn provider_root(provider: SessionProvider) -> Result<PathBuf, String> {
    let home_dir = dirs::home_dir().ok_or_else(|| "无法确定当前 Windows 用户目录".to_owned())?;
    Ok(match provider {
        SessionProvider::Claude => home_dir.join(".claude").join("projects"),
        SessionProvider::Codex => env::var_os("CODEX_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| home_dir.join(".codex")),
    })
}

fn is_within_root(root: &Path, candidate: &Path) -> bool {
    let root = normalize_windows_path(root);
    let candidate = normalize_windows_path(candidate);
    candidate == root || candidate.starts_with(&(root + "\\"))
}

fn normalize_windows_path(path: &Path) -> String {
    let mut value = path.to_string_lossy().replace('/', "\\").to_lowercase();
    if let Some(stripped) = value.strip_prefix(r"\\?\") {
        value = stripped.to_owned();
    }
    value.trim_end_matches('\\').to_owned()
}

fn read_claude_file(path: &Path, limit: usize) -> Result<Vec<String>, String> {
    let file = File::open(path).map_err(|error| format!("无法读取 Claude 会话：{error}"))?;
    let mut previews = VecDeque::with_capacity(limit);

    for line in BufReader::new(file).lines() {
        let Ok(line) = line else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) != Some("user")
            || value.get("isMeta").and_then(Value::as_bool) == Some(true)
        {
            continue;
        }

        let text = value
            .get("message")
            .and_then(|message| message.get("content"))
            .and_then(extract_text);
        push_recent(&mut previews, text, limit);
    }

    Ok(previews.into_iter().collect())
}

fn read_codex_file(path: &Path, limit: usize) -> Result<Vec<String>, String> {
    let file = File::open(path).map_err(|error| format!("无法读取 Codex 会话：{error}"))?;
    let mut explicit_user_messages = VecDeque::with_capacity(limit);
    let mut fallback_user_messages = VecDeque::with_capacity(limit);

    for line in BufReader::new(file).lines() {
        let Ok(line) = line else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let Some(payload) = value.get("payload") else {
            continue;
        };

        match value.get("type").and_then(Value::as_str) {
            Some("event_msg")
                if payload.get("type").and_then(Value::as_str) == Some("user_message") =>
            {
                let text = payload
                    .get("message")
                    .and_then(extract_text)
                    .or_else(|| payload.get("text").and_then(extract_text));
                push_recent(&mut explicit_user_messages, text, limit);
            }
            Some("response_item")
                if payload.get("type").and_then(Value::as_str) == Some("message")
                    && payload.get("role").and_then(Value::as_str) == Some("user") =>
            {
                let text = payload.get("content").and_then(extract_text);
                push_recent(&mut fallback_user_messages, text, limit);
            }
            _ => {}
        }
    }

    if explicit_user_messages.is_empty() {
        Ok(fallback_user_messages.into_iter().collect())
    } else {
        Ok(explicit_user_messages.into_iter().collect())
    }
}

fn push_recent(buffer: &mut VecDeque<String>, value: Option<String>, limit: usize) {
    let Some(value) = value.and_then(|value| clean_preview(&value)) else {
        return;
    };
    buffer.push_back(value);
    while buffer.len() > limit {
        buffer.pop_front();
    }
}

fn extract_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(items) => {
            let text = items
                .iter()
                .filter(|item| {
                    matches!(
                        item.get("type").and_then(Value::as_str),
                        Some("text") | Some("input_text")
                    )
                })
                .filter_map(|item| item.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join(" ");
            (!text.trim().is_empty()).then_some(text)
        }
        Value::Object(_) => value
            .get("content")
            .and_then(extract_text)
            .or_else(|| value.get("text").and_then(extract_text)),
        _ => None,
    }
}

fn clean_preview(value: &str) -> Option<String> {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty()
        || normalized.starts_with("<command-")
        || normalized.starts_with("<local-command-")
        || normalized.starts_with("<system-reminder")
        || normalized.starts_with("<environment_context")
    {
        return None;
    }

    let mut chars = normalized.chars();
    let preview: String = chars.by_ref().take(MAX_PREVIEW_CHARS).collect();
    if chars.next().is_some() {
        Some(format!("{preview}..."))
    } else {
        Some(preview)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use serde_json::json;

    use super::*;

    fn fixture_path(extension: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        env::temp_dir().join(format!("ccsm-message-preview-{nonce}.{extension}"))
    }

    fn write_json_lines(path: &Path, values: &[Value]) {
        let contents = values
            .iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(path, format!("{contents}\n")).expect("write fixture");
    }

    #[test]
    fn reads_recent_claude_user_messages_in_order() {
        let path = fixture_path("jsonl");
        write_json_lines(
            &path,
            &[
                json!({"type": "user", "message": {"role": "user", "content": "第一条"}}),
                json!({"type": "assistant", "message": {"role": "assistant", "content": [{"type": "text", "text": "忽略"}]}}),
                json!({"type": "user", "isMeta": true, "message": {"content": "忽略元消息"}}),
                json!({"type": "user", "message": {"role": "user", "content": "第二条"}}),
                json!({"type": "user", "message": {"role": "user", "content": "第三条"}}),
            ],
        );

        assert_eq!(
            read_claude_file(&path, 2).expect("read Claude fixture"),
            vec!["第二条".to_owned(), "第三条".to_owned()]
        );
        fs::remove_file(path).expect("remove fixture");
    }

    #[test]
    fn prefers_codex_explicit_user_events_over_duplicate_response_items() {
        let path = fixture_path("jsonl");
        write_json_lines(
            &path,
            &[
                json!({"type": "response_item", "payload": {"type": "message", "role": "user", "content": [{"type": "input_text", "text": "重复记录"}]}}),
                json!({"type": "event_msg", "payload": {"type": "user_message", "message": "第一条"}}),
                json!({"type": "event_msg", "payload": {"type": "user_message", "message": "第二条"}}),
            ],
        );

        assert_eq!(
            read_codex_file(&path, 5).expect("read Codex fixture"),
            vec!["第一条".to_owned(), "第二条".to_owned()]
        );
        fs::remove_file(path).expect("remove fixture");
    }

    #[test]
    fn trims_and_truncates_previews() {
        let value = format!("  {}  ", "a".repeat(MAX_PREVIEW_CHARS + 4));
        let preview = clean_preview(&value).expect("preview");

        assert!(preview.ends_with("..."));
        assert_eq!(preview.chars().count(), MAX_PREVIEW_CHARS + 3);
        assert!(clean_preview("<system-reminder>ignore</system-reminder>").is_none());
    }
}
