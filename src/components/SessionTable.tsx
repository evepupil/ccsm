import {
  Archive,
  Copy,
  Gauge,
  GitBranch,
  GitFork,
  MessageSquare,
  Play,
  TerminalSquare,
} from "lucide-react";
import { Button, EmptyState, Table } from "@heroui/react";

import {
  formatAbsoluteTime,
  formatBytes,
  formatCount,
  formatRelativeTime,
  titleSourceLabel,
} from "../lib/format";
import type { SessionSummary } from "../types";

interface SessionTableProps {
  sessions: SessionSummary[];
  launchingSessionKey: string | null;
  onResume: (session: SessionSummary, fork: boolean) => void;
  onCopyId: (sessionId: string) => void;
}

export function SessionTable({
  sessions,
  launchingSessionKey,
  onResume,
  onCopyId,
}: SessionTableProps) {
  if (sessions.length === 0) {
    return (
      <EmptyState className="flex min-h-[240px] flex-col items-center justify-center p-8 text-center text-muted">
        <MessageSquare size={26} />
        <h2 className="m-0 mt-3 text-[15px] font-bold text-foreground">没有可显示的会话</h2>
        <p className="mt-1.5 text-[12px] text-muted">调整搜索条件或打开“显示归档”。</p>
      </EmptyState>
    );
  }

  return (
    <div className="min-h-0 min-w-0 flex-1">
      <Table className="text-[12px]">
        <Table.ScrollContainer className="min-h-0 overflow-auto">
          <Table.Content className="min-w-[760px]">
            <Table.Header>
              <Table.Column className="sticky top-0 z-[2] h-[34px] w-[39%] bg-surface-secondary px-3 text-[10px] font-bold uppercase text-muted">
                会话
              </Table.Column>
              <Table.Column className="sticky top-0 z-[2] h-[34px] w-[17%] bg-surface-secondary px-3 text-[10px] font-bold uppercase text-muted">
                最近活动
              </Table.Column>
              <Table.Column className="sticky top-0 z-[2] h-[34px] w-[25%] bg-surface-secondary px-3 text-[10px] font-bold uppercase text-muted">
                上下文
              </Table.Column>
              <Table.Column className="sticky top-0 z-[2] h-[34px] w-[138px] bg-surface-secondary px-3 text-[10px] font-bold uppercase text-muted">
                操作
              </Table.Column>
            </Table.Header>
            <Table.Body>
              {sessions.map((session) => {
                const sessionKey = `${session.provider}:${session.id}`;
                const launching = launchingSessionKey === sessionKey;
                return (
                  <Table.Row
                    key={sessionKey}
                    className="border-b border-border hover:bg-surface-secondary"
                  >
                    <Table.Cell className="h-[70px] px-3 align-middle">
                      <div className="flex min-w-0 items-center gap-1.5">
                        <strong
                          className="truncate text-[12px] font-semibold text-foreground"
                          title={session.title}
                        >
                          {session.title}
                        </strong>
                        {session.isArchived && (
                          <Archive
                            size={13}
                            className="flex-[0_0_auto] text-warning"
                            aria-label="已归档"
                          />
                        )}
                      </div>
                      <div className="mt-1.5 flex min-w-0 items-center gap-1 text-[10px] text-muted">
                        <span
                          className={`inline-flex flex-[0_0_auto] items-center rounded px-1.5 py-0.5 font-bold ${
                            session.provider === "claude"
                              ? "bg-[#f8e7df] text-[#8c3b1c] dark:bg-[#4b2c20] dark:text-[#f1b293]"
                              : "bg-[#e3eef9] text-[#205b9e] dark:bg-[#20394f] dark:text-[#9bc6ef]"
                          }`}
                        >
                          {session.provider === "claude" ? "Claude" : "Codex"}
                        </span>
                        <code
                          className="max-w-[120px] truncate font-mono text-muted"
                          title={session.id}
                        >
                          {session.id}
                        </code>
                        <Button
                          isIconOnly
                          variant="ghost"
                          size="sm"
                          aria-label="复制 Session ID"
                          onPress={() => onCopyId(session.id)}
                        >
                          <Copy size={13} />
                        </Button>
                        <span className="truncate">{titleSourceLabel(session.titleSource)}</span>
                      </div>
                    </Table.Cell>
                    <Table.Cell className="h-[70px] px-3 align-middle">
                      <time
                        dateTime={session.lastActivity}
                        title={formatAbsoluteTime(session.lastActivity)}
                        className="block font-semibold text-foreground"
                      >
                        {formatRelativeTime(session.lastActivity)}
                      </time>
                      <small className="mt-1 block text-[10px] text-muted">
                        {formatAbsoluteTime(session.lastActivity)}
                      </small>
                    </Table.Cell>
                    <Table.Cell className="h-[70px] px-3 align-middle">
                      <div className="flex items-center gap-2.5 text-[11px] text-foreground">
                        {session.messageCount !== null && (
                          <span
                            title="消息记录数"
                            className="inline-flex min-w-0 items-center gap-1 truncate"
                          >
                            <MessageSquare size={13} /> {formatCount(session.messageCount)} 条消息
                          </span>
                        )}
                        {session.tokensUsed !== null && (
                          <span
                            title="累计 Token 数"
                            className="inline-flex min-w-0 items-center gap-1 truncate"
                          >
                            <Gauge size={13} /> {formatCount(session.tokensUsed)} Token
                          </span>
                        )}
                        <span
                          title="会话文件大小"
                          className="inline-flex min-w-0 items-center gap-1 truncate"
                        >
                          {formatBytes(session.fileSize)}
                        </span>
                      </div>
                      <div className="mt-1.5 flex items-center gap-2.5 text-[10px] text-muted">
                        {session.branch && (
                          <span
                            title="Git 分支"
                            className="inline-flex min-w-0 items-center gap-1 truncate"
                          >
                            <GitBranch size={13} /> {session.branch}
                          </span>
                        )}
                        <span className="truncate">
                          {session.model ?? session.cliVersion ?? "版本未知"}
                        </span>
                        {session.sourceDetail && (
                          <span className="truncate">{session.sourceDetail}</span>
                        )}
                      </div>
                    </Table.Cell>
                    <Table.Cell className="h-[70px] px-3 align-middle">
                      <div className="flex items-center justify-end gap-1.5">
                        <Button
                          variant="primary"
                          size="sm"
                          isDisabled={!session.canResume || launching}
                          aria-label={
                            session.canResume ? "在新终端中续接会话" : "原项目目录已不存在"
                          }
                          onPress={() => onResume(session, false)}
                          className="w-[78px]"
                        >
                          {launching ? <TerminalSquare size={15} /> : <Play size={15} />}
                          <span>{launching ? "启动中" : "续接"}</span>
                        </Button>
                        <Button
                          isIconOnly
                          variant="outline"
                          size="sm"
                          isDisabled={!session.canResume || launching}
                          aria-label="分叉续接，保留原 Session ID"
                          onPress={() => onResume(session, true)}
                        >
                          <GitFork size={16} />
                        </Button>
                      </div>
                    </Table.Cell>
                  </Table.Row>
                );
              })}
            </Table.Body>
          </Table.Content>
        </Table.ScrollContainer>
      </Table>
    </div>
  );
}
