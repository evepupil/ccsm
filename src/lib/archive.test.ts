import { describe, expect, it } from "vitest";

import {
  parseArchivedSessionKeys,
  serializeArchivedSessionKeys,
  sessionKey,
  updateArchivedSessionKeys,
} from "./archive";
import type { SessionSummary } from "../types";

const claudeSession = {
  provider: "claude",
  id: "00000000-0000-4000-8000-000000000001",
} as Pick<SessionSummary, "provider" | "id">;

const codexSession = {
  provider: "codex",
  id: "00000000-0000-4000-8000-000000000001",
} as Pick<SessionSummary, "provider" | "id">;

describe("archive state", () => {
  it("keeps provider session keys distinct", () => {
    expect(sessionKey(claudeSession)).not.toBe(sessionKey(codexSession));
  });

  it("ignores malformed persisted keys and serializes deterministically", () => {
    const parsed = parseArchivedSessionKeys(
      JSON.stringify(["codex:two", "invalid", "claude:one", 3]),
    );

    expect(serializeArchivedSessionKeys(parsed)).toBe('["claude:one","codex:two"]');
  });

  it("archives and restores a batch without touching the original set", () => {
    const current = new Set([sessionKey(claudeSession)]);
    const archived = updateArchivedSessionKeys(current, [codexSession], true);
    const restored = updateArchivedSessionKeys(archived, [claudeSession], false);

    expect([...current]).toEqual(["claude:00000000-0000-4000-8000-000000000001"]);
    expect([...archived].sort()).toEqual([
      "claude:00000000-0000-4000-8000-000000000001",
      "codex:00000000-0000-4000-8000-000000000001",
    ]);
    expect([...restored]).toEqual(["codex:00000000-0000-4000-8000-000000000001"]);
  });
});
