use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use chrono::Local;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use serde_json::Value;
use uuid::Uuid;

use crate::{
    catalog,
    models::{
        CliStatus, LaunchResult, NewSessionResult, OpenSessionSummary, ProjectSummary,
        SessionCatalog, SessionProvider, SessionSummary, TEMPORARY_PROJECT_ID,
        WorkspaceRestoreResult,
    },
    terminal::{self, ActiveTerminal, TerminalHost},
};

#[cfg(target_os = "windows")]
const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

struct TerminalLaunch {
    name: String,
    reused_window: bool,
}

pub fn cli_statuses() -> Vec<CliStatus> {
    [SessionProvider::Claude, SessionProvider::Codex]
        .into_iter()
        .map(cli_status)
        .collect()
}

fn cli_status(provider: SessionProvider) -> CliStatus {
    let Some(executable) = find_cli(provider) else {
        return unavailable_status(provider);
    };
    let Ok(version_output) = command_output(&executable, &["--version"]) else {
        return unavailable_status(provider);
    };
    if !version_output.status.success() {
        return unavailable_status(provider);
    }

    let version = output_text(&version_output);
    let (logged_in, auth_method, api_provider) = match provider {
        SessionProvider::Claude => claude_auth_status(&executable),
        SessionProvider::Codex => codex_auth_status(&executable),
    };

    CliStatus {
        provider,
        available: true,
        version,
        logged_in,
        auth_method,
        api_provider,
    }
}

pub fn resume_session(
    provider: SessionProvider,
    session_id: &str,
    fork: bool,
    highest_permissions: bool,
) -> Result<LaunchResult, String> {
    let session_id = Uuid::parse_str(session_id)
        .map_err(|_| "Session ID 格式无效".to_owned())?
        .to_string();
    let catalog = catalog::scan_sessions().map_err(|error| error.to_string())?;
    let session = catalog
        .projects
        .iter()
        .flat_map(|project| project.sessions.iter())
        .find(|session| session.provider == provider && session.id == session_id)
        .ok_or_else(|| format!("本机 {provider} 会话索引中找不到该 Session ID"))?;

    if !session.can_resume || !Path::new(&session.project_path).is_dir() {
        return Err(format!("项目目录已不存在：{}", session.project_path));
    }

    let executable = find_cli(provider).ok_or_else(|| match provider {
        SessionProvider::Claude => "未找到可用的 Claude Code CLI".to_owned(),
        SessionProvider::Codex => "未找到可用的 Codex CLI".to_owned(),
    })?;
    let command = build_resume_command(
        provider,
        &executable,
        &session_id,
        fork,
        highest_permissions,
    );
    let active_terminals = terminal::scan_active_terminals();
    let reuse_window =
        has_windows_terminal_for_project(&session.project_path, &catalog, &active_terminals);

    launch_terminal(&session.project_path, &command, reuse_window).map(|launch| LaunchResult {
        session_id,
        provider,
        terminal: launch.name,
        forked: fork,
        highest_permissions,
        reused_window: launch.reused_window,
    })
}

pub fn scan_open_sessions(catalog: &SessionCatalog) -> Vec<OpenSessionSummary> {
    let active_terminals = terminal::scan_active_terminals();
    let mut seen = HashSet::new();
    let mut open_sessions = active_terminals
        .into_iter()
        .filter_map(|active| {
            let provider = active.provider?;
            let session = active_session_for_scope(catalog, &active, None, &seen)?;
            let session_id = session.id.clone();
            if !seen.insert((provider, session_id.clone())) {
                return None;
            }

            Some(OpenSessionSummary {
                session_id,
                provider,
                project_path: session.project_path.clone(),
                working_directory: active
                    .working_directory
                    .unwrap_or_else(|| session.project_path.clone()),
            })
        })
        .collect::<Vec<_>>();
    open_sessions.sort_by(|left, right| {
        left.project_path
            .cmp(&right.project_path)
            .then_with(|| left.provider.cmp(&right.provider))
            .then_with(|| left.session_id.cmp(&right.session_id))
    });
    open_sessions
}

