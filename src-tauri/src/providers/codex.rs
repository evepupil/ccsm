use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, OpenFlags};

use super::ProviderScan;
use crate::models::{SessionProvider, SessionSummary};

pub fn scan_sessions(home_dir: &Path) -> ProviderScan {
    let codex_home = env::var_os("CODEX_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir.join(".codex"));

    let Some(database_path) = select_state_database(&codex_home) else {
        return ProviderScan::unavailable(
            SessionProvider::Codex,
            codex_home.display().to_string(),
            "Codex 状态数据库不存在".to_owned(),
        );
    };
    let location = database_path.display().to_string();

    match scan_database(&database_path) {
        Ok((sessions, skipped_rows)) => {
            let warnings = (skipped_rows > 0)
                .then(|| format!("Codex 索引中有 {skipped_rows} 条记录无法解析，已忽略"))
                .into_iter()
                .collect();
            ProviderScan::available(SessionProvider::Codex, location, sessions, warnings)
        }
        Err(error) => ProviderScan::unavailable(SessionProvider::Codex, location, error),
    }
}

fn select_state_database(codex_home: &Path) -> Option<PathBuf> {
    fs::read_dir(codex_home)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file()
                || path.extension().and_then(|value| value.to_str()) != Some("sqlite")
            {
                return None;
            }
            let version = path
                .file_stem()
                .and_then(|value| value.to_str())?
                .strip_prefix("state_")?
                .parse::<u64>()
                .ok()?;
            Some((version, path))
        })
        .max_by_key(|(version, _)| *version)
        .map(|(_, path)| path)
}

fn scan_database(database_path: &Path) -> Result<(Vec<SessionSummary>, u32), String> {
    let connection = Connection::open_with_flags(
        database_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|error| format!("无法只读打开 Codex 状态数据库：{error}"))?;
    connection
        .busy_timeout(Duration::from_secs(2))
        .map_err(|error| format!("无法配置 Codex 数据库读取超时：{error}"))?;
    connection
        .pragma_update(None, "query_only", true)
        .map_err(|error| format!("无法启用 Codex 数据库只读模式：{error}"))?;

    let columns = thread_columns(&connection)?;
    for required in ["id", "rollout_path", "cwd"] {
        if !columns.contains(required) {
            return Err(format!("Codex threads 表缺少字段：{required}"));
        }
    }

    let title_expression = coalesce_text_expression(
        &columns,
        &["name", "title", "preview", "first_user_message"],
        "'未命名会话'",
    );
    let created_expression = timestamp_expression(&columns, "created_at_ms", "created_at");
    let updated_expression = if columns.contains("recency_at_ms") {
        "recency_at_ms".to_owned()
    } else {
        timestamp_expression(&columns, "updated_at_ms", "updated_at")
    };
    let query = format!(
        "SELECT id, rollout_path, cwd, {title_expression}, \
         {created_expression}, {updated_expression}, {}, {}, {}, {}, {}, {}, {}, {}, {} \
         FROM threads ORDER BY {updated_expression} DESC",
        column_or(&columns, "tokens_used", "NULL"),
        column_or(&columns, "archived", "0"),
        column_or(&columns, "git_branch", "NULL"),
        column_or(&columns, "cli_version", "NULL"),
        column_or(&columns, "model", "NULL"),
        column_or(&columns, "reasoning_effort", "NULL"),
        column_or(&columns, "thread_source", "NULL"),
        column_or(&columns, "source", "NULL"),
        column_or(&columns, "has_user_event", "NULL"),
    );

    let mut statement = connection
        .prepare(&query)
        .map_err(|error| format!("无法查询 Codex threads 表：{error}"))?;
    let mut rows = statement
        .query([])
        .map_err(|error| format!("无法读取 Codex threads 表：{error}"))?;
    let mut sessions = Vec::new();
    let mut skipped_rows = 0_u32;

    while let Some(row) = rows
        .next()
        .map_err(|error| format!("读取 Codex 会话记录失败：{error}"))?
    {
        let parsed = (|| -> rusqlite::Result<SessionSummary> {
            let session_id: String = row.get(0)?;
            let source_path: String = row.get(1)?;
            let project_path: String = row.get(2)?;
            let title: String = row.get(3)?;
            let created_ms: Option<i64> = row.get(4)?;
            let updated_ms: Option<i64> = row.get(5)?;
            let tokens_used: Option<i64> = row.get(6)?;
            let archived: i64 = row.get(7)?;
            let branch: Option<String> = non_empty(row.get(8)?);
            let cli_version: Option<String> = non_empty(row.get(9)?);
            let model: Option<String> = non_empty(row.get(10)?);
            let reasoning_effort: Option<String> = non_empty(row.get(11)?);
            let thread_source: Option<String> = non_empty(row.get(12)?);
            let source: Option<String> = non_empty(row.get(13)?);
            let _has_user_event: Option<i64> = row.get(14)?;

            let fallback_time = fs::metadata(&source_path)
                .and_then(|metadata| metadata.modified())
                .map(DateTime::<Utc>::from)
                .unwrap_or_else(|_| Utc::now());
            let last_activity = format_millis(updated_ms)
                .unwrap_or_else(|| fallback_time.to_rfc3339_opts(SecondsFormat::Secs, true));
            let file_size = fs::metadata(&source_path)
                .map(|metadata| metadata.len())
                .unwrap_or(0);

            Ok(SessionSummary {
                id: session_id,
                provider: SessionProvider::Codex,
                title,
                title_source: "codexIndex".to_owned(),
                project_path: project_path.clone(),
                source_path,
                source_detail: format_source_detail(
                    source.as_deref(),
                    thread_source.as_deref(),
                    reasoning_effort.as_deref(),
                ),
                created_at: format_millis(created_ms),
                last_activity,
                message_count: None,
                tokens_used: tokens_used.and_then(|value| u64::try_from(value).ok()),
                branch,
                model,
                cli_version,
                file_size,
                is_archived: archived != 0,
                can_resume: Path::new(&project_path).is_dir(),
            })
        })();

        match parsed {
            Ok(session) => sessions.push(session),
            Err(_) => skipped_rows += 1,
        }
    }

    Ok((sessions, skipped_rows))
}

