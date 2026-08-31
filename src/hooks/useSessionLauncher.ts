import { useCallback, useState } from "react";
import { toast } from "@heroui/react";

import { restoreWorkspace, resumeSession, startNewSession } from "../api";
import { launchSessionKey, newSessionLaunchKey, workspaceLaunchKey } from "../lib/launch";
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

  const restoreWorkspaceSessions = useCallback(
    async (projectId: string) => {
      setLaunchingSessionKey(workspaceLaunchKey(projectId));
      try {
        const result = await restoreWorkspace(projectId, highestPermissions);
        if (
          result.restoredCount === 0 &&
          result.alreadyOpenCount === 0 &&
          result.unidentifiedCount === 0 &&
          result.skippedCount === 0
        ) {
          toast.warning("当前项目还没有可恢复的工作区快照");
        } else {
          if (result.restoredCount > 0) {
            const location = result.oneWindow ? "同一窗口" : result.terminal;
            toast.success(`已在${location}创建 ${result.restoredCount} 个恢复标签`);
          }
          if (result.alreadyOpenCount > 0) {
            toast.success(`${result.alreadyOpenCount} 个会话仍在运行，已跳过重复续接`);
          }
          if (result.unidentifiedCount > 0) {
            toast.warning(`${result.unidentifiedCount} 个运行中会话身份未确认，已保留原窗口`);
          }
        }
        if (result.skippedCount > 0) {
          toast.danger(`${result.skippedCount} 个快照会话未能创建恢复标签`);
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

  return {
    copySessionId,
    launch,
    launchNew,
    launchingSessionKey,
    restoreWorkspace: restoreWorkspaceSessions,
  };
}
