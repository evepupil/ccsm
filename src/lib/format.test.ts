import { describe, expect, it } from "vitest";

import { formatBytes, formatRelativeTime, normalizeSearch } from "./format";

describe("formatRelativeTime", () => {
  const now = Date.parse("2026-07-27T06:00:00Z");

  it("formats recent minutes", () => {
    expect(formatRelativeTime("2026-07-27T05:55:00Z", now)).toBe("5 分钟前");
  });

  it("formats elapsed days", () => {
    expect(formatRelativeTime("2026-07-24T06:00:00Z", now)).toBe("3 天前");
  });

  it("handles invalid timestamps", () => {
    expect(formatRelativeTime("invalid", now)).toBe("时间未知");
  });
});

describe("formatBytes", () => {
  it("selects a readable unit", () => {
    expect(formatBytes(1_536)).toBe("1.5 KB");
    expect(formatBytes(2 * 1024 * 1024)).toBe("2.0 MB");
  });
});

describe("normalizeSearch", () => {
  it("trims and normalizes case", () => {
    expect(normalizeSearch("  EdgeDisk  ")).toBe("edgedisk");
  });
});
