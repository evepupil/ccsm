import {
  AlertCircle,
  Database,
  RefreshCw,
  Search,
  ShieldAlert,
  TerminalSquare,
  X,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { Alert, Button, EmptyState, Input, Spinner, Switch, Tabs, toast } from "@heroui/react";

import { resumeSession, scanSessionCatalog } from "./api";
import { ProjectList } from "./components/ProjectTree";
import { SessionTable } from "./components/SessionTable";
import { formatAbsoluteTime, normalizeSearch } from "./lib/format";
import { highestPermissionWarning, launchSessionKey } from "./lib/launch";
import { selectVisibleSessions } from "./lib/sessions";
import type { ProjectSummary, SessionProvider, SessionSort, SessionSummary } from "./types";

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
    try {
      const result = await resumeSession(session.provider, session.id, fork, highestPermissions);
      toast.success(
        `${result.terminal} 已启动 ${providerLabel(result.provider)} ${
          result.forked ? "分叉会话" : "原会话"
        }${result.highestPermissions ? "（最高权限）" : ""}`,
      );
    } catch (cause) {
      toast.danger(toErrorMessage(cause));
    } finally {
      setLaunchingKey(null);
    }
  }

  async function copySessionId(sessionId: string) {
    try {
      await navigator.clipboard.writeText(sessionId);
      toast.success("Session ID 已复制");
    } catch {
      toast.danger("无法访问系统剪贴板");
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
    <div className="grid h-full min-w-[860px] grid-rows-[58px_minmax(0,1fr)_28px] bg-background text-foreground">
      <header className="grid grid-cols-[250px_minmax(260px,620px)_minmax(250px,1fr)] items-center gap-[18px] border-b border-border bg-surface px-4 max-[1020px]:grid-cols-[190px_minmax(220px,1fr)_250px]">
        <div className="flex min-w-0 items-center gap-2.5">
          <div className="grid h-[34px] w-[34px] flex-[0_0_34px] place-items-center rounded-md bg-foreground text-background">
            <TerminalSquare size={20} />
          </div>
          <div className="min-w-0">
            <h1 className="m-0 truncate text-[15px] font-bold leading-tight">CCSM</h1>
            <p className="mt-0.5 text-[11px] text-muted">{sessionCount} 条本机会话</p>
          </div>
        </div>

        <div className="relative h-9">
          <Search
            size={17}
            className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-muted"
          />
          <Input
            type="search"
            value={searchQuery}
            onChange={(event) => setSearchQuery(event.target.value)}
            placeholder="搜索标题、项目、分支或 Session ID"
            className="h-full w-full pl-9 pr-8"
            aria-label="搜索会话"
          />
          {searchQuery && (
            <button
              type="button"
              title="清空搜索"
              aria-label="清空搜索"
              onClick={() => setSearchQuery("")}
              className="absolute right-1.5 top-1/2 grid h-6 w-6 -translate-y-1/2 place-items-center rounded text-muted hover:bg-surface-hover"
            >
              <X size={15} />
            </button>
          )}
        </div>

        <div className="flex items-center justify-end gap-2">
          <Tabs
            selectedKey={provider}
            onSelectionChange={(key) => switchProvider(String(key) as SessionProvider)}
            aria-label="会话来源切换"
          >
            <Tabs.List>
              <Tabs.Tab id="claude">
                <span className="mr-1.5 inline-block h-[7px] w-[7px] rounded-full bg-[#b4542d]" />
                Claude Code
              </Tabs.Tab>
              <Tabs.Tab id="codex">
                <span className="mr-1.5 inline-block h-[7px] w-[7px] rounded-full bg-[#2c6fbb]" />
                Codex
              </Tabs.Tab>
            </Tabs.List>
          </Tabs>
          <Button
            isIconOnly
            variant="outline"
            size="sm"
            isDisabled={loading}
            onPress={() => void refresh()}
            aria-label="重新扫描本机会话"
          >
            <RefreshCw size={17} className={loading ? "animate-spin" : ""} />
          </Button>
        </div>
      </header>

      <div className="grid min-h-0 grid-cols-[284px_minmax(0,1fr)] max-[1020px]:grid-cols-[250px_minmax(0,1fr)]">
        <ProjectList
          projects={catalog?.projects ?? []}
          selectedProjectId={selectedProjectId}
          searchQuery={searchQuery}
          provider={provider}
          onSelectProject={selectProject}
        />

        <main className="grid min-w-0 grid-rows-[auto_minmax(0,1fr)] bg-surface">
          <div className="border-b border-border">
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
              <Alert status="danger" className="rounded-none border-t">
                <Alert.Indicator>
                  <ShieldAlert size={15} />
                </Alert.Indicator>
                <Alert.Content>
                  <Alert.Description>{permissionWarning}</Alert.Description>
                </Alert.Content>
              </Alert>
            )}
          </div>

          {error ? (
            <EmptyState className="flex min-h-[240px] flex-col items-center justify-center p-8 text-center text-muted">
              <AlertCircle size={28} className="text-danger" />
              <h2 className="m-0 mt-3 text-[15px] font-bold text-foreground">无法读取本机会话</h2>
              <p className="mt-1.5 max-w-[520px] text-[12px] text-muted">{error}</p>
              <Button variant="outline" size="sm" className="mt-3.5" onPress={() => void refresh()}>
                <RefreshCw size={15} /> 重试
              </Button>
            </EmptyState>
          ) : loading && !catalog ? (
            <div className="flex min-h-[240px] items-center justify-center gap-2 text-[12px] text-muted">
              <Spinner size="sm" /> 正在扫描本机会话...
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

      <footer className="flex items-center justify-between gap-4 border-t border-border bg-surface-secondary px-2.5 text-[10px] text-muted">
        <span className="inline-flex min-w-0 items-center gap-1.5 truncate" title={sourceLocations}>
          <Database size={13} /> {sourceSummary}
        </span>
        <span className="truncate">
          {catalog?.warnings[0] ??
            (catalog ? `上次扫描：${formatAbsoluteTime(catalog.scannedAt)}` : "仅读取本地元数据")}
        </span>
      </footer>
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
    <div className="flex min-h-[60px] flex-wrap items-center justify-between gap-4 bg-surface px-4 py-2">
      <div className="min-w-0 flex-[1_1_170px]">
        <h2 className="m-0 truncate text-[15px] font-bold">
          {searching ? "搜索结果" : (selectedProject?.name ?? "选择一个项目")}
        </h2>
        <p
          className="mt-0.5 max-w-[520px] truncate text-[10px] text-muted"
          title={selectedProject?.path}
        >
          {searching ? `${resultCount} 条匹配会话` : selectedProject?.path}
        </p>
      </div>
      <div className="flex flex-[0_1_auto] flex-wrap items-center justify-end gap-[13px]">
        <Switch
          isSelected={highestPermissions}
          onChange={onHighestPermissionsChange}
          className={highestPermissions ? "font-bold text-danger" : "text-muted"}
        >
          最高权限
        </Switch>
        <Switch isSelected={showArchived} onChange={onShowArchivedChange} className="text-muted">
          显示归档
        </Switch>
        <Tabs
          selectedKey={sort}
          onSelectionChange={(key) => onSortChange(String(key) as SessionSort)}
          aria-label="会话排序方式"
        >
          <Tabs.List>
            <Tabs.Tab id="recent">最近活动</Tabs.Tab>
            <Tabs.Tab id="title">标题</Tabs.Tab>
          </Tabs.List>
        </Tabs>
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
