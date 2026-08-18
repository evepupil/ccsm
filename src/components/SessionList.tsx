import { isSessionArchived, sessionKey, type SessionKey } from "../lib/archive";
import type { SessionSort, SessionSummary } from "../types";
import { SessionItem } from "./SessionItem";

interface SessionListProps {
  archivedSessionKeys: ReadonlySet<SessionKey>;
  launchingSessionKey: string | null;
  onArchive: (session: SessionSummary, archived: boolean) => void;
  sessions: SessionSummary[];
  selectedSessionKeys: ReadonlySet<SessionKey>;
  selectionMode: boolean;
  showProject: boolean;
  sort: SessionSort;
  onCopyId: (sessionId: string) => void;
  onResume: (session: SessionSummary, fork: boolean) => void;
  onToggleSelection: (session: SessionSummary) => void;
}

interface SessionGroup {
  label: string;
  sessions: SessionSummary[];
}

const recentGroupLabels = ["今天", "昨天", "本周", "更早"] as const;

function recentGroupLabel(
  lastActivity: string,
  now = new Date(),
): (typeof recentGroupLabels)[number] {
  const activity = new Date(lastActivity);
  if (Number.isNaN(activity.getTime())) return "更早";

  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const activityDay = new Date(activity.getFullYear(), activity.getMonth(), activity.getDate());
  const daysAgo = Math.round((today.getTime() - activityDay.getTime()) / 86_400_000);
  if (daysAgo <= 0) return "今天";
  if (daysAgo === 1) return "昨天";
  if (daysAgo < 7) return "本周";
  return "更早";
}

function groupSessions(sessions: SessionSummary[], sort: SessionSort): SessionGroup[] {
  if (sort === "title") return [{ label: "按标题排序", sessions }];

  return recentGroupLabels
    .map((label) => ({
      label,
      sessions: sessions.filter((session) => recentGroupLabel(session.lastActivity) === label),
    }))
    .filter((group) => group.sessions.length > 0);
}

export function SessionList({
  archivedSessionKeys,
  launchingSessionKey,
  onArchive,
  sessions,
  selectedSessionKeys,
  selectionMode,
  showProject,
  sort,
  onCopyId,
  onResume,
  onToggleSelection,
}: SessionListProps) {
  const groups = groupSessions(sessions, sort);

  return (
    <div className="session-groups">
      {groups.map((group) => (
        <section className="session-group" aria-label={group.label} key={group.label}>
          <div className="session-group-heading">
            <span>{group.label}</span>
            <small>{group.sessions.length}</small>
          </div>
          <div className="session-list">
            {group.sessions.map((session) => {
              const key = sessionKey(session);
              return (
                <SessionItem
                  key={key}
                  archived={isSessionArchived(session, archivedSessionKeys)}
                  ccsmArchived={archivedSessionKeys.has(key)}
                  session={session}
                  selectable={selectionMode}
                  selected={selectedSessionKeys.has(sessionKey(session))}
                  showProject={showProject}
                  launching={launchingSessionKey === key}
                  launchBlocked={launchingSessionKey !== null}
                  onArchive={onArchive}
                  onResume={onResume}
                  onCopyId={onCopyId}
                  onToggleSelection={onToggleSelection}
                />
              );
            })}
          </div>
        </section>
      ))}
    </div>
  );
}
