import { MessageSquareOff } from "lucide-react";
import { EmptyState, Table } from "@heroui/react";

import type { SessionSummary } from "../types";
import { SessionRow } from "./SessionRow";

interface SessionTableProps {
  sessions: SessionSummary[];
  launchingSessionKey: string | null;
  searching: boolean;
  onResume: (session: SessionSummary, fork: boolean) => void;
  onCopyId: (sessionId: string) => void;
}

export function SessionTable({
  sessions,
  launchingSessionKey,
  searching,
  onResume,
  onCopyId,
}: SessionTableProps) {
  if (sessions.length === 0) {
    return (
      <EmptyState className="flex min-h-[240px] flex-col items-center justify-center p-8 text-center text-muted">
        <MessageSquareOff size={28} />
        <h2 className="m-0 mt-3 text-[15px] font-bold text-foreground">没有可显示的会话</h2>
        <p className="mt-1.5 text-[12px] text-muted">
          {searching ? "没有找到匹配结果。" : "当前筛选条件下没有会话。"}
        </p>
      </EmptyState>
    );
  }

  return (
    <div className="h-full min-h-0 min-w-0">
      <Table variant="secondary" className="h-full min-h-0 rounded-none border-0 text-[12px]">
        <Table.ScrollContainer className="h-full min-h-0 overflow-auto rounded-none border-0">
          <Table.Content className="min-w-[520px] table-fixed max-[1000px]:min-w-[480px]">
            <Table.Header>
              <Table.Column className="sticky top-0 z-[2] h-9 bg-surface-secondary px-4 text-[11px] font-bold text-muted">
                会话
              </Table.Column>
              <Table.Column className="sticky top-0 z-[2] h-9 w-[132px] bg-surface-secondary px-3 text-[11px] font-bold text-muted max-[1000px]:hidden">
                最近活动
              </Table.Column>
              <Table.Column className="sticky top-0 z-[2] h-9 w-[126px] bg-surface-secondary px-3 text-right text-[11px] font-bold text-muted">
                <span className="block text-right">操作</span>
              </Table.Column>
            </Table.Header>
            <Table.Body>
              {sessions.map((session) => {
                const sessionKey = `${session.provider}:${session.id}`;
                return (
                  <SessionRow
                    key={sessionKey}
                    session={session}
                    launching={launchingSessionKey === sessionKey}
                    launchBlocked={launchingSessionKey !== null}
                    onResume={onResume}
                    onCopyId={onCopyId}
                  />
                );
              })}
            </Table.Body>
          </Table.Content>
        </Table.ScrollContainer>
      </Table>
    </div>
  );
}
