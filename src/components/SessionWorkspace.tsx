import { AlertCircle, MessageSquareOff, RefreshCw, ShieldAlert } from "lucide-react";
import { Button, Spinner } from "@heroui/react";

import type { ProjectSummary, SessionProvider, SessionSort, SessionSummary } from "../types";
import { SessionList } from "./SessionList";
import { SessionToolbar } from "./SessionToolbar";

interface SessionWorkspaceProps {
  error: string | null;
  highestPermissions: boolean;
  initialLoading: boolean;
  launchingSessionKey: string | null;
  permissionWarning: string | null;
  provider: SessionProvider;
  searchQuery: string;
  searching: boolean;
  selectedProject: ProjectSummary | null;
  sessions: SessionSummary[];
  showArchived: boolean;
  sort: SessionSort;
  onCopyId: (sessionId: string) => void;
  onHighestPermissionsChange: (enabled: boolean) => void;
  onRefresh: () => void;
  onResume: (session: SessionSummary, fork: boolean) => void;
  onShowArchivedChange: (show: boolean) => void;
  onSortChange: (sort: SessionSort) => void;
}

export function SessionWorkspace({
  error,
  highestPermissions,
  initialLoading,
  launchingSessionKey,
  permissionWarning,
  provider,
  searchQuery,
  searching,
  selectedProject,
  sessions,
  showArchived,
  sort,
  onCopyId,
  onHighestPermissionsChange,
  onRefresh,
  onResume,
  onShowArchivedChange,
  onSortChange,
}: SessionWorkspaceProps) {
  return (
    <main className="main-workspace">
      <SessionToolbar
        selectedProject={selectedProject}
        provider={provider}
        searching={searching}
        resultCount={sessions.length}
        sort={sort}
        showArchived={showArchived}
        highestPermissions={highestPermissions}
        onSortChange={onSortChange}
        onShowArchivedChange={onShowArchivedChange}
        onHighestPermissionsChange={onHighestPermissionsChange}
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
          {searching && <span className="active-filter">“{searchQuery}”</span>}
        </div>

        {error ? (
          <div className="empty-state">
            <span className="empty-state-icon error" aria-hidden="true">
              <AlertCircle />
            </span>
            <h3>无法读取本机会话</h3>
            <p>{error}</p>
            <Button variant="outline" size="sm" className="retry-button" onPress={onRefresh}>
              <RefreshCw aria-hidden="true" />
              重试
            </Button>
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
            sessions={sessions}
            sort={sort}
            showProject={searching}
            launchingSessionKey={launchingSessionKey}
            onResume={onResume}
            onCopyId={onCopyId}
          />
        )}
      </section>
    </main>
  );
}
