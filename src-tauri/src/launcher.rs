use std::{path::Path, process::Command};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use serde_json::Value;
use uuid::Uuid;

use crate::{
    models::{CliStatus, LaunchResult},
    sessions,
};

#[cfg(target_os = "windows")]
const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn cli_status() -> CliStatus {
    let Some(version) = command_output(&["--version"]) else {
        return CliStatus {
            available: false,
            version: None,
            logged_in: None,
            auth_method: None,
            api_provider: None,
        };
    };

    let auth = command_output(&["auth", "status"])
        .and_then(|output| serde_json::from_str::<Value>(&output).ok());

    CliStatus {
        available: true,
        version: Some(version.trim().to_owned()),
        logged_in: auth
            .as_ref()
            .and_then(|value| value.get("loggedIn"))
            .and_then(Value::as_bool),
        auth_method: auth
            .as_ref()
            .and_then(|value| value.get("authMethod"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        api_provider: auth
            .as_ref()
            .and_then(|value| value.get("apiProvider"))
            .and_then(Value::as_str)
            .map(str::to_owned),
    }
}

pub fn resume_session(session_id: &str, fork: bool) -> Result<LaunchResult, String> {
    let session_id = Uuid::parse_str(session_id)
        .map_err(|_| "Session ID 格式无效".to_owned())?
        .to_string();
    let catalog = sessions::scan_sessions().map_err(|error| error.to_string())?;
    let session = catalog
        .projects
        .iter()
        .flat_map(|project| project.sessions.iter())
        .find(|session| session.id == session_id)
        .ok_or_else(|| "本机 Claude 会话索引中找不到该 Session ID".to_owned())?;

    if !session.can_resume || !Path::new(&session.project_path).is_dir() {
        return Err(format!("项目目录已不存在：{}", session.project_path));
    }

    launch_terminal(&session.project_path, &session_id, fork).map(|terminal| LaunchResult {
        session_id,
        terminal,
        forked: fork,
    })
}

#[cfg(target_os = "windows")]
fn launch_terminal(project_path: &str, session_id: &str, fork: bool) -> Result<String, String> {
    let command = build_resume_command(session_id, fork);

    let windows_terminal = Command::new("wt.exe")
        .arg("new-tab")
        .arg("--startingDirectory")
        .arg(project_path)
        .arg("powershell.exe")
        .arg("-NoLogo")
        .arg("-NoExit")
        .arg("-Command")
        .arg(&command)
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
fn launch_terminal(_project_path: &str, _session_id: &str, _fork: bool) -> Result<String, String> {
    Err("当前版本仅支持 Windows".to_owned())
}

fn command_output(arguments: &[&str]) -> Option<String> {
    let mut command = Command::new("claude");
    command.args(arguments);
    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW);

    let output = command.output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!stdout.is_empty()).then_some(stdout)
}

fn build_resume_command(session_id: &str, fork: bool) -> String {
    if fork {
        format!("claude --resume {session_id} --fork-session")
    } else {
        format!("claude --resume {session_id}")
    }
}

#[cfg(test)]
mod tests {
    use super::build_resume_command;

    #[test]
    fn builds_resume_command() {
        assert_eq!(
            build_resume_command("00000000-0000-4000-8000-000000000001", false),
            "claude --resume 00000000-0000-4000-8000-000000000001"
        );
    }

    #[test]
    fn builds_fork_command() {
        assert_eq!(
            build_resume_command("00000000-0000-4000-8000-000000000001", true),
            "claude --resume 00000000-0000-4000-8000-000000000001 --fork-session"
        );
    }
}
