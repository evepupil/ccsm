import { AlertCircle, RefreshCw, ShieldAlert } from "lucide-react";
import { Alert, Button, EmptyState, Spinner } from "@heroui/react";

import type { ProjectSummary, SessionProvider, SessionSort, SessionSummary } from "../types";
import { SessionTable } from "./SessionTable";
import { SessionToolbar } from "./SessionToolbar";

interface SessionWorkspaceProps {
  error: string | null;
  highestPermissions: boolean;
  initialLoading: boolean;
  launchingSessionKey: string | null;
  permissionWarning: string | null;
  provider: SessionProvider;
  searchQuery: string;
  searching: boolean;
  selectedProject: ProjectSummary | null;
  sessions: SessionSummary[];
  showArchived: boolean;
  sort: SessionSort;
  onCopyId: (sessionId: string) => void;
  onHighestPermissionsChange: (enabled: boolean) => void;
  onRefresh: () => void;
  onResume: (session: SessionSummary, fork: boolean) => void;
  onShowArchivedChange: (show: boolean) => void;
  onSortChange: (sort: SessionSort) => void;
}

export function SessionWorkspace({
  error,
  highestPermissions,
  initialLoading,
  launchingSessionKey,
  permissionWarning,
  provider,
  searchQuery,
  searching,
  selectedProject,
  sessions,
  showArchived,
  sort,
  onCopyId,
  onHighestPermissionsChange,
  onRefresh,
  onResume,
  onShowArchivedChange,
  onSortChange,
}: SessionWorkspaceProps) {
  return (
    <main className="flex min-h-0 min-w-0 flex-col bg-surface">
      <SessionToolbar
        selectedProject={selectedProject}
        provider={provider}
        searchQuery={searchQuery}
        searching={searching}
        resultCount={sessions.length}
        sort={sort}
        showArchived={showArchived}
        highestPermissions={highestPermissions}
        onSortChange={onSortChange}
        onShowArchivedChange={onShowArchivedChange}
        onHighestPermissionsChange={onHighestPermissionsChange}
      />

      {permissionWarning && (
        <Alert
          status="danger"
          className="rounded-none border-x-0 border-t-0 border-l-[3px] border-l-danger py-2"
        >
          <Alert.Indicator>
            <ShieldAlert size={15} />
          </Alert.Indicator>
          <Alert.Content>
            <Alert.Description>{permissionWarning}</Alert.Description>
          </Alert.Content>
        </Alert>
      )}

      <div className="min-h-0 flex-1">
        {error ? (
          <EmptyState className="flex min-h-[240px] flex-col items-center justify-center p-8 text-center text-muted">
            <AlertCircle size={28} className="text-danger" />
            <h2 className="m-0 mt-3 text-[15px] font-bold text-foreground">无法读取本机会话</h2>
            <p className="mt-1.5 max-w-[520px] text-[12px] text-muted">{error}</p>
            <Button variant="outline" size="sm" className="mt-3.5" onPress={onRefresh}>
              <RefreshCw size={15} /> 重试
            </Button>
          </EmptyState>
        ) : initialLoading ? (
          <div className="flex min-h-[240px] items-center justify-center gap-2 text-[12px] text-muted">
            <Spinner size="sm" /> 正在扫描本机会话...
          </div>
        ) : (
          <SessionTable
            sessions={sessions}
            searching={searching}
            launchingSessionKey={launchingSessionKey}
            onResume={onResume}
            onCopyId={onCopyId}
          />
        )}
      </div>
    </main>
  );
}
