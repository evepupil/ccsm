import {
  AlertCircle,
  CheckCircle2,
  Database,
  RefreshCw,
  Search,
  ShieldAlert,
  TerminalSquare,
  X,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";

import { resumeSession, scanSessionCatalog } from "./api";
import { ProjectList } from "./components/ProjectTree";
import { SessionTable } from "./components/SessionTable";
import { formatAbsoluteTime, normalizeSearch } from "./lib/format";
import { highestPermissionWarning, launchSessionKey } from "./lib/launch";
import { selectVisibleSessions } from "./lib/sessions";
import type { ProjectSummary, SessionProvider, SessionSort, SessionSummary } from "./types";

interface Notice {
  kind: "success" | "error";
  message: string;
}

export default function App() {
  const [catalog, setCatalog] = useState<Awaited<ReturnType<typeof scanSessionCatalog>> | null>(
    null,
  );
  const [provider, setProvider] = useState<SessionProvider>("claude");
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(() =>
    localStorage.getItem("selectedProjectId"),
  );
  const [searchQuery, setSearchQuery] = useState("");
  const [sort, setSort] = useState<SessionSort>("recent");
  const [showArchived, setShowArchived] = useState(false);
  const [highestPermissions, setHighestPermissions] = useState(false);
  const [loading, setLoading] = useState(true);
  const [launchingKey, setLaunchingKey] = useState<string | null>(null);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const nextCatalog = await scanSessionCatalog();
      setCatalog(nextCatalog);
      setSelectedProjectId((current) => {
        const stillExists = nextCatalog.projects.some((project) => project.id === current);
        const nextId = stillExists ? current : (nextCatalog.projects[0]?.id ?? null);
        if (nextId) localStorage.setItem("selectedProjectId", nextId);
        return nextId;
      });
    } catch (cause) {
      setError(toErrorMessage(cause));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const selectedProject = useMemo(
    () => catalog?.projects.find((project) => project.id === selectedProjectId) ?? null,
    [catalog, selectedProjectId],
  );

  const normalizedQuery = normalizeSearch(searchQuery);
  const sessions = useMemo(
    () =>
      selectVisibleSessions({
        projects: catalog?.projects ?? [],
        selectedProjectId,
        searchQuery,
        providerFilter: provider,
        showArchived,
        sort,
      }),
    [catalog, provider, searchQuery, selectedProjectId, showArchived, sort],
  );
  const sessionCount =
    catalog?.projects.reduce((total, project) => total + project.sessionCount, 0) ?? 0;
  const permissionWarning = highestPermissionWarning(highestPermissions);

  function selectProject(projectId: string) {
    setSelectedProjectId(projectId);
    localStorage.setItem("selectedProjectId", projectId);
    if (searchQuery) setSearchQuery("");
  }

  function switchProvider(next: SessionProvider) {
    setProvider(next);
    setSelectedProjectId((current) => {
      const projects = catalog?.projects ?? [];
      const currentProject = projects.find((project) => project.id === current);
      if (currentProject && currentProject.providers.includes(next)) return current;
      const nextId = projects.find((project) => project.providers.includes(next))?.id ?? null;
      if (nextId) localStorage.setItem("selectedProjectId", nextId);
      return nextId;
    });
  }

  async function launch(session: SessionSummary, fork: boolean) {
    setLaunchingKey(launchSessionKey(session.provider, session.id));
    setNotice(null);
    try {
      const result = await resumeSession(session.provider, session.id, fork, highestPermissions);
      setNotice({
        kind: "success",
        message: `${result.terminal} 已启动 ${providerLabel(result.provider)} ${
          result.forked ? "分叉会话" : "原会话"
        }${result.highestPermissions ? "（最高权限）" : ""}`,
      });
    } catch (cause) {
      setNotice({ kind: "error", message: toErrorMessage(cause) });
    } finally {
      setLaunchingKey(null);
    }
  }

  async function copySessionId(sessionId: string) {
    try {
      await navigator.clipboard.writeText(sessionId);
      setNotice({ kind: "success", message: "Session ID 已复制" });
    } catch {
      setNotice({ kind: "error", message: "无法访问系统剪贴板" });
    }
  }

  const sourceSummary =
    catalog?.sources
      .map((source) =>
        source.available
          ? `${providerLabel(source.provider)} ${source.sessionCount} 条`
          : `${providerLabel(source.provider)} 未发现`,
      )
      .join(" · ") ?? "等待扫描";
  const sourceLocations = catalog?.sources
    .map((source) => (source.error ? `${source.location}\n${source.error}` : source.location))
    .join("\n\n");

  return (
    <div className="app-shell">
      <header className="app-header">
        <div className="brand">
          <div className="brand-mark" aria-hidden="true">
            <TerminalSquare size={20} />
          </div>
          <div>
            <h1>CCSM</h1>
            <p>{sessionCount} 条本机会话</p>
          </div>
        </div>

        <label className="global-search">
          <Search size={17} />
          <input
            type="search"
            value={searchQuery}
            placeholder="搜索标题、项目、分支或 Session ID"
            onChange={(event) => setSearchQuery(event.target.value)}
          />
          {searchQuery && (
            <button
              className="search-clear"
              type="button"
              title="清空搜索"
              aria-label="清空搜索"
              onClick={() => setSearchQuery("")}
            >
              <X size={15} />
            </button>
          )}
        </label>

        <div className="header-tools">
          <div className="provider-tabs" role="tablist" aria-label="会话来源切换">
            {(["claude", "codex"] as const).map((p) => (
              <button
                type="button"
                role="tab"
                aria-selected={provider === p}
                key={p}
                className={`provider-tab${provider === p ? " active" : ""}`}
                onClick={() => switchProvider(p)}
              >
                <span className={`provider-dot ${p}`} />
                {providerLabel(p)}
              </button>
            ))}
          </div>
          <button
            className="icon-button"
            type="button"
            title="重新扫描本机会话"
            aria-label="重新扫描本机会话"
            disabled={loading}
            onClick={() => void refresh()}
          >
            <RefreshCw className={loading ? "spinning" : ""} size={17} />
          </button>
        </div>
      </header>

      <div className="workspace">
        <ProjectList
          projects={catalog?.projects ?? []}
          selectedProjectId={selectedProjectId}
          searchQuery={searchQuery}
          provider={provider}
          onSelectProject={selectProject}
        />

        <main className="session-pane">
          <div className="session-toolbar-stack">
            <SessionToolbar
              selectedProject={selectedProject}
              searching={Boolean(normalizedQuery)}
              resultCount={sessions.length}
              sort={sort}
              showArchived={showArchived}
              highestPermissions={highestPermissions}
              onSortChange={setSort}
              onShowArchivedChange={setShowArchived}
              onHighestPermissionsChange={setHighestPermissions}
            />
            {permissionWarning && (
              <div className="permission-warning" role="alert">
                <ShieldAlert size={15} />
                <span>{permissionWarning}</span>
              </div>
            )}
          </div>

          {error ? (
            <div className="fatal-state" role="alert">
              <AlertCircle size={28} />
              <h2>无法读取本机会话</h2>
              <p>{error}</p>
              <button type="button" onClick={() => void refresh()}>
                <RefreshCw size={15} /> 重试
              </button>
            </div>
          ) : loading && !catalog ? (
            <div className="loading-state">
              <RefreshCw className="spinning" size={24} />
              <span>正在扫描本机会话...</span>
            </div>
          ) : (
            <SessionTable
              sessions={sessions}
              launchingSessionKey={launchingKey}
              onResume={(session, fork) => void launch(session, fork)}
              onCopyId={(sessionId) => void copySessionId(sessionId)}
            />
          )}
        </main>
      </div>

      <footer className="status-bar">
        <span title={sourceLocations}>
          <Database size={13} /> {sourceSummary}
        </span>
        <span>
          {catalog?.warnings[0] ??
            (catalog ? `上次扫描：${formatAbsoluteTime(catalog.scannedAt)}` : "仅读取本地元数据")}
        </span>
      </footer>

      {notice && (
        <div className={`notice ${notice.kind}`} role="status">
          {notice.kind === "success" ? <CheckCircle2 size={17} /> : <AlertCircle size={17} />}
          <span>{notice.message}</span>
          <button type="button" title="关闭" aria-label="关闭提示" onClick={() => setNotice(null)}>
            <X size={15} />
          </button>
        </div>
      )}
    </div>
  );
}

interface SessionToolbarProps {
  selectedProject: ProjectSummary | null;
  searching: boolean;
  resultCount: number;
  sort: SessionSort;
  showArchived: boolean;
  highestPermissions: boolean;
  onSortChange: (sort: SessionSort) => void;
  onShowArchivedChange: (show: boolean) => void;
  onHighestPermissionsChange: (enabled: boolean) => void;
}

function SessionToolbar({
  selectedProject,
  searching,
  resultCount,
  sort,
  showArchived,
  highestPermissions,
  onSortChange,
  onShowArchivedChange,
  onHighestPermissionsChange,
}: SessionToolbarProps) {
  return (
    <div className="session-toolbar">
      <div className="pane-title">
        <h2>{searching ? "搜索结果" : (selectedProject?.name ?? "选择一个项目")}</h2>
        <p title={selectedProject?.path}>
          {searching ? `${resultCount} 条匹配会话` : selectedProject?.path}
        </p>
      </div>
      <div className="view-controls">
        <label className={`permission-toggle${highestPermissions ? " enabled" : ""}`}>
          <input
            type="checkbox"
            checked={highestPermissions}
            onChange={(event) => onHighestPermissionsChange(event.target.checked)}
          />
          <span>最高权限</span>
        </label>
        <label className="archive-toggle">
          <input
            type="checkbox"
            checked={showArchived}
            onChange={(event) => onShowArchivedChange(event.target.checked)}
          />
          <span>显示归档</span>
        </label>
        <div className="segmented-control sort-control" aria-label="会话排序方式">
          <button
            type="button"
            className={sort === "recent" ? " active" : ""}
            onClick={() => onSortChange("recent")}
          >
            最近活动
          </button>
          <button
            type="button"
            className={sort === "title" ? " active" : ""}
            onClick={() => onSortChange("title")}
          >
            标题
          </button>
        </div>
      </div>
    </div>
  );
}

function providerLabel(provider: SessionProvider): string {
  return provider === "claude" ? "Claude Code" : "Codex";
}

function toErrorMessage(cause: unknown): string {
  if (cause instanceof Error) return cause.message;
  if (typeof cause === "string") return cause;
  return "发生未知错误";
}
