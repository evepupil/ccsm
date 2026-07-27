export interface SessionCatalog {
  projects: ProjectSummary[];
  scannedAt: string;
  sessionsRoot: string;
  warnings: string[];
}

export interface ProjectSummary {
  id: string;
  name: string;
  path: string;
  encodedDirectory: string;
  lastActivity: string;
  sessionCount: number;
  totalSize: number;
  sessions: SessionSummary[];
}

export interface SessionSummary {
  id: string;
  title: string;
  titleSource: string;
  projectPath: string;
  filePath: string;
  createdAt: string | null;
  lastActivity: string;
  messageCount: number;
  branch: string | null;
  model: string | null;
  claudeVersion: string | null;
  fileSize: number;
  isArchived: boolean;
  canResume: boolean;
}

export interface CliStatus {
  available: boolean;
  version: string | null;
  loggedIn: boolean | null;
  authMethod: string | null;
  apiProvider: string | null;
}

export interface LaunchResult {
  sessionId: string;
  terminal: string;
  forked: boolean;
}

export type SessionSort = "recent" | "title";
