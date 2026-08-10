import { ArrowUpDown, Shield } from "lucide-react";
import { Chip, ListBox, Select, Switch } from "@heroui/react";

import { providerLabel } from "../lib/presentation";
import type { ProjectSummary, SessionProvider, SessionSort } from "../types";

interface SessionToolbarProps {
  highestPermissions: boolean;
  provider: SessionProvider;
  resultCount: number;
  searchQuery: string;
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
  searchQuery,
  searching,
  selectedProject,
  showArchived,
  sort,
  onHighestPermissionsChange,
  onShowArchivedChange,
  onSortChange,
}: SessionToolbarProps) {
  return (
    <section className="flex min-h-[72px] items-center justify-between gap-[18px] border-b border-border bg-surface px-4 py-2.5 max-[1000px]:min-h-[108px] max-[1000px]:flex-col max-[1000px]:items-start max-[1000px]:gap-2.5">
      <div className="min-w-0 flex-1">
        <div className="flex min-w-0 items-center gap-2">
          <h1 className="m-0 truncate text-[17px] font-bold leading-tight">
            {searching ? "搜索结果" : (selectedProject?.name ?? "选择一个项目")}
          </h1>
          <Chip size="sm" variant="soft" color="default">
            <Chip.Label>{resultCount} 条</Chip.Label>
          </Chip>
        </div>
        <p
          className="mt-1 max-w-[540px] truncate text-[12px] text-muted"
          title={searching ? searchQuery : selectedProject?.path}
        >
          {searching
            ? `在 ${providerLabel(provider)} 会话中搜索“${searchQuery}”`
            : selectedProject?.path}
        </p>
      </div>

      <div className="flex flex-[0_0_auto] flex-wrap items-center justify-end gap-1 rounded-md border border-border bg-surface-secondary p-1 max-[1000px]:w-full max-[1000px]:justify-start">
        <Select
          selectedKey={sort}
          onSelectionChange={(key) => {
            const nextSort = String(key);
            if (nextSort === "recent" || nextSort === "title") onSortChange(nextSort);
          }}
          aria-label="排序方式"
          className="w-[132px]"
        >
          <Select.Trigger className="h-8 min-w-[132px] border-transparent bg-surface text-[12px] shadow-none">
            <ArrowUpDown size={14} className="text-muted" />
            <Select.Value />
            <Select.Indicator />
          </Select.Trigger>
          <Select.Popover className="min-w-[132px]">
            <ListBox>
              <ListBox.Item id="recent" textValue="最近活动" className="min-h-9 text-[12px]">
                最近活动
              </ListBox.Item>
              <ListBox.Item id="title" textValue="标题" className="min-h-9 text-[12px]">
                标题
              </ListBox.Item>
            </ListBox>
          </Select.Popover>
        </Select>

        <Switch
          size="sm"
          isSelected={showArchived}
          onChange={onShowArchivedChange}
          className="px-2"
        >
          <Switch.Content className="gap-2 text-[12px] text-muted">
            <Switch.Control>
              <Switch.Thumb />
            </Switch.Control>
            <span>显示归档</span>
          </Switch.Content>
        </Switch>

        <Switch
          size="sm"
          isSelected={highestPermissions}
          onChange={onHighestPermissionsChange}
          className={highestPermissions ? "px-2 font-semibold text-danger" : "px-2 text-muted"}
        >
          <Switch.Content className="gap-2 text-[12px]">
            <Switch.Control className={highestPermissions ? "bg-danger" : ""}>
              <Switch.Thumb />
            </Switch.Control>
            <Shield size={14} />
            <span>最高权限</span>
          </Switch.Content>
        </Switch>
      </div>
    </section>
  );
}
