import { useMemo, useState } from "react";

import { normalizeSearch } from "../lib/format";
import { highestPermissionWarning } from "../lib/launch";
import { selectVisibleSessions } from "../lib/sessions";
import type { ProjectSummary, SessionProvider, SessionSort } from "../types";
import type { SessionKey } from "../lib/archive";

interface UseSessionFiltersOptions {
  archivedSessionKeys: ReadonlySet<SessionKey>;
  projects: ProjectSummary[];
  provider: SessionProvider;
  selectedProjectId: string | null;
}

export function useSessionFilters({
  archivedSessionKeys,
  projects,
  provider,
  selectedProjectId,
}: UseSessionFiltersOptions) {
  const [searchQuery, setSearchQuery] = useState("");
  const [sort, setSort] = useState<SessionSort>("recent");
  const [showArchived, setShowArchived] = useState(false);
  const [highestPermissions, setHighestPermissions] = useState(false);

  const sessions = useMemo(
    () =>
      selectVisibleSessions({
        archivedSessionKeys,
        projects,
        selectedProjectId,
        searchQuery,
        providerFilter: provider,
        showArchived,
        sort,
      }),
    [archivedSessionKeys, projects, provider, searchQuery, selectedProjectId, showArchived, sort],
  );

  return {
    highestPermissions,
    permissionWarning: highestPermissionWarning(highestPermissions),
    searching: Boolean(normalizeSearch(searchQuery)),
    searchQuery,
    sessions,
    showArchived,
    sort,
    setHighestPermissions,
    setSearchQuery,
    setShowArchived,
    setSort,
  };
}
