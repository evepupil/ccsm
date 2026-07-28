import { Database } from "lucide-react";

import { formatAbsoluteTime } from "../lib/format";
import { providerLabel } from "../lib/presentation";
import type { SessionCatalog } from "../types";

interface StatusBarProps {
  catalog: SessionCatalog | null;
}

export function StatusBar({ catalog }: StatusBarProps) {
  const sourceSummary =
    catalog?.sources
      .map((source) =>
        source.available
          ? `${providerLabel(source.provider)} ${source.sessionCount} 条`
          : `${providerLabel(source.provider)} 未发现`,
      )
      .join(" · ") ?? "等待扫描";
  const sourceLocations = catalog?.sources
    .map((source) => (source.error ? `${source.location}\n${source.error}` : source.location))
    .join("\n\n");

  return (
    <footer className="flex items-center justify-between gap-4 border-t border-border bg-surface-secondary px-2.5 text-[11px] text-muted">
      <span className="inline-flex min-w-0 items-center gap-1.5 truncate" title={sourceLocations}>
        <Database size={13} /> {sourceSummary}
      </span>
      <span className="truncate">
        {catalog?.warnings[0] ??
          (catalog ? `上次扫描：${formatAbsoluteTime(catalog.scannedAt)}` : "仅读取本地元数据")}
      </span>
    </footer>
  );
}
