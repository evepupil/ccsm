import { describe, expect, it } from "vitest";

import { highestPermissionWarning, launchSessionKey, workspaceLaunchKey } from "./launch";

describe("launchSessionKey", () => {
  it("keeps equal session ids from different providers distinct", () => {
    expect(launchSessionKey("claude", "same-id")).not.toBe(launchSessionKey("codex", "same-id"));
  });
});

describe("highestPermissionWarning", () => {
  it("is disabled by default and explicit when enabled", () => {
    expect(highestPermissionWarning(false)).toBeNull();
    expect(highestPermissionWarning(true)).toContain("--yolo");
  });
});

describe("workspaceLaunchKey", () => {
  it("keeps workspace restoration separate from session launches", () => {
    expect(workspaceLaunchKey("c:\\code\\ccsm")).toBe("workspace:c:\\code\\ccsm");
  });
});
