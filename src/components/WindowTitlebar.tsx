import { Minus, Square, SquareTerminal, X } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { providerLabel } from "../lib/presentation";
import type { SessionProvider } from "../types";

interface WindowTitlebarProps {
  provider: SessionProvider;
}

type WindowAction = "close" | "minimize" | "toggleMaximize";

function isTauriWindow(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function runWindowAction(action: WindowAction) {
  if (!isTauriWindow()) return;
  const appWindow = getCurrentWindow();
  await appWindow[action]();
}

export function WindowTitlebar({ provider }: WindowTitlebarProps) {
  return (
    <header
      className="window-titlebar"
      data-tauri-drag-region
      onDoubleClick={(event) => {
        if ((event.target as HTMLElement).closest("button")) return;
        void runWindowAction("toggleMaximize");
      }}
    >
      <div className="window-identity" data-tauri-drag-region>
        <span className="window-logo" aria-hidden="true">
          <SquareTerminal />
        </span>
        <strong>CCSM</strong>
        <span className="title-divider" aria-hidden="true" />
        <span>{providerLabel(provider)}</span>
      </div>

      <div className="window-actions" aria-label="窗口控制">
        <button
          className="window-button"
          type="button"
          title="最小化"
          aria-label="最小化"
          onClick={() => void runWindowAction("minimize")}
        >
          <Minus aria-hidden="true" />
        </button>
        <button
          className="window-button"
          type="button"
          title="最大化"
          aria-label="最大化"
          onClick={() => void runWindowAction("toggleMaximize")}
        >
          <Square aria-hidden="true" />
        </button>
        <button
          className="window-button window-close"
          type="button"
          title="关闭"
          aria-label="关闭"
          onClick={() => void runWindowAction("close")}
        >
          <X aria-hidden="true" />
        </button>
      </div>
    </header>
  );
}
