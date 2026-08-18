import { useEffect, useMemo } from "react";
import { toast } from "@heroui/react";

import { ProjectSidebar } from "./components/ProjectSidebar";
import { SessionWorkspace } from "./components/SessionWorkspace";
import { WindowTitlebar } from "./components/WindowTitlebar";
import { isSessionArchived, sessionKey } from "./lib/archive";
import { newSessionLaunchKey } from "./lib/launch";
import { useSessionCatalog } from "./hooks/useSessionCatalog";
import { useSessionArchive } from "./hooks/useSessionArchive";
import { useSessionFilters } from "./hooks/useSessionFilters";
import { useSessionLauncher } from "./hooks/useSessionLauncher";
import { useSessionSelection } from "./hooks/useSessionSelection";
import type { SessionSummary } from "./types";

export default function App() {
  const sessionCatalog = useSessionCatalog();
  const archive = useSessionArchive();
  const selection = useSessionSelection();
  const projects = sessionCatalog.catalog?.projects ?? [];
  const filters = useSessionFilters({
    archivedSessionKeys: archive.archivedSessionKeys,
    projects,
    provider: sessionCatalog.provider,
    selectedProjectId: sessionCatalog.selectedProjectId,
  });
  const launcher = useSessionLauncher(filters.highestPermissions);
  const selectedSessions = useMemo(
    () =>
      filters.sessions.filter((session) => selection.selectedSessionKeys.has(sessionKey(session))),
    [filters.sessions, selection.selectedSessionKeys],
  );
  const activeSelectionCount = selectedSessions.filter(
    (session) => !isSessionArchived(session, archive.archivedSessionKeys),
  ).length;
  const archivedSelectionCount = selectedSessions.filter((session) =>
    archive.archivedSessionKeys.has(sessionKey(session)),
  ).length;
  const newSessionLaunching = Boolean(
    sessionCatalog.selectedProject &&
    launcher.launchingSessionKey ===
      newSessionLaunchKey(sessionCatalog.provider, sessionCatalog.selectedProject.id),
  );
  useEffect(() => {
    selection.clearSelection();
  }, [
    filters.searchQuery,
    filters.showArchived,
    selection.clearSelection,
    sessionCatalog.provider,
    sessionCatalog.selectedProjectId,
  ]);

  function selectProject(projectId: string) {
    filters.setSearchQuery("");
    sessionCatalog.selectProject(projectId);
  }

  function updateArchive(sessions: readonly SessionSummary[], archived: boolean) {
    if (sessions.length === 0) return;
    if (!archive.updateArchive(sessions, archived)) {
      toast.danger("无法保存归档状态，请检查 CCSM 的本地存储权限");
      return;
    }

    selection.clearSelection();
    toast.success(
      sessions.length === 1
        ? archived
          ? "会话已归档"
          : "会话已取消归档"
        : `${sessions.length} 个会话${archived ? "已归档" : "已取消归档"}`,
    );
  }

  function archiveSelected() {
    updateArchive(
      selectedSessions.filter(
        (session) => !isSessionArchived(session, archive.archivedSessionKeys),
      ),
      true,
    );
  }

  function unarchiveSelected() {
    updateArchive(
      selectedSessions.filter((session) => archive.archivedSessionKeys.has(sessionKey(session))),
      false,
    );
  }

  return (
    <div className="window-shell">
      <WindowTitlebar provider={sessionCatalog.provider} />

      <div className="app-body">
        <ProjectSidebar
          archivedSessionKeys={archive.archivedSessionKeys}
          projects={projects}
          selectedProjectId={sessionCatalog.selectedProjectId}
          searchQuery={filters.searchQuery}
          provider={sessionCatalog.provider}
          showArchived={filters.showArchived}
          onProviderChange={sessionCatalog.switchProvider}
          onSearchChange={filters.setSearchQuery}
          onSelectProject={selectProject}
        />

        <SessionWorkspace
          activeSelectionCount={activeSelectionCount}
          archivedSelectionCount={archivedSelectionCount}
          archivedSessionKeys={archive.archivedSessionKeys}
          error={sessionCatalog.error}
          initialLoading={sessionCatalog.loading && !sessionCatalog.catalog}
          newSessionLaunching={newSessionLaunching}
          provider={sessionCatalog.provider}
          selectedProject={sessionCatalog.selectedProject}
          searchQuery={filters.searchQuery}
          searching={filters.searching}
          selectedCount={selectedSessions.length}
          selectedSessionKeys={selection.selectedSessionKeys}
          sessions={filters.sessions}
          selectionMode={selection.selectionMode}
          sort={filters.sort}
          showArchived={filters.showArchived}
          highestPermissions={filters.highestPermissions}
          permissionWarning={filters.permissionWarning}
          launchingSessionKey={launcher.launchingSessionKey}
          onArchive={(session, archived) => updateArchive([session], archived)}
          onArchiveSelected={archiveSelected}
          onClearSelection={selection.clearSelection}
          onSortChange={filters.setSort}
          onSelectAll={() => selection.selectSessions(filters.sessions)}
          onShowArchivedChange={filters.setShowArchived}
          onHighestPermissionsChange={filters.setHighestPermissions}
          onNewSession={() => {
            if (!sessionCatalog.selectedProject || filters.searching) return;
            void launcher.launchNew(sessionCatalog.provider, sessionCatalog.selectedProject.id);
          }}
          onResume={(session, fork) => void launcher.launch(session, fork)}
          onCopyId={(sessionId) => void launcher.copySessionId(sessionId)}
          onToggleSelection={selection.toggleSession}
          onToggleSelectionMode={selection.toggleSelectionMode}
          onUnarchiveSelected={unarchiveSelected}
        />
      </div>
    </div>
  );
}
