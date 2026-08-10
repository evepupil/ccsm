import { ProjectSidebar } from "./components/ProjectSidebar";
import { SessionWorkspace } from "./components/SessionWorkspace";
import { WindowTitlebar } from "./components/WindowTitlebar";
import { useSessionCatalog } from "./hooks/useSessionCatalog";
import { useSessionFilters } from "./hooks/useSessionFilters";
import { useSessionLauncher } from "./hooks/useSessionLauncher";

export default function App() {
  const sessionCatalog = useSessionCatalog();
  const projects = sessionCatalog.catalog?.projects ?? [];
  const filters = useSessionFilters({
    projects,
    provider: sessionCatalog.provider,
    selectedProjectId: sessionCatalog.selectedProjectId,
  });
  const launcher = useSessionLauncher(filters.highestPermissions);
  const sessionCount = projects.reduce((total, project) => total + project.sessionCount, 0);

  function selectProject(projectId: string) {
    filters.setSearchQuery("");
    sessionCatalog.selectProject(projectId);
  }

  return (
    <div className="window-shell">
      <WindowTitlebar provider={sessionCatalog.provider} />

      <div className="app-body">
        <ProjectSidebar
          projects={projects}
          selectedProjectId={sessionCatalog.selectedProjectId}
          searchQuery={filters.searchQuery}
          provider={sessionCatalog.provider}
          loading={sessionCatalog.loading}
          scannedAt={sessionCatalog.catalog?.scannedAt ?? null}
          sessionCount={sessionCount}
          showArchived={filters.showArchived}
          warning={sessionCatalog.catalog?.warnings[0] ?? null}
          onProviderChange={sessionCatalog.switchProvider}
          onRefresh={() => void sessionCatalog.refresh()}
          onSearchChange={filters.setSearchQuery}
          onSelectProject={selectProject}
        />

        <SessionWorkspace
          error={sessionCatalog.error}
          initialLoading={sessionCatalog.loading && !sessionCatalog.catalog}
          provider={sessionCatalog.provider}
          selectedProject={sessionCatalog.selectedProject}
          searchQuery={filters.searchQuery}
          searching={filters.searching}
          sessions={filters.sessions}
          sort={filters.sort}
          showArchived={filters.showArchived}
          highestPermissions={filters.highestPermissions}
          permissionWarning={filters.permissionWarning}
          launchingSessionKey={launcher.launchingSessionKey}
          onRefresh={() => void sessionCatalog.refresh()}
          onSortChange={filters.setSort}
          onShowArchivedChange={filters.setShowArchived}
          onHighestPermissionsChange={filters.setHighestPermissions}
          onResume={(session, fork) => void launcher.launch(session, fork)}
          onCopyId={(sessionId) => void launcher.copySessionId(sessionId)}
        />
      </div>
    </div>
  );
}
