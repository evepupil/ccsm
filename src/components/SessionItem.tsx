import {
  Archive,
  ArchiveRestore,
  Copy,
  Cpu,
  Folder,
  GitBranch,
  GitFork,
  LoaderCircle,
  MessagesSquare,
  Play,
} from "lucide-react";

import { formatAbsoluteTime, formatCount, formatRelativeTime } from "../lib/format";
import type { SessionSummary } from "../types";
import { ProviderLogo } from "./ProviderLogo";

interface SessionItemProps {
  archived: boolean;
  ccsmArchived: boolean;
  launchBlocked: boolean;
  launching: boolean;
  session: SessionSummary;
  selectable: boolean;
  selected: boolean;
  showProject: boolean;
  onArchive: (session: SessionSummary, archived: boolean) => void;
  onCopyId: (sessionId: string) => void;
  onResume: (session: SessionSummary, fork: boolean) => void;
  onToggleSelection: (session: SessionSummary) => void;
}

export function SessionItem({
  archived,
  ccsmArchived,
  launchBlocked,
  launching,
  session,
  selectable,
  selected,
  showProject,
  onArchive,
  onCopyId,
  onResume,
  onToggleSelection,
}: SessionItemProps) {
  const usage =
    session.messageCount !== null
      ? `${formatCount(session.messageCount)} 条消息`
      : session.tokensUsed !== null
        ? `${formatCount(session.tokensUsed)} Token`
        : null;
  const model = session.model ?? session.cliVersion;
  const projectName = session.projectPath.split(/[\\/]/).filter(Boolean).at(-1);
  const actionsDisabled = !session.canResume || launchBlocked;
  const archiveDisabled = launchBlocked || (session.isArchived && !ccsmArchived);
  const archiveLabel = ccsmArchived ? "取消归档" : session.isArchived ? "已归档" : "归档";

  return (
    <article
      className={`session-card ${selected ? "is-selected" : ""} ${selectable ? "is-selectable" : ""}`}
      data-session-id={session.id}
    >
      <div className="session-selection-slot">
        {selectable && (
          <label className="session-selection" title={selected ? "取消选择" : "选择会话"}>
            <input
              type="checkbox"
              checked={selected}
              aria-label={`${selected ? "取消选择" : "选择"} ${session.title}`}
              onChange={() => onToggleSelection(session)}
            />
          </label>
        )}
      </div>
      <span className={`session-provider-icon ${session.provider}`} aria-hidden="true">
        <ProviderLogo provider={session.provider} />
      </span>

      <div className="session-copy">
        <div className="session-heading-line">
          <h3 className="session-title" title={session.title}>
            {session.title}
          </h3>
          {archived && (
            <span className="archived-badge">
              <Archive aria-hidden="true" />
              已归档
            </span>
          )}
        </div>

        <div className="session-meta">
          {showProject && projectName && (
            <span>
              <Folder aria-hidden="true" />
              {projectName}
            </span>
          )}
          {session.branch && (
            <span>
              <GitBranch aria-hidden="true" />
              {session.branch}
            </span>
          )}
          {model && (
            <span className="model-meta">
              <Cpu aria-hidden="true" />
              {model}
            </span>
          )}
          {usage && (
            <span>
              <MessagesSquare aria-hidden="true" />
              {usage}
            </span>
          )}
        </div>

        <div className="session-id-line">
          <code className="session-id" title={session.id}>
            {session.id}
          </code>
          <button
            className="copy-button"
            type="button"
            title="复制 Session ID"
            aria-label="复制 Session ID"
            onClick={() => onCopyId(session.id)}
          >
            <Copy aria-hidden="true" />
          </button>
        </div>
      </div>

      <time
        className="session-activity"
        dateTime={session.lastActivity}
        title={formatAbsoluteTime(session.lastActivity)}
      >
        {formatRelativeTime(session.lastActivity)}
        <small>{formatAbsoluteTime(session.lastActivity)}</small>
      </time>

      <div className="session-actions">
        <button
          className={`resume-button ${launching ? "is-launching" : ""}`}
          type="button"
          disabled={actionsDisabled}
          title={session.canResume ? "在新终端中继续会话" : "原项目目录已不存在"}
          onClick={() => onResume(session, false)}
        >
          {launching ? (
            <LoaderCircle className="is-spinning" aria-hidden="true" />
          ) : (
            <Play aria-hidden="true" />
          )}
          <span>{launching ? "启动中" : session.canResume ? "继续" : "不可用"}</span>
        </button>
        <button
          className="icon-button fork-button"
          type="button"
          disabled={actionsDisabled}
          title="分叉续接"
          aria-label="分叉续接"
          onClick={() => onResume(session, true)}
        >
          <GitFork aria-hidden="true" />
        </button>
        <button
          className={`icon-button archive-button ${archived ? "is-archived" : ""}`}
          type="button"
          disabled={archiveDisabled}
          title={archiveLabel}
          aria-label={archiveLabel}
          onClick={() => onArchive(session, !ccsmArchived)}
        >
          {ccsmArchived ? <ArchiveRestore aria-hidden="true" /> : <Archive aria-hidden="true" />}
        </button>
      </div>
    </article>
  );
}
