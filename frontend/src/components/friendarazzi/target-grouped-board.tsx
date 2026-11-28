"use client";

import { Camera, CheckCircle2, Lock, Pin, PinOff } from "lucide-react";

import type {
  BoardTileView,
  BoardView,
  CaptureFeedItem,
} from "@/api/friendarazzi-api";
import { PlayerBadgeLink } from "@/components/friendarazzi/player-avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import {
  groupTilesByTarget,
  type MatchTargetAggregate,
  tileToCaptureFeedItem,
} from "@/lib/board-grouping";
import { cn } from "@/lib/cn";

import { AuthenticatedImage } from "./media-image";

type TargetGroupedBoardProps = {
  board: BoardView;
  feed?: CaptureFeedItem[];
  pinnedTargetIds?: ReadonlySet<string>;
  onTogglePin?: (targetUserId: string) => void;
  hideCaptured?: boolean;
  searchQuery?: string;
  onEmptyTileSelect?: (tile: BoardTileView) => void;
  onViewCapture?: (capture: CaptureFeedItem) => void;
};

/**
 * Target-grouped alternative representation of a Bingo board (#26):
 * groups prompts under each target friend card so players hunt one person
 * at a time with legible text and quick pin/focus support.
 */
export function TargetGroupedBoard({
  board,
  feed = [],
  pinnedTargetIds,
  onTogglePin,
  hideCaptured = false,
  searchQuery = "",
  onEmptyTileSelect,
  onViewCapture,
}: TargetGroupedBoardProps) {
  const groups = groupTilesByTarget(board.tiles, {
    pinnedTargetIds,
    hideCaptured,
    query: searchQuery,
  });

  if (groups.length === 0) {
    return (
      <div className="brutal-shadow-sm rounded-md border-2 border-strong bg-surface p-6 text-center">
        <p className="text-sm font-bold text-ink">
          Keine passenden Aufträge gefunden.
        </p>
        <p className="mt-1 text-xs font-medium text-secondary">
          Passe Suchbegriff oder Filter an, um wieder alle Ziele zu sehen.
        </p>
      </div>
    );
  }

  const interactiveUpload = Boolean(onEmptyTileSelect);

  return (
    <div className="grid gap-4 md:grid-cols-2">
      {groups.map((group) => (
        <Card
          key={group.target.id}
          className={cn(
            "overflow-hidden",
            group.isPinned && "border-strong bg-primary-subtle/20",
          )}
        >
          <CardHeader
            className={cn(
              "px-4 py-3",
              group.isPinned ? "bg-primary-subtle" : "bg-cyan-subtle",
            )}
          >
            <div className="flex items-center justify-between gap-2">
              <div className="min-w-0 flex-1">
                <PlayerBadgeLink user={group.target} compact />
              </div>
              <div className="flex shrink-0 items-center gap-1.5">
                <span className="rounded-sm border border-strong bg-surface px-2 py-0.5 font-mono text-xs font-bold text-ink tabular-nums">
                  {group.capturedCount}/{group.totalCount}
                </span>
                {onTogglePin ? (
                  <Button
                    type="button"
                    variant={group.isPinned ? "primary" : "secondary"}
                    size="icon"
                    aria-pressed={group.isPinned}
                    aria-label={
                      group.isPinned
                        ? `${group.target.displayName} aus Fokus entfernen`
                        : `${group.target.displayName} anheften`
                    }
                    title={
                      group.isPinned
                        ? "Aus Fokus entfernen"
                        : "Im Fokus anheften"
                    }
                    onClick={() => onTogglePin(group.target.id)}
                  >
                    {group.isPinned ? (
                      <PinOff className="size-4" aria-hidden="true" />
                    ) : (
                      <Pin className="size-4" aria-hidden="true" />
                    )}
                  </Button>
                ) : null}
              </div>
            </div>
          </CardHeader>

          <CardContent className="space-y-2 p-3 sm:p-4">
            {group.tiles.map((tile) => {
              const captureItem = tileToCaptureFeedItem(
                tile,
                board.owner,
                feed,
              );
              const canOpenCapture = Boolean(captureItem && onViewCapture);
              const canUpload = !tile.isCaptured && interactiveUpload;
              const tileAriaLabel = `${tile.targetUser.displayName}: ${tile.promptText}${
                tile.isCaptured
                  ? canOpenCapture
                    ? " (erfasst — Foto öffnen)"
                    : " (erfasst)"
                  : canUpload
                    ? " (Foto aufnehmen)"
                    : ""
              }`;

              if (tile.isCaptured && tile.imageStorageKey) {
                return (
                  <button
                    key={tile.id}
                    type="button"
                    disabled={!canOpenCapture}
                    aria-label={tileAriaLabel}
                    onClick={() => {
                      if (canOpenCapture && captureItem && onViewCapture) {
                        onViewCapture(captureItem);
                      }
                    }}
                    className={cn(
                      "flex min-h-14 w-full items-center gap-3 rounded-sm border-2 border-strong bg-surface p-2 text-left transition-colors",
                      canOpenCapture
                        ? "brutal-press cursor-pointer hover:bg-primary-subtle/40 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
                        : "cursor-default",
                    )}
                  >
                    <div className="size-12 shrink-0 overflow-hidden rounded-sm border border-strong bg-surface-muted">
                      <AuthenticatedImage
                        storageKey={tile.imageStorageKey}
                        alt={tile.promptText}
                        className="h-full w-full object-cover"
                      />
                    </div>
                    <p className="min-w-0 flex-1 text-sm leading-snug font-bold text-ink">
                      {tile.promptText}
                    </p>
                    <CheckCircle2
                      className="size-4 shrink-0 text-success"
                      aria-hidden="true"
                    />
                  </button>
                );
              }

              if (tile.isCaptured) {
                return (
                  <div
                    key={tile.id}
                    className="flex min-h-12 items-center justify-between gap-3 rounded-sm border-2 border-strong bg-cyan-subtle px-3 py-2"
                  >
                    <p className="min-w-0 flex-1 text-sm leading-snug font-bold text-ink">
                      {tile.promptText}
                    </p>
                    <Lock
                      className="size-4 shrink-0 text-ink"
                      aria-hidden="true"
                    />
                  </div>
                );
              }

              if (canUpload) {
                return (
                  <button
                    key={tile.id}
                    type="button"
                    aria-label={tileAriaLabel}
                    onClick={() => onEmptyTileSelect?.(tile)}
                    className="brutal-press flex min-h-12 w-full cursor-pointer items-center justify-between gap-3 rounded-sm border-2 border-control bg-surface px-3 py-2.5 text-left hover:bg-primary-subtle focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
                  >
                    <span className="min-w-0 flex-1 text-sm leading-snug font-bold text-ink">
                      {tile.promptText}
                    </span>
                    <span className="inline-flex size-8 shrink-0 items-center justify-center rounded-sm border border-strong bg-primary text-primary-foreground">
                      <Camera className="size-4" aria-hidden="true" />
                    </span>
                  </button>
                );
              }

              return (
                <div
                  key={tile.id}
                  className="flex min-h-11 items-center justify-between gap-3 rounded-sm border-2 border-subtle bg-canvas px-3 py-2"
                >
                  <p className="min-w-0 flex-1 text-sm leading-snug font-medium text-secondary">
                    {tile.promptText}
                  </p>
                  <Badge variant="outline">Offen</Badge>
                </div>
              );
            })}
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

/**
 * Completed-match overview grouped by target person across every player's board (#26).
 */
export function MatchTargetOverview({
  aggregates,
  onViewCapture,
}: {
  aggregates: readonly MatchTargetAggregate[];
  onViewCapture?: (capture: CaptureFeedItem) => void;
}) {
  if (aggregates.length === 0) {
    return (
      <div className="brutal-shadow-sm rounded-md border-2 border-strong bg-surface p-6 text-center">
        <p className="text-sm font-bold text-ink">
          Keine passenden Einträge gefunden.
        </p>
      </div>
    );
  }

  return (
    <div className="grid gap-4 md:grid-cols-2">
      {aggregates.map((group) => (
        <Card key={group.target.id} className="overflow-hidden">
          <CardHeader className="bg-primary-subtle px-4 py-3">
            <div className="flex items-center justify-between gap-2">
              <div className="min-w-0 flex-1">
                <PlayerBadgeLink user={group.target} compact />
              </div>
              <span className="shrink-0 rounded-sm border border-strong bg-surface px-2 py-0.5 font-mono text-xs font-bold text-ink tabular-nums">
                {group.capturedCount}/{group.totalCount} erwischt
              </span>
            </div>
          </CardHeader>
          <CardContent className="space-y-2 p-3 sm:p-4">
            {group.entries.map(({ boardOwner, tile, capture }) => {
              const canOpen = Boolean(capture && onViewCapture);
              return canOpen && capture ? (
                <button
                  key={`${boardOwner.id}-${tile.id}`}
                  type="button"
                  onClick={() => onViewCapture?.(capture)}
                  className="brutal-press flex min-h-14 w-full cursor-pointer items-center gap-3 rounded-sm border-2 border-strong bg-surface p-2 text-left hover:bg-primary-subtle/40 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
                >
                  {tile.imageStorageKey ? (
                    <div className="size-12 shrink-0 overflow-hidden rounded-sm border border-strong bg-surface-muted">
                      <AuthenticatedImage
                        storageKey={tile.imageStorageKey}
                        alt={tile.promptText}
                        className="h-full w-full object-cover"
                      />
                    </div>
                  ) : null}
                  <div className="min-w-0 flex-1">
                    <p className="text-sm leading-snug font-bold text-ink">
                      {tile.promptText}
                    </p>
                    <p className="font-mono text-[11px] font-bold text-secondary">
                      Brett: {boardOwner.displayName}
                    </p>
                  </div>
                  <CheckCircle2
                    className="size-4 shrink-0 text-success"
                    aria-hidden="true"
                  />
                </button>
              ) : (
                <div
                  key={`${boardOwner.id}-${tile.id}`}
                  className="flex min-h-11 items-center justify-between gap-3 rounded-sm border-2 border-subtle bg-canvas px-3 py-2"
                >
                  <div className="min-w-0 flex-1">
                    <p className="text-sm leading-snug font-medium text-secondary">
                      {tile.promptText}
                    </p>
                    <p className="font-mono text-[11px] font-bold text-secondary">
                      Brett: {boardOwner.displayName}
                    </p>
                  </div>
                  <Badge variant="outline">Offen</Badge>
                </div>
              );
            })}
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
