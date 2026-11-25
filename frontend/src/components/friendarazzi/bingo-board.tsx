"use client";

import { Camera, CheckCircle2, Lock } from "lucide-react";

import type {
  BoardTileView,
  BoardView,
  CaptureFeedItem,
} from "@/api/friendarazzi-api";
import { cn } from "@/lib/cn";

import { AuthenticatedImage } from "./media-image";

type BingoBoardProps = {
  board: BoardView;
  gridSize: number;
  feed?: CaptureFeedItem[];
  onEmptyTileSelect?: (tile: BoardTileView) => void;
  onViewCapture?: (capture: CaptureFeedItem) => void;
};

function toCaptureFeedItem(
  tile: BoardTileView,
  board: BoardView,
  feed: CaptureFeedItem[],
): CaptureFeedItem | null {
  if (!tile.isCaptured) return null;
  if (tile.captureId) {
    const fromFeed = feed.find((item) => item.id === tile.captureId);
    if (fromFeed) {
      return {
        ...fromFeed,
        imageStorageKey:
          fromFeed.imageStorageKey ?? tile.imageStorageKey ?? null,
        promptText: fromFeed.promptText ?? tile.promptText,
        target: fromFeed.target ?? tile.targetUser,
      };
    }
  }
  const key = tile.imageStorageKey ?? null;
  if (!key) return null;
  return {
    id: tile.captureId ?? tile.id,
    tileId: tile.id,
    catcher: board.owner,
    target: tile.targetUser,
    promptText: tile.promptText,
    imageStorageKey: key,
    thumbnailStorageKey: key,
    isBlurred: false,
    capturedAt: tile.capturedAt ?? new Date().toISOString(),
  };
}

const CAPTURED_TILTS = [
  "-rotate-2",
  "rotate-1",
  "-rotate-1",
  "rotate-2",
] as const;

const CAPTURED_WELLS = [
  "bg-primary-subtle",
  "bg-cyan-subtle",
  "bg-pink-subtle",
] as const;

function tileHash(tileId: string): number {
  let hash = 0;
  for (let i = 0; i < tileId.length; i += 1) {
    hash = (hash * 31 + tileId.charCodeAt(i)) | 0;
  }
  return Math.abs(hash);
}

function tileTiltClass(tileId: string): string {
  return CAPTURED_TILTS[tileHash(tileId) % CAPTURED_TILTS.length] ?? "rotate-1";
}

function tileWellClass(tileId: string): string {
  return (
    CAPTURED_WELLS[tileHash(tileId) % CAPTURED_WELLS.length] ??
    "bg-primary-subtle"
  );
}

/**
 * Unified single-card Bingo board shared between active play and archive view.
 * Uncaptured tiles focus on target + prompt action; captured tiles showcase the
 * photo full-bleed inside a subtly tilted frame with a compact bottom caption.
 */
