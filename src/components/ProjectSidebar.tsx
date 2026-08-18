import { FolderCode, LoaderCircle, MessageSquare, Plus, Settings2 } from "lucide-react";
import { Button, ListBox, SearchField, Tabs } from "@heroui/react";

import { isSessionArchived, type SessionKey } from "../lib/archive";
import { formatRelativeTime, normalizeSearch } from "../lib/format";
import { isTemporaryProject, selectVisibleProjects } from "../lib/sessions";
import { TEMPORARY_PROJECT_ID } from "../lib/temporary";
import type { ProjectSummary, SessionProvider } from "../types";
import { ProviderLogo } from "./ProviderLogo";

interface ProjectSidebarProps {
  archivedSessionKeys: ReadonlySet<SessionKey>;
  projects: ProjectSummary[];
  provider: SessionProvider;
  searchQuery: string;
  selectedProjectId: string | null;
  showArchived: boolean;
  temporarySessionLaunching: boolean;
  onProviderChange: (provider: SessionProvider) => void;
  onCreateTemporarySession: () => void;
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
  temporarySessionLaunching,
  onProviderChange,
  onCreateTemporarySession,
  onSearchChange,
  onSelectProject,
}: ProjectSidebarProps) {
  const visibleProjects = selectVisibleProjects({ projects, provider, searchQuery });
  const projectList = visibleProjects.filter((project) => !isTemporaryProject(project));
  const temporaryProject = projects.find((project) => isTemporaryProject(project));
  const showTemporaryProject =
    !normalizeSearch(searchQuery) || visibleProjects.some((project) => isTemporaryProject(project));
  const temporarySessionCount =
    temporaryProject?.sessions.filter(
      (session) =>
        session.provider === provider &&
        (showArchived || !isSessionArchived(session, archivedSessionKeys)),
    ).length ?? 0;
  const selectedKeys = new Set(
    !normalizeSearch(searchQuery) &&
      selectedProjectId &&
      projectList.some((project) => project.id === selectedProjectId)
      ? [selectedProjectId]
      : [],
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

      <section className="sidebar-chat" aria-label="聊天">
        <div className="sidebar-heading sidebar-chat-heading">
          <span>聊天</span>
        </div>
        {showTemporaryProject && (
          <div
            className={`temporary-session-card ${selectedProjectId === TEMPORARY_PROJECT_ID ? "is-selected" : ""} ${temporarySessionLaunching ? "is-loading" : ""}`}
          >
            <button
              className="temporary-session-item"
              type="button"
              aria-pressed={selectedProjectId === TEMPORARY_PROJECT_ID}
              onClick={() => onSelectProject(TEMPORARY_PROJECT_ID)}
            >
              <span className="temporary-session-icon" aria-hidden="true">
                <MessageSquare />
              </span>
              <span className="project-copy">
                <strong>临时会话</strong>
                <small>
                  {temporarySessionCount > 0 ? `${temporarySessionCount} 个会话` : "新建即用"}
                </small>
              </span>
              <span className="project-session-count">{temporarySessionCount}</span>
            </button>
            <button
              className="temporary-add-button"
              type="button"
              disabled={temporarySessionLaunching}
              title="新建临时会话"
              aria-label="新建临时会话"
              onClick={onCreateTemporarySession}
            >
              {temporarySessionLaunching ? (
                <LoaderCircle className="is-spinning" aria-hidden="true" />
              ) : (
                <Plus aria-hidden="true" />
              )}
            </button>
          </div>
        )}
      </section>

      <div className="sidebar-heading">
        <span>项目</span>
        <span className="sidebar-count">{projectList.length}</span>
      </div>

      {projectList.length === 0 ? (
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
          {projectList.map((project) => {
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
