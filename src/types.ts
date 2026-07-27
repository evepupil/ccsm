export interface SessionCatalog {
  projects: ProjectSummary[];
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

export interface CliStatus {
  provider: SessionProvider;
  available: boolean;
  version: string | null;
  loggedIn: boolean | null;
  authMethod: string | null;
  apiProvider: string | null;
}

export interface LaunchResult {
  sessionId: string;
  provider: SessionProvider;
  terminal: string;
  forked: boolean;
  highestPermissions: boolean;
}

export type SessionSort = "recent" | "title";
export type ProviderFilter = "all" | SessionProvider;