fn thread_columns(connection: &Connection) -> Result<HashSet<String>, String> {
    let mut statement = connection
        .prepare("PRAGMA table_info(threads)")
        .map_err(|error| format!("无法检查 Codex threads 表：{error}"))?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| format!("无法读取 Codex threads 表结构：{error}"))?
        .filter_map(Result::ok)
        .collect::<HashSet<_>>();
    if columns.is_empty() {
        Err("Codex 状态数据库中没有 threads 表".to_owned())
    } else {
        Ok(columns)
    }
}

fn coalesce_text_expression(
    columns: &HashSet<String>,
    candidates: &[&str],
    fallback: &str,
) -> String {
    let mut expressions = candidates
        .iter()
        .filter(|candidate| columns.contains(**candidate))
        .map(|candidate| format!("NULLIF({candidate}, '')"))
        .collect::<Vec<_>>();
    expressions.push(fallback.to_owned());
    format!("COALESCE({})", expressions.join(", "))
}

fn timestamp_expression(columns: &HashSet<String>, milliseconds: &str, seconds: &str) -> String {
    if columns.contains(milliseconds) {
        milliseconds.to_owned()
    } else if columns.contains(seconds) {
        format!("{seconds} * 1000")
    } else {
        "NULL".to_owned()
    }
}

fn column_or(columns: &HashSet<String>, column: &str, fallback: &str) -> String {
    if columns.contains(column) {
        column.to_owned()
    } else {
        fallback.to_owned()
    }
}

