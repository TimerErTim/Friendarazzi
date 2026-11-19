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

/**
 * Unified single-card Bingo board with classical 2px grid lines and
 * square Polaroid-style cells shared 1:1 between active play and archive view.
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
      <div className={cn("grid gap-[2px] bg-strong", gridColsClass)}>
        {board.tiles.map((tile) => {
          const captureItem = toCaptureFeedItem(tile, board, feed);
          const canOpenCapture = Boolean(captureItem && onViewCapture);
          const canUpload = !tile.isCaptured && interactiveUpload;
          const isClickable = canOpenCapture || canUpload;

          const cellContent = (
            <>
              {/* Polaroid Photo Window */}
              <div
                className={cn(
                  "relative min-h-0 w-full flex-1 overflow-hidden rounded-[2px] border border-strong",
                  tile.isCaptured ? "bg-surface-muted" : "bg-canvas",
                )}
              >
                {tile.isCaptured && tile.imageStorageKey ? (
                  <AuthenticatedImage
                    storageKey={tile.imageStorageKey}
                    alt={tile.promptText}
                    fit="contain-blurred"
                    className="h-full w-full"
                  />
                ) : tile.isCaptured ? (
                  <div className="flex h-full w-full items-center justify-center bg-cyan-subtle text-ink">
                    <Lock className="size-4" aria-hidden="true" />
                  </div>
                ) : (
                  <div
                    className={cn(
                      "flex h-full w-full items-center justify-center border border-dashed border-strong/40 transition-colors",
                      interactiveUpload
                        ? "bg-surface-muted group-hover:bg-primary-subtle"
                        : "bg-surface-muted",
                    )}
                  >
                    {interactiveUpload ? (
                      <Camera
                        className="size-4 text-secondary transition-transform group-hover:scale-110 group-hover:text-ink sm:size-5"
                        aria-hidden="true"
                      />
                    ) : (
                      <span
                        aria-hidden="true"
                        className="size-3 rounded-full border border-dashed border-strong/40"
                      />
                    )}
                  </div>
                )}

                {tile.isCaptured ? (
                  <span className="absolute top-1 right-1 z-20 inline-flex size-5 items-center justify-center rounded-sm border border-strong bg-surface text-success">
                    <CheckCircle2 className="size-3.5" aria-hidden="true" />
                  </span>
                ) : null}

                {/* Compact Target Highlight Tag */}
                <span className="absolute bottom-1 left-1 z-20 max-w-[calc(100%-0.5rem)] truncate rounded-[2px] border border-strong bg-primary/90 px-1 py-0.5 font-mono text-[9px] leading-none font-bold text-primary-foreground sm:text-[10px]">
                  {tile.targetUser.displayName}
                </span>
              </div>

              {/* Polaroid Chin / Prompt Caption */}
              <p className="mt-1 line-clamp-2 shrink-0 text-[10px] leading-tight font-bold text-ink sm:mt-1.5 sm:text-xs">
                {tile.promptText}
              </p>
            </>
          );

          if (isClickable) {
            return (
              <button
                key={tile.id}
                type="button"
                onClick={() => {
                  if (canOpenCapture && captureItem && onViewCapture) {
                    onViewCapture(captureItem);
                  } else if (canUpload && onEmptyTileSelect) {
                    onEmptyTileSelect(tile);
                  }
                }}
                className={cn(
                  "group flex aspect-square min-h-24 w-full cursor-pointer flex-col justify-between bg-surface p-1.5 text-left transition-colors sm:p-2",
                  "hover:bg-primary-subtle/40 focus-visible:z-10 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-focus",
                )}
              >
                {cellContent}
              </button>
            );
          }

          return (
            <div
              key={tile.id}
              className="flex aspect-square min-h-24 w-full flex-col justify-between bg-surface p-1.5 text-left sm:p-2"
            >
              {cellContent}
            </div>
          );
        })}
      </div>
    </div>
  );
}
