use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use serde_json::Value;
use uuid::Uuid;

use crate::{
    catalog,
    models::{CliStatus, LaunchResult, SessionProvider},
};

#[cfg(target_os = "windows")]
const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

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

    launch_terminal(&session.project_path, &command).map(|terminal| LaunchResult {
        session_id,
        provider,
        terminal,
        forked: fork,
        highest_permissions,
    })
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
fn launch_terminal(project_path: &str, command: &str) -> Result<String, String> {
    let windows_terminal = Command::new("wt.exe")
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
        Ok(_) => Ok("Windows Terminal".to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Command::new("powershell.exe")
                .current_dir(project_path)
                .arg("-NoLogo")
                .arg("-NoExit")
                .arg("-Command")
                .arg(command)
                .creation_flags(CREATE_NEW_CONSOLE)
                .spawn()
                .map(|_| "PowerShell".to_owned())
                .map_err(|fallback_error| format!("无法启动终端：{fallback_error}"))
        }
        Err(error) => Err(format!("无法启动 Windows Terminal：{error}")),
    }
}

#[cfg(not(target_os = "windows"))]
fn launch_terminal(_project_path: &str, _command: &str) -> Result<String, String> {
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
    fn sorts_bundled_versions_numerically() {
        assert!(version_key("2.10.0") > version_key("2.9.9"));
    }
}
