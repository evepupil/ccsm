import { AppHeader } from "./components/AppHeader";
import { ProjectSidebar } from "./components/ProjectSidebar";
import { SessionWorkspace } from "./components/SessionWorkspace";
import { StatusBar } from "./components/StatusBar";
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
    <div className="grid h-full min-w-[760px] grid-rows-[58px_minmax(0,1fr)_28px] bg-background text-foreground">
      <AppHeader
        loading={sessionCatalog.loading}
        provider={sessionCatalog.provider}
        searchQuery={filters.searchQuery}
        sessionCount={sessionCount}
        onProviderChange={sessionCatalog.switchProvider}
        onRefresh={() => void sessionCatalog.refresh()}
        onSearchChange={filters.setSearchQuery}
      />

      <div className="grid min-h-0 grid-cols-[252px_minmax(0,1fr)] max-[1000px]:grid-cols-[220px_minmax(0,1fr)]">
        <ProjectSidebar
          projects={projects}
          selectedProjectId={sessionCatalog.selectedProjectId}
          searchQuery={filters.searchQuery}
          provider={sessionCatalog.provider}
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

      <StatusBar catalog={sessionCatalog.catalog} />
    </div>
  );
}
