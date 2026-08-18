import { AlertCircle, LoaderCircle, MessageSquareOff, Plus, ShieldAlert } from "lucide-react";
import { Spinner } from "@heroui/react";

import type { SessionKey } from "../lib/archive";
import type { ProjectSummary, SessionProvider, SessionSort, SessionSummary } from "../types";
import { SessionList } from "./SessionList";
import { SessionToolbar } from "./SessionToolbar";

interface SessionWorkspaceProps {
  activeSelectionCount: number;
  archivedSelectionCount: number;
  archivedSessionKeys: ReadonlySet<SessionKey>;
  error: string | null;
  highestPermissions: boolean;
  initialLoading: boolean;
  newSessionLaunching: boolean;
  launchingSessionKey: string | null;
  permissionWarning: string | null;
  provider: SessionProvider;
  searchQuery: string;
  searching: boolean;
  selectedCount: number;
  selectedSessionKeys: ReadonlySet<SessionKey>;
  selectedProject: ProjectSummary | null;
  sessions: SessionSummary[];
  selectionMode: boolean;
  showArchived: boolean;
  sort: SessionSort;
  onArchive: (session: SessionSummary, archived: boolean) => void;
  onArchiveSelected: () => void;
  onCopyId: (sessionId: string) => void;
  onClearSelection: () => void;
  onHighestPermissionsChange: (enabled: boolean) => void;
  onNewSession: () => void;
  onResume: (session: SessionSummary, fork: boolean) => void;
  onSelectAll: () => void;
  onShowArchivedChange: (show: boolean) => void;
  onSortChange: (sort: SessionSort) => void;
  onToggleSelection: (session: SessionSummary) => void;
  onToggleSelectionMode: () => void;
  onUnarchiveSelected: () => void;
}

export function SessionWorkspace({
  activeSelectionCount,
  archivedSelectionCount,
  archivedSessionKeys,
  error,
  highestPermissions,
  initialLoading,
  newSessionLaunching,
  launchingSessionKey,
  permissionWarning,
  provider,
  searchQuery,
  searching,
  selectedCount,
  selectedSessionKeys,
  selectedProject,
  sessions,
  selectionMode,
  showArchived,
  sort,
  onArchive,
  onArchiveSelected,
  onCopyId,
  onClearSelection,
  onHighestPermissionsChange,
  onNewSession,
  onResume,
  onSelectAll,
  onShowArchivedChange,
  onSortChange,
  onToggleSelection,
  onToggleSelectionMode,
  onUnarchiveSelected,
}: SessionWorkspaceProps) {
  return (
    <main className="main-workspace">
      <SessionToolbar
        activeSelectionCount={activeSelectionCount}
        archivedSelectionCount={archivedSelectionCount}
        selectedProject={selectedProject}
        provider={provider}
        searching={searching}
        resultCount={sessions.length}
        selectedCount={selectedCount}
        selectionMode={selectionMode}
        sort={sort}
        showArchived={showArchived}
        highestPermissions={highestPermissions}
        onArchiveSelected={onArchiveSelected}
        onClearSelection={onClearSelection}
        onSortChange={onSortChange}
        onSelectAll={onSelectAll}
        onShowArchivedChange={onShowArchivedChange}
        onHighestPermissionsChange={onHighestPermissionsChange}
        onToggleSelectionMode={onToggleSelectionMode}
        onUnarchiveSelected={onUnarchiveSelected}
      />

      {permissionWarning && (
        <div className="permission-warning" role="status">
          <ShieldAlert aria-hidden="true" />
          <span>{permissionWarning}</span>
        </div>
      )}

      <section className="session-content" aria-label="会话列表">
        <div className="content-heading">
          <div>
            <h2>{searching ? "搜索结果" : "会话"}</h2>
            <p>{`${sessions.length} 个${showArchived ? "匹配" : "可续接"}会话`}</p>
          </div>
          <div className="content-heading-actions">
            {searching && <span className="active-filter">“{searchQuery}”</span>}
            <button
              className="new-session-button"
              type="button"
              disabled={!selectedProject || searching || newSessionLaunching}
              title={
                searching
                  ? "请先退出搜索，再创建新会话"
                  : selectedProject
                    ? `在 ${selectedProject.name} 中创建新会话`
                    : "请选择一个项目"
              }
              aria-label="新会话"
              onClick={onNewSession}
            >
              {newSessionLaunching ? (
                <LoaderCircle className="is-spinning" aria-hidden="true" />
              ) : (
                <Plus aria-hidden="true" />
              )}
              <span>{newSessionLaunching ? "启动中" : "新会话"}</span>
            </button>
          </div>
        </div>

        {error ? (
          <div className="empty-state">
            <span className="empty-state-icon error" aria-hidden="true">
              <AlertCircle />
            </span>
            <h3>无法读取本机会话</h3>
            <p>{error}</p>
          </div>
        ) : initialLoading ? (
          <div className="loading-state" role="status">
            <Spinner size="sm" />
            <span>正在扫描本机会话...</span>
          </div>
        ) : sessions.length === 0 ? (
          <div className="empty-state">
            <span className="empty-state-icon" aria-hidden="true">
              <MessageSquareOff />
            </span>
            <h3>没有可显示的会话</h3>
            <p>{searching ? "换个关键词再试一次。" : "当前筛选条件下没有会话。"}</p>
          </div>
        ) : (
          <SessionList
            archivedSessionKeys={archivedSessionKeys}
            sessions={sessions}
            sort={sort}
            selectedSessionKeys={selectedSessionKeys}
            selectionMode={selectionMode}
            showProject={searching}
            launchingSessionKey={launchingSessionKey}
            onArchive={onArchive}
            onResume={onResume}
            onCopyId={onCopyId}
            onToggleSelection={onToggleSelection}
          />
        )}
      </section>
    </main>
  );
}
