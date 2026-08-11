import { Database, FolderCode, RefreshCw } from "lucide-react";
import { Button, ListBox, SearchField, Tabs, Tooltip } from "@heroui/react";

import { formatRelativeTime, normalizeSearch } from "../lib/format";
import { selectVisibleProjects } from "../lib/sessions";
import type { ProjectSummary, SessionProvider } from "../types";
import { ProviderLogo } from "./ProviderLogo";

interface ProjectSidebarProps {
  loading: boolean;
  projects: ProjectSummary[];
  provider: SessionProvider;
  scannedAt: string | null;
  searchQuery: string;
  selectedProjectId: string | null;
  sessionCount: number;
  showArchived: boolean;
  warning: string | null;
  onProviderChange: (provider: SessionProvider) => void;
  onRefresh: () => void;
  onSearchChange: (query: string) => void;
  onSelectProject: (projectId: string) => void;
}

function scanStatus(loading: boolean, scannedAt: string | null, warning: string | null): string {
  if (loading) return "正在扫描...";
  if (warning) return warning;
  if (!scannedAt) return "尚未扫描";
  const relative = formatRelativeTime(scannedAt);
  return relative === "刚刚" ? "刚刚更新" : `${relative}更新`;
}

export function ProjectSidebar({
  loading,
  projects,
  provider,
  scannedAt,
  searchQuery,
  selectedProjectId,
  sessionCount,
  showArchived,
  warning,
  onProviderChange,
  onRefresh,
  onSearchChange,
  onSelectProject,
}: ProjectSidebarProps) {
  const visibleProjects = selectVisibleProjects({ projects, provider, searchQuery });
  const selectedKeys = new Set(
    !normalizeSearch(searchQuery) && selectedProjectId ? [selectedProjectId] : [],
  );

  return (
    <aside className="sidebar" aria-label="会话导航">
      <div className="sidebar-tools">
        <Tabs
          selectedKey={provider}
          onSelectionChange={(key) => {
            const nextProvider = String(key);
            if (nextProvider === "claude" || nextProvider === "codex") {
              onProviderChange(nextProvider);
            }
          }}
          aria-label="会话来源"
          className="provider-tabs"
        >
          <Tabs.List className="provider-switch">
            <Tabs.Tab id="claude" className="provider-option">
              <Tabs.Indicator className="provider-indicator" />
              <span className="provider-mark claude" aria-hidden="true">
                <ProviderLogo provider="claude" />
              </span>
              <span>Claude</span>
            </Tabs.Tab>
            <Tabs.Tab id="codex" className="provider-option">
              <Tabs.Indicator className="provider-indicator" />
              <span className="provider-mark codex" aria-hidden="true">
                <ProviderLogo provider="codex" />
              </span>
              <span>Codex</span>
            </Tabs.Tab>
          </Tabs.List>
        </Tabs>

        <SearchField
          fullWidth
          value={searchQuery}
          onChange={onSearchChange}
          aria-label="搜索项目或会话"
          className="sidebar-search"
        >
          <SearchField.Group className="sidebar-search-group">
            <SearchField.SearchIcon />
            <SearchField.Input placeholder="搜索项目或会话" />
            <SearchField.ClearButton aria-label="清空搜索" />
          </SearchField.Group>
        </SearchField>
      </div>

      <div className="sidebar-heading">
        <span>项目</span>
        <span className="sidebar-count">{visibleProjects.length}</span>
      </div>

      {visibleProjects.length === 0 ? (
        <p className="sidebar-empty">没有匹配的项目</p>
      ) : (
        <ListBox
          selectionMode="single"
          disallowEmptySelection
          selectedKeys={selectedKeys}
          onSelectionChange={(keys) => {
            if (keys === "all") return;
            const projectId = keys.values().next().value;
            if (projectId !== undefined) onSelectProject(String(projectId));
          }}
          aria-label="项目列表"
          className="project-list"
        >
          {visibleProjects.map((project) => {
            const providerSessions = project.sessions.filter(
              (session) => session.provider === provider && (showArchived || !session.isArchived),
            );
            return (
              <ListBox.Item
                key={project.id}
                id={project.id}
                textValue={project.name}
                className="project-item"
              >
                <span className="project-icon" aria-hidden="true">
                  <FolderCode />
                </span>
                <span className="project-copy">
                  <strong title={project.path}>{project.name}</strong>
                  <small>{formatRelativeTime(project.lastActivity)}</small>
                </span>
                <span className="project-session-count">{providerSessions.length}</span>
              </ListBox.Item>
            );
          })}
        </ListBox>
      )}

      <footer className="sidebar-footer">
        <div className="source-summary">
          <Database aria-hidden="true" />
          <span>
            <strong>{sessionCount} 个本机会话</strong>
            <small title={warning ?? undefined}>{scanStatus(loading, scannedAt, warning)}</small>
          </span>
        </div>
        <Tooltip delay={500}>
          <Button
            isIconOnly
            variant="ghost"
            className="icon-button sidebar-refresh"
            isDisabled={loading}
            aria-label="重新扫描"
            onPress={onRefresh}
          >
            <RefreshCw className={loading ? "is-spinning" : ""} aria-hidden="true" />
          </Button>
          <Tooltip.Content>重新扫描</Tooltip.Content>
        </Tooltip>
      </footer>
    </aside>
  );
}
