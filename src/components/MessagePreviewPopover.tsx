import { MessagesSquare, X } from "lucide-react";
import { createPortal } from "react-dom";
import { useCallback, useEffect, useRef, useState } from "react";

import { getUserMessagePreviews } from "../api";
import type { SessionSummary } from "../types";
import { ProviderLogo } from "./ProviderLogo";

const PREVIEW_LIMIT = 5;
const OPEN_DELAY_MS = 240;
const CLOSE_DELAY_MS = 160;

type PreviewStatus = "idle" | "loading" | "ready" | "error";

interface MessagePreviewPopoverProps {
  session: SessionSummary;
}

interface PopoverPosition {
  left: number;
  top: number;
}

const previewCache = new Map<string, string[]>();
const previewRequests = new Map<string, Promise<string[]>>();

function previewKey(session: SessionSummary): string {
  return [
    session.provider,
    session.sourcePath,
    session.fileSize,
    session.lastActivity,
    PREVIEW_LIMIT,
  ].join("|");
}

function loadPreviews(session: SessionSummary): Promise<string[]> {
  const key = previewKey(session);
  const cached = previewCache.get(key);
  if (cached) return Promise.resolve(cached);

  const pending = previewRequests.get(key);
  if (pending) return pending;

  const request = getUserMessagePreviews(session.provider, session.sourcePath, PREVIEW_LIMIT)
    .then((messages) => {
      previewCache.set(key, messages);
      return messages;
    })
    .finally(() => {
      previewRequests.delete(key);
    });
  previewRequests.set(key, request);
  return request;
}

function clampPosition(rect: DOMRect): PopoverPosition {
  const width = Math.min(390, window.innerWidth - 24);
  const height = Math.min(250, window.innerHeight - 24);
  const left = Math.min(Math.max(12, rect.left - 8), window.innerWidth - width - 12);
  const belowTop = rect.bottom + 8;
  const top =
    belowTop + height <= window.innerHeight - 12 ? belowTop : Math.max(12, rect.top - height - 8);

  return { left, top };
}

