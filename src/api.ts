import { invoke } from "@tauri-apps/api/core";

import type { CliStatus, LaunchResult, SessionCatalog } from "./types";

export function scanSessionCatalog(): Promise<SessionCatalog> {
  return invoke<SessionCatalog>("scan_session_catalog");
}

export function getCliStatus(): Promise<CliStatus> {
  return invoke<CliStatus>("get_cli_status");
}

export function resumeSession(sessionId: string, fork: boolean): Promise<LaunchResult> {
  return invoke<LaunchResult>("resume_session", { sessionId, fork });
}
