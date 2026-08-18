import { useCallback, useState } from "react";
import { toast } from "@heroui/react";

import { resumeSession, startNewSession } from "../api";
import { launchSessionKey, newSessionLaunchKey } from "../lib/launch";
import { providerLabel, toErrorMessage } from "../lib/presentation";
import { TEMPORARY_PROJECT_ID } from "../lib/temporary";
import type { SessionProvider, SessionSummary } from "../types";

export function useSessionLauncher(highestPermissions: boolean) {
  const [launchingSessionKey, setLaunchingSessionKey] = useState<string | null>(null);

  const launch = useCallback(
    async (session: SessionSummary, fork: boolean) => {
      setLaunchingSessionKey(launchSessionKey(session.provider, session.id));
      try {
        const result = await resumeSession(session.provider, session.id, fork, highestPermissions);
        toast.success(
          `${result.terminal} 已启动 ${providerLabel(result.provider)} ${
            result.forked ? "分叉会话" : "原会话"
          }${result.highestPermissions ? "（最高权限）" : ""}`,
        );
      } catch (cause) {
        toast.danger(toErrorMessage(cause));
      } finally {
        setLaunchingSessionKey(null);
      }
    },
    [highestPermissions],
  );

  const launchNew = useCallback(
    async (provider: SessionProvider, projectId: string) => {
      setLaunchingSessionKey(newSessionLaunchKey(provider, projectId));
      try {
        const result = await startNewSession(provider, projectId, highestPermissions);
        toast.success(
          `${result.terminal} 已启动 ${providerLabel(result.provider)} ${
            projectId === TEMPORARY_PROJECT_ID ? "临时会话" : "新会话"
          }${result.highestPermissions ? "（最高权限）" : ""}`,
        );
        if (projectId === TEMPORARY_PROJECT_ID) {
          toast.success(`工作目录：${result.workingDirectory}`);
        }
      } catch (cause) {
        toast.danger(toErrorMessage(cause));
      } finally {
        setLaunchingSessionKey(null);
      }
    },
    [highestPermissions],
  );

  const copySessionId = useCallback(async (sessionId: string) => {
    try {
      await navigator.clipboard.writeText(sessionId);
      toast.success("Session ID 已复制");
    } catch {
      toast.danger("无法访问系统剪贴板");
    }
  }, []);

  return { copySessionId, launch, launchNew, launchingSessionKey };
}
