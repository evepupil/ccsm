import { useCallback, useState } from "react";

import {
  isSessionArchived,
  readArchivedSessionKeys,
  updateArchivedSessionKeys,
  writeArchivedSessionKeys,
  type SessionKey,
} from "../lib/archive";
import type { SessionSummary } from "../types";

export function useSessionArchive() {
  const [archivedSessionKeys, setArchivedSessionKeys] =
    useState<Set<SessionKey>>(readArchivedSessionKeys);

  const updateArchive = useCallback(
    (sessions: readonly SessionSummary[], archived: boolean): boolean => {
      const next = updateArchivedSessionKeys(archivedSessionKeys, sessions, archived);
      if (!writeArchivedSessionKeys(next)) return false;
      setArchivedSessionKeys(next);
      return true;
    },
    [archivedSessionKeys],
  );

  const isArchived = useCallback(
    (session: SessionSummary) => isSessionArchived(session, archivedSessionKeys),
    [archivedSessionKeys],
  );

  return { archivedSessionKeys, isArchived, updateArchive };
}
