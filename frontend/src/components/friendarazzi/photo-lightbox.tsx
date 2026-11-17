"use client";

import { ArrowRight, Clock } from "lucide-react";

import type { CaptureFeedItem } from "@/api/friendarazzi-api";
import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

import { AuthenticatedImage } from "./media-image";
import { PlayerBadgeLink } from "./player-avatar";

export function PhotoLightbox({
  capture,
  open,
  onOpenChange,
}: {
  capture: CaptureFeedItem | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  if (!capture) return null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>
            {capture.isBlurred
              ? "Verpixelter Teaser"
              : (capture.promptText ?? "Beweisfoto")}
          </DialogTitle>
          <DialogDescription className="flex items-center gap-1.5 tabular-nums">
            <Clock className="size-3.5" aria-hidden="true" />
            {new Date(capture.capturedAt).toLocaleString("de-AT")}
          </DialogDescription>
        </DialogHeader>

        <div className="overflow-hidden rounded-md border border-subtle bg-surface-muted">
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

        <div className="flex flex-wrap items-center justify-between gap-3 border-t border-subtle pt-3">
          <div className="flex flex-wrap items-center gap-2">
            <PlayerBadgeLink user={capture.catcher} compact subtitle="Fänger" />
            {capture.target ? (
              <>
                <ArrowRight
                  className="size-4 text-secondary"
                  aria-hidden="true"
                />
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
        </div>
      </DialogContent>
    </Dialog>
  );
}
