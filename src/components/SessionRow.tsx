import { Archive, Clock3, Copy, GitFork, LoaderCircle, Play } from "lucide-react";
import { Button, Chip, Table, Tooltip } from "@heroui/react";

import {
  formatAbsoluteTime,
  formatBytes,
  formatCount,
  formatRelativeTime,
  titleSourceLabel,
} from "../lib/format";
import type { SessionSummary } from "../types";

interface SessionRowProps {
  launchBlocked: boolean;
  launching: boolean;
  session: SessionSummary;
  onCopyId: (sessionId: string) => void;
  onResume: (session: SessionSummary, fork: boolean) => void;
}

export function SessionRow({
  launchBlocked,
  launching,
  session,
  onCopyId,
  onResume,
}: SessionRowProps) {
  const usage =
    session.messageCount !== null
      ? `${formatCount(session.messageCount)} 条消息`
      : session.tokensUsed !== null
        ? `${formatCount(session.tokensUsed)} Token`
        : null;
  const context = [
    session.branch,
    session.model ?? session.cliVersion ?? "版本未知",
    usage,
    formatBytes(session.fileSize),
    session.sourceDetail,
  ].filter((value): value is string => Boolean(value));
  const actionsDisabled = !session.canResume || launchBlocked;

  return (
    <Table.Row className="border-b border-border hover:bg-surface-secondary">
      <Table.Cell className="h-[88px] min-w-0 px-4 py-2 align-middle">
        <div className="flex min-w-0 items-center gap-1.5">
          <strong className="truncate text-[14px] font-semibold" title={session.title}>
            {session.title}
          </strong>
          <Chip
            size="sm"
            variant="soft"
            color="default"
            className={
              session.provider === "claude"
                ? "bg-[#f7e8df] text-[#8e4020] dark:bg-[#4a3025] dark:text-[#e59a76]"
                : "bg-[#e3eef8] text-[#205b91] dark:bg-[#243b50] dark:text-[#8dc1ef]"
            }
          >
            <Chip.Label>{session.provider === "claude" ? "Claude" : "Codex"}</Chip.Label>
          </Chip>
          {session.isArchived && (
            <Chip size="sm" variant="soft" color="warning">
              <Archive size={11} />
              <Chip.Label>已归档</Chip.Label>
            </Chip>
          )}
        </div>

        <div className="mt-1.5 flex min-w-0 items-center overflow-hidden text-[12px] text-muted">
          {context.map((value, index) => (
            <span
              key={`${index}:${value}`}
              className="truncate before:mx-1.5 before:text-muted before:content-['·'] first:before:hidden"
              title={value}
            >
              {value}
            </span>
          ))}
          <span className="hidden shrink-0 items-center gap-1 before:mx-1.5 before:content-['·'] max-[1000px]:inline-flex">
            <Clock3 size={12} />
            {formatRelativeTime(session.lastActivity)}
          </span>
        </div>

        <div className="mt-1 flex min-w-0 items-center gap-1 text-[11px] text-muted">
          <code className="max-w-[240px] truncate font-mono" title={session.id}>
            {session.id}
          </code>
          <Tooltip delay={500}>
            <Button
              isIconOnly
              variant="ghost"
              size="sm"
              className="h-6 w-6 min-w-6"
              aria-label="复制 Session ID"
              onPress={() => onCopyId(session.id)}
            >
              <Copy size={12} />
            </Button>
            <Tooltip.Content>复制 Session ID</Tooltip.Content>
          </Tooltip>
          <span className="truncate">{titleSourceLabel(session.titleSource)}</span>
        </div>
      </Table.Cell>

      <Table.Cell className="h-[88px] w-[132px] px-3 align-middle max-[1000px]:hidden">
        <time
          dateTime={session.lastActivity}
          title={formatAbsoluteTime(session.lastActivity)}
          className="block text-[12px] font-semibold"
        >
          {formatRelativeTime(session.lastActivity)}
        </time>
        <small className="mt-1 block text-[11px] text-muted">
          {formatAbsoluteTime(session.lastActivity)}
        </small>
      </Table.Cell>

      <Table.Cell className="h-[88px] w-[126px] px-3 align-middle">
        <div className="flex items-center justify-end gap-1.5">
          <Button
            variant="primary"
            size="sm"
            isDisabled={actionsDisabled}
            aria-label={session.canResume ? "在新终端中继续会话" : "原项目目录已不存在"}
            onPress={() => onResume(session, false)}
            className="w-[84px]"
          >
            {launching ? <LoaderCircle size={14} className="animate-spin" /> : <Play size={14} />}
            <span>{launching ? "启动中" : "继续"}</span>
          </Button>
          <Tooltip delay={500}>
            <Button
              isIconOnly
              variant="outline"
              size="sm"
              isDisabled={actionsDisabled}
              aria-label="分叉续接"
              onPress={() => onResume(session, true)}
            >
              <GitFork size={15} />
            </Button>
            <Tooltip.Content>分叉续接</Tooltip.Content>
          </Tooltip>
        </div>
      </Table.Cell>
    </Table.Row>
  );
}
