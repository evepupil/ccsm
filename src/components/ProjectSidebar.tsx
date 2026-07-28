import { Folder } from "lucide-react";
import { Chip, ListBox } from "@heroui/react";

import { formatRelativeTime, normalizeSearch } from "../lib/format";
import { selectVisibleProjects } from "../lib/sessions";
import type { ProjectSummary, SessionProvider } from "../types";

interface ProjectSidebarProps {
  projects: ProjectSummary[];
  provider: SessionProvider;
  searchQuery: string;
  selectedProjectId: string | null;
  onSelectProject: (projectId: string) => void;
}

export function ProjectSidebar({
  projects,
  provider,
  searchQuery,
  selectedProjectId,
  onSelectProject,
}: ProjectSidebarProps) {
  const visibleProjects = selectVisibleProjects({ projects, provider, searchQuery });
  const selectedKeys = new Set(
    !normalizeSearch(searchQuery) && selectedProjectId ? [selectedProjectId] : [],
  );

  return (
    <aside
      className="grid min-h-0 grid-rows-[42px_minmax(0,1fr)] border-r border-border bg-surface-secondary"
      aria-label="项目列表"
    >
      <div className="flex items-center justify-between border-b border-border px-[14px] pl-4 text-[11px] font-bold text-muted">
        <span>项目</span>
        <span className="tabular-nums">{visibleProjects.length}</span>
      </div>

      {visibleProjects.length === 0 ? (
        <p className="m-0 p-4 text-center text-[12px] text-muted">没有匹配的项目</p>
      ) : (
        <ListBox
          selectionMode="single"
          disallowEmptySelection
          selectedKeys={selectedKeys}
          onSelectionChange={(keys) => {
            if (keys === "all") return;
            const nextProjectId = keys.values().next().value;
            if (nextProjectId !== undefined) onSelectProject(String(nextProjectId));
          }}
          aria-label="项目列表"
          className="min-h-0 overflow-auto p-1.5"
        >
          {visibleProjects.map((project) => {
            const providerSessionCount = project.sessions.filter(
              (session) => session.provider === provider,
            ).length;
            return (
              <ListBox.Item
                key={project.id}
                id={project.id}
                textValue={project.name}
                className="group flex min-h-[52px] items-center gap-2.5 rounded-md border border-transparent px-2.5 py-1.5 data-[selected]:border-[#b8d7cc] data-[selected]:bg-[#e7f2ee] dark:data-[selected]:border-[#315b4f] dark:data-[selected]:bg-[#213a33]"
              >
                <Folder
                  size={16}
                  className="flex-[0_0_16px] text-muted group-data-[selected]:text-accent"
                />
                <span className="block min-w-0 flex-1">
                  <strong className="block truncate text-[12px] font-semibold" title={project.path}>
                    {project.name}
                  </strong>
                  <small className="mt-1 block truncate text-[11px] text-muted">
                    {formatRelativeTime(project.lastActivity)}
                  </small>
                </span>
                <Chip size="sm" variant="soft" color="default" className="min-w-6 justify-center">
                  <Chip.Label>{providerSessionCount}</Chip.Label>
                </Chip>
              </ListBox.Item>
            );
          })}
        </ListBox>
      )}
    </aside>
  );
}
