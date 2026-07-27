import {
  Archive,
  Copy,
  GitBranch,
  GitFork,
  MessageSquare,
  Play,
  TerminalSquare,
} from "lucide-react";

import {
  formatAbsoluteTime,
  formatBytes,
  formatRelativeTime,
  titleSourceLabel,
} from "../lib/format";
import type { SessionSummary } from "../types";

interface SessionTableProps {
  sessions: SessionSummary[];
  launchingSessionId: string | null;
  onResume: (session: SessionSummary, fork: boolean) => void;
  onCopyId: (sessionId: string) => void;
}

export function SessionTable({
  sessions,
  launchingSessionId,
  onResume,
  onCopyId,
}: SessionTableProps) {
  if (sessions.length === 0) {
    return (
      <div className="empty-state">
        <MessageSquare size={26} />
        <h2>没有可显示的会话</h2>
        <p>调整搜索条件或打开“显示归档”。</p>
      </div>
    );
  }

  return (
    <div className="session-table-wrap">
      <table className="session-table">
        <thead>
          <tr>
            <th>会话</th>
            <th>最近活动</th>
            <th>上下文</th>
            <th className="actions-column">操作</th>
          </tr>
        </thead>
        <tbody>
          {sessions.map((session) => {
            const launching = launchingSessionId === session.id;
            return (
              <tr key={session.id}>
                <td>
                  <div className="session-title-line">
                    <strong title={session.title}>{session.title}</strong>
                    {session.isArchived && <Archive size={13} aria-label="已归档" />}
                  </div>
                  <div className="session-id-line">
                    <code title={session.id}>{session.id}</code>
                    <button
                      className="icon-button compact"
                      type="button"
                      title="复制 Session ID"
                      aria-label="复制 Session ID"
                      onClick={() => onCopyId(session.id)}
                    >
                      <Copy size={13} />
                    </button>
                    <span>{titleSourceLabel(session.titleSource)}</span>
                  </div>
                </td>
                <td>
                  <time
                    dateTime={session.lastActivity}
                    title={formatAbsoluteTime(session.lastActivity)}
                  >
                    {formatRelativeTime(session.lastActivity)}
                  </time>
                  <small>{formatAbsoluteTime(session.lastActivity)}</small>
                </td>
                <td>
                  <div className="metadata-line">
                    <span title="消息记录数">
                      <MessageSquare size={13} /> {session.messageCount}
                    </span>
                    <span title="会话文件大小">{formatBytes(session.fileSize)}</span>
                  </div>
                  <div className="metadata-line subdued">
                    {session.branch && (
                      <span title="Git 分支">
                        <GitBranch size={13} /> {session.branch}
                      </span>
                    )}
                    <span>{session.model ?? session.claudeVersion ?? "版本未知"}</span>
                  </div>
                </td>
                <td>
                  <div className="row-actions">
                    <button
                      className="resume-button"
                      type="button"
                      disabled={!session.canResume || launching}
                      title={session.canResume ? "在新终端中续接会话" : "原项目目录已不存在"}
                      onClick={() => onResume(session, false)}
                    >
                      {launching ? <TerminalSquare size={15} /> : <Play size={15} />}
                      <span>{launching ? "启动中" : "续接"}</span>
                    </button>
                    <button
                      className="icon-button"
                      type="button"
                      disabled={!session.canResume || launching}
                      title="分叉续接，保留原 Session ID"
                      aria-label="分叉续接"
                      onClick={() => onResume(session, true)}
                    >
                      <GitFork size={16} />
                    </button>
                  </div>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