export function MessagePreviewPopover({ session }: MessagePreviewPopoverProps) {
  const triggerRef = useRef<HTMLButtonElement>(null);
  const closeTimerRef = useRef<number | null>(null);
  const loadTimerRef = useRef<number | null>(null);
  const loadedKeyRef = useRef<string | null>(null);
  const requestKeyRef = useRef<string | null>(null);
  const popoverId = `message-preview-${session.provider}-${session.id}`;
  const sessionPreviewKey = previewKey(session);
  const [open, setOpen] = useState(false);
  const [status, setStatus] = useState<PreviewStatus>("idle");
  const [messages, setMessages] = useState<string[]>([]);
  const [position, setPosition] = useState<PopoverPosition | null>(null);

  const clearCloseTimer = useCallback(() => {
    if (closeTimerRef.current !== null) {
      window.clearTimeout(closeTimerRef.current);
      closeTimerRef.current = null;
    }
  }, []);

  const updatePosition = useCallback(() => {
    const trigger = triggerRef.current;
    if (!trigger) return;
    setPosition(clampPosition(trigger.getBoundingClientRect()));
  }, []);

  const close = useCallback(() => {
    clearCloseTimer();
    if (loadTimerRef.current !== null) {
      window.clearTimeout(loadTimerRef.current);
      loadTimerRef.current = null;
      requestKeyRef.current = null;
      setStatus("idle");
    }
    setOpen(false);
  }, [clearCloseTimer]);

  const scheduleClose = useCallback(() => {
    clearCloseTimer();
    closeTimerRef.current = window.setTimeout(close, CLOSE_DELAY_MS);
  }, [clearCloseTimer, close]);

  const openPreview = useCallback(() => {
    clearCloseTimer();
    updatePosition();
    setOpen(true);
    if (loadedKeyRef.current === sessionPreviewKey || requestKeyRef.current === sessionPreviewKey) {
      return;
    }

    if (loadTimerRef.current !== null) window.clearTimeout(loadTimerRef.current);
    requestKeyRef.current = sessionPreviewKey;
    setStatus("loading");
    loadTimerRef.current = window.setTimeout(() => {
      loadTimerRef.current = null;
      void loadPreviews(session)
        .then((nextMessages) => {
          if (requestKeyRef.current !== sessionPreviewKey) return;
          loadedKeyRef.current = sessionPreviewKey;
          setMessages(nextMessages);
          setStatus("ready");
        })
        .catch(() => {
          if (requestKeyRef.current !== sessionPreviewKey) return;
          requestKeyRef.current = null;
          setStatus("error");
        });
    }, OPEN_DELAY_MS);
  }, [clearCloseTimer, session, sessionPreviewKey, updatePosition]);

  useEffect(() => {
    if (loadTimerRef.current !== null) {
      window.clearTimeout(loadTimerRef.current);
      loadTimerRef.current = null;
    }
    loadedKeyRef.current = null;
    requestKeyRef.current = null;
    setMessages([]);
    setStatus("idle");
  }, [sessionPreviewKey]);

  useEffect(() => {
    if (!open) return;

    const handleViewportChange = () => updatePosition();
    window.addEventListener("resize", handleViewportChange);
    window.addEventListener("scroll", handleViewportChange, true);
    return () => {
      window.removeEventListener("resize", handleViewportChange);
      window.removeEventListener("scroll", handleViewportChange, true);
    };
  }, [open, updatePosition]);

  useEffect(
    () => () => {
      clearCloseTimer();
      if (loadTimerRef.current !== null) window.clearTimeout(loadTimerRef.current);
    },
    [clearCloseTimer],
  );

  const popover =
    open && position
      ? createPortal(
          <div
            id={popoverId}
            className="message-preview-popover"
            role="dialog"
            aria-labelledby={`${popoverId}-title`}
            style={{ left: position.left, top: position.top }}
            onMouseEnter={clearCloseTimer}
            onMouseLeave={scheduleClose}
          >
            <div className="message-preview-heading">
              <MessagesSquare aria-hidden="true" />
              <span id={`${popoverId}-title`}>最近用户消息</span>
              <button
                className="message-preview-close"
                type="button"
                title="关闭消息预览"
                aria-label="关闭消息预览"
                onClick={close}
              >
                <X aria-hidden="true" />
              </button>
            </div>
            {(status === "idle" || status === "loading") && (
              <div className="message-preview-status" role="status">
                正在读取...
              </div>
            )}
            {status === "error" && (
              <div className="message-preview-status message-preview-error" role="status">
                暂时无法读取消息
              </div>
            )}
            {status === "ready" && messages.length === 0 && (
              <div className="message-preview-status" role="status">
                暂无用户消息
              </div>
            )}
            {status === "ready" && messages.length > 0 && (
              <ol className="message-preview-list">
                {messages.map((message, index) => (
                  <li key={`${index}-${message}`}>
                    <span>{message}</span>
                  </li>
                ))}
              </ol>
            )}
          </div>,
          document.body,
        )
      : null;

  return (
    <>
      <button
        ref={triggerRef}
        className={`session-provider-icon ${session.provider} message-preview-trigger ${open ? "is-open" : ""}`}
        type="button"
        aria-label="预览最近用户消息"
        aria-describedby={open ? popoverId : undefined}
        aria-expanded={open}
        onMouseEnter={openPreview}
        onMouseLeave={scheduleClose}
        onFocus={openPreview}
        onBlur={scheduleClose}
        onClick={() => (open ? close() : openPreview())}
        onKeyDown={(event) => {
          if (event.key === "Escape") close();
        }}
      >
        <ProviderLogo provider={session.provider} />
      </button>
      {popover}
    </>
  );
}