pub fn restore_workspace(
    project_id: &str,
    highest_permissions: bool,
) -> Result<WorkspaceRestoreResult, String> {
    let project_id = project_id.trim();
    if project_id.is_empty() {
        return Err("未选择项目".to_owned());
    }
    if project_id == TEMPORARY_PROJECT_ID {
        return Err("临时会话不支持恢复工作区".to_owned());
    }

    let catalog = catalog::scan_sessions().map_err(|error| error.to_string())?;
    let project = catalog
        .projects
        .iter()
        .find(|project| project.id == project_id)
        .ok_or_else(|| "本机项目索引中找不到该项目".to_owned())?;
    if !Path::new(&project.path).is_dir() {
        return Err(format!("项目目录已不存在：{}", project.path));
    }

    let active_terminals = terminal::scan_active_terminals();
    let (sessions, mut skipped_count) =
        resolve_workspace_sessions(project, &catalog, &active_terminals);
    let mut restored_count = 0;
    let mut one_window = !sessions.is_empty();
    let mut terminal_name = "Windows Terminal".to_owned();

    for session in sessions {
        let Some(executable) = find_cli(session.provider) else {
            skipped_count += 1;
            one_window = false;
            continue;
        };
        let command = build_resume_command(
            session.provider,
            &executable,
            &session.session_id,
            false,
            highest_permissions,
        );
        match launch_terminal(&session.project_path, &command, true) {
            Ok(launch) => {
                restored_count += 1;
                terminal_name = launch.name;
                one_window = one_window && launch.reused_window;
            }
            Err(_) => {
                skipped_count += 1;
                one_window = false;
            }
        }
    }

    Ok(WorkspaceRestoreResult {
        project_path: project.path.clone(),
        terminal: terminal_name,
        restored_count,
        skipped_count,
        one_window,
    })
}

pub fn start_new_session(
    provider: SessionProvider,
    project_id: &str,
    highest_permissions: bool,
) -> Result<NewSessionResult, String> {
    let project_id = project_id.trim();
    if project_id == TEMPORARY_PROJECT_ID {
        let executable = find_cli(provider).ok_or_else(|| match provider {
            SessionProvider::Claude => "未找到可用的 Claude Code CLI".to_owned(),
            SessionProvider::Codex => "未找到可用的 Codex CLI".to_owned(),
        })?;
        let working_directory = create_temporary_session_directory()?;
        let command = build_new_session_command(provider, &executable, highest_permissions);
        let terminal = launch_terminal(&working_directory.display().to_string(), &command, false)?;
        return Ok(NewSessionResult {
            provider,
            terminal: terminal.name,
            working_directory: working_directory.display().to_string(),
            highest_permissions,
        });
    }

    if project_id.is_empty() {
        return Err("未选择项目".to_owned());
    }

    let catalog = catalog::scan_sessions().map_err(|error| error.to_string())?;
    let project = catalog
        .projects
        .iter()
        .find(|project| project.id == project_id && project.providers.contains(&provider))
        .ok_or_else(|| format!("本机 {provider} 项目索引中找不到该项目"))?;

    if !Path::new(&project.path).is_dir() {
        return Err(format!("项目目录已不存在：{}", project.path));
    }

    let executable = find_cli(provider).ok_or_else(|| match provider {
        SessionProvider::Claude => "未找到可用的 Claude Code CLI".to_owned(),
        SessionProvider::Codex => "未找到可用的 Codex CLI".to_owned(),
    })?;
    let command = build_new_session_command(provider, &executable, highest_permissions);

    launch_terminal(&project.path, &command, false).map(|terminal| NewSessionResult {
        provider,
        terminal: terminal.name,
        working_directory: project.path.clone(),
        highest_permissions,
    })
}

