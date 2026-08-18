import { describe, expect, it } from "vitest";

import { toggleSessionKey } from "./selection";

describe("toggleSessionKey", () => {
  it("adds and removes a selected session key immutably", () => {
    const initial = new Set(["claude:one"] as const);

    const added = toggleSessionKey(initial, "codex:two");
    const removed = toggleSessionKey(added, "claude:one");

    expect([...initial]).toEqual(["claude:one"]);
    expect([...added].sort()).toEqual(["claude:one", "codex:two"]);
    expect([...removed]).toEqual(["codex:two"]);
  });
});
