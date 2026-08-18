import { FolderCode, Settings2 } from "lucide-react";
import { Button, ListBox, SearchField, Tabs } from "@heroui/react";

import { isSessionArchived, type SessionKey } from "../lib/archive";
import { formatRelativeTime, normalizeSearch } from "../lib/format";
import { selectVisibleProjects } from "../lib/sessions";
import type { ProjectSummary, SessionProvider } from "../types";
import { ProviderLogo } from "./ProviderLogo";

interface ProjectSidebarProps {
  archivedSessionKeys: ReadonlySet<SessionKey>;
  projects: ProjectSummary[];
  provider: SessionProvider;
  searchQuery: string;
  selectedProjectId: string | null;
  showArchived: boolean;
  onProviderChange: (provider: SessionProvider) => void;
  onSearchChange: (query: string) => void;
  onSelectProject: (projectId: string) => void;
}

export function ProjectSidebar({
  archivedSessionKeys,
  projects,
  provider,
  searchQuery,
  selectedProjectId,
  showArchived,
  onProviderChange,
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
              (session) =>
                session.provider === provider &&
                (showArchived || !isSessionArchived(session, archivedSessionKeys)),
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
        <Settings2 className="settings-logo" aria-hidden="true" />
        <Button variant="ghost" className="settings-button" aria-label="设置">
          设置
        </Button>
      </footer>
    </aside>
  );
}
