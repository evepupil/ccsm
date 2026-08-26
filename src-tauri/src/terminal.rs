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

pub fn extract_session_id(command_line: &str) -> Option<String> {
    for token in command_line.split(|character: char| {
        character.is_whitespace()
            || matches!(
                character,
                '\'' | '"' | '`' | ',' | ';' | '(' | ')' | '[' | ']' | '{' | '}'
            )
    }) {
        if token.is_empty() {
            continue;
        }
        let candidate = token.rsplit_once('=').map_or(token, |(_, value)| value);
        if let Ok(session_id) = Uuid::parse_str(candidate.trim_matches('=')) {
            return Some(session_id.to_string());
        }
    }
    None
}

fn normalize_path(path: &str) -> String {
    let mut normalized = path.trim().replace('/', "\\").to_ascii_lowercase();
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
        Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE},
        System::{
            Diagnostics::{
                Debug::ReadProcessMemory,
                ToolHelp::{
                    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
                    TH32CS_SNAPPROCESS,
                },
            },
            Threading::{
                OpenProcess, PEB, PROCESS_BASIC_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION,
                PROCESS_VM_READ,
            },
        },
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
    handle: usize,
    dos_path: RemoteUnicodeString,
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
    let mut terminals = Vec::new();

    for record in records_by_pid.values() {
        let provider = provider_for_process(record);
        if provider.is_none() && !is_shell_process(&record.name) {
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
            provider,
            session_id: record.command_line.as_deref().and_then(extract_session_id),
            working_directory,
            host,
        });
    }

    deduplicate_terminals(terminals)
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
            current_directory: details.and_then(|details| details.current_directory),
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
    }
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

#[cfg(all(target_os = "windows", target_pointer_width = "64"))]
fn deduplicate_terminals(terminals: Vec<ActiveTerminal>) -> Vec<ActiveTerminal> {
    let mut deduplicated = Vec::new();
    for terminal in terminals {
        let duplicate = deduplicated.iter().position(|existing: &ActiveTerminal| {
            existing.provider == terminal.provider
                && existing.session_id == terminal.session_id
                && existing.working_directory == terminal.working_directory
                && existing.host == terminal.host
        });
        if duplicate.is_none() {
            deduplicated.push(terminal);
        }
    }
    deduplicated
}

#[cfg(test)]
mod tests {
    use super::*;

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
            extract_session_id(
                "& 'C:\\Tools\\codex.exe' 'resume' '00000000-0000-4000-8000-000000000001'"
            ),
            Some("00000000-0000-4000-8000-000000000001".to_owned())
        );
        assert_eq!(
            extract_session_id("claude --resume=00000000-0000-4000-8000-000000000002"),
            Some("00000000-0000-4000-8000-000000000002".to_owned())
        );
    }
}