fn find_session<'a>(
    catalog: &'a SessionCatalog,
    provider: SessionProvider,
    session_id: &str,
) -> Option<&'a SessionSummary> {
    catalog
        .projects
        .iter()
        .flat_map(|project| project.sessions.iter())
        .find(|session| session.provider == provider && session.id == session_id)
}

fn resolve_workspace_sessions(
    project: &ProjectSummary,
    catalog: &SessionCatalog,
    active_terminals: &[ActiveTerminal],
) -> (Vec<OpenSessionSummary>, usize) {
    let mut seen = HashSet::new();
    let mut skipped_count = 0;
    let mut sessions = Vec::new();

    for active in active_terminals {
        let Some(provider) = active.provider else {
            continue;
        };
        let Some(session) = active_session_for_scope(catalog, active, Some(&project.path), &seen)
        else {
            if active
                .working_directory
                .as_deref()
                .is_some_and(|path| terminal::is_path_within(&project.path, path))
            {
                skipped_count += 1;
            }
            continue;
        };
        if !seen.insert((provider, session.id.clone())) {
            continue;
        }

        sessions.push(OpenSessionSummary {
            session_id: session.id.clone(),
            provider,
            project_path: session.project_path.clone(),
            working_directory: active
                .working_directory
                .clone()
                .unwrap_or_else(|| session.project_path.clone()),
        });
    }

    sessions.sort_by(|left, right| {
        left.project_path
            .cmp(&right.project_path)
            .then_with(|| left.provider.cmp(&right.provider))
            .then_with(|| left.session_id.cmp(&right.session_id))
    });
    (sessions, skipped_count)
}

fn active_session_for_scope<'a>(
    catalog: &'a SessionCatalog,
    active: &ActiveTerminal,
    scope_path: Option<&str>,
    seen: &HashSet<(SessionProvider, String)>,
) -> Option<&'a SessionSummary> {
    let provider = active.provider?;
    if let Some(session_id) = active.session_id.as_deref() {
        let session = find_session(catalog, provider, session_id)?;
        if !session.can_resume || seen.contains(&(provider, session.id.clone())) {
            return None;
        }
        if let Some(scope_path) = scope_path {
            let in_scope = terminal::is_path_within(scope_path, &session.project_path)
                || active
                    .working_directory
                    .as_deref()
                    .is_some_and(|path| terminal::is_path_within(scope_path, path));
            if !in_scope {
                return None;
            }
        }
        return Some(session);
    }

    let working_directory = active.working_directory.as_deref()?;
    if scope_path.is_some_and(|scope| !terminal::is_path_within(scope, working_directory)) {
        return None;
    }

    catalog
        .projects
        .iter()
        .flat_map(|project| project.sessions.iter())
        .filter(|session| {
            session.provider == provider
                && session.can_resume
                && !seen.contains(&(provider, session.id.clone()))
                && scope_path
                    .is_none_or(|scope| terminal::is_path_within(scope, &session.project_path))
                && (terminal::is_path_within(&session.project_path, working_directory)
                    || terminal::is_path_within(working_directory, &session.project_path))
        })
        .max_by(|left, right| {
            left.last_activity
                .cmp(&right.last_activity)
                .then_with(|| left.id.cmp(&right.id))
        })
}

fn has_windows_terminal_for_project(
    project_path: &str,
    catalog: &SessionCatalog,
    active_terminals: &[ActiveTerminal],
) -> bool {
    active_terminals.iter().any(|active| {
        if active.host != TerminalHost::WindowsTerminal {
            return false;
        }
        if active
            .working_directory
            .as_deref()
            .is_some_and(|path| terminal::is_path_within(project_path, path))
        {
            return true;
        }

        let (Some(provider), Some(session_id)) = (active.provider, active.session_id.as_deref())
        else {
            return false;
        };
        find_session(catalog, provider, session_id)
            .is_some_and(|session| terminal::is_path_within(project_path, &session.project_path))
    })
}

