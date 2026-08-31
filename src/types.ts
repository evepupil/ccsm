export interface SessionCatalog {
  projects: ProjectSummary[];
  openSessions: OpenSessionSummary[];
  scannedAt: string;
  sources: SessionSource[];
  warnings: string[];
}

export type SessionProvider = "claude" | "codex";

export interface SessionSource {
  provider: SessionProvider;
  location: string;
  available: boolean;
  sessionCount: number;
  error: string | null;
}

export interface ProjectSummary {
  id: string;
  name: string;
  path: string;
  isTemporary: boolean;
  lastActivity: string;
  sessionCount: number;
  totalSize: number;
  providers: SessionProvider[];
  sessions: SessionSummary[];
}

export interface SessionSummary {
  id: string;
  provider: SessionProvider;
  title: string;
  titleSource: string;
  projectPath: string;
  sourcePath: string;
  sourceDetail: string | null;
  createdAt: string | null;
  lastActivity: string;
  messageCount: number | null;
  tokensUsed: number | null;
  branch: string | null;
  model: string | null;
  cliVersion: string | null;
  fileSize: number;
  isArchived: boolean;
  canResume: boolean;
}

export interface OpenSessionSummary {
  sessionId: string;
  provider: SessionProvider;
  projectPath: string;
  workingDirectory: string;
}

export interface LaunchResult {
  sessionId: string;
  provider: SessionProvider;
  terminal: string;
  forked: boolean;
  highestPermissions: boolean;
  reusedWindow: boolean;
}

export interface NewSessionResult {
  provider: SessionProvider;
  terminal: string;
  workingDirectory: string;
  highestPermissions: boolean;
  sessionId: string | null;
}

export interface WorkspaceRestoreResult {
  projectPath: string;
  terminal: string;
  restoredCount: number;
  alreadyOpenCount: number;
  unidentifiedCount: number;
  skippedCount: number;
  oneWindow: boolean;
}

export type SessionSort = "recent" | "title";
