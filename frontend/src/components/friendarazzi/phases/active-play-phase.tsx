"use client";

import {
  ChevronDown,
  ChevronUp,
  LayoutGrid,
  List,
  Pin,
  Search,
  Users,
  X,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import {
  type BoardTileView,
  type CaptureFeedItem,
  type GameDetail,
  VisibilityMode,
} from "@/api/friendarazzi-api";
import { BingoBoard } from "@/components/friendarazzi/bingo-board";
import { CaptureTileDialog } from "@/components/friendarazzi/capture-tile-dialog";
import { AuthenticatedImage } from "@/components/friendarazzi/media-image";
import { PhotoLightbox } from "@/components/friendarazzi/photo-lightbox";
import { PlayerBadgeLink } from "@/components/friendarazzi/player-avatar";
import { TargetGroupedBoard } from "@/components/friendarazzi/target-grouped-board";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { CalloutNote, CodeTag } from "@/components/ui/brutal-elements";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { SearchInput } from "@/components/ui/search-input";
import { Skeleton } from "@/components/ui/skeleton";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import {
  type BoardViewMode,
  dismissCaptureCoachMark,
  filterTileMatches,
  hasDismissedCaptureCoachMark,
  loadBoardViewMode,
  loadPinnedTiles,
  saveBoardViewMode,
  savePinnedTiles,
  togglePinnedTile,
} from "@/lib/board-grouping";
import { newestCapturesFirst } from "@/lib/capture-feed";
import { cn } from "@/lib/cn";

const INITIAL_FEED_ITEMS = 3;

export function ActivePlayPhase({
  game,
  initialCaptureId = null,
  onConsumeInitialCapture,
  initialTileId = null,
  onConsumeInitialTile,
}: {
  game: GameDetail;
  initialCaptureId?: string | null;
  onConsumeInitialCapture?: () => void;
  initialTileId?: string | null;
  onConsumeInitialTile?: () => void;
}) {
  const [selectedTile, setSelectedTile] = useState<BoardTileView | null>(null);
  const [lightboxCapture, setLightboxCapture] =
    useState<CaptureFeedItem | null>(null);
  const [mobileFeedOpen, setMobileFeedOpen] = useState(false);
  const [showAllFeed, setShowAllFeed] = useState(false);
  const [viewMode, setViewMode] = useState<BoardViewMode>(() =>
    loadBoardViewMode(),
  );
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [pinnedTileIds, setPinnedTileIds] = useState<string[]>(() =>
    loadPinnedTiles(game.id),
  );
  const [coachDismissed, setCoachDismissed] = useState(() =>
    hasDismissedCaptureCoachMark(),
  );
  const openedCaptureRef = useRef<string | null>(null);
  const openedTileRef = useRef<string | null>(null);

  useEffect(() => {
    setPinnedTileIds(loadPinnedTiles(game.id));
  }, [game.id]);

  useEffect(() => {
    if (!initialCaptureId || openedCaptureRef.current === initialCaptureId) {
      return;
    }
    const found = game.feed.find((item) => item.id === initialCaptureId);
    if (found) {
      openedCaptureRef.current = initialCaptureId;
      setLightboxCapture(found);
    }
  }, [initialCaptureId, game.feed]);

  const board = game.myBoard;

  useEffect(() => {
    if (!initialTileId || openedTileRef.current === initialTileId || !board) {
      return;
    }
    const found = board.tiles.find(
      (tile) => tile.id === initialTileId && !tile.isCaptured,
    );
    if (found) {
      openedTileRef.current = initialTileId;
      setSelectedTile(found);
    }
  }, [initialTileId, board]);

  const hasActiveFilter = searchQuery.trim().length > 0;

  const matchingTileIds = useMemo(() => {
    if (!board || !hasActiveFilter) return undefined;
    return new Set(
      board.tiles
        .filter((tile) => filterTileMatches(tile, { query: searchQuery }))
        .map((tile) => tile.id),
    );
  }, [board, hasActiveFilter, searchQuery]);

  const pinnedTiles = useMemo(() => {
    if (!board) return [];
    const tileById = new Map(board.tiles.map((tile) => [tile.id, tile]));
    return pinnedTileIds
      .map((id) => tileById.get(id))
      .filter((tile): tile is BoardTileView =>
        Boolean(tile && !tile.isCaptured),
      );
  }, [board, pinnedTileIds]);

  const pinnedTileSet = useMemo(
    () => new Set(pinnedTiles.map((tile) => tile.id)),
    [pinnedTiles],
  );

  function handleViewModeChange(next: string) {
    if (next !== "grid" && next !== "list") return;
    setViewMode(next);
    saveBoardViewMode(next);
  }

  function handleTogglePinTile(tileId: string) {
    setPinnedTileIds((prev) => {
      const next = togglePinnedTile(prev, tileId);
      savePinnedTiles(game.id, next);
      return next;
    });
  }

  function handleTileCaptured(tileId: string) {
    if (pinnedTileSet.has(tileId)) {
      setPinnedTileIds((prev) => {
        const next = prev.filter((id) => id !== tileId);
        savePinnedTiles(game.id, next);
        return next;
      });
    }
    if (initialTileId) {
      onConsumeInitialTile?.();
    }
  }

  if (!board) {
    return (
      <div aria-busy="true" className="space-y-3">
        <Skeleton className="h-10 w-full" />
        <Skeleton className="h-80 w-full" />
      </div>
    );
  }

  const capturedCount = board.tiles.filter((t) => t.isCaptured).length;
  const newestFirstFeed = newestCapturesFirst(game.feed);
  const visibleFeed = showAllFeed
    ? newestFirstFeed
    : newestFirstFeed.slice(0, INITIAL_FEED_ITEMS);
  const hiddenFeedCount = Math.max(0, game.feed.length - visibleFeed.length);

  return (
    <div className="grid items-start gap-4 lg:grid-cols-[1.5fr_0.75fr] lg:gap-5">
      {/* Left: Single compact control bar + always-visible Focus strip directly above the hero board */}
      <div className="space-y-2.5">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <ToggleGroup
            type="single"
            value={viewMode}
            onValueChange={handleViewModeChange}
            aria-label="Brett-Ansicht wählen"
          >
            <ToggleGroupItem value="grid" aria-label="Raster-Ansicht">
              <LayoutGrid className="size-4" aria-hidden="true" />
              <span>Raster</span>
            </ToggleGroupItem>
            <ToggleGroupItem value="list" aria-label="Liste nach Zielperson">
              <List className="size-4" aria-hidden="true" />
              <span>Nach Ziel</span>
            </ToggleGroupItem>
          </ToggleGroup>

          <div className="flex items-center gap-1.5">
            <Popover>
              <PopoverTrigger asChild>
                <Button
                  type="button"
                  variant="secondary"
                  size="sm"
                  aria-label={`Crew (${game.players.length})`}
                  className="gap-1.5"
                >
                  <Users className="size-4" aria-hidden="true" />
                  <span className="hidden sm:inline">Crew</span>
                  <span className="font-mono tabular-nums">
                    ({game.players.length})
                  </span>
                </Button>
              </PopoverTrigger>
              <PopoverContent align="end" className="w-72 p-3">
                <p className="mb-2 font-mono text-[11px] font-bold tracking-wider text-secondary uppercase">
                  Mitspieler ({game.players.length})
                </p>
                <ul className="divide-y divide-subtle">
                  {game.players.map((p) => (
                    <li key={p.id} className="py-1.5 first:pt-0 last:pb-0">
                      <PlayerBadgeLink user={p} compact />
                    </li>
                  ))}
                </ul>
              </PopoverContent>
            </Popover>

            <Button
              type="button"
              variant={searchOpen || hasActiveFilter ? "primary" : "secondary"}
              size="icon"
              aria-expanded={searchOpen || hasActiveFilter}
              aria-label="Aufträge oder Ziele durchsuchen"
              title="Aufträge durchsuchen"
              onClick={() => {
                if (searchOpen && !hasActiveFilter) {
                  setSearchOpen(false);
                } else if (hasActiveFilter) {
                  setSearchQuery("");
                  setSearchOpen(false);
                } else {
                  setSearchOpen(true);
                }
              }}
            >
              <Search className="size-4" aria-hidden="true" />
            </Button>
          </div>
        </div>

        {searchOpen || hasActiveFilter ? (
          <SearchInput
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Ziel oder Auftrag suchen …"
            aria-label="Suchfeld für Aufträge oder Ziele"
            autoFocus
          />
        ) : null}

        {pinnedTiles.length > 0 ? (
          <div className="flex items-center gap-1.5 overflow-x-auto rounded-md border-2 border-info bg-surface px-2.5 py-1.5">
            <Pin
              className="size-3.5 shrink-0 rotate-45 fill-info text-info"
              aria-hidden="true"
            />
            <span className="shrink-0 font-mono text-[10px] font-bold tracking-wider text-info uppercase">
              Fokus
            </span>
            <div className="flex items-center gap-1.5">
              {pinnedTiles.map((tile) => (
                <div
                  key={tile.id}
                  className="inline-flex shrink-0 items-center overflow-hidden rounded-sm border-2 border-info bg-info/10"
                >
                  <button
                    type="button"
                    onClick={() => setSelectedTile(tile)}
                    className="relative inline-flex h-8 max-w-52 cursor-pointer items-center gap-1 px-2 text-left text-xs font-bold text-ink before:absolute before:inset-x-0 before:-inset-y-[6px] before:content-[''] hover:bg-info/20 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
                  >
                    <span className="shrink-0 font-mono text-[10px] font-bold uppercase">
                      {tile.targetUser.displayName}:
                    </span>
                    <span className="truncate">{tile.promptText}</span>
                  </button>
                  <button
                    type="button"
                    aria-label={`Auftrag „${tile.promptText}“ aus Fokus entfernen`}
                    onClick={() => handleTogglePinTile(tile.id)}
                    className="relative inline-flex h-8 w-8 shrink-0 cursor-pointer items-center justify-center border-l border-info text-ink before:absolute before:-inset-[6px] before:content-[''] hover:bg-info/20 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
                  >
                    <X className="size-3.5" aria-hidden="true" />
                  </button>
                </div>
              ))}
            </div>
          </div>
        ) : null}

        {viewMode === "list" ? (
          <TargetGroupedBoard
            board={board}
            feed={game.feed}
            pinnedTileIds={pinnedTileSet}
            onTogglePinTile={handleTogglePinTile}
            hideCaptured
            searchQuery={searchQuery}
            onEmptyTileSelect={(tile) => setSelectedTile(tile)}
            onViewCapture={(cap) => setLightboxCapture(cap)}
          />
        ) : (
          <BingoBoard
            board={board}
            gridSize={game.gridSize}
            feed={game.feed}
            matchingTileIds={matchingTileIds}
            pinnedTileIds={pinnedTileSet}
            onEmptyTileSelect={(tile) => setSelectedTile(tile)}
            onViewCapture={(cap) => setLightboxCapture(cap)}
          />
        )}

        {capturedCount === 0 && !coachDismissed ? (
          <Alert variant="info">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <p>
                <strong>Tipp für die Jagd:</strong> Wechsle zwischen Raster und
                „Nach Ziel“ oder hefte einzelne Aufträge an – deine angehefteten
                Aufgaben erscheinen auch direkt auf Home.
              </p>
              <Button
                type="button"
                variant="secondary"
                size="sm"
                onClick={() => {
                  dismissCaptureCoachMark();
                  setCoachDismissed(true);
                }}
              >
                Verstanden
              </Button>
            </div>
          </Alert>
        ) : null}
      </div>

      {/* Right: Activity Stream (collapsed on mobile so the board stays first) */}
      <Card>
        <CardHeader className="bg-pink-subtle py-3.5">
          <div className="flex items-center justify-between gap-2">
            <div className="flex items-center gap-2">
              <CardTitle>Live-Aktivität</CardTitle>
              {game.visibilityMode !== VisibilityMode.Undercover ? (
                <CodeTag tone="cyan">{game.feed.length}</CodeTag>
              ) : null}
            </div>
            <button
              type="button"
              aria-expanded={mobileFeedOpen}
              onClick={() => setMobileFeedOpen((prev) => !prev)}
              className="inline-flex min-h-11 items-center gap-1 rounded-sm px-2 text-xs font-bold text-ink hover:bg-surface/60 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus lg:hidden"
            >
              {mobileFeedOpen ? (
                <>
                  <span>Ausblenden</span>
                  <ChevronUp className="size-4" aria-hidden="true" />
                </>
              ) : (
                <>
                  <span>Anzeigen</span>
                  <ChevronDown className="size-4" aria-hidden="true" />
                </>
              )}
            </button>
          </div>
        </CardHeader>
        <CardContent
          className={cn(
            "space-y-3 pt-4",
            mobileFeedOpen ? "block" : "hidden lg:block",
          )}
        >
          {game.visibilityMode === VisibilityMode.Undercover ? (
            <CalloutNote label="Undercover aktiv" tone="pink">
              Alle Beweisfotos bleiben bis zum Bingo geheim.
            </CalloutNote>
          ) : game.feed.length === 0 ? (
            <p className="text-sm font-medium text-secondary">
              Noch keine Beweisfotos hochgeladen.
            </p>
          ) : (
            <div className="space-y-2.5">
              {visibleFeed.map((item) => (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => setLightboxCapture(item)}
                  className="brutal-press flex w-full cursor-pointer items-center gap-3 rounded-md border-2 border-strong bg-surface p-2 text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
                >
                  <div className="size-14 shrink-0 overflow-hidden rounded-sm border-2 border-strong bg-surface-muted">
                    <AuthenticatedImage
                      storageKey={
                        item.isBlurred
                          ? item.thumbnailStorageKey
                          : (item.imageStorageKey ?? item.thumbnailStorageKey)
                      }
                      alt={item.promptText ?? "Teaser"}
                      blurred={item.isBlurred}
                      className="h-full w-full"
                    />
                  </div>
                  <div className="min-w-0 flex-1 space-y-1">
                    <div className="flex items-center justify-between gap-2 text-xs">
                      <span className="truncate font-bold text-ink">
                        {item.catcher.displayName}
                      </span>
                      <span className="font-mono text-secondary tabular-nums">
                        {new Date(item.capturedAt).toLocaleString("de-AT", {
                          day: "2-digit",
                          month: "2-digit",
                          year: "numeric",
                          hour: "2-digit",
                          minute: "2-digit",
                        })}
                      </span>
                    </div>
                    {item.isBlurred ? (
                      <Badge variant="warning">Teaser verpixelt</Badge>
                    ) : (
                      <p className="line-clamp-2 text-xs font-medium text-ink">
                        {item.target ? (
                          <span className="mr-1.5 inline-block rounded-[2px] border border-strong bg-primary px-1 py-0.5 font-mono text-[10px] leading-none font-bold text-primary-foreground">
                            {item.target.displayName}
                          </span>
                        ) : null}
                        {item.promptText}
                      </p>
                    )}
                  </div>
                </button>
              ))}
              {hiddenFeedCount > 0 ? (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="w-full"
                  onClick={() => setShowAllFeed(true)}
                >
                  Alle anzeigen ({game.feed.length})
                </Button>
              ) : null}
            </div>
          )}
        </CardContent>
      </Card>

      <CaptureTileDialog
        gameId={game.id}
        tile={selectedTile}
        isPinned={selectedTile ? pinnedTileSet.has(selectedTile.id) : false}
        onTogglePin={handleTogglePinTile}
        onClose={() => {
          setSelectedTile(null);
          if (initialTileId) {
            onConsumeInitialTile?.();
          }
        }}
        onCaptured={handleTileCaptured}
      />

      <PhotoLightbox
        capture={lightboxCapture}
        open={Boolean(lightboxCapture)}
        onOpenChange={(open) => {
          if (!open) {
            setLightboxCapture(null);
            if (initialCaptureId) {
              onConsumeInitialCapture?.();
            }
          }
        }}
      />
    </div>
  );
}
