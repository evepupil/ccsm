import { describe, expect, it } from "vitest";

import type { ProjectSummary, SessionSummary } from "../types";
import { selectVisibleSessions } from "./sessions";

const baseSession: SessionSummary = {
  id: "00000000-0000-4000-8000-000000000001",
  title: "实现会话管理器",
  titleSource: "summary",
  projectPath: "C:\\code\\manager",
  filePath: "session.jsonl",
  createdAt: "2026-07-26T06:00:00Z",
  lastActivity: "2026-07-27T06:00:00Z",
  messageCount: 8,
  branch: "main",
  model: "claude-sonnet",
  claudeVersion: "2.1.217",
  fileSize: 1024,
  isArchived: false,
  canResume: true,
};

const projects: ProjectSummary[] = [
  {
    id: "manager",
    name: "manager",
    path: "C:\\code\\manager",
    encodedDirectory: "C--code-manager",
    lastActivity: baseSession.lastActivity,
    sessionCount: 2,
    totalSize: 2048,
    sessions: [
      baseSession,
      {
        ...baseSession,
        id: "00000000-0000-4000-8000-000000000002",
        title: "旧的归档记录",
        lastActivity: "2026-07-20T06:00:00Z",
        isArchived: true,
      },
    ],
  },
  {
    id: "edgedisk",
    name: "EdgeDisk",
    path: "C:\\code\\EdgeDisk",
    encodedDirectory: "C--code-EdgeDisk",
    lastActivity: "2026-07-25T06:00:00Z",
    sessionCount: 1,
    totalSize: 512,
    sessions: [
      {
        ...baseSession,
        id: "00000000-0000-4000-8000-000000000003",
        title: "对象存储优化",
        projectPath: "C:\\code\\EdgeDisk",
        lastActivity: "2026-07-25T06:00:00Z",
      },
    ],
  },
];

describe("selectVisibleSessions", () => {
  it("shows the selected project's active sessions by default", () => {
    const result = selectVisibleSessions({
      projects,
      selectedProjectId: "manager",
      searchQuery: "",
      showArchived: false,
      sort: "recent",
    });

    expect(result.map((session) => session.title)).toEqual(["实现会话管理器"]);
  });

  it("searches across all projects", () => {
    const result = selectVisibleSessions({
      projects,
      selectedProjectId: "manager",
      searchQuery: "EdgeDisk",
      showArchived: false,
      sort: "recent",
    });

    expect(result.map((session) => session.title)).toEqual(["对象存储优化"]);
  });

  it("can include archived sessions and sort by title", () => {
    const result = selectVisibleSessions({
      projects,
      selectedProjectId: "manager",
      searchQuery: "",
      showArchived: true,
      sort: "title",
    });

    expect(result.map((session) => session.title)).toEqual(["旧的归档记录", "实现会话管理器"]);
  });
});
