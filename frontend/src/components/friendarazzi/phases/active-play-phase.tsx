"use client";

import {
  ChevronDown,
  ChevronUp,
  LayoutGrid,
  List,
  Pin,
  Upload,
  Users,
  X,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import { apiErrorMessage } from "@/api/errors";
import {
  type BoardTileView,
  type CaptureFeedItem,
  type GameDetail,
  VisibilityMode,
} from "@/api/friendarazzi-api";
import { useFriendarazziMutations } from "@/api/hooks";
import { BingoBoard } from "@/components/friendarazzi/bingo-board";
import { CaptureUploadControl } from "@/components/friendarazzi/capture-upload-control";
import { AuthenticatedImage } from "@/components/friendarazzi/media-image";
import { PhotoLightbox } from "@/components/friendarazzi/photo-lightbox";
import {
  PlayerAvatar,
  PlayerBadgeLink,
} from "@/components/friendarazzi/player-avatar";
import { TargetGroupedBoard } from "@/components/friendarazzi/target-grouped-board";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { CalloutNote, CodeTag } from "@/components/ui/brutal-elements";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { Progress } from "@/components/ui/progress";
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
import { normalizeImageForUpload } from "@/lib/media-capture";

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
  const mutations = useFriendarazziMutations();
  const [selectedTile, setSelectedTile] = useState<BoardTileView | null>(null);
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [isPreparingUpload, setIsPreparingUpload] = useState(false);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [lightboxCapture, setLightboxCapture] =
    useState<CaptureFeedItem | null>(null);
  const [mobileFeedOpen, setMobileFeedOpen] = useState(false);
  const [showAllFeed, setShowAllFeed] = useState(false);
  const [viewMode, setViewMode] = useState<BoardViewMode>(() =>
    loadBoardViewMode(),
  );
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
      setSelectedFile(null);
      setSelectedTile(found);
    }
  }, [initialTileId, board]);

  useEffect(() => {
    if (!selectedFile) {
      setPreviewUrl(null);
      return;
    }
    const url = URL.createObjectURL(selectedFile);
    setPreviewUrl(url);
    return () => URL.revokeObjectURL(url);
  }, [selectedFile]);

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

  if (!board) {
    return (
      <div aria-busy="true" className="space-y-3">
        <Skeleton className="h-10 w-full" />
        <Skeleton className="h-80 w-full" />
      </div>
    );
  }

  const capturedCount = board.tiles.filter((t) => t.isCaptured).length;
  const totalCount = board.tiles.length;
  const pct =
    totalCount > 0 ? Math.round((capturedCount / totalCount) * 100) : 0;

  const isUploading = isPreparingUpload || mutations.submitCapture.isPending;
  const newestFirstFeed = newestCapturesFirst(game.feed);
  const visibleFeed = showAllFeed
    ? newestFirstFeed
    : newestFirstFeed.slice(0, INITIAL_FEED_ITEMS);
  const hiddenFeedCount = Math.max(0, game.feed.length - visibleFeed.length);

  async function submitTilePhoto() {
    if (!selectedTile || !selectedFile || isUploading) return;
    const submittedTileId = selectedTile.id;
    setIsPreparingUpload(true);
    try {
      const normalized = await normalizeImageForUpload(selectedFile);
      await mutations.submitCapture.mutateAsync({
        gameId: game.id,
        tileId: submittedTileId,
        file: normalized,
      });
      if (pinnedTileSet.has(submittedTileId)) {
        setPinnedTileIds((prev) => {
          const next = prev.filter((id) => id !== submittedTileId);
          savePinnedTiles(game.id, next);
          return next;
        });
      }
      setSelectedTile(null);
      setSelectedFile(null);
      if (initialTileId) {
        onConsumeInitialTile?.();
      }
    } finally {
      setIsPreparingUpload(false);
    }
  }

  return (
    <div className="grid items-start gap-5 lg:grid-cols-[1.5fr_0.75fr]">
      {/* Left: Compact progress + Crew popover directly above the hero board */}
      <div className="space-y-3">
        <div className="flex items-center justify-between gap-3">
          <div className="flex min-w-0 flex-1 items-center gap-3">
            <Progress
              value={pct}
              aria-label="Brett-Fortschritt"
              className="h-3 max-w-52 flex-1"
            />
            <span className="shrink-0 font-mono text-xs font-bold text-ink tabular-nums">
              {capturedCount} / {totalCount}
            </span>
          </div>

          <Popover>
            <PopoverTrigger asChild>
              <Button
                type="button"
                variant="secondary"
                size="sm"
                className="shrink-0 gap-1.5"
              >
                <Users className="size-4" aria-hidden="true" />
                <span>Crew ({game.players.length})</span>
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
        </div>

        {/* Hunt Toolbar: Grid/List toggle + Search */}
        <div className="flex flex-wrap items-center gap-2">
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

          <SearchInput
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Ziel oder Auftrag …"
            aria-label="Aufträge oder Ziele durchsuchen"
            className="min-w-44 flex-1 sm:max-w-64"
          />
        </div>

        {/* Pinned Tasks Strip */}
        {pinnedTiles.length > 0 ? (
          <div className="flex flex-wrap items-center gap-2 rounded-md border-2 border-strong bg-primary-subtle px-3 py-2">
            <span className="inline-flex items-center gap-1 font-mono text-[11px] font-bold text-ink uppercase">
              <Pin className="size-3.5 rotate-45" aria-hidden="true" />
              Fokus:
            </span>
            {pinnedTiles.map((tile) => (
              <div
                key={tile.id}
                className="inline-flex max-w-full items-center overflow-hidden rounded-sm border-2 border-strong bg-surface"
              >
                <button
                  type="button"
                  onClick={() => {
                    mutations.submitCapture.reset();
                    setSelectedFile(null);
                    setSelectedTile(tile);
                  }}
                  className="inline-flex min-h-11 min-w-0 cursor-pointer items-center gap-1.5 px-2.5 text-left text-xs font-bold text-ink transition-colors hover:bg-surface-muted focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
                >
                  <span className="shrink-0 rounded-[2px] border border-strong bg-primary px-1 py-0.5 font-mono text-[10px] leading-none font-bold text-primary-foreground">
                    {tile.targetUser.displayName}
                  </span>
                  <span className="truncate">{tile.promptText}</span>
                </button>
                <button
                  type="button"
                  aria-label={`Auftrag „${tile.promptText}“ aus Fokus entfernen`}
                  onClick={() => handleTogglePinTile(tile.id)}
                  className="inline-flex min-h-11 min-w-11 shrink-0 cursor-pointer items-center justify-center border-l-2 border-strong text-ink hover:bg-surface-muted focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
                >
                  <X className="size-3.5" aria-hidden="true" />
                </button>
              </div>
            ))}
          </div>
        ) : null}

        {capturedCount === 0 && !coachDismissed ? (
          <Alert variant="info">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <p>
                <strong>Tipp für die Jagd:</strong> Wechsle zwischen Raster und
                „Nach Ziel“ oder hefte einzelne Aufträge im Fokus an – deine
                angehefteten Aufgaben erscheinen auch direkt in der
                Spielzentrale.
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

        {viewMode === "list" ? (
          <TargetGroupedBoard
            board={board}
            feed={game.feed}
            pinnedTileIds={pinnedTileSet}
            onTogglePinTile={handleTogglePinTile}
            hideCaptured
            searchQuery={searchQuery}
            onEmptyTileSelect={(tile) => {
              mutations.submitCapture.reset();
              setSelectedFile(null);
              setSelectedTile(tile);
            }}
            onViewCapture={(cap) => setLightboxCapture(cap)}
          />
        ) : (
          <BingoBoard
            board={board}
            gridSize={game.gridSize}
            feed={game.feed}
            matchingTileIds={matchingTileIds}
            pinnedTileIds={pinnedTileSet}
            onEmptyTileSelect={(tile) => {
              mutations.submitCapture.reset();
              setSelectedFile(null);
              setSelectedTile(tile);
            }}
            onViewCapture={(cap) => setLightboxCapture(cap)}
          />
        )}
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

      {/* Camera / Upload Capture Dialog */}
      <Dialog
        open={Boolean(selectedTile)}
        onOpenChange={(open) => {
          if (!open) {
            setSelectedTile(null);
            setSelectedFile(null);
            if (initialTileId) {
              onConsumeInitialTile?.();
            }
          }
        }}
      >
        <DialogContent>
          <DialogHeader className="space-y-2.5">
            {selectedTile?.targetUser ? (
              <div className="flex items-center justify-between gap-3 rounded-md border-2 border-strong bg-cyan-subtle p-2.5">
                <div className="flex min-w-0 items-center gap-3">
                  <PlayerAvatar
                    user={selectedTile.targetUser}
                    size="sm"
                    linkToProfile={false}
                  />
                  <div className="min-w-0">
                    <p className="truncate font-display text-sm leading-tight text-ink">
                      {selectedTile.targetUser.displayName}
                    </p>
                    <p className="truncate font-mono text-[11px] font-bold text-secondary">
                      {selectedTile.targetUser.effectiveTitle}
                    </p>
                  </div>
                </div>
                <div className="flex shrink-0 items-center gap-1.5">
                  <span className="rounded-sm border border-strong bg-surface px-1.5 py-0.5 font-mono text-[10px] font-bold tracking-wider text-ink uppercase">
                    Zielperson
                  </span>
                  <Button
                    type="button"
                    variant={
                      pinnedTileSet.has(selectedTile.id)
                        ? "primary"
                        : "secondary"
                    }
                    size="icon"
                    aria-pressed={pinnedTileSet.has(selectedTile.id)}
                    aria-label={
                      pinnedTileSet.has(selectedTile.id)
                        ? `Auftrag „${selectedTile.promptText}“ aus Fokus entfernen`
                        : `Auftrag „${selectedTile.promptText}“ anheften`
                    }
                    title={
                      pinnedTileSet.has(selectedTile.id)
                        ? "Aus Fokus entfernen"
                        : "Im Fokus anheften"
                    }
                    onClick={() => handleTogglePinTile(selectedTile.id)}
                  >
                    <Pin
                      className={cn(
                        "size-4 rotate-45",
                        pinnedTileSet.has(selectedTile.id) && "fill-current",
                      )}
                      aria-hidden="true"
                    />
                  </Button>
                </div>
              </div>
            ) : null}
            <DialogTitle className="font-display text-2xl leading-tight tracking-tight text-ink">
              {selectedTile?.promptText}
            </DialogTitle>
            <DialogDescription className="sr-only">
              Lade ein Beweisfoto für diesen Auftrag hoch.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-3">
            {previewUrl ? (
              <div className="space-y-2">
                <div className="brutal-shadow-sm overflow-hidden rounded-md border-2 border-strong bg-surface p-1.5">
                  {/* biome-ignore lint/performance/noImgElement: local File preview blob URL */}
                  <img
                    src={previewUrl}
                    alt="Vorschau"
                    className="max-h-72 w-full rounded-sm object-contain"
                  />
                </div>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  disabled={isUploading}
                  onClick={() => setSelectedFile(null)}
                >
                  Anderes Foto wählen
                </Button>
              </div>
            ) : (
              <CaptureUploadControl
                onFileSelect={(file) => setSelectedFile(file)}
              />
            )}

            {mutations.submitCapture.isError ? (
              <Alert variant="error">
                {apiErrorMessage(mutations.submitCapture.error, {
                  fallback: "Das Foto konnte nicht hochgeladen werden.",
                })}
              </Alert>
            ) : null}
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="secondary"
              onClick={() => {
                setSelectedTile(null);
                setSelectedFile(null);
                if (initialTileId) {
                  onConsumeInitialTile?.();
                }
              }}
              disabled={isUploading}
            >
              Abbrechen
            </Button>
            <Button
              type="button"
              disabled={!selectedFile || isUploading}
              onClick={() => void submitTilePhoto()}
            >
              <Upload className="size-4" aria-hidden="true" />
              {isUploading ? "Wird hochgeladen …" : "Beweis einreichen"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

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