fn format_millis(value: Option<i64>) -> Option<String> {
    value
        .and_then(DateTime::<Utc>::from_timestamp_millis)
        .map(|timestamp| timestamp.to_rfc3339_opts(SecondsFormat::Secs, true))
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|text| !text.trim().is_empty())
}

fn format_source_detail(
    source: Option<&str>,
    thread_source: Option<&str>,
    reasoning_effort: Option<&str>,
) -> Option<String> {
    let source = match source {
        Some("vscode") => Some("Codex Desktop"),
        Some("cli") => Some("Codex CLI"),
        Some(value) => Some(value),
        None => None,
    };
    let thread_source = match thread_source {
        Some("user") | None => None,
        Some("automation") => Some("自动任务"),
        Some("subagent") => Some("子代理"),
        Some(value) => Some(value),
    };
    let reasoning = reasoning_effort.map(|value| format!("推理 {value}"));
    let mut parts = source.into_iter().map(str::to_owned).collect::<Vec<_>>();
    parts.extend(thread_source.into_iter().map(str::to_owned));
    parts.extend(reasoning);
    (!parts.is_empty()).then(|| parts.join(" · "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn selects_the_highest_state_database_version() {
        let root = env::temp_dir().join(format!("ccsm-codex-state-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create state fixture");
        fs::write(root.join("state_2.sqlite"), []).expect("write state 2");
        fs::write(root.join("state_10.sqlite"), []).expect("write state 10");
        fs::write(root.join("state_bad.sqlite"), []).expect("write invalid state");

        assert_eq!(
            select_state_database(&root),
            Some(root.join("state_10.sqlite"))
        );

        fs::remove_dir_all(root).expect("remove state fixture");
    }

    #[test]
    fn reads_codex_threads_from_a_read_only_fixture() {
        let root = env::temp_dir().join(format!("ccsm-codex-db-{}", Uuid::new_v4()));
        let project = root.join("project");
        fs::create_dir_all(&project).expect("create project fixture");
        let rollout = root.join("rollout.jsonl");
        fs::write(&rollout, "{}\n").expect("write rollout fixture");
        let database = root.join("state_5.sqlite");
        let connection = Connection::open(&database).expect("create database fixture");
        connection
            .execute_batch(
                "CREATE TABLE threads (
                    id TEXT PRIMARY KEY,
                    rollout_path TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL,
                    source TEXT NOT NULL,
                    cwd TEXT NOT NULL,
                    title TEXT NOT NULL,
                    tokens_used INTEGER NOT NULL,
                    archived INTEGER NOT NULL,
                    git_branch TEXT,
                    cli_version TEXT,
                    model TEXT,
                    reasoning_effort TEXT,
                    thread_source TEXT,
                    preview TEXT NOT NULL,
                    first_user_message TEXT NOT NULL
                );",
            )
            .expect("create threads table");
        let session_id = Uuid::new_v4().to_string();
        connection
            .execute(
                "INSERT INTO threads (
                    id, rollout_path, created_at, updated_at, source, cwd, title,
                    tokens_used, archived, git_branch, cli_version, model,
                    reasoning_effort, thread_source, preview, first_user_message
                ) VALUES (?1, ?2, 1785128400, 1785132000, 'cli', ?3, 'Codex 会话',
                    4096, 0, 'main', '0.144.6', 'gpt-5', 'high', 'user', '', '')",
                (
                    &session_id,
                    rollout.to_string_lossy().as_ref(),
                    project.to_string_lossy().as_ref(),
                ),
            )
            .expect("insert thread fixture");
        drop(connection);

        let (sessions, skipped) = scan_database(&database).expect("scan database fixture");

        assert_eq!(skipped, 0);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].provider, SessionProvider::Codex);
        assert_eq!(sessions[0].title, "Codex 会话");
        assert_eq!(sessions[0].tokens_used, Some(4096));
        assert_eq!(sessions[0].branch.as_deref(), Some("main"));
        assert!(sessions[0].can_resume);

        fs::remove_dir_all(root).expect("remove database fixture");
    }
}
