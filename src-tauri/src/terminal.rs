#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::models::SessionProvider;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalHost {
    WindowsTerminal,
    Console,
}

#[derive(Debug, Clone)]
pub struct ActiveTerminal {
    pub process_id: u32,
    pub started_at_ms: Option<i64>,
    pub provider: Option<SessionProvider>,
    pub session_id: Option<String>,
    pub working_directory: Option<String>,
    pub host: TerminalHost,
}

pub fn scan_active_terminals() -> Vec<ActiveTerminal> {
    #[cfg(all(target_os = "windows", target_pointer_width = "64"))]
    {
        scan_windows_terminals()
    }

    #[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
    {
        Vec::new()
    }
}

pub fn is_path_within(root: &str, candidate: &str) -> bool {
    let root = normalize_path(root);
    let candidate = normalize_path(candidate);
    if root.is_empty() || candidate.is_empty() {
        return false;
    }
    if root == candidate {
        return true;
    }

    let prefix = if root.ends_with('\\') {
        root
    } else {
        format!("{root}\\")
    };
    candidate.starts_with(&prefix)
}

fn command_tokens(command_line: &str) -> Vec<&str> {
    command_line
        .split(|character: char| {
            character.is_whitespace()
                || matches!(
                    character,
                    '\'' | '"' | '`' | ',' | ';' | '(' | ')' | '[' | ']' | '{' | '}'
                )
        })
        .filter(|token| !token.is_empty())
        .collect()
}

fn provider_session_id(provider: SessionProvider, command_line: &str) -> Option<String> {
    let tokens = command_tokens(command_line);
    for (index, token) in tokens.iter().enumerate() {
        let normalized = token.to_ascii_lowercase();
        let inline_value = normalized
            .strip_prefix("--resume=")
            .or_else(|| normalized.strip_prefix("--session-id="));
        if provider == SessionProvider::Claude {
            if let Some(value) = inline_value.and_then(valid_session_id) {
                return Some(value);
            }
            if matches!(normalized.as_str(), "--resume" | "-r" | "--session-id")
                && let Some(value) = tokens
                    .get(index + 1)
                    .and_then(|value| valid_session_id(value))
            {
                return Some(value);
            }
        }

        if provider == SessionProvider::Codex
            && matches!(normalized.as_str(), "resume" | "fork")
            && let Some(value) = tokens[index + 1..]
                .iter()
                .find_map(|value| valid_session_id(value))
        {
            return Some(value);
        }
    }
    None
}

fn valid_session_id(value: &str) -> Option<String> {
    Uuid::parse_str(value.trim_matches('='))
        .ok()
        .map(|id| id.to_string())
}

