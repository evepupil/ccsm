use std::{
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
        CliStatus, LaunchResult, NewSessionResult, OpenSessionSummary, SessionCatalog,
        SessionProvider, SessionSummary, TEMPORARY_PROJECT_ID, WorkspaceRestoreResult,
    },
    terminal::{self, ActiveTerminal, TerminalHost},
    workspace,
};

#[cfg(target_os = "windows")]
const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

struct TerminalLaunch {
    name: String,
    reused_window: bool,
}

#[derive(Debug, Clone)]
enum TerminalTarget {
    New,
    LastUsed,
    Named(String),
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
    let matching_window =
        has_windows_terminal_for_project(&session.project_path, &catalog, &active_terminals);
    let managed_window = workspace::managed_window_for_path(&session.project_path)
        .ok()
        .flatten();
    let mut project_window = None;
    let target = if let Some(window_name) = managed_window {
        TerminalTarget::Named(window_name)
    } else if matching_window && terminal::windows_terminal_window_count() == 1 {
        TerminalTarget::LastUsed
    } else if matching_window {
        let window_name = workspace_window_name(&session.project_path);
        project_window = Some(window_name.clone());
        TerminalTarget::Named(window_name)
    } else {
        TerminalTarget::New
    };

    let launch = launch_terminal(&session.project_path, &command, &target)?;
    if let Some(window_name) = project_window
        && let Some(project) = catalog.projects.iter().find(|project| {
            terminal::normalize_path(&project.path)
                == terminal::normalize_path(&session.project_path)
        })
    {
        let _ = workspace::mark_project_window(&project.id, &project.path, &window_name);
    }
    Ok(LaunchResult {
        session_id,
        provider,
        terminal: launch.name,
        forked: fork,
        highest_permissions,
        reused_window: launch.reused_window,
    })
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
    let sessions = workspace::project_snapshot(project_id)?;
    let capture = workspace::capture(&catalog, &active_terminals)?;
    let live_session_keys = capture
        .live_sessions
        .iter()
        .map(open_session_key)
        .collect::<std::collections::BTreeSet<_>>();
    let unresolved_in_project = capture
        .unresolved_terminals
        .iter()
        .filter(|terminal| {
            terminal
                .working_directory
                .as_deref()
                .is_some_and(|path| terminal::is_path_within(&project.path, path))
        })
        .collect::<Vec<_>>();
    let mut restored_count = 0;
    let mut already_open_count = 0;
    let unidentified_count = unresolved_in_project.len();
    let mut skipped_count = 0;
    let mut one_window = false;
    let mut terminal_name = "Windows Terminal".to_owned();
    let window_name = workspace_window_name(&project.path);
    let target = TerminalTarget::Named(window_name.clone());

    for saved_session in sessions {
        let Some(current_session) =
            find_session(&catalog, saved_session.provider, &saved_session.session_id)
        else {
            skipped_count += 1;
            continue;
        };
        if !current_session.can_resume || !Path::new(&current_session.project_path).is_dir() {
            skipped_count += 1;
            continue;
        }
        let session = OpenSessionSummary {
            session_id: current_session.id.clone(),
            provider: current_session.provider,
            project_path: current_session.project_path.clone(),
            working_directory: saved_session.working_directory,
        };
        if live_session_keys.contains(&open_session_key(&session)) {
            already_open_count += 1;
            continue;
        }
        if unresolved_in_project
            .iter()
            .any(|terminal| unresolved_may_be_session(terminal, &session))
        {
            skipped_count += 1;
            continue;
        }
        let Some(executable) = find_cli(session.provider) else {
            skipped_count += 1;
            continue;
        };
        let command = build_resume_command(
            session.provider,
            &executable,
            &session.session_id,
            false,
            highest_permissions,
        );
        match launch_terminal(&session.project_path, &command, &target) {
            Ok(launch) => {
                restored_count += 1;
                terminal_name = launch.name;
                one_window = one_window || launch.reused_window;
            }
            Err(_) => {
                skipped_count += 1;
            }
        }
    }
    if restored_count > 0 {
        let _ = workspace::mark_project_window(project_id, &project.path, &window_name);
    }

