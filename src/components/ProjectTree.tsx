import { Folder } from "lucide-react";
import { useMemo } from "react";

import { formatRelativeTime, normalizeSearch } from "../lib/format";
import type { ProjectSummary, SessionProvider } from "../types";

interface ProjectListProps {
  projects: ProjectSummary[];
  selectedProjectId: string | null;
  searchQuery: string;
  provider: SessionProvider;
  onSelectProject: (projectId: string) => void;
}

export function ProjectList({
  projects,
  selectedProjectId,
  searchQuery,
  provider,
  onSelectProject,
}: ProjectListProps) {
  const normalizedQuery = normalizeSearch(searchQuery);

  const visibleProjects = useMemo(() => {
    return projects.filter((project) => {
      if (!project.providers.includes(provider)) return false;
      if (!normalizedQuery) return true;
      const projectMatch = `${project.name} ${project.path}`.toLocaleLowerCase("zh-CN");
      return (
        projectMatch.includes(normalizedQuery) ||
        project.sessions.some(
          (session) =>
            session.provider === provider &&
            `${session.title} ${session.id} ${session.sourceDetail ?? ""}`
              .toLocaleLowerCase("zh-CN")
              .includes(normalizedQuery),
        )
      );
    });
  }, [normalizedQuery, projects, provider]);

  return (
    <aside className="project-sidebar" aria-label="项目列表">
      <div className="sidebar-heading">
        <span>项目</span>
        <span className="count-label">{visibleProjects.length}</span>
      </div>

      <nav className="project-list" aria-label="项目列表">
        {visibleProjects.map((project) => {
          const selected = selectedProjectId === project.id;
          const sessionCount = project.sessions.filter(
            (session) => session.provider === provider,
          ).length;
          return (
            <button
              type="button"
              className={`project-item${selected ? " selected" : ""}`}
              key={project.id}
              title={project.path}
              onClick={() => onSelectProject(project.id)}
            >
              <Folder size={16} />
              <span className="project-label">
                <strong>{project.name}</strong>
                <small>
                  {sessionCount} 条 · {formatRelativeTime(project.lastActivity)}
                </small>
              </span>
            </button>
          );
        })}

        {visibleProjects.length === 0 && <p className="tree-empty">没有匹配的项目</p>}
      </nav>
    </aside>
  );
}