pub fn normalize_path(path: &str) -> String {
    let mut normalized = path.trim().replace('/', "\\").to_lowercase();
    if let Some(rest) = normalized.strip_prefix("\\\\?\\unc\\") {
        normalized = format!("\\\\{rest}");
    } else if let Some(rest) = normalized.strip_prefix("\\\\?\\") {
        normalized = rest.to_owned();
    } else if let Some(rest) = normalized.strip_prefix("\\??\\") {
        normalized = rest.to_owned();
    }

    while normalized.len() > 3 && normalized.ends_with('\\') {
        normalized.pop();
    }
    normalized
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
use std::{ffi::c_void, mem::size_of};

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
use windows_sys::{
    Wdk::System::Threading::{NtQueryInformationProcess, ProcessBasicInformation},
    Win32::{
        Foundation::{CloseHandle, FILETIME, HANDLE, HWND, INVALID_HANDLE_VALUE, LPARAM},
        System::{
            Diagnostics::{
                Debug::ReadProcessMemory,
                ToolHelp::{
                    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
                    TH32CS_SNAPPROCESS,
                },
            },
            Threading::{
                GetProcessTimes, OpenProcess, PEB, PROCESS_BASIC_INFORMATION,
                PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
            },
        },
        UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId, IsWindowVisible},
    },
};

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
#[derive(Debug)]
struct ProcessRecord {
    pid: u32,
    parent_pid: u32,
    name: String,
    command_line: Option<String>,
    current_directory: Option<String>,
    started_at_ms: Option<i64>,
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
struct OwnedHandle(HANDLE);

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RemoteUnicodeString {
    length: u16,
    maximum_length: u16,
    _padding: u32,
    buffer: usize,
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RemoteCurrentDirectory {
    dos_path: RemoteUnicodeString,
    handle: usize,
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
#[repr(C)]
#[derive(Clone, Copy)]
// The x64 prefix mirrors the private Windows structure up to CommandLine.
struct RemoteProcessParameters {
    _reserved: [u8; 0x38],
    current_directory: RemoteCurrentDirectory,
    _dll_path: RemoteUnicodeString,
    _image_path_name: RemoteUnicodeString,
    command_line: RemoteUnicodeString,
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
impl Default for RemoteProcessParameters {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
fn scan_windows_terminals() -> Vec<ActiveTerminal> {
    let records = snapshot_processes();
    let records_by_pid: HashMap<_, _> = records
        .into_iter()
        .map(|record| (record.pid, record))
        .collect();
    let provider_by_pid = records_by_pid
        .values()
        .filter_map(|record| provider_for_process(record).map(|provider| (record.pid, provider)))
        .collect::<HashMap<_, _>>();
    let mut terminals = Vec::new();

    for record in records_by_pid.values() {
        let provider = provider_by_pid.get(&record.pid).copied();
        if provider.is_none() && !is_shell_process(&record.name) {
            continue;
        }
        if provider.is_some_and(|candidate| {
            has_provider_descendant(record.pid, candidate, &records_by_pid, &provider_by_pid)
        }) {
            continue;
        }

        let Some(host) = terminal_host_for(record.pid, &records_by_pid) else {
            continue;
        };
        let working_directory = process_working_directory(record.pid, &records_by_pid);
        if provider.is_none() && working_directory.is_none() {
            continue;
        }

        terminals.push(ActiveTerminal {
            process_id: record.pid,
            started_at_ms: record.started_at_ms,
            provider,
            session_id: provider
                .and_then(|provider| session_id_for_process(record.pid, provider, &records_by_pid)),
            working_directory,
            host,
        });
    }

    terminals
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
fn snapshot_processes() -> Vec<ProcessRecord> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot.is_null() || snapshot == INVALID_HANDLE_VALUE {
        return Vec::new();
    }
    let _snapshot = OwnedHandle(snapshot);

    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut has_entry = unsafe { Process32FirstW(snapshot, &mut entry) } != 0;
    let mut records = Vec::new();
    while has_entry {
        let name = wide_string(&entry.szExeFile);
        let details =
            should_read_process_details(&name).then(|| read_process_details(entry.th32ProcessID));
        records.push(ProcessRecord {
            pid: entry.th32ProcessID,
            parent_pid: entry.th32ParentProcessID,
            name,
            command_line: details
                .as_ref()
                .and_then(|details| details.command_line.clone()),
            current_directory: details
                .as_ref()
                .and_then(|details| details.current_directory.clone()),
            started_at_ms: details.as_ref().and_then(|details| details.started_at_ms),
        });
        has_entry = unsafe { Process32NextW(snapshot, &mut entry) } != 0;
    }
    records
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
#[derive(Default)]
struct ProcessDetails {
    command_line: Option<String>,
    current_directory: Option<String>,
    started_at_ms: Option<i64>,
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
fn read_process_details(pid: u32) -> ProcessDetails {
    let handle =
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ, 0, pid) };
    if handle.is_null() {
        return ProcessDetails::default();
    }
    let _handle = OwnedHandle(handle);

    let mut basic_information = PROCESS_BASIC_INFORMATION::default();
    let mut return_length = 0_u32;
    let status = unsafe {
        NtQueryInformationProcess(
            handle,
            ProcessBasicInformation,
            &mut basic_information as *mut _ as *mut c_void,
            size_of::<PROCESS_BASIC_INFORMATION>() as u32,
            &mut return_length,
        )
    };
    if status < 0 || basic_information.PebBaseAddress.is_null() {
        return ProcessDetails::default();
    }

    let Some(peb) = read_process_memory::<PEB>(handle, basic_information.PebBaseAddress as usize)
    else {
        return ProcessDetails::default();
    };
    if peb.ProcessParameters.is_null() {
        return ProcessDetails::default();
    }

    let Some(parameters) =
        read_process_memory::<RemoteProcessParameters>(handle, peb.ProcessParameters as usize)
    else {
        return ProcessDetails::default();
    };

    ProcessDetails {
        command_line: read_remote_text(handle, parameters.command_line),
        current_directory: read_remote_text(handle, parameters.current_directory.dos_path),
        started_at_ms: process_started_at_ms(handle),
    }
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
fn process_started_at_ms(handle: HANDLE) -> Option<i64> {
    let mut created = FILETIME::default();
    let mut exited = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    let success =
        unsafe { GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user) };
    if success == 0 {
        return None;
    }

    let windows_ticks =
        (u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime);
    const WINDOWS_TO_UNIX_EPOCH_TICKS: u64 = 116_444_736_000_000_000;
    windows_ticks
        .checked_sub(WINDOWS_TO_UNIX_EPOCH_TICKS)
        .and_then(|ticks| i64::try_from(ticks / 10_000).ok())
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
fn read_process_memory<T: Copy + Default>(handle: HANDLE, address: usize) -> Option<T> {
    if address == 0 {
        return None;
    }
    let mut value = T::default();
    let mut bytes_read = 0_usize;
    let success = unsafe {
        ReadProcessMemory(
            handle,
            address as *const c_void,
            &mut value as *mut T as *mut c_void,
            size_of::<T>(),
            &mut bytes_read,
        )
    };
    (success != 0 && bytes_read == size_of::<T>()).then_some(value)
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
fn read_remote_text(handle: HANDLE, value: RemoteUnicodeString) -> Option<String> {
    let length = usize::from(value.length);
    if length == 0 || length % 2 != 0 || length > 32 * 1024 || value.buffer == 0 {
        return None;
    }

    let mut buffer = vec![0_u16; length / 2];
    let mut bytes_read = 0_usize;
    let success = unsafe {
        ReadProcessMemory(
            handle,
            value.buffer as *const c_void,
            buffer.as_mut_ptr() as *mut c_void,
            length,
            &mut bytes_read,
        )
    };
    if success == 0 || bytes_read < length {
        return None;
    }
    Some(String::from_utf16_lossy(&buffer))
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
fn wide_string(value: &[u16]) -> String {
    let length = value
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(value.len());
    String::from_utf16_lossy(&value[..length])
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
fn should_read_process_details(name: &str) -> bool {
    let normalized = process_name(name);
    is_shell_process(&normalized)
        || matches!(
            normalized.as_str(),
            "claude.exe" | "codex.exe" | "node.exe" | "bun.exe" | "deno.exe"
        )
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
fn provider_for_process(record: &ProcessRecord) -> Option<SessionProvider> {
    let name = process_name(&record.name);
    if is_shell_process(&name) {
        return None;
    }
    match name.as_str() {
        "claude.exe" | "claude" => return Some(SessionProvider::Claude),
        "codex.exe" | "codex" => return Some(SessionProvider::Codex),
        _ => {}
    }

    let command_line = record.command_line.as_deref()?;
    if command_line_mentions_tool(command_line, "claude") {
        return Some(SessionProvider::Claude);
    }
    if command_line_mentions_tool(command_line, "codex") {
        return Some(SessionProvider::Codex);
    }
    None
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
fn command_line_mentions_tool(command_line: &str, tool: &str) -> bool {
    let command_line = command_line.to_ascii_lowercase();
    let markers = [
        format!("\\{tool}.exe"),
        format!("/{tool}.exe"),
        format!("\\{tool}.cmd"),
        format!("/{tool}.cmd"),
        format!("\\{tool}.bat"),
        format!("/{tool}.bat"),
        format!("'{tool}'"),
        format!("\"{tool}\""),
        format!(" {tool} "),
        format!("&{tool}"),
        format!("& {tool}"),
    ];
    if markers.iter().any(|marker| command_line.contains(marker)) {
        return true;
    }

    match tool {
        "claude" => command_line.contains("claude-code"),
        "codex" => command_line.contains("@openai/codex") || command_line.contains("codex-cli"),
        _ => false,
    }
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
fn is_shell_process(name: &str) -> bool {
    matches!(
        process_name(name).as_str(),
        "powershell.exe"
            | "powershell"
            | "pwsh.exe"
            | "pwsh"
            | "cmd.exe"
            | "cmd"
            | "bash.exe"
            | "bash"
            | "wsl.exe"
            | "wsl"
            | "zsh.exe"
            | "zsh"
            | "fish.exe"
            | "fish"
    )
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
fn process_name(name: &str) -> String {
    name.rsplit(['\\', '/'])
        .next()
        .unwrap_or(name)
        .to_ascii_lowercase()
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
fn terminal_host_for(pid: u32, records: &HashMap<u32, ProcessRecord>) -> Option<TerminalHost> {
    let mut current_pid = pid;
    let mut visited = HashSet::new();
    let mut saw_console = false;

    for _ in 0..64 {
        if !visited.insert(current_pid) {
            break;
        }
        let Some(record) = records.get(&current_pid) else {
            break;
        };
        match process_name(&record.name).as_str() {
            "windowsterminal.exe" | "windowsterminal" => {
                return Some(TerminalHost::WindowsTerminal);
            }
            "conhost.exe" | "conhost" | "openconsole.exe" | "openconsolehost.exe" => {
                saw_console = true;
            }
            name if is_shell_process(name) => {
                saw_console = true;
            }
            _ => {}
        }

        if record.parent_pid == 0 || record.parent_pid == current_pid {
            break;
        }
        current_pid = record.parent_pid;
    }

    saw_console.then_some(TerminalHost::Console)
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
fn session_id_for_process(
    pid: u32,
    provider: SessionProvider,
    records: &HashMap<u32, ProcessRecord>,
) -> Option<String> {
    let mut current_pid = pid;
    let mut visited = HashSet::new();
    for _ in 0..64 {
        if !visited.insert(current_pid) {
            break;
        }
        let record = records.get(&current_pid)?;
        if let Some(session_id) = record
            .command_line
            .as_deref()
            .and_then(|command_line| provider_session_id(provider, command_line))
        {
            return Some(session_id);
        }
        if matches!(
            process_name(&record.name).as_str(),
            "windowsterminal.exe" | "windowsterminal" | "openconsole.exe" | "openconsolehost.exe"
        ) {
            break;
        }
        if record.parent_pid == 0 || record.parent_pid == current_pid {
            break;
        }
        current_pid = record.parent_pid;
    }
    None
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
fn has_provider_descendant(
    ancestor_pid: u32,
    provider: SessionProvider,
    records: &HashMap<u32, ProcessRecord>,
    provider_by_pid: &HashMap<u32, SessionProvider>,
) -> bool {
    provider_by_pid
        .iter()
        .any(|(candidate_pid, candidate_provider)| {
            *candidate_pid != ancestor_pid
                && *candidate_provider == provider
                && is_process_ancestor(ancestor_pid, *candidate_pid, records)
        })
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
fn is_process_ancestor(
    ancestor_pid: u32,
    child_pid: u32,
    records: &HashMap<u32, ProcessRecord>,
) -> bool {
    let mut current_pid = child_pid;
    let mut visited = HashSet::new();
    for _ in 0..64 {
        if !visited.insert(current_pid) {
            return false;
        }
        let Some(record) = records.get(&current_pid) else {
            return false;
        };
        if record.parent_pid == ancestor_pid {
            return true;
        }
        if record.parent_pid == 0 || record.parent_pid == current_pid {
            return false;
        }
        current_pid = record.parent_pid;
    }
    false
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
fn process_working_directory(pid: u32, records: &HashMap<u32, ProcessRecord>) -> Option<String> {
    let mut current_pid = pid;
    let mut visited = HashSet::new();
    for _ in 0..64 {
        if !visited.insert(current_pid) {
            break;
        }
        let Some(record) = records.get(&current_pid) else {
            break;
        };
        if let Some(directory) = record
            .current_directory
            .as_deref()
            .filter(|directory| !directory.trim().is_empty())
        {
            return Some(directory.to_owned());
        }
        if record.parent_pid == 0 || record.parent_pid == current_pid {
            break;
        }
        current_pid = record.parent_pid;
    }
    None
}

pub fn windows_terminal_window_count() -> usize {
    #[cfg(all(target_os = "windows", target_pointer_width = "64"))]
    {
        let records = snapshot_processes();
        let process_ids = records
            .iter()
            .filter(|record| {
                matches!(
                    process_name(&record.name).as_str(),
                    "windowsterminal.exe" | "windowsterminal"
                )
            })
            .map(|record| record.pid)
            .collect::<HashSet<_>>();
        let mut context = WindowCountContext {
            process_ids: &process_ids,
            count: 0,
        };
        unsafe {
            EnumWindows(
                Some(count_windows_terminal_window),
                &mut context as *mut WindowCountContext<'_> as LPARAM,
            );
        }
        context.count
    }

    #[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
    {
        0
    }
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
struct WindowCountContext<'a> {
    process_ids: &'a HashSet<u32>,
    count: usize,
}

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
unsafe extern "system" fn count_windows_terminal_window(window: HWND, parameter: LPARAM) -> i32 {
    if unsafe { IsWindowVisible(window) } == 0 {
        return 1;
    }
    let mut process_id = 0_u32;
    unsafe {
        GetWindowThreadProcessId(window, &mut process_id);
    }
    let context = unsafe { &mut *(parameter as *mut WindowCountContext<'_>) };
    if context.process_ids.contains(&process_id) {
        context.count += 1;
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "windows")]
    use std::{fs, os::windows::process::CommandExt, process::Command, thread, time::Duration};

    #[test]
    fn matches_a_project_directory_and_its_descendants() {
        assert!(is_path_within("C:\\Code\\CCSM", "c:/code/ccsm/src"));
        assert!(is_path_within("C:\\Code\\CCSM", "C:\\code\\ccsm"));
        assert!(!is_path_within("C:\\Code\\CCSM", "C:\\Code\\CCSM-old"));
        assert!(!is_path_within("C:\\Code\\CCSM", "C:\\Code\\Other"));
    }

    #[test]
    fn handles_extended_windows_paths() {
        assert!(is_path_within(
            "C:\\Code\\CCSM",
            "\\\\?\\C:\\Code\\CCSM\\src"
        ));
        assert!(is_path_within(
            "\\\\server\\share",
            "\\\\?\\UNC\\server\\share\\repo"
        ));
    }

    #[test]
    fn extracts_a_uuid_from_a_cli_command() {
        assert_eq!(
            provider_session_id(
                SessionProvider::Codex,
                "& 'C:\\Tools\\codex.exe' 'resume' '00000000-0000-4000-8000-000000000001'"
            ),
            Some("00000000-0000-4000-8000-000000000001".to_owned())
        );
        assert_eq!(
            provider_session_id(
                SessionProvider::Claude,
                "claude --resume=00000000-0000-4000-8000-000000000002"
            ),
            Some("00000000-0000-4000-8000-000000000002".to_owned())
        );
    }

    #[cfg(all(target_os = "windows", target_pointer_width = "64"))]
    #[test]
    fn finds_a_resume_id_in_the_parent_command() {
        let session_id = "00000000-0000-4000-8000-000000000003";
        let records = [
            ProcessRecord {
                pid: 10,
                parent_pid: 20,
                name: "claude.exe".to_owned(),
                command_line: Some("claude.exe --permission-mode default".to_owned()),
                current_directory: Some("C:\\code\\demo".to_owned()),
                started_at_ms: Some(1),
            },
            ProcessRecord {
                pid: 20,
                parent_pid: 30,
                name: "powershell.exe".to_owned(),
                command_line: Some(format!("claude.exe --resume {session_id}")),
                current_directory: Some("C:\\code\\demo".to_owned()),
                started_at_ms: Some(1),
            },
            ProcessRecord {
                pid: 30,
                parent_pid: 0,
                name: "WindowsTerminal.exe".to_owned(),
                command_line: None,
                current_directory: None,
                started_at_ms: Some(1),
            },
        ]
        .into_iter()
        .map(|record| (record.pid, record))
        .collect::<HashMap<_, _>>();

        assert_eq!(
            session_id_for_process(10, SessionProvider::Claude, &records),
            Some(session_id.to_owned())
        );
    }

    #[cfg(all(target_os = "windows", target_pointer_width = "64"))]
    #[test]
    fn keeps_the_windows_current_directory_layout() {
        assert_eq!(
            std::mem::offset_of!(RemoteProcessParameters, current_directory),
            0x38
        );
        assert_eq!(std::mem::offset_of!(RemoteCurrentDirectory, dos_path), 0);
        assert_eq!(
            std::mem::offset_of!(RemoteCurrentDirectory, handle),
            std::mem::size_of::<RemoteUnicodeString>()
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn reads_a_real_powershell_working_directory() {
        let root = std::env::temp_dir().join(format!("ccsm-terminal-scan-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create terminal scan fixture");
        let mut process = Command::new("powershell.exe")
            .current_dir(&root)
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Start-Sleep -Seconds 15",
            ])
            .creation_flags(0x0800_0000)
            .spawn()
            .expect("start powershell fixture");

        let mut detected_directory = None;
        for _ in 0..30 {
            detected_directory = scan_active_terminals()
                .into_iter()
                .find(|terminal| terminal.process_id == process.id())
                .and_then(|terminal| terminal.working_directory);
            if detected_directory.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }

        process.kill().expect("stop powershell fixture");
        process.wait().expect("wait for powershell fixture");
        fs::remove_dir_all(&root).expect("remove terminal scan fixture");

        assert_eq!(
            detected_directory.map(|path| normalize_path(&path)),
            Some(normalize_path(&root.display().to_string()))
        );
    }
}
