import type { SessionProvider } from "../types";

export function launchSessionKey(provider: SessionProvider, sessionId: string): string {
  return `${provider}:${sessionId}`;
}

export function highestPermissionWarning(enabled: boolean): string | null {
  if (!enabled) return null;
  return "最高权限已开启：Claude Code 将跳过权限确认，Codex 将使用 --yolo。";
}
