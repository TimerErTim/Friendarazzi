"use client";

import { Camera, Check, Video, VideoOff, X } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import { CaptureUploadControl } from "@/components/friendarazzi/capture-upload-control";
import { Alert } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { cn } from "@/lib/cn";
import type {
  CachedBoardTile,
  QueuedCaptureItem,
} from "@/lib/offline-capture-queue";

/**
 * Shutter-first capture dialog: shoot first (via native camera, file picker, or
 * live viewfinder burst into the local tray) and assign to an open Bingo tile when ready.
 */
export function QuickCaptureSheet({
  open,
  onOpenChange,
  openTiles,
  trayItems,
  initialSelectedTrayId = null,
  onAddTrayPhoto,
  onAssignTrayItem,
  onDiscardTrayItem,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  openTiles: CachedBoardTile[];
  trayItems: QueuedCaptureItem[];
  initialSelectedTrayId?: string | null;
  onAddTrayPhoto: (file: File) => Promise<QueuedCaptureItem>;
  onAssignTrayItem: (itemId: string, tileId: string) => Promise<void>;
  onDiscardTrayItem: (itemId: string) => Promise<void>;
}) {
  const [selectedItemId, setSelectedItemId] = useState<string | null>(
    initialSelectedTrayId,
  );
  const [targetFilter, setTargetFilter] = useState<string>("all");
  const [cameraActive, setCameraActive] = useState(false);
  const [cameraError, setCameraError] = useState<string | null>(null);
  const [isCapturing, setIsCapturing] = useState(false);
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const streamRef = useRef<MediaStream | null>(null);

  useEffect(() => {
    if (open) {
      setSelectedItemId(initialSelectedTrayId ?? trayItems[0]?.id ?? null);
    } else {
      setCameraActive(false);
      setCameraError(null);
    }
  }, [open, initialSelectedTrayId, trayItems]);

  useEffect(() => {
    if (!selectedItemId && trayItems.length > 0) {
      setSelectedItemId(trayItems[0]?.id ?? null);
    }
  }, [selectedItemId, trayItems]);

  useEffect(() => {
    if (!cameraActive) {
      if (streamRef.current) {
        for (const track of streamRef.current.getTracks()) {
          track.stop();
        }
        streamRef.current = null;
      }
      return;
    }

    let cancelled = false;
    async function startStream() {
      if (
        typeof navigator === "undefined" ||
        !navigator.mediaDevices?.getUserMedia
      ) {
        setCameraError("Live-Kamera wird in diesem Browser nicht unterstützt.");
        setCameraActive(false);
        return;
      }
      try {
        const stream = await navigator.mediaDevices.getUserMedia({
          video: { facingMode: "environment" },
          audio: false,
        });
        if (cancelled) {
          for (const track of stream.getTracks()) track.stop();
          return;
        }
        streamRef.current = stream;
        if (videoRef.current) {
          videoRef.current.srcObject = stream;
        }
        setCameraError(null);
      } catch {
        setCameraError(
          "Kamera-Zugriff abgelehnt oder nicht verfügbar – nutze stattdessen den Foto-Button.",
        );
        setCameraActive(false);
      }
    }

    void startStream();
    return () => {
      cancelled = true;
      if (streamRef.current) {
        for (const track of streamRef.current.getTracks()) {
          track.stop();
        }
        streamRef.current = null;
      }
    };
  }, [cameraActive]);

  const activeItem =
    trayItems.find((item) => item.id === selectedItemId) ??
    trayItems[0] ??
    null;

  const previewUrl = useMemo(() => {
    if (!activeItem) return null;
    return URL.createObjectURL(activeItem.blob);
  }, [activeItem]);

  useEffect(() => {
    return () => {
      if (previewUrl) URL.revokeObjectURL(previewUrl);
    };
  }, [previewUrl]);

  const targetOptions = useMemo(() => {
    const map = new Map<string, string>();
    for (const tile of openTiles) {
      if (!map.has(tile.targetUserId)) {
        map.set(tile.targetUserId, tile.targetDisplayName);
      }
    }
    return Array.from(map.entries())
      .map(([id, displayName]) => ({ id, displayName }))
      .sort((a, b) => a.displayName.localeCompare(b.displayName, "de"));
  }, [openTiles]);

  const filteredTiles = useMemo(() => {
    if (targetFilter === "all") return openTiles;
    return openTiles.filter((t) => t.targetUserId === targetFilter);
  }, [openTiles, targetFilter]);

  async function handleFileSelect(file: File) {
    setIsCapturing(true);
    try {
      const added = await onAddTrayPhoto(file);
      setSelectedItemId(added.id);
    } finally {
      setIsCapturing(false);
    }
  }

  async function handleLiveSnap() {
    const video = videoRef.current;
    if (!video || video.videoWidth === 0 || video.videoHeight === 0) return;
    setIsCapturing(true);
    try {
      const canvas = document.createElement("canvas");
      canvas.width = video.videoWidth;
      canvas.height = video.videoHeight;
      const ctx = canvas.getContext("2d");
      if (!ctx) return;
      ctx.drawImage(video, 0, 0, canvas.width, canvas.height);
      const blob = await new Promise<Blob | null>((resolve) =>
        canvas.toBlob(resolve, "image/jpeg", 0.9),
      );
      if (!blob) return;
      const file = new File([blob], `snap-${Date.now()}.jpg`, {
        type: "image/jpeg",
      });
      const added = await onAddTrayPhoto(file);
      setSelectedItemId(added.id);
    } finally {
      setIsCapturing(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Camera className="size-5" aria-hidden="true" />
            Schnellfoto &amp; Ablage
          </DialogTitle>
          <DialogDescription>
            Knipse erst deinen Schnappschuss und ordne ihn direkt einem offenen
            Auftrag zu – oder speichere ihn für später.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {cameraError ? <Alert variant="info">{cameraError}</Alert> : null}

          {cameraActive ? (
            <div className="space-y-2">
              <div className="brutal-shadow-sm overflow-hidden rounded-md border-2 border-strong bg-surface-muted">
                <video
                  ref={videoRef}
                  autoPlay
                  playsInline
                  muted
                  className="max-h-64 w-full object-cover"
                />
              </div>
              <div className="flex flex-wrap items-center justify-between gap-2">
                <Button
                  type="button"
                  variant="secondary"
                  size="sm"
                  onClick={() => setCameraActive(false)}
                >
                  <VideoOff className="size-4" aria-hidden="true" />
                  Kamera schließen
                </Button>
                <Button
                  type="button"
                  variant="primary"
                  disabled={isCapturing}
                  onClick={() => void handleLiveSnap()}
                >
                  <Camera className="size-4" aria-hidden="true" />
                  Auslösen (in Ablage)
                </Button>
              </div>
            </div>
          ) : (
            <div className="space-y-2">
              <CaptureUploadControl
                onFileSelect={(file) => void handleFileSelect(file)}
              />
              {typeof navigator !== "undefined" &&
              Boolean(navigator.mediaDevices?.getUserMedia) ? (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="w-full"
                  onClick={() => setCameraActive(true)}
                >
                  <Video className="size-4" aria-hidden="true" />
                  Live-Sucher für Serienbilder öffnen
                </Button>
              ) : null}
            </div>
          )}

          {trayItems.length > 0 ? (
            <div className="space-y-3 border-t-2 border-strong pt-3">
              <div className="flex items-center justify-between gap-2">
                <span className="font-mono text-xs font-bold tracking-wider text-ink uppercase">
                  Ablage ({trayItems.length})
                </span>
                {activeItem ? (
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => void onDiscardTrayItem(activeItem.id)}
                  >
                    <X className="size-4" aria-hidden="true" />
                    Foto verwerfen
                  </Button>
                ) : null}
              </div>

              {trayItems.length > 1 ? (
                <div className="flex flex-wrap gap-1.5">
                  {trayItems.map((item, idx) => (
                    <button
                      key={item.id}
                      type="button"
                      onClick={() => setSelectedItemId(item.id)}
                      className={cn(
                        "min-h-11 cursor-pointer rounded-sm border-2 border-control px-3 py-1 font-mono text-xs font-bold",
                        activeItem?.id === item.id
                          ? "bg-primary text-primary-foreground"
                          : "bg-surface text-ink hover:bg-surface-muted",
                      )}
                    >
                      Foto #{idx + 1}
                    </button>
                  ))}
                </div>
              ) : null}

              {previewUrl ? (
                <div className="brutal-shadow-sm overflow-hidden rounded-md border-2 border-strong bg-surface p-1.5">
                  {/* biome-ignore lint/performance/noImgElement: local File preview blob URL */}
                  <img
                    src={previewUrl}
                    alt="Ausgewählter Schnappschuss"
                    className="max-h-48 w-full rounded-sm object-contain"
                  />
                </div>
              ) : null}

              {activeItem ? (
                <div className="space-y-2">
                  <p className="font-mono text-xs font-bold text-ink uppercase">
                    Einem offenen Auftrag zuordnen:
                  </p>
                  {targetOptions.length > 1 ? (
                    <div className="flex flex-wrap gap-1.5">
                      <button
                        type="button"
                        onClick={() => setTargetFilter("all")}
                        className={cn(
                          "min-h-11 cursor-pointer rounded-sm border-2 border-control px-2.5 py-1 font-mono text-xs font-bold",
                          targetFilter === "all"
                            ? "bg-cyan text-cyan-foreground"
                            : "bg-surface text-ink hover:bg-surface-muted",
                        )}
                      >
                        Alle
                      </button>
                      {targetOptions.map((target) => (
                        <button
                          key={target.id}
                          type="button"
                          onClick={() => setTargetFilter(target.id)}
                          className={cn(
                            "min-h-11 cursor-pointer rounded-sm border-2 border-control px-2.5 py-1 font-mono text-xs font-bold",
                            targetFilter === target.id
                              ? "bg-cyan text-cyan-foreground"
                              : "bg-surface text-ink hover:bg-surface-muted",
                          )}
                        >
                          {target.displayName}
                        </button>
                      ))}
                    </div>
                  ) : null}

                  {filteredTiles.length === 0 ? (
                    <p className="text-xs font-medium text-secondary">
                      Keine passenden offenen Kacheln übrig.
                    </p>
                  ) : (
                    <div className="max-h-48 space-y-1.5 overflow-y-auto pr-1">
                      {filteredTiles.map((tile) => (
                        <button
                          key={tile.id}
                          type="button"
                          onClick={() => {
                            void onAssignTrayItem(activeItem.id, tile.id).then(
                              () => {
                                if (trayItems.length <= 1) {
                                  onOpenChange(false);
                                }
                              },
                            );
                          }}
                          className="brutal-press flex min-h-11 w-full cursor-pointer items-center justify-between gap-2 rounded-sm border-2 border-control bg-surface px-3 py-2 text-left text-xs font-semibold text-ink hover:bg-primary-subtle focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
                        >
                          <span className="min-w-0 flex-1">
                            <span className="mr-1.5 inline-block rounded-xs border border-strong bg-cyan-subtle px-1 py-0.5 font-mono text-[10px] font-bold text-ink">
                              {tile.targetDisplayName}
                            </span>
                            {tile.promptText}
                          </span>
                          <Check
                            className="size-4 shrink-0 text-ink"
                            aria-hidden="true"
                          />
                        </button>
                      ))}
                    </div>
                  )}
                </div>
              ) : null}
            </div>
          ) : null}
        </div>

        <DialogFooter>
          <Button
            type="button"
            variant="secondary"
            onClick={() => onOpenChange(false)}
          >
            {trayItems.length > 0 ? "Für später merken" : "Schließen"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
