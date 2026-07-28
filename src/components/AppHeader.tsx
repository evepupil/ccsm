import { RefreshCw, SquareTerminal } from "lucide-react";
import { Button, SearchField, Tabs, Tooltip } from "@heroui/react";

import type { SessionProvider } from "../types";

interface AppHeaderProps {
  loading: boolean;
  provider: SessionProvider;
  searchQuery: string;
  sessionCount: number;
  onProviderChange: (provider: SessionProvider) => void;
  onRefresh: () => void;
  onSearchChange: (query: string) => void;
}

export function AppHeader({
  loading,
  provider,
  searchQuery,
  sessionCount,
  onProviderChange,
  onRefresh,
  onSearchChange,
}: AppHeaderProps) {
  return (
    <header className="grid grid-cols-[236px_minmax(260px,620px)_minmax(280px,1fr)] items-center gap-[18px] border-b border-border bg-surface px-[14px] pl-4 max-[1000px]:grid-cols-[204px_minmax(240px,1fr)_270px] max-[1000px]:gap-3">
      <div className="flex min-w-0 items-center gap-2.5" aria-label="CCSM">
        <span className="grid h-[34px] w-[34px] flex-[0_0_34px] place-items-center rounded-md bg-foreground text-background">
          <SquareTerminal size={19} aria-hidden="true" />
        </span>
        <span className="flex min-w-0 flex-col leading-tight">
          <strong className="truncate text-[15px]">CCSM</strong>
          <small className="mt-0.5 truncate text-[11px] text-muted">
            {sessionCount} 条本机会话
          </small>
        </span>
      </div>

      <SearchField fullWidth value={searchQuery} onChange={onSearchChange} aria-label="搜索会话">
        <SearchField.Group className="h-9">
          <SearchField.SearchIcon />
          <SearchField.Input placeholder="搜索标题、项目、分支或 Session ID" />
          <SearchField.ClearButton aria-label="清空搜索" />
        </SearchField.Group>
      </SearchField>

      <div className="flex min-w-0 items-center justify-end gap-2">
        <Tabs
          selectedKey={provider}
          onSelectionChange={(key) => {
            const nextProvider = String(key);
            if (nextProvider === "claude" || nextProvider === "codex") {
              onProviderChange(nextProvider);
            }
          }}
          aria-label="会话来源切换"
        >
          <Tabs.List className="h-[34px] min-w-[216px] p-[3px]">
            <Tabs.Tab id="claude" className="min-w-[104px] whitespace-nowrap text-[12px]">
              <span className="mr-1.5 inline-block h-[7px] w-[7px] rounded-full bg-[#ad542f]" />
              Claude Code
            </Tabs.Tab>
            <Tabs.Tab id="codex" className="min-w-[104px] whitespace-nowrap text-[12px]">
              <span className="mr-1.5 inline-block h-[7px] w-[7px] rounded-full bg-[#256aa7]" />
              Codex
            </Tabs.Tab>
          </Tabs.List>
        </Tabs>

        <Tooltip delay={500}>
          <Button
            isIconOnly
            variant="outline"
            size="sm"
            isDisabled={loading}
            onPress={onRefresh}
            aria-label="重新扫描本机会话"
          >
            <RefreshCw size={16} className={loading ? "animate-spin" : ""} />
          </Button>
          <Tooltip.Content>重新扫描</Tooltip.Content>
        </Tooltip>
      </div>
    </header>
  );
}