    Ok(WorkspaceRestoreResult {
        project_path: project.path.clone(),
        terminal: terminal_name,
        restored_count,
        already_open_count,
        unidentified_count,
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
        let session_id = new_session_id(provider);
        let command = build_new_session_command(
            provider,
            &executable,
            highest_permissions,
            session_id.as_deref(),
        );
        let terminal = launch_terminal(
            &working_directory.display().to_string(),
            &command,
            &TerminalTarget::New,
        )?;
        return Ok(NewSessionResult {
            provider,
            terminal: terminal.name,
            working_directory: working_directory.display().to_string(),
            highest_permissions,
            session_id,
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
    let session_id = new_session_id(provider);
    let command = build_new_session_command(
        provider,
        &executable,
        highest_permissions,
        session_id.as_deref(),
    );

    launch_terminal(&project.path, &command, &TerminalTarget::New).map(|terminal| {
        NewSessionResult {
            provider,
            terminal: terminal.name,
            working_directory: project.path.clone(),
            highest_permissions,
            session_id,
        }
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

fn open_session_key(session: &OpenSessionSummary) -> String {
    format!("{}:{}", session.provider, session.session_id)
}

fn unresolved_may_be_session(terminal: &ActiveTerminal, session: &OpenSessionSummary) -> bool {
    terminal.provider == Some(session.provider)
        && terminal
            .working_directory
            .as_deref()
            .is_some_and(|working_directory| {
                terminal::is_path_within(&session.project_path, working_directory)
                    || terminal::is_path_within(working_directory, &session.project_path)
                    || terminal::is_path_within(&session.working_directory, working_directory)
                    || terminal::is_path_within(working_directory, &session.working_directory)
            })
}

fn new_session_id(provider: SessionProvider) -> Option<String> {
    (provider == SessionProvider::Claude).then(|| Uuid::new_v4().to_string())
}

fn workspace_window_name(project_path: &str) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in terminal::normalize_path(project_path).bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("ccsm-{hash:016x}")
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
    target: &TerminalTarget,
) -> Result<TerminalLaunch, String> {
    let mut windows_terminal = Command::new("wt.exe");
    match target {
        TerminalTarget::New => {}
        TerminalTarget::LastUsed => {
            windows_terminal.arg("-w").arg("0");
        }
        TerminalTarget::Named(window_name) => {
            windows_terminal.arg("-w").arg(window_name);
        }
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
        .status();

    match windows_terminal {
        Ok(status) if status.success() => Ok(TerminalLaunch {
            name: "Windows Terminal".to_owned(),
            reused_window: !matches!(target, TerminalTarget::New),
        }),
        Ok(status) => Err(format!(
            "Windows Terminal 拒绝了启动请求（退出码 {}）",
            status
                .code()
                .map_or_else(|| "未知".to_owned(), |code| code.to_string())
        )),
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
    _target: &TerminalTarget,
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
    session_id: Option<&str>,
) -> String {
    let mut arguments = Vec::new();
    if provider == SessionProvider::Claude {
        if let Some(session_id) = session_id {
            arguments.extend(["--session-id", session_id]);
        }
        if highest_permissions {
            arguments.push("--dangerously-skip-permissions");
        }
    } else if highest_permissions {
        arguments.push("--yolo");
    }
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
                false,
                Some(SESSION_ID)
            ),
            "& 'C:\\Tools\\claude.exe' '--session-id' '00000000-0000-4000-8000-000000000001'"
        );
        assert_eq!(
            build_new_session_command(
                SessionProvider::Codex,
                Path::new("C:\\Tools\\codex.cmd"),
                true,
                None
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

    #[test]
    fn creates_a_stable_window_name_per_project() {
        assert_eq!(
            workspace_window_name("C:\\Code\\CCSM"),
            workspace_window_name("c:/code/ccsm/")
        );
        assert_ne!(
            workspace_window_name("C:\\Code\\CCSM"),
            workspace_window_name("C:\\Code\\Other")
        );
    }
}
