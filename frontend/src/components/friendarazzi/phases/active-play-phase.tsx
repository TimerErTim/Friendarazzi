"use client";

import { Camera, Crosshair, Upload, Users } from "lucide-react";
import { useEffect, useState } from "react";

import { apiErrorMessage } from "@/api/errors";
import {
  type BoardTileView,
  type CaptureFeedItem,
  type GameDetail,
  VisibilityMode,
} from "@/api/friendarazzi-api";
import { useFriendarazziMutations } from "@/api/hooks";
import { BingoBoard } from "@/components/friendarazzi/bingo-board";
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
import { Progress } from "@/components/ui/progress";

export function ActivePlayPhase({ game }: { game: GameDetail }) {
  const mutations = useFriendarazziMutations();
  const [selectedTile, setSelectedTile] = useState<BoardTileView | null>(null);
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [lightboxCapture, setLightboxCapture] =
    useState<CaptureFeedItem | null>(null);

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
      <CalloutNote label="Brett wird geladen" tone="yellow">
        Dein Bingo-Brett wird vorbereitet …
      </CalloutNote>
    );
  }

  const capturedCount = board.tiles.filter((t) => t.isCaptured).length;
  const totalCount = board.tiles.length;
  const pct =
    totalCount > 0 ? Math.round((capturedCount / totalCount) * 100) : 0;

  async function submitTilePhoto() {
    if (!selectedTile || !selectedFile) return;
    await mutations.submitCapture.mutateAsync({
      gameId: game.id,
      tileId: selectedTile.id,
      file: selectedFile,
    });
    setSelectedTile(null);
    setSelectedFile(null);
  }

  return (
    <div className="grid items-start gap-6 lg:grid-cols-[1.35fr_0.65fr]">
      {/* Left: Hero Bingo Board */}
      <div className="space-y-4">
        <div className="brutal-shadow rounded-md border-[2.5px] border-strong bg-surface p-4">
          <div className="mb-2.5 flex items-center justify-between gap-2">
            <span className="font-mono text-xs font-bold text-secondary uppercase">
              Dein Brett-Fortschritt
            </span>
            <span className="font-mono text-xs font-bold text-ink tabular-nums">
              {capturedCount} / {totalCount} Felder
            </span>
          </div>
          <Progress value={pct} aria-label="Spielfortschritt" />
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

      {/* Right: Activity Stream & Crew */}
      <div className="space-y-6">
        <Card>
          <CardHeader className="bg-pink-subtle">
            <div className="flex items-center justify-between gap-2">
              <CardTitle>Live-Aktivität</CardTitle>
              <CodeTag tone="cyan">{game.feed.length}</CodeTag>
            </div>
          </CardHeader>
          <CardContent className="space-y-3 pt-4">
            {game.visibilityMode === VisibilityMode.Undercover ? (
              <CalloutNote label="Undercover aktiv" tone="pink">
                Alle Beweisfotos bleiben bis zum Bingo geheim. Deine eigenen
                Treffer siehst du direkt auf deinem Brett.
              </CalloutNote>
            ) : game.feed.length === 0 ? (
              <CalloutNote label="Noch ruhig">
                Noch keine Beweisfotos hochgeladen – schnapp dir dein erstes
                Motiv!
              </CalloutNote>
            ) : (
              <div className="space-y-3">
                {game.feed.map((item) => (
                  <button
                    key={item.id}
                    type="button"
                    onClick={() => setLightboxCapture(item)}
                    className="brutal-press flex w-full cursor-pointer items-center gap-3 rounded-md border-2 border-strong bg-surface p-2 text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
                  >
                    <div className="size-16 shrink-0 overflow-hidden rounded-sm border-2 border-strong bg-surface-muted">
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
                          {new Date(item.capturedAt).toLocaleTimeString(
                            "de-AT",
                            { hour: "2-digit", minute: "2-digit" },
                          )}
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
              </div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Users className="size-5" aria-hidden="true" />
              Mitspieler
            </CardTitle>
          </CardHeader>
          <CardContent className="pt-4">
            <ul className="divide-y divide-subtle">
              {game.players.map((p) => (
                <li key={p.id} className="py-2 first:pt-0 last:pb-0">
                  <PlayerBadgeLink user={p} compact />
                </li>
              ))}
            </ul>
          </CardContent>
        </Card>
      </div>

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
          <DialogHeader className="space-y-3">
            <span className="inline-flex w-fit items-center gap-1.5 rounded-sm border-2 border-strong bg-surface-muted px-2 py-0.5 font-mono text-[11px] font-bold tracking-wider text-ink uppercase">
              <Camera className="size-3.5" aria-hidden="true" />
              Beweisfoto hochladen
            </span>
            <DialogTitle className="font-display text-2xl leading-tight tracking-tight text-ink sm:text-3xl">
              {selectedTile?.promptText}
            </DialogTitle>
            <DialogDescription className="sr-only">
              Lade ein Beweisfoto für diesen Auftrag hoch.
            </DialogDescription>
          </DialogHeader>

          {selectedTile?.targetUser ? (
            <div className="brutal-shadow-sm flex items-center gap-3 rounded-md border-2 border-strong bg-cyan-subtle p-2.5">
              <div className="relative shrink-0">
                <PlayerAvatar
                  user={selectedTile.targetUser}
                  size="md"
                  linkToProfile={false}
                  showMedalBadge={false}
                />
                <span
                  aria-hidden="true"
                  className="absolute -right-1 -bottom-1 flex size-5 items-center justify-center rounded-full border-2 border-strong bg-primary text-primary-foreground"
                >
                  <Crosshair className="size-3" />
                </span>
              </div>
              <div className="min-w-0 flex-1">
                <p className="truncate font-display text-base leading-tight text-ink">
                  {selectedTile.targetUser.displayName}
                </p>
                <p className="truncate font-mono text-[11px] font-bold text-secondary">
                  vor der Linse erwischen ·{" "}
                  {selectedTile.targetUser.effectiveTitle}
                </p>
              </div>
            </div>
          ) : null}

          <div className="space-y-4">
            <label className="brutal-press flex min-h-11 cursor-pointer flex-col items-center justify-center gap-2 rounded-md border-2 border-dashed border-control bg-primary-subtle p-6 text-center hover:bg-primary/25">
              <Camera className="size-7 text-ink" aria-hidden="true" />
              <span className="text-sm font-bold text-ink">
                Kamera öffnen oder Foto auswählen
              </span>
              <span className="font-mono text-xs text-secondary">
                JPEG, PNG oder WebP (wird automatisch optimiert)
              </span>
              <input
                type="file"
                accept="image/*"
                capture="environment"
                className="sr-only"
                onChange={(e) => {
                  const file = e.target.files?.[0] ?? null;
                  setSelectedFile(file);
                }}
              />
            </label>

            {previewUrl ? (
              <div className="brutal-shadow-sm overflow-hidden rounded-md border-2 border-strong bg-surface p-1.5">
                {/* biome-ignore lint/performance/noImgElement: local File preview blob URL */}
                <img
                  src={previewUrl}
                  alt="Vorschau"
                  className="max-h-64 w-full rounded-sm object-contain"
                />
              </div>
            ) : null}

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
              disabled={mutations.submitCapture.isPending}
            >
              Abbrechen
            </Button>
            <Button
              type="button"
              disabled={!selectedFile || mutations.submitCapture.isPending}
              onClick={() => void submitTilePhoto()}
            >
              <Upload className="size-4" aria-hidden="true" />
              {mutations.submitCapture.isPending
                ? "Wird hochgeladen …"
                : "Beweis einreichen"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <PhotoLightbox
        capture={lightboxCapture}
        open={Boolean(lightboxCapture)}
        onOpenChange={(open) => {
          if (!open) setLightboxCapture(null);
        }}
      />
    </div>
  );
}
