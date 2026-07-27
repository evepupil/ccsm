import type { ProjectSummary, SessionSort, SessionSummary } from "../types";
import { normalizeSearch } from "./format";

interface SessionFilter {
  projects: ProjectSummary[];
  selectedProjectId: string | null;
  searchQuery: string;
  showArchived: boolean;
  sort: SessionSort;
}

export function selectVisibleSessions({
  projects,
  selectedProjectId,
  searchQuery,
  showArchived,
  sort,
}: SessionFilter): SessionSummary[] {
  const normalizedQuery = normalizeSearch(searchQuery);
  const selectedProject = projects.find((project) => project.id === selectedProjectId);
  const source = normalizedQuery
    ? projects.flatMap((project) => project.sessions)
    : (selectedProject?.sessions ?? []);

  const filtered = source.filter((session) => {
    if (!showArchived && session.isArchived) return false;
    if (!normalizedQuery) return true;
    const haystack = [
      session.title,
      session.id,
      session.projectPath,
      session.branch ?? "",
      session.model ?? "",
    ]
      .join(" ")
      .toLocaleLowerCase("zh-CN");
    return haystack.includes(normalizedQuery);
  });

  return [...filtered].sort((left, right) => {
    if (sort === "title") return left.title.localeCompare(right.title, "zh-CN");
    return right.lastActivity.localeCompare(left.lastActivity);
  });
}
