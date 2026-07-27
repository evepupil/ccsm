import { invoke } from "@tauri-apps/api/core";

import type { CliStatus, LaunchResult, SessionCatalog, SessionProvider } from "./types";

export function scanSessionCatalog(): Promise<SessionCatalog> {
  return invoke<SessionCatalog>("scan_session_catalog");
}

export function getCliStatuses(): Promise<CliStatus[]> {
  return invoke<CliStatus[]>("get_cli_statuses");
}

export function resumeSession(
  provider: SessionProvider,
  sessionId: string,
  fork: boolean,
  highestPermissions: boolean,
): Promise<LaunchResult> {
  return invoke<LaunchResult>("resume_session", {
    provider,
    sessionId,
    fork,
    highestPermissions,
  });
}
