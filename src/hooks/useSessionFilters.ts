import { useMemo, useState } from "react";

import { normalizeSearch } from "../lib/format";
import { highestPermissionWarning } from "../lib/launch";
import { selectVisibleSessions } from "../lib/sessions";
import type { ProjectSummary, SessionProvider, SessionSort } from "../types";

interface UseSessionFiltersOptions {
  projects: ProjectSummary[];
  provider: SessionProvider;
  selectedProjectId: string | null;
}

export function useSessionFilters({
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
        projects,
        selectedProjectId,
        searchQuery,
        providerFilter: provider,
        showArchived,
        sort,
      }),
    [projects, provider, searchQuery, selectedProjectId, showArchived, sort],
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
