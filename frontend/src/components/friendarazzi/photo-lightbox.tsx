"use client";

import { ChevronRight, Clock, Trash2 } from "lucide-react";
import { useState } from "react";

import { apiErrorMessage } from "@/api/errors";
import type { CaptureFeedItem } from "@/api/friendarazzi-api";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { cn } from "@/lib/cn";

import { AuthenticatedImage } from "./media-image";
import { PlayerBadgeLink } from "./player-avatar";

export function LongArrow({ className }: { className?: string }) {
  return (
    <span aria-hidden="true" className={cn("flex items-center", className)}>
      <span className="-mr-2.5 h-[2px] flex-1 rounded-full bg-current" />
      <ChevronRight className="size-4 shrink-0 stroke-[2.5]" />
    </span>
  );
}

export function PhotoLightbox({
  capture,
  open,
  onOpenChange,
  canRetract = false,
  isRetracting = false,
  retractError = null,
  onRetract,
}: {
  capture: CaptureFeedItem | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  canRetract?: boolean;
  isRetracting?: boolean;
  retractError?: unknown;
  onRetract?: (capture: CaptureFeedItem) => Promise<void> | void;
}) {
  const [confirmCaptureId, setConfirmCaptureId] = useState<string | null>(null);

  if (!capture) return null;

  const confirmRetract = open && confirmCaptureId === capture.id;

  async function handleConfirmRetract() {
    if (!capture || !onRetract || isRetracting) return;
    await onRetract(capture);
    setConfirmCaptureId(null);
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(nextOpen) => {
        if (!nextOpen) setConfirmCaptureId(null);
        onOpenChange(nextOpen);
      }}
    >
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>
            {capture.isBlurred
              ? "Verpixelter Teaser"
              : (capture.promptText ?? "Beweisfoto")}
          </DialogTitle>
          <DialogDescription className="flex items-center gap-1.5 font-mono text-xs tabular-nums">
            <Clock className="size-3.5" aria-hidden="true" />
            {new Date(capture.capturedAt).toLocaleString("de-AT")}
          </DialogDescription>
        </DialogHeader>

        {/* Single framed viewport around the photo */}
        <div className="brutal-shadow-sm overflow-hidden rounded-md border-2 border-strong bg-surface-muted">
          <AuthenticatedImage
            storageKey={
              capture.isBlurred
                ? capture.thumbnailStorageKey
                : (capture.imageStorageKey ?? capture.thumbnailStorageKey)
            }
            alt={capture.promptText ?? "Beweisfoto"}
            blurred={capture.isBlurred}
            className="max-h-[60vh] w-full object-contain"
          />
        </div>

        <div className="border-t-2 border-strong pt-3">
          <div className="flex flex-wrap items-center justify-between gap-2 sm:flex-nowrap sm:gap-3">
            <PlayerBadgeLink user={capture.catcher} compact subtitle="Fänger" />
            {capture.target ? (
              <>
                <LongArrow className="min-w-8 flex-1 text-ink" />
                <PlayerBadgeLink
                  user={capture.target}
                  compact
                  subtitle="Zielperson"
                />
              </>
            ) : (
              <Badge variant="warning">Ziel verborgen</Badge>
            )}
          </div>

          {canRetract && onRetract ? (
            <div className="mt-3 space-y-2 border-t-2 border-subtle pt-3">
              {retractError ? (
                <Alert variant="error">
                  {apiErrorMessage(retractError, {
                    fallback: "Foto konnte nicht zurückgezogen werden.",
                  })}
                </Alert>
              ) : null}

              {confirmRetract ? (
                <div className="flex flex-wrap items-center justify-between gap-2 rounded-sm border-2 border-impact bg-surface-muted p-2.5">
                  <span className="text-xs font-bold text-ink">
                    Foto wirklich zurückziehen und Feld wieder freigeben?
                  </span>
                  <div className="flex items-center gap-2">
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      disabled={isRetracting}
                      onClick={() => setConfirmCaptureId(null)}
                    >
                      Abbrechen
                    </Button>
                    <Button
                      type="button"
                      variant="destructive"
                      size="sm"
                      disabled={isRetracting}
                      onClick={() => void handleConfirmRetract()}
                    >
                      <Trash2 className="size-4" aria-hidden="true" />
                      {isRetracting ? "Wird entfernt …" : "Endgültig entfernen"}
                    </Button>
                  </div>
                </div>
              ) : (
                <div className="flex justify-end">
                  <Button
                    type="button"
                    variant="destructive"
                    size="sm"
                    disabled={isRetracting}
                    onClick={() => setConfirmCaptureId(capture.id)}
                  >
                    <Trash2 className="size-4" aria-hidden="true" />
                    Foto zurückziehen
                  </Button>
                </div>
              )}
            </div>
          ) : null}
        </div>
      </DialogContent>
    </Dialog>
  );
}
