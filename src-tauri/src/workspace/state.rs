use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::models::OpenSessionSummary;

pub const WORKSPACE_STATE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspaceState {
    pub version: u32,
    pub snapshots: BTreeMap<String, WorkspaceSnapshot>,
    pub known_sessions: BTreeSet<String>,
    pub process_bindings: BTreeMap<String, String>,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        Self {
            version: WORKSPACE_STATE_VERSION,
            snapshots: BTreeMap::new(),
            known_sessions: BTreeSet::new(),
            process_bindings: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspaceSnapshot {
    pub project_path: String,
    pub captured_at: String,
    pub sessions: Vec<OpenSessionSummary>,
    pub window_name: Option<String>,
    pub pending_sessions: Vec<OpenSessionSummary>,
    pub pending_observations: u8,
}

pub struct WorkspaceRepository {
    path: PathBuf,
}

impl WorkspaceRepository {
    pub fn local() -> Result<Self, String> {
        let root =
            dirs::data_local_dir().ok_or_else(|| "无法确定 Windows 本地应用数据目录".to_owned())?;
        Ok(Self {
            path: root.join("CCSM").join("workspace-state.json"),
        })
    }

    #[cfg(test)]
    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Result<WorkspaceState, String> {
        if !self.path.is_file() {
            let backup_path = backup_path(&self.path);
            return if backup_path.is_file() {
                read_state(&backup_path)
            } else {
                Ok(WorkspaceState::default())
            };
        }

        match read_state(&self.path) {
            Ok(state) => Ok(state),
            Err(primary_error) => {
                let backup_path = backup_path(&self.path);
                read_state(&backup_path).map_err(|backup_error| {
                    format!("工作区快照已损坏：{primary_error}；备份也无法读取：{backup_error}")
                })
            }
        }
    }

    pub fn save(&self, state: &WorkspaceState) -> Result<(), String> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| "工作区快照路径无效".to_owned())?;
        fs::create_dir_all(parent).map_err(|error| format!("无法创建工作区快照目录：{error}"))?;

        let bytes = serde_json::to_vec_pretty(state)
            .map_err(|error| format!("无法序列化工作区快照：{error}"))?;
        let next_path = next_path(&self.path);
        let mut next_file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&next_path)
            .map_err(|error| format!("无法创建工作区快照临时文件：{error}"))?;
        next_file
            .write_all(&bytes)
            .and_then(|()| next_file.sync_all())
            .map_err(|error| format!("无法写入工作区快照：{error}"))?;

        if self.path.is_file() {
            fs::copy(&self.path, backup_path(&self.path))
                .map_err(|error| format!("无法备份工作区快照：{error}"))?;
            fs::remove_file(&self.path).map_err(|error| format!("无法替换工作区快照：{error}"))?;
        }
        fs::rename(&next_path, &self.path).map_err(|error| format!("无法保存工作区快照：{error}"))
    }
}

fn read_state(path: &Path) -> Result<WorkspaceState, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}：{error}", path.display()))?;
    let state = serde_json::from_slice::<WorkspaceState>(&bytes)
        .map_err(|error| format!("{}：{error}", path.display()))?;
    if state.version != WORKSPACE_STATE_VERSION {
        return Err(format!(
            "{}：不支持的快照版本 {}",
            path.display(),
            state.version
        ));
    }
    Ok(state)
}

fn backup_path(path: &Path) -> PathBuf {
    path.with_extension("json.bak")
}

fn next_path(path: &Path) -> PathBuf {
    path.with_extension("json.next")
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn persists_and_recovers_workspace_state() {
        let root = std::env::temp_dir().join(format!("ccsm-workspace-state-{}", Uuid::new_v4()));
        let repository = WorkspaceRepository::at(root.join("workspace-state.json"));
        let mut state = WorkspaceState::default();
        state.known_sessions.insert("claude:session-1".to_owned());

        repository.save(&state).expect("save workspace state");
        let loaded = repository.load().expect("load workspace state");

        assert!(loaded.known_sessions.contains("claude:session-1"));
        fs::remove_dir_all(root).expect("remove workspace state fixture");
    }

    #[test]
    fn loads_the_backup_when_a_replacement_was_interrupted() {
        let root = std::env::temp_dir().join(format!("ccsm-workspace-backup-{}", Uuid::new_v4()));
        let path = root.join("workspace-state.json");
        let repository = WorkspaceRepository::at(path.clone());
        let mut first = WorkspaceState::default();
        first.known_sessions.insert("claude:session-1".to_owned());
        repository.save(&first).expect("save first state");

        let mut second = first.clone();
        second.known_sessions.insert("claude:session-2".to_owned());
        repository.save(&second).expect("save second state");
        fs::remove_file(path).expect("simulate interrupted replacement");

        let recovered = repository.load().expect("load backup state");
        assert!(recovered.known_sessions.contains("claude:session-1"));
        fs::remove_dir_all(root).expect("remove workspace backup fixture");
    }
}