export function BingoBoard({
  board,
  gridSize,
  feed = [],
  onEmptyTileSelect,
  onViewCapture,
}: BingoBoardProps) {
  const gridColsClass =
    gridSize === 3
      ? "grid-cols-3"
      : gridSize === 4
        ? "grid-cols-4"
        : "grid-cols-5";

  const interactiveUpload = Boolean(onEmptyTileSelect);

  return (
    <div className="brutal-shadow-lg overflow-hidden rounded-md border-[2.5px] border-strong bg-surface">
      <div className={cn("grid gap-px bg-strong/20", gridColsClass)}>
        {board.tiles.map((tile) => {
          const captureItem = toCaptureFeedItem(tile, board, feed);
          const canOpenCapture = Boolean(captureItem && onViewCapture);
          const canUpload = !tile.isCaptured && interactiveUpload;
          const isClickable = canOpenCapture || canUpload;

          const tileAriaLabel = `${tile.targetUser.displayName}: ${tile.promptText}${
            tile.isCaptured
              ? canOpenCapture
                ? " (erfasst — Foto öffnen)"
                : " (erfasst)"
              : canUpload
                ? " (Foto aufnehmen)"
                : ""
          }`;

          const cellContent =
            tile.isCaptured && tile.imageStorageKey ? (
              <div
                className={cn(
                  "brutal-shadow-sm relative h-full w-full overflow-hidden rounded-[2px] border-[1.5px] border-strong bg-surface transition-transform duration-150 sm:border-2",
                  tileTiltClass(tile.id),
                  "group-hover:rotate-0",
                )}
              >
                <AuthenticatedImage
                  storageKey={tile.imageStorageKey}
                  alt={tile.promptText}
                  fit="contain-blurred"
                  className="h-full w-full"
                />
                <div className="absolute inset-x-0 bottom-0 z-20 flex items-center gap-1 border-t border-strong bg-surface px-1 py-0.5 sm:px-1.5">
                  <CheckCircle2
                    className="size-2.5 shrink-0 text-success sm:size-3"
                    aria-hidden="true"
                  />
                  <p className="min-w-0 truncate text-[9px] leading-tight text-ink sm:text-[10px]">
                    <strong className="font-mono font-bold uppercase">
                      {tile.targetUser.displayName}:
                    </strong>{" "}
                    <span className="font-semibold">{tile.promptText}</span>
                  </p>
                </div>
              </div>
            ) : tile.isCaptured ? (
              <div className="flex h-full w-full flex-col justify-between rounded-[2px] bg-cyan-subtle p-1.5 sm:p-2">
                <div className="flex items-center justify-between gap-1">
                  <span className="max-w-full truncate rounded-[2px] border border-strong bg-surface px-1 py-0.5 font-mono text-[9px] leading-none font-bold text-ink sm:text-[10px]">
                    {tile.targetUser.displayName}
                  </span>
                  <Lock
                    className="size-3.5 shrink-0 text-ink"
                    aria-hidden="true"
                  />
                </div>
                <p className="mt-1 line-clamp-3 text-[10px] leading-snug font-bold break-words text-ink sm:text-xs">
                  {tile.promptText}
                </p>
              </div>
            ) : (
              <div className="flex h-full w-full flex-col justify-between gap-1">
                <div className="flex items-center justify-between gap-1">
                  <span
                    className={cn(
                      "max-w-full truncate rounded-[2px] border border-strong px-1 py-0.5 font-mono text-[9px] leading-none font-bold sm:text-[10px]",
                      interactiveUpload
                        ? "bg-primary text-primary-foreground"
                        : "bg-surface-muted text-secondary",
                    )}
                  >
                    {tile.targetUser.displayName}
                  </span>
                  {interactiveUpload ? (
                    <Camera
                      className="size-3.5 shrink-0 text-secondary transition-transform group-hover:scale-110 group-hover:text-ink"
                      aria-hidden="true"
                    />
                  ) : null}
                </div>
                <p
                  className={cn(
                    "line-clamp-4 flex-1 text-[10px] leading-snug font-bold break-words sm:text-xs",
                    interactiveUpload ? "text-ink" : "text-secondary",
                  )}
                >
                  {tile.promptText}
                </p>
              </div>
            );

          const cellPadding =
            tile.isCaptured && tile.imageStorageKey
              ? cn("p-1 sm:p-1.5", tileWellClass(tile.id))
              : tile.isCaptured
                ? "p-0.5 bg-surface"
                : interactiveUpload
                  ? "p-1.5 sm:p-2.5 bg-surface"
                  : "p-1.5 sm:p-2.5 bg-canvas";

          if (isClickable) {
            return (
              <button
                key={tile.id}
                type="button"
                aria-label={tileAriaLabel}
                onClick={() => {
                  if (canOpenCapture && captureItem && onViewCapture) {
                    onViewCapture(captureItem);
                  } else if (canUpload && onEmptyTileSelect) {
                    onEmptyTileSelect(tile);
                  }
                }}
                className={cn(
                  "group flex aspect-square min-h-24 w-full cursor-pointer overflow-hidden text-left transition-colors",
                  cellPadding,
                  "hover:bg-primary-subtle/50 focus-visible:z-10 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-focus",
                )}
              >
                {cellContent}
              </button>
            );
          }

          return (
            <div
              key={tile.id}
              className={cn(
                "flex aspect-square min-h-24 w-full overflow-hidden text-left",
                cellPadding,
              )}
            >
              {cellContent}
            </div>
          );
        })}
      </div>
    </div>
  );
}
