export const SESSION_REFRESH_INTERVAL_MS = 10_000;

export function createSessionRefreshGate(refresh: () => Promise<void>): () => Promise<void> {
  let running = false;

  return async () => {
    if (running) return;

    running = true;
    try {
      await refresh();
    } finally {
      running = false;
    }
  };
}

export function startSessionAutoRefresh(refresh: () => void): () => void {
  const timer = globalThis.setInterval(refresh, SESSION_REFRESH_INTERVAL_MS);
  return () => globalThis.clearInterval(timer);
}
