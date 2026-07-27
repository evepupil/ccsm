import { invoke } from "@tauri-apps/api/core";

import type { LaunchResult, SessionCatalog, SessionProvider } from "./types";

export function scanSessionCatalog(): Promise<SessionCatalog> {
  return invoke<SessionCatalog>("scan_session_catalog");
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
