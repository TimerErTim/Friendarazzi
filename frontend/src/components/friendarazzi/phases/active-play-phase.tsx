"use client";

import { ChevronDown, ChevronUp, Upload, Users } from "lucide-react";
import { useEffect, useRef, useState } from "react";

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
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/cn";
import { normalizeImageForUpload } from "@/lib/media-capture";

const INITIAL_FEED_ITEMS = 3;

export function ActivePlayPhase({
  game,
  initialCaptureId = null,
  onConsumeInitialCapture,
}: {
  game: GameDetail;
  initialCaptureId?: string | null;
  onConsumeInitialCapture?: () => void;
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
  const openedCaptureRef = useRef<string | null>(null);

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

  useEffect(() => {
    if (!selectedFile) {
      setPreviewUrl(null);
      return;
    }
    const url = URL.createObjectURL(selectedFile);
    setPreviewUrl(url);
    return () => URL.revokeObjectURL(url);
  }, [selectedFile]);

  const board = game.myBoard;
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
  const visibleFeed = showAllFeed
    ? game.feed
    : game.feed.slice(0, INITIAL_FEED_ITEMS);
  const hiddenFeedCount = Math.max(0, game.feed.length - visibleFeed.length);

  async function submitTilePhoto() {
    if (!selectedTile || !selectedFile || isUploading) return;
    setIsPreparingUpload(true);
    try {
      const normalized = await normalizeImageForUpload(selectedFile);
      await mutations.submitCapture.mutateAsync({
        gameId: game.id,
        tileId: selectedTile.id,
        file: normalized,
      });
      setSelectedTile(null);
      setSelectedFile(null);
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

        <BingoBoard
          board={board}
          gridSize={game.gridSize}
          feed={game.feed}
          onEmptyTileSelect={(tile) => {
            mutations.submitCapture.reset();
            setSelectedFile(null);
            setSelectedTile(tile);
          }}
          onViewCapture={(cap) => setLightboxCapture(cap)}
        />
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
                        {new Date(item.capturedAt).toLocaleTimeString("de-AT", {
                          hour: "2-digit",
                          minute: "2-digit",
                        })}
                      </span>
                    </div>
                    {item.isBlurred ? (
                      <Badge variant="warning">Teaser verpixelt</Badge>
                    ) : (
                      <p className="line-clamp-2 text-xs font-medium text-ink">
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
          }
        }}
      >
        <DialogContent>
          <DialogHeader className="space-y-2.5">
            {selectedTile?.targetUser ? (
              <div className="flex items-center gap-3 rounded-md border-2 border-strong bg-cyan-subtle p-2.5">
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
              onClick={() => setSelectedTile(null)}
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
