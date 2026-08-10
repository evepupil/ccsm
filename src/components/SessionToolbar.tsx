import { ArrowUpDown, ChevronDown, Shield } from "lucide-react";

import { providerLabel } from "../lib/presentation";
import type { ProjectSummary, SessionProvider, SessionSort } from "../types";

interface SessionToolbarProps {
  highestPermissions: boolean;
  provider: SessionProvider;
  resultCount: number;
  searching: boolean;
  selectedProject: ProjectSummary | null;
  showArchived: boolean;
  sort: SessionSort;
  onHighestPermissionsChange: (enabled: boolean) => void;
  onShowArchivedChange: (show: boolean) => void;
  onSortChange: (sort: SessionSort) => void;
}

export function SessionToolbar({
  highestPermissions,
  provider,
  resultCount,
  searching,
  selectedProject,
  showArchived,
  sort,
  onHighestPermissionsChange,
  onShowArchivedChange,
  onSortChange,
}: SessionToolbarProps) {
  const title = searching ? "全局搜索" : (selectedProject?.name ?? "选择一个项目");
  const path = searching
    ? `正在 ${providerLabel(provider)} 的全部项目中搜索`
    : (selectedProject?.path ?? "");

  return (
    <header className="workspace-header">
      <div className="workspace-title">
        <div className="workspace-title-line">
          <h1>{title}</h1>
          <span className="workspace-count">{resultCount}</span>
        </div>
        <p title={path}>{path}</p>
      </div>

      <div className="workspace-actions">
        <label className="sort-picker">
          <ArrowUpDown aria-hidden="true" />
          <select
            value={sort}
            aria-label="排序方式"
            onChange={(event) => onSortChange(event.target.value as SessionSort)}
          >
            <option value="recent">最近活动</option>
            <option value="title">标题</option>
          </select>
          <ChevronDown aria-hidden="true" />
        </label>

        <label className="compact-switch">
          <input
            type="checkbox"
            role="switch"
            checked={showArchived}
            onChange={(event) => onShowArchivedChange(event.target.checked)}
          />
          <span className="switch-track" aria-hidden="true">
            <span />
          </span>
          <span>归档</span>
        </label>

        <label className="compact-switch permission-toggle">
          <input
            type="checkbox"
            role="switch"
            checked={highestPermissions}
            onChange={(event) => onHighestPermissionsChange(event.target.checked)}
          />
          <span className="switch-track" aria-hidden="true">
            <span />
          </span>
          <Shield aria-hidden="true" />
          <span>最高权限</span>
        </label>
      </div>
    </header>
  );
}
