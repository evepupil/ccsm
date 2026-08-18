import type { ProjectSummary, SessionProvider, SessionSort, SessionSummary } from "../types";
import { isSessionArchived, type SessionKey } from "./archive";
import { normalizeSearch } from "./format";
import { TEMPORARY_PROJECT_ID } from "./temporary";

const EMPTY_ARCHIVED_SESSION_KEYS: ReadonlySet<SessionKey> = new Set();

interface SessionFilter {
  archivedSessionKeys?: ReadonlySet<SessionKey>;
  projects: ProjectSummary[];
  selectedProjectId: string | null;
  searchQuery: string;
  providerFilter: SessionProvider;
  showArchived: boolean;
  sort: SessionSort;
}

interface ProjectFilter {
  projects: ProjectSummary[];
  provider: SessionProvider;
  searchQuery: string;
}

export function selectVisibleProjects({
  projects,
  provider,
  searchQuery,
}: ProjectFilter): ProjectSummary[] {
  const normalizedQuery = normalizeSearch(searchQuery);

  return projects.filter((project) => {
    if (!project.providers.includes(provider)) return false;
    if (!normalizedQuery) return true;

    const projectText = `${project.name} ${project.path}`.toLocaleLowerCase("zh-CN");
    if (projectText.includes(normalizedQuery)) return true;

    return project.sessions.some((session) => {
      if (session.provider !== provider) return false;
      const sessionText = [
        session.title,
        session.id,
        session.branch ?? "",
        session.model ?? "",
        session.sourceDetail ?? "",
      ]
        .join(" ")
        .toLocaleLowerCase("zh-CN");
      return sessionText.includes(normalizedQuery);
    });
  });
}

export function isTemporaryProject(project: Pick<ProjectSummary, "id" | "isTemporary">): boolean {
  return project.isTemporary || project.id === TEMPORARY_PROJECT_ID;
}

export function selectVisibleSessions({
  archivedSessionKeys = EMPTY_ARCHIVED_SESSION_KEYS,
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
    if (!showArchived && isSessionArchived(session, archivedSessionKeys)) return false;
    if (session.provider !== providerFilter) return false;
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
