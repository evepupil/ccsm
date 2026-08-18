import {
  Archive,
  ArchiveRestore,
  ArrowUpDown,
  ChevronDown,
  ListChecks,
  Shield,
  X,
} from "lucide-react";

import { providerLabel } from "../lib/presentation";
import type { ProjectSummary, SessionProvider, SessionSort } from "../types";

interface SessionToolbarProps {
  activeSelectionCount: number;
  highestPermissions: boolean;
  provider: SessionProvider;
  resultCount: number;
  searching: boolean;
  archivedSelectionCount: number;
  selectedCount: number;
  selectedProject: ProjectSummary | null;
  selectionMode: boolean;
  showArchived: boolean;
  sort: SessionSort;
  onArchiveSelected: () => void;
  onClearSelection: () => void;
  onHighestPermissionsChange: (enabled: boolean) => void;
  onSelectAll: () => void;
  onShowArchivedChange: (show: boolean) => void;
  onSortChange: (sort: SessionSort) => void;
  onToggleSelectionMode: () => void;
  onUnarchiveSelected: () => void;
}

export function SessionToolbar({
  activeSelectionCount,
  archivedSelectionCount,
  highestPermissions,
  provider,
  resultCount,
  searching,
  selectedCount,
  selectedProject,
  selectionMode,
  showArchived,
  sort,
  onArchiveSelected,
  onClearSelection,
  onHighestPermissionsChange,
  onSelectAll,
  onShowArchivedChange,
  onSortChange,
  onToggleSelectionMode,
  onUnarchiveSelected,
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
        {selectionMode && <span className="selection-count">已选 {selectedCount}</span>}
        {selectionMode && (
          <button
            className="selection-text-button"
            type="button"
            disabled={resultCount === 0}
            onClick={onSelectAll}
          >
            全选当前结果
          </button>
        )}
        {selectionMode && (
          <button
            className="icon-button selection-action"
            type="button"
            disabled={selectedCount === 0}
            title="清空选择"
            aria-label="清空选择"
            onClick={onClearSelection}
          >
            <X aria-hidden="true" />
          </button>
        )}
        {selectionMode && (
          <button
            className="icon-button selection-action"
            type="button"
            disabled={activeSelectionCount === 0}
            title="归档选中会话"
            aria-label="归档选中会话"
            onClick={onArchiveSelected}
          >
            <Archive aria-hidden="true" />
          </button>
        )}
        {selectionMode && (
          <button
            className="icon-button selection-action"
            type="button"
            disabled={archivedSelectionCount === 0}
            title="取消归档选中会话"
            aria-label="取消归档选中会话"
            onClick={onUnarchiveSelected}
          >
            <ArchiveRestore aria-hidden="true" />
          </button>
        )}
        <button
          className={`icon-button selection-toggle ${selectionMode ? "is-active" : ""}`}
          type="button"
          title={selectionMode ? "退出多选" : "多选"}
          aria-label={selectionMode ? "退出多选" : "多选"}
          aria-pressed={selectionMode}
          onClick={onToggleSelectionMode}
        >
          <ListChecks aria-hidden="true" />
        </button>

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
          <span>显示已归档</span>
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
