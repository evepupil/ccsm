import type { ProjectSummary, ProviderFilter, SessionSort, SessionSummary } from "../types";
import { normalizeSearch } from "./format";

interface SessionFilter {
  projects: ProjectSummary[];
  selectedProjectId: string | null;
  searchQuery: string;
  providerFilter: ProviderFilter;
  showArchived: boolean;
  sort: SessionSort;
}

export function selectVisibleSessions({
  projects,
  selectedProjectId,
  searchQuery,
  providerFilter,
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
    if (providerFilter !== "all" && session.provider !== providerFilter) return false;
    if (!normalizedQuery) return true;
    const haystack = [
      session.title,
      session.id,
      session.projectPath,
      session.branch ?? "",
      session.model ?? "",
      session.provider,
      session.sourceDetail ?? "",
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
