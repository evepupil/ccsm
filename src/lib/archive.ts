import type { SessionProvider, SessionSummary } from "../types";

export type SessionKey = `${SessionProvider}:${string}`;

export const ARCHIVED_SESSIONS_STORAGE_KEY = "ccsm.archivedSessions.v1";

function isSessionKey(value: unknown): value is SessionKey {
  return typeof value === "string" && /^(claude|codex):[^:]+$/.test(value);
}

export function sessionKey(session: Pick<SessionSummary, "provider" | "id">): SessionKey {
  return `${session.provider}:${session.id}`;
}

export function parseArchivedSessionKeys(raw: string | null): Set<SessionKey> {
  if (!raw) return new Set();

  try {
    const values: unknown = JSON.parse(raw);
    if (!Array.isArray(values)) return new Set();
    return new Set(values.filter(isSessionKey));
  } catch {
    return new Set();
  }
}

export function serializeArchivedSessionKeys(keys: ReadonlySet<SessionKey>): string {
  return JSON.stringify([...keys].sort());
}

export function updateArchivedSessionKeys(
  current: ReadonlySet<SessionKey>,
  sessions: readonly Pick<SessionSummary, "provider" | "id">[],
  archived: boolean,
): Set<SessionKey> {
  const next = new Set(current);
  for (const session of sessions) {
    const key = sessionKey(session);
    if (archived) {
      next.add(key);
    } else {
      next.delete(key);
    }
  }
  return next;
}

export function isSessionArchived(
  session: Pick<SessionSummary, "provider" | "id" | "isArchived">,
  archivedSessionKeys: ReadonlySet<SessionKey>,
): boolean {
  return session.isArchived || archivedSessionKeys.has(sessionKey(session));
}

export function readArchivedSessionKeys(): Set<SessionKey> {
  try {
    return parseArchivedSessionKeys(localStorage.getItem(ARCHIVED_SESSIONS_STORAGE_KEY));
  } catch {
    return new Set();
  }
}

export function writeArchivedSessionKeys(keys: ReadonlySet<SessionKey>): boolean {
  try {
    localStorage.setItem(ARCHIVED_SESSIONS_STORAGE_KEY, serializeArchivedSessionKeys(keys));
    return true;
  } catch {
    return false;
  }
}