fn create_temporary_session_directory() -> Result<PathBuf, String> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| "\u{65e0}\u{6cd5}\u{786e}\u{5b9a}\u{5f53}\u{524d} Windows \u{7528}\u{6237}\u{76ee}\u{5f55}".to_owned())?;
    let sessions_root = catalog::temporary_sessions_root(&home_dir);
    let date = Local::now().format("%Y-%m-%d").to_string();
    create_unique_temporary_directory(&sessions_root.join(date))
}

fn create_unique_temporary_directory(date_root: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(date_root).map_err(|error| {
        format!(
            "\u{65e0}\u{6cd5}\u{521b}\u{5efa}\u{4e34}\u{65f6}\u{4f1a}\u{8bdd}\u{76ee}\u{5f55}: {error}"
        )
    })?;

    for index in 1..=10_000_u32 {
        let name = if index == 1 {
            "new-chat".to_owned()
        } else {
            format!("new-chat-{index}")
        };
        let candidate = date_root.join(name);
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "\u{65e0}\u{6cd5}\u{521b}\u{5efa}\u{4e34}\u{65f6}\u{4f1a}\u{8bdd}\u{76ee}\u{5f55}: {error}"
                ));
            }
        }
    }

    Err("\u{5f53}\u{5929}\u{4e34}\u{65f6}\u{4f1a}\u{8bdd}\u{76ee}\u{5f55}\u{6570}\u{91cf}\u{5df2}\u{8fbe}\u{5230}\u{4e0a}\u{9650}".to_owned())
}

fn unavailable_status(provider: SessionProvider) -> CliStatus {
    CliStatus {
        provider,
        available: false,
        version: None,
        logged_in: None,
        auth_method: None,
        api_provider: None,
    }
}

