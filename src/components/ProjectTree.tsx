import {
  ChevronDown,
  ChevronRight,
  Folder,
  FolderOpen,
  Library,
  MessageSquare,
} from "lucide-react";
import { useMemo, useState } from "react";

import { formatRelativeTime, normalizeSearch } from "../lib/format";
import type { ProjectSummary } from "../types";

interface ProjectTreeProps {
  projects: ProjectSummary[];
  selectedProjectId: string | null;
  searchQuery: string;
  onSelectProject: (projectId: string) => void;
}

export function ProjectTree({
  projects,
  selectedProjectId,
  searchQuery,
  onSelectProject,
}: ProjectTreeProps) {
  const [rootOpen, setRootOpen] = useState(true);
  const [expandedProjects, setExpandedProjects] = useState<Set<string>>(() => new Set());
  const normalizedQuery = normalizeSearch(searchQuery);

  const visibleProjects = useMemo(() => {
    if (!normalizedQuery) return projects;
    return projects.filter((project) => {
      const projectMatch = `${project.name} ${project.path}`.toLocaleLowerCase("zh-CN");
      return (
        projectMatch.includes(normalizedQuery) ||
        project.sessions.some((session) =>
          `${session.title} ${session.id}`.toLocaleLowerCase("zh-CN").includes(normalizedQuery),
        )
      );
    });
  }, [normalizedQuery, projects]);

  function selectProject(project: ProjectSummary) {
    onSelectProject(project.id);
    setExpandedProjects((current) => {
      const next = new Set(current);
      next.add(project.id);
      return next;
    });
  }

  function toggleProject(projectId: string) {
    setExpandedProjects((current) => {
      const next = new Set(current);
      if (next.has(projectId)) next.delete(projectId);
      else next.add(projectId);
      return next;
    });
  }

  return (
    <aside className="project-sidebar" aria-label="项目目录">
      <div className="sidebar-heading">
        <span>项目目录</span>
        <span className="count-label">{projects.length}</span>
      </div>

      <nav className="tree" aria-label="Claude 项目树">
        <button className="tree-root" type="button" onClick={() => setRootOpen((open) => !open)}>
          {rootOpen ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
          <Library size={17} />
          <span>本机 Claude 项目</span>
        </button>

        {rootOpen && (
          <div className="tree-children">
            {visibleProjects.map((project) => {
              const expanded = expandedProjects.has(project.id);
              const selected = selectedProjectId === project.id;
              return (
                <div className="project-node" key={project.id}>
                  <div className={`project-row${selected ? " selected" : ""}`}>
                    <button
                      className="tree-toggle"
                      type="button"
                      title={expanded ? "收起会话" : "展开会话"}
                      aria-label={expanded ? `收起 ${project.name}` : `展开 ${project.name}`}
                      onClick={() => toggleProject(project.id)}
                    >
                      {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                    </button>
                    <button
                      className="project-select"
                      type="button"
                      onClick={() => selectProject(project)}
                      title={project.path}
                    >
                      {expanded ? <FolderOpen size={17} /> : <Folder size={17} />}
                      <span className="project-label">
                        <strong>{project.name}</strong>
                        <small>
                          {project.sessionCount} 条 · {formatRelativeTime(project.lastActivity)}
                        </small>
                      </span>
                    </button>
                  </div>

                  {expanded && (
                    <div className="session-branches">
                      {project.sessions.slice(0, 6).map((session) => (
                        <button
                          type="button"
                          className="session-branch"
                          key={session.id}
                          title={session.title}
                          onClick={() => selectProject(project)}
                        >
                          <MessageSquare size={13} />
                          <span>{session.title}</span>
                        </button>
                      ))}
                      {project.sessions.length > 6 && (
                        <span className="more-sessions">还有 {project.sessions.length - 6} 条</span>
                      )}
                    </div>
                  )}
                </div>
              );
            })}

            {visibleProjects.length === 0 && <p className="tree-empty">没有匹配的项目</p>}
          </div>
        )}
      </nav>
    </aside>
  );
}
