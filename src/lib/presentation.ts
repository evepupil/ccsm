import type { SessionProvider } from "../types";

export function providerLabel(provider: SessionProvider): string {
  return provider === "claude" ? "Claude Code" : "Codex";
}

export function toErrorMessage(cause: unknown): string {
  if (cause instanceof Error) return cause.message;
  if (typeof cause === "string") return cause;
  return "发生未知错误";
}