fn claude_auth_status(executable: &Path) -> (Option<bool>, Option<String>, Option<String>) {
    let Ok(output) = command_output(executable, &["auth", "status"]) else {
        return (None, None, None);
    };
    let auth = output_text(&output).and_then(|text| serde_json::from_str::<Value>(&text).ok());
    (
        auth.as_ref()
            .and_then(|value| value.get("loggedIn"))
            .and_then(Value::as_bool)
            .or(Some(output.status.success())),
        auth.as_ref()
            .and_then(|value| value.get("authMethod"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        auth.as_ref()
            .and_then(|value| value.get("apiProvider"))
            .and_then(Value::as_str)
            .map(str::to_owned),
    )
}

fn codex_auth_status(executable: &Path) -> (Option<bool>, Option<String>, Option<String>) {
    match command_output(executable, &["login", "status"]) {
        Ok(output) => (Some(output.status.success()), None, None),
        Err(_) => (None, None, None),
    }
}

fn find_cli(provider: SessionProvider) -> Option<PathBuf> {
    match provider {
        SessionProvider::Claude => find_bundled_claude().or_else(|| find_in_path("claude")),
        SessionProvider::Codex => find_in_path("codex"),
    }
}

#[cfg(target_os = "windows")]
fn find_bundled_claude() -> Option<PathBuf> {
    let packages = PathBuf::from(env::var_os("LOCALAPPDATA")?).join("Packages");
    let mut candidates = Vec::new();
    for package in fs::read_dir(packages).ok()?.flatten() {
        if !package.file_name().to_string_lossy().starts_with("Claude_") {
            continue;
        }
        let versions = package
            .path()
            .join("LocalCache")
            .join("Roaming")
            .join("Claude")
            .join("claude-code");
        let Ok(entries) = fs::read_dir(versions) else {
            continue;
        };
        for entry in entries.flatten() {
            let executable = entry.path().join("claude.exe");
            if executable.is_file() {
                candidates.push((
                    version_key(&entry.file_name().to_string_lossy()),
                    executable,
                ));
            }
        }
    }
    candidates
        .into_iter()
        .max_by(|left, right| left.0.cmp(&right.0))
        .map(|(_, path)| path)
}

#[cfg(not(target_os = "windows"))]
fn find_bundled_claude() -> Option<PathBuf> {
    None
}

fn version_key(value: &str) -> Vec<u64> {
    value
        .split('.')
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect()
}

fn find_in_path(command: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    let candidates = if cfg!(target_os = "windows") {
        vec![
            format!("{command}.exe"),
            format!("{command}.cmd"),
            format!("{command}.bat"),
            command.to_owned(),
        ]
    } else {
        vec![command.to_owned()]
    };

    env::split_paths(&path)
        .flat_map(|directory| {
            candidates
                .iter()
                .map(move |candidate| directory.join(candidate))
        })
        .find(|candidate| candidate.is_file())
}

#[cfg(target_os = "windows")]
fn command_output(executable: &Path, arguments: &[&str]) -> Result<Output, std::io::Error> {
    let command = build_powershell_invocation(executable, arguments);
    let mut process = Command::new("powershell.exe");
    process
        .arg("-NoLogo")
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg(command)
        .creation_flags(CREATE_NO_WINDOW);
    process.output()
}

#[cfg(not(target_os = "windows"))]
fn command_output(executable: &Path, arguments: &[&str]) -> Result<Output, std::io::Error> {
    Command::new(executable).args(arguments).output()
}

fn output_text(output: &Output) -> Option<String> {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !stdout.is_empty() {
        return Some(stdout);
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    (!stderr.is_empty()).then_some(stderr)
}

#[cfg(target_os = "windows")]
fn launch_terminal(
    project_path: &str,
    command: &str,
    reuse_existing_window: bool,
) -> Result<TerminalLaunch, String> {
    let mut windows_terminal = Command::new("wt.exe");
    if reuse_existing_window {
        windows_terminal.arg("-w").arg("0");
    }
    let windows_terminal = windows_terminal
        .arg("new-tab")
        .arg("--startingDirectory")
        .arg(project_path)
        .arg("powershell.exe")
        .arg("-NoLogo")
        .arg("-NoExit")
        .arg("-Command")
        .arg(command)
        .spawn();

    match windows_terminal {
        Ok(_) => Ok(TerminalLaunch {
            name: "Windows Terminal".to_owned(),
            reused_window: reuse_existing_window,
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Command::new("powershell.exe")
                .current_dir(project_path)
                .arg("-NoLogo")
                .arg("-NoExit")
                .arg("-Command")
                .arg(command)
                .creation_flags(CREATE_NEW_CONSOLE)
                .spawn()
                .map(|_| TerminalLaunch {
                    name: "PowerShell".to_owned(),
                    reused_window: false,
                })
                .map_err(|fallback_error| format!("无法启动终端：{fallback_error}"))
        }
        Err(error) => Err(format!("无法启动 Windows Terminal：{error}")),
    }
}

#[cfg(not(target_os = "windows"))]
fn launch_terminal(
    _project_path: &str,
    _command: &str,
    _reuse_existing_window: bool,
) -> Result<TerminalLaunch, String> {
    Err("当前版本仅支持 Windows".to_owned())
}

fn build_resume_command(
    provider: SessionProvider,
    executable: &Path,
    session_id: &str,
    fork: bool,
    highest_permissions: bool,
) -> String {
    let mut arguments = match provider {
        SessionProvider::Claude => vec!["--resume", session_id],
        SessionProvider::Codex if fork => vec!["fork"],
        SessionProvider::Codex => vec!["resume"],
    };
    match provider {
        SessionProvider::Claude => {
            if fork {
                arguments.push("--fork-session");
            }
            if highest_permissions {
                arguments.push("--dangerously-skip-permissions");
            }
        }
        SessionProvider::Codex => {
            if highest_permissions {
                arguments.push("--yolo");
            }
            arguments.push(session_id);
        }
    }
    build_powershell_invocation(executable, &arguments)
}

fn build_new_session_command(
    provider: SessionProvider,
    executable: &Path,
    highest_permissions: bool,
) -> String {
    let arguments = match (provider, highest_permissions) {
        (SessionProvider::Claude, true) => vec!["--dangerously-skip-permissions"],
        (SessionProvider::Codex, true) => vec!["--yolo"],
        (_, false) => Vec::new(),
    };
    build_powershell_invocation(executable, &arguments)
}

fn build_powershell_invocation(executable: &Path, arguments: &[&str]) -> String {
    let mut command = format!("& {}", quote_powershell(&executable.display().to_string()));
    for argument in arguments {
        command.push(' ');
        command.push_str(&quote_powershell(argument));
    }
    command
}

fn quote_powershell(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SESSION_ID: &str = "00000000-0000-4000-8000-000000000001";

    #[test]
    fn builds_claude_resume_and_privileged_fork_commands() {
        let executable = Path::new("C:\\Tools\\claude.exe");
        assert_eq!(
            build_resume_command(
                SessionProvider::Claude,
                executable,
                SESSION_ID,
                false,
                false
            ),
            "& 'C:\\Tools\\claude.exe' '--resume' '00000000-0000-4000-8000-000000000001'"
        );
        assert_eq!(
            build_resume_command(SessionProvider::Claude, executable, SESSION_ID, true, true),
            "& 'C:\\Tools\\claude.exe' '--resume' '00000000-0000-4000-8000-000000000001' '--fork-session' '--dangerously-skip-permissions'"
        );
    }

    #[test]
    fn builds_codex_resume_and_privileged_fork_commands() {
        let executable = Path::new("C:\\Tools\\codex.cmd");
        assert_eq!(
            build_resume_command(SessionProvider::Codex, executable, SESSION_ID, false, false),
            "& 'C:\\Tools\\codex.cmd' 'resume' '00000000-0000-4000-8000-000000000001'"
        );
        assert_eq!(
            build_resume_command(SessionProvider::Codex, executable, SESSION_ID, true, true),
            "& 'C:\\Tools\\codex.cmd' 'fork' '--yolo' '00000000-0000-4000-8000-000000000001'"
        );
    }

    #[test]
    fn builds_new_session_commands_for_both_providers() {
        assert_eq!(
            build_new_session_command(
                SessionProvider::Claude,
                Path::new("C:\\Tools\\claude.exe"),
                false
            ),
            "& 'C:\\Tools\\claude.exe'"
        );
        assert_eq!(
            build_new_session_command(
                SessionProvider::Codex,
                Path::new("C:\\Tools\\codex.cmd"),
                true
            ),
            "& 'C:\\Tools\\codex.cmd' '--yolo'"
        );
    }

    #[test]
    fn creates_numbered_temporary_directories_without_overwriting() {
        let root = env::temp_dir().join(format!("ccsm-temporary-{}", Uuid::new_v4()));
        let date_root = root.join("2026-08-18");
        fs::create_dir_all(date_root.join("new-chat")).expect("create first fixture");

        let second =
            create_unique_temporary_directory(&date_root).expect("create second directory");
        assert_eq!(
            second.file_name().and_then(|name| name.to_str()),
            Some("new-chat-2")
        );
        assert!(second.is_dir());

        fs::remove_dir_all(root).expect("remove temporary fixture");
    }

    #[test]
    fn sorts_bundled_versions_numerically() {
        assert!(version_key("2.10.0") > version_key("2.9.9"));
    }

    fn session(id: &str, provider: SessionProvider, project_path: &str) -> SessionSummary {
        SessionSummary {
            id: id.to_owned(),
            provider,
            title: format!("{provider} session"),
            title_source: "fixture".to_owned(),
            project_path: project_path.to_owned(),
            source_path: format!("{id}.jsonl"),
            source_detail: None,
            created_at: Some("2026-08-26T05:00:00Z".to_owned()),
            last_activity: "2026-08-26T06:00:00Z".to_owned(),
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

    fn active(
        provider: Option<SessionProvider>,
        session_id: Option<&str>,
        working_directory: &str,
        host: TerminalHost,
    ) -> ActiveTerminal {
        ActiveTerminal {
            provider,
            session_id: session_id.map(str::to_owned),
            working_directory: Some(working_directory.to_owned()),
            host,
        }
    }

    #[test]
    fn resolves_open_sessions_from_a_project_and_its_descendants() {
        let root = "C:\\Code\\CCSM";
        let sessions = vec![
            session(
                "00000000-0000-4000-8000-000000000001",
                SessionProvider::Claude,
                root,
            ),
            session(
                "00000000-0000-4000-8000-000000000002",
                SessionProvider::Codex,
                "C:\\Code\\CCSM\\packages\\api",
            ),
            session(
                "00000000-0000-4000-8000-000000000003",
                SessionProvider::Claude,
                "C:\\Code\\Other",
            ),
            session(
                "00000000-0000-4000-8000-000000000004",
                SessionProvider::Claude,
                "C:\\Code\\CCSM\\packages\\web",
            ),
        ];
        let catalog = SessionCatalog {
            projects: vec![ProjectSummary {
                id: root.to_ascii_lowercase(),
                name: "CCSM".to_owned(),
                path: root.to_owned(),
                is_temporary: false,
                last_activity: "2026-08-26T06:00:00Z".to_owned(),
                session_count: sessions.len(),
                total_size: 400,
                providers: vec![SessionProvider::Claude, SessionProvider::Codex],
                sessions,
            }],
            open_sessions: Vec::new(),
            scanned_at: "2026-08-26T06:00:00Z".to_owned(),
            sources: Vec::new(),
            warnings: Vec::new(),
        };
        let active_terminals = vec![
            active(
                Some(SessionProvider::Claude),
                Some("00000000-0000-4000-8000-000000000001"),
                root,
                TerminalHost::WindowsTerminal,
            ),
            active(
                Some(SessionProvider::Codex),
                Some("00000000-0000-4000-8000-000000000002"),
                "C:\\Code\\CCSM\\packages\\api\\src",
                TerminalHost::WindowsTerminal,
            ),
            active(
                Some(SessionProvider::Claude),
                Some("00000000-0000-4000-8000-000000000003"),
                "C:\\Code\\Other",
                TerminalHost::WindowsTerminal,
            ),
            active(
                Some(SessionProvider::Claude),
                None,
                "C:\\Code\\CCSM\\packages\\api",
                TerminalHost::WindowsTerminal,
            ),
            active(
                Some(SessionProvider::Claude),
                None,
                "C:\\Code\\CCSM\\packages\\web",
                TerminalHost::WindowsTerminal,
            ),
        ];

        let (resolved, skipped) =
            resolve_workspace_sessions(&catalog.projects[0], &catalog, &active_terminals);
        assert_eq!(
            resolved
                .iter()
                .map(|session| session.session_id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "00000000-0000-4000-8000-000000000001",
                "00000000-0000-4000-8000-000000000002",
                "00000000-0000-4000-8000-000000000004",
            ]
        );
        assert_eq!(skipped, 1);
    }

    #[test]
    fn detects_an_existing_windows_terminal_in_a_project_subdirectory() {
        let catalog = SessionCatalog {
            projects: Vec::new(),
            open_sessions: Vec::new(),
            scanned_at: String::new(),
            sources: Vec::new(),
            warnings: Vec::new(),
        };
        let active_terminals = vec![active(
            None,
            None,
            "C:\\Code\\CCSM\\packages\\api",
            TerminalHost::WindowsTerminal,
        )];

        assert!(has_windows_terminal_for_project(
            "C:\\Code\\CCSM",
            &catalog,
            &active_terminals
        ));
        assert!(!has_windows_terminal_for_project(
            "C:\\Code\\Other",
            &catalog,
            &active_terminals
        ));
    }
}
