"use client";

import { ChevronRight, Clock } from "lucide-react";

import type { CaptureFeedItem } from "@/api/friendarazzi-api";
import { Badge } from "@/components/ui/badge";
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
        </div>
      </DialogContent>
    </Dialog>
  );
}
