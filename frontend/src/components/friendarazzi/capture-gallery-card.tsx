"use client";

import type { CaptureFeedItem } from "@/api/friendarazzi-api";

import { AuthenticatedImage } from "./media-image";
import { LongArrow } from "./photo-lightbox";
import { PlayerAvatar } from "./player-avatar";

/**
 * Polaroid-style capture card shared across Spiel-Galerie and profile/user galleries:
 * `contain-blurred` photo window, prompt caption, and catcher -> target attribution.
 */
export function CaptureGalleryCard({
  capture,
  onSelect,
}: {
  capture: CaptureFeedItem;
  onSelect: (capture: CaptureFeedItem) => void;
}) {
  return (
    <div className="brutal-shadow-sm flex flex-col overflow-hidden rounded-md border-2 border-strong bg-surface p-2">
      <button
        type="button"
        onClick={() => onSelect(capture)}
        className="aspect-square w-full cursor-pointer overflow-hidden rounded-sm border border-strong bg-surface-muted focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
      >
        <AuthenticatedImage
          storageKey={
            capture.isBlurred
              ? capture.thumbnailStorageKey
              : (capture.imageStorageKey ?? capture.thumbnailStorageKey)
          }
          alt={capture.promptText ?? "Beweisfoto"}
          fit="contain-blurred"
          blurred={capture.isBlurred}
          className="h-full w-full"
        />
      </button>

      <div className="space-y-2 pt-2.5">
        <p className="line-clamp-2 text-xs font-bold text-ink">
          {capture.isBlurred ? "Verpixelter Teaser" : capture.promptText}
        </p>
        <div className="flex items-center justify-between gap-1.5">
          <div className="flex min-w-0 flex-1 items-center gap-1.5">
            <PlayerAvatar user={capture.catcher} size="sm" />
            {capture.target ? (
              <>
                <LongArrow className="min-w-5 max-w-10 flex-1 text-secondary" />
                <PlayerAvatar user={capture.target} size="sm" />
              </>
            ) : null}
          </div>
          <span className="font-mono text-[11px] font-semibold text-secondary tabular-nums">
            {new Date(capture.capturedAt).toLocaleTimeString("de-AT", {
              hour: "2-digit",
              minute: "2-digit",
            })}
          </span>
        </div>
      </div>
    </div>
  );
}
