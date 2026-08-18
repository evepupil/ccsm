import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { scanSessionCatalog } from "../api";
import { createSessionRefreshGate, startSessionAutoRefresh } from "../lib/autoRefresh";
import { toErrorMessage } from "../lib/presentation";
import type { SessionProvider } from "../types";

type SessionCatalog = Awaited<ReturnType<typeof scanSessionCatalog>>;

function readStoredProjectId(): string | null {
  try {
    return localStorage.getItem("selectedProjectId");
  } catch {
    return null;
  }
}

function storeProjectId(projectId: string | null) {
  try {
    if (projectId) {
      localStorage.setItem("selectedProjectId", projectId);
    } else {
      localStorage.removeItem("selectedProjectId");
    }
  } catch {
    // A blocked localStorage should not prevent browsing local sessions.
  }
}

export function useSessionCatalog() {
  const [catalog, setCatalog] = useState<SessionCatalog | null>(null);
  const [provider, setProvider] = useState<SessionProvider>("claude");
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(readStoredProjectId);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const mountedRef = useRef(true);
  const providerRef = useRef(provider);

  const refresh = useCallback(async () => {
    if (!mountedRef.current) return;

    setLoading(true);
    setError(null);
    try {
      const nextCatalog = await scanSessionCatalog();
      if (!mountedRef.current) return;
      setCatalog(nextCatalog);
      setSelectedProjectId((current) => {
        const currentProject = nextCatalog.projects.find((project) => project.id === current);
        const nextId = currentProject?.providers.includes(providerRef.current)
          ? current
          : (nextCatalog.projects.find((project) => project.providers.includes(providerRef.current))
              ?.id ?? null);
        storeProjectId(nextId);
        return nextId;
      });
    } catch (cause) {
      if (mountedRef.current) setError(toErrorMessage(cause));
    } finally {
      if (mountedRef.current) setLoading(false);
    }
  }, []);

  const refreshIfIdle = useMemo(() => createSessionRefreshGate(refresh), [refresh]);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  useEffect(() => {
    void refreshIfIdle();
    return startSessionAutoRefresh(() => {
      void refreshIfIdle();
    });
  }, [refreshIfIdle]);

  const selectedProject = useMemo(
    () => catalog?.projects.find((project) => project.id === selectedProjectId) ?? null,
    [catalog, selectedProjectId],
  );

  const selectProject = useCallback((projectId: string) => {
    setSelectedProjectId(projectId);
    storeProjectId(projectId);
  }, []);

  const switchProvider = useCallback(
    (nextProvider: SessionProvider) => {
      providerRef.current = nextProvider;
      setProvider(nextProvider);
      setSelectedProjectId((current) => {
        const projects = catalog?.projects ?? [];
        const currentProject = projects.find((project) => project.id === current);
        const nextId = currentProject?.providers.includes(nextProvider)
          ? current
          : (projects.find((project) => project.providers.includes(nextProvider))?.id ?? null);
        storeProjectId(nextId);
        return nextId;
      });
    },
    [catalog],
  );

  return {
    catalog,
    error,
    loading,
    provider,
    selectedProject,
    selectedProjectId,
    selectProject,
    switchProvider,
  };
}
