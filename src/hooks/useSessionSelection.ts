import { useCallback, useState } from "react";

import { sessionKey, type SessionKey } from "../lib/archive";
import { sessionKeys, toggleSessionKey } from "../lib/selection";
import type { SessionSummary } from "../types";

export function useSessionSelection() {
  const [selectionMode, setSelectionMode] = useState(false);
  const [selectedSessionKeys, setSelectedSessionKeys] = useState<Set<SessionKey>>(() => new Set());

  const clearSelection = useCallback(() => {
    setSelectedSessionKeys(new Set());
  }, []);

  const toggleSelectionMode = useCallback(() => {
    setSelectionMode((current) => !current);
    setSelectedSessionKeys(new Set());
  }, []);

  const toggleSession = useCallback((session: Pick<SessionSummary, "provider" | "id">) => {
    const key = sessionKey(session);
    setSelectedSessionKeys((current) => toggleSessionKey(current, key));
  }, []);

  const selectSessions = useCallback((sessions: readonly SessionSummary[]) => {
    setSelectedSessionKeys(sessionKeys(sessions));
  }, []);

  return {
    clearSelection,
    isSelected: (session: Pick<SessionSummary, "provider" | "id">) =>
      selectedSessionKeys.has(sessionKey(session)),
    selectedSessionKeys,
    selectSessions,
    selectionMode,
    toggleSelectionMode,
    toggleSession,
  };
}
