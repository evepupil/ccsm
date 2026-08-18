import { sessionKey, type SessionKey } from "./archive";
import type { SessionSummary } from "../types";

export function toggleSessionKey(
  current: ReadonlySet<SessionKey>,
  key: SessionKey,
): Set<SessionKey> {
  const next = new Set(current);
  if (next.has(key)) {
    next.delete(key);
  } else {
    next.add(key);
  }
  return next;
}

export function sessionKeys(sessions: readonly SessionSummary[]): Set<SessionKey> {
  return new Set(sessions.map(sessionKey));
}
