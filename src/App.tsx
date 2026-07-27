import {
  AlertCircle,
  CheckCircle2,
  Database,
  RefreshCw,
  Search,
  TerminalSquare,
  X,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";

import { getCliStatus, resumeSession, scanSessionCatalog } from "./api";
import { ProjectTree } from "./components/ProjectTree";
import { SessionTable } from "./components/SessionTable";
import { formatAbsoluteTime, normalizeSearch } from "./lib/format";
import { selectVisibleSessions } from "./lib/sessions";
import type { CliStatus, ProjectSummary, SessionSort, SessionSummary } from "./types";

interface Notice {
  kind: "success" | "error";
  message: string;
}

export default function App() {
  const [catalog, setCatalog] = useState<Awaited<ReturnType<typeof scanSessionCatalog>> | null>(
    null,
  );
  const [cliStatus, setCliStatus] = useState<CliStatus | null>(null);
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(() =>
    localStorage.getItem("selectedProjectId"),
  );
  const [searchQuery, setSearchQuery] = useState("");
  const [sort, setSort] = useState<SessionSort>("recent");
  const [showArchived, setShowArchived] = useState(false);
  const [loading, setLoading] = useState(true);
  const [launchingSessionId, setLaunchingSessionId] = useState<string | null>(null);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [nextCatalog, nextCliStatus] = await Promise.all([
        scanSessionCatalog(),
        getCliStatus(),
      ]);
      setCatalog(nextCatalog);
      setCliStatus(nextCliStatus);
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
        showArchived,
        sort,
      }),
    [catalog, searchQuery, selectedProjectId, showArchived, sort],
  );

  const sessionCount =
    catalog?.projects.reduce((total, project) => total + project.sessionCount, 0) ?? 0;

  function selectProject(projectId: string) {
    setSelectedProjectId(projectId);
    localStorage.setItem("selectedProjectId", projectId);
    if (searchQuery) setSearchQuery("");
  }

  async function launch(session: SessionSummary, fork: boolean) {
    setLaunchingSessionId(session.id);
    setNotice(null);
    try {
      const result = await resumeSession(session.id, fork);
      setNotice({
        kind: "success",
        message: `${result.terminal} 已启动${result.forked ? "分叉会话" : "原会话"}`,
      });
    } catch (cause) {
      setNotice({ kind: "error", message: toErrorMessage(cause) });
    } finally {
      setLaunchingSessionId(null);
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

  return (
    <div className="app-shell">
      <header className="app-header">
        <div className="brand">
          <div className="brand-mark" aria-hidden="true">
            <TerminalSquare size={20} />
          </div>
          <div>
            <h1>Claude 会话管理器</h1>
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
          <CliIndicator status={cliStatus} />
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
        <ProjectTree
          projects={catalog?.projects ?? []}
          selectedProjectId={selectedProjectId}
          searchQuery={searchQuery}
          onSelectProject={selectProject}
        />

        <main className="session-pane">
          <SessionToolbar
            selectedProject={selectedProject}
            searching={Boolean(normalizedQuery)}
            resultCount={sessions.length}
            sort={sort}
            showArchived={showArchived}
            onSortChange={setSort}
            onShowArchivedChange={setShowArchived}
          />

          {error ? (
            <div className="fatal-state" role="alert">
              <AlertCircle size={28} />
              <h2>无法读取 Claude 会话</h2>
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
              launchingSessionId={launchingSessionId}
              onResume={(session, fork) => void launch(session, fork)}
              onCopyId={(sessionId) => void copySessionId(sessionId)}
            />
          )}
        </main>
      </div>

      <footer className="status-bar">
        <span>
          <Database size={13} /> {catalog?.sessionsRoot ?? "等待扫描"}
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
  onSortChange: (sort: SessionSort) => void;
  onShowArchivedChange: (show: boolean) => void;
}

function SessionToolbar({
  selectedProject,
  searching,
  resultCount,
  sort,
  showArchived,
  onSortChange,
  onShowArchivedChange,
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
        <label className="archive-toggle">
          <input
            type="checkbox"
            checked={showArchived}
            onChange={(event) => onShowArchivedChange(event.target.checked)}
          />
          <span>显示归档</span>
        </label>
        <div className="segmented-control" aria-label="会话排序方式">
          <button
            type="button"
            className={sort === "recent" ? "active" : ""}
            onClick={() => onSortChange("recent")}
          >
            最近活动
          </button>
          <button
            type="button"
            className={sort === "title" ? "active" : ""}
            onClick={() => onSortChange("title")}
          >
            标题
          </button>
        </div>
      </div>
    </div>
  );
}

function CliIndicator({ status }: { status: CliStatus | null }) {
  if (!status) return <span className="cli-indicator neutral">检测 CLI</span>;
  if (!status.available) return <span className="cli-indicator error">CLI 未安装</span>;
  if (status.loggedIn === false) {
    return (
      <span className="cli-indicator warning" title="新终端可能要求登录或配置 Gateway">
        CLI 未登录
      </span>
    );
  }
  return (
    <span className="cli-indicator ready" title={status.version ?? undefined}>
      CLI 就绪
    </span>
  );
}

function toErrorMessage(cause: unknown): string {
  if (cause instanceof Error) return cause.message;
  if (typeof cause === "string") return cause;
  return "发生未知错误";
}
