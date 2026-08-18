import { afterEach, describe, expect, it, vi } from "vitest";

import {
  createSessionRefreshGate,
  SESSION_REFRESH_INTERVAL_MS,
  startSessionAutoRefresh,
} from "./autoRefresh";

afterEach(() => {
  vi.useRealTimers();
});

describe("session auto refresh", () => {
  it("skips a scheduled run while the previous scan is active", async () => {
    const releases: Array<() => void> = [];
    const refresh = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          releases.push(resolve);
        }),
    );
    const refreshIfIdle = createSessionRefreshGate(refresh);

    const firstRun = refreshIfIdle();
    const skippedRun = refreshIfIdle();
    expect(refresh).toHaveBeenCalledTimes(1);

    releases[0]?.();
    await firstRun;
    await skippedRun;
    const secondRun = refreshIfIdle();
    expect(refresh).toHaveBeenCalledTimes(2);
    releases[1]?.();
    await secondRun;
  });

  it("runs every ten seconds and stops when cleaned up", () => {
    vi.useFakeTimers();
    const refresh = vi.fn();
    const stop = startSessionAutoRefresh(refresh);

    vi.advanceTimersByTime(SESSION_REFRESH_INTERVAL_MS - 1);
    expect(refresh).not.toHaveBeenCalled();

    vi.advanceTimersByTime(1);
    vi.advanceTimersByTime(SESSION_REFRESH_INTERVAL_MS);
    expect(refresh).toHaveBeenCalledTimes(2);

    stop();
    vi.advanceTimersByTime(SESSION_REFRESH_INTERVAL_MS);
    expect(refresh).toHaveBeenCalledTimes(2);
  });
});
