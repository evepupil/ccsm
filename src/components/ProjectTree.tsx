import { Folder } from "lucide-react";
import { useMemo } from "react";
import { ListBox, ListBoxItem } from "@heroui/react";

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
    <aside
      className="flex min-h-0 flex-col border-r border-border bg-surface"
      aria-label="项目列表"
    >
      <div className="flex h-[42px] items-center justify-between border-b border-border px-3 pl-4 text-[11px] font-bold uppercase text-muted">
        <span>项目</span>
        <span className="tabular-nums">{visibleProjects.length}</span>
      </div>

      {visibleProjects.length === 0 ? (
        <p className="m-3 text-[12px] text-muted">没有匹配的项目</p>
      ) : (
        <ListBox
          selectionMode="single"
          disallowEmptySelection
          selectedKeys={new Set(selectedProjectId ? [selectedProjectId] : [])}
          onAction={(key) => onSelectProject(String(key))}
          aria-label="项目列表"
          className="flex-1 overflow-auto p-1.5"
        >
          {visibleProjects.map((project) => {
            const sessionCount = project.sessions.filter(
              (session) => session.provider === provider,
            ).length;
            return (
              <ListBoxItem
                key={project.id}
                id={project.id}
                textValue={project.name}
                className="flex min-h-[48px] items-center gap-2.5 rounded px-2.5 py-1.5"
              >
                <Folder size={16} className="flex-[0_0_16px] text-muted" />
                <span className="block min-w-0 flex-1">
                  <strong
                    className="block truncate text-[12px] font-semibold text-foreground"
                    title={project.path}
                  >
                    {project.name}
                  </strong>
                  <small className="mt-0.5 block truncate text-[10px] text-muted">
                    {sessionCount} 条 · {formatRelativeTime(project.lastActivity)}
                  </small>
                </span>
              </ListBoxItem>
            );
          })}
        </ListBox>
      )}
    </aside>
  );
}
