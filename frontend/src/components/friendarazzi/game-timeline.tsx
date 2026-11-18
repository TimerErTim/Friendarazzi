"use client";

import { Camera, Clock } from "lucide-react";
import { useState } from "react";

import type { CaptureFeedItem } from "@/api/friendarazzi-api";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { MarginNote } from "@/components/ui/paper-rule";
import { cn } from "@/lib/cn";

import { AuthenticatedImage } from "./media-image";
import { PhotoLightbox } from "./photo-lightbox";

/**
 * 1:1 linear-scaled timeline proportional to total match duration (`startedAt` .. `endedAt`).
 * Quiet stretches remain visually open while clusters highlight chaotic bursts.
 */
export function GameTimeline({
  startedAt,
  endedAt,
  captures,
}: {
  startedAt: string | null | undefined;
  endedAt: string | null | undefined;
  captures: CaptureFeedItem[];
}) {
  const [selected, setSelected] = useState<CaptureFeedItem | null>(null);

  if (captures.length === 0) {
    return (
      <MarginNote label="1:1 Zeitachse">
        In dieser Runde wurden keine Fotos aufgenommen.
      </MarginNote>
    );
  }

  const times = captures.map((c) => Date.parse(c.capturedAt));
  const startMs = startedAt ? Date.parse(startedAt) : Math.min(...times);
  const endMs = endedAt ? Date.parse(endedAt) : Math.max(...times);
  const totalMs = Math.max(endMs - startMs, 1000);

  const formatDuration = (ms: number) => {
    const mins = Math.floor(ms / 60000);
    const secs = Math.floor((ms % 60000) / 1000);
    if (mins >= 60) {
      const hrs = Math.floor(mins / 60);
      return `${hrs} Std. ${mins % 60} Min.`;
    }
    return mins > 0 ? `${mins} Min. ${secs} Sek.` : `${secs} Sek.`;
  };

  return (
    <Card>
      <CardHeader>
        <div className="flex flex-wrap items-center justify-between gap-2">
          <CardTitle>1:1 Spiel-Zeitachse</CardTitle>
          <span className="inline-flex items-center gap-1.5 text-xs text-secondary tabular-nums">
            <Clock className="size-3.5" aria-hidden="true" />
            Gesamtdauer: {formatDuration(totalMs)}
          </span>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        <p className="text-xs text-secondary">
          Maßstabsgetreue Darstellung vom Spielstart bis zum Bingo. Tippe auf
          einen Punkt, um Beweisfoto und Details zu öffnen.
        </p>

        {/* Scrollable 1:1 track on narrow mobile screens, full width on desktop */}
        <div className="overflow-x-auto pb-2">
          <div className="relative min-w-[540px] pt-12 pb-8">
            {/* Baseline track */}
            <div className="millimetre-grid absolute inset-x-4 top-6 h-10 rounded-sm border border-subtle" />
            <div className="absolute inset-x-4 top-1/2 h-0.5 -translate-y-1/2 bg-strong" />

            {/* Start / End labels */}
            <div className="absolute top-0 left-4 text-[11px] text-secondary tabular-nums">
              Start:{" "}
              {new Date(startMs).toLocaleTimeString("de-AT", {
                hour: "2-digit",
                minute: "2-digit",
              })}
            </div>
            <div className="absolute top-0 right-4 text-[11px] text-secondary tabular-nums">
              Bingo:{" "}
              {new Date(endMs).toLocaleTimeString("de-AT", {
                hour: "2-digit",
                minute: "2-digit",
              })}
            </div>

            {/* Proportional 1:1 nodes */}
            <div className="relative mx-6 h-12">
              {captures.map((cap, index) => {
                const capMs = Date.parse(cap.capturedAt);
                const ratio = Math.min(
                  1,
                  Math.max(0, (capMs - startMs) / totalMs),
                );
                const leftPct = ratio * 100;
                const staggerTop = index % 2 === 0 ? "top-0" : "top-4";

                return (
                  <button
                    key={cap.id}
                    type="button"
                    onClick={() => setSelected(cap)}
                    style={{ left: `${leftPct}%` }}
                    className={cn(
                      "group absolute -translate-x-1/2 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary",
                      staggerTop,
                    )}
                    title={`${cap.catcher.displayName}: ${cap.promptText ?? "Foto"} (${new Date(cap.capturedAt).toLocaleTimeString("de-AT")})`}
                    aria-label={`Foto von ${cap.catcher.displayName} um ${new Date(cap.capturedAt).toLocaleTimeString("de-AT")}`}
                  >
                    <span className="flex size-11 items-center justify-center">
                      <span className="size-8 overflow-hidden rounded-full border-2 border-primary bg-elevated transition-transform group-hover:scale-110">
                        <AuthenticatedImage
                          storageKey={cap.thumbnailStorageKey}
                          alt={cap.promptText ?? "Capture"}
                          className="h-full w-full"
                        />
                      </span>
                    </span>
                  </button>
                );
              })}
            </div>
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-2 text-xs text-secondary">
          <Camera className="size-3.5 text-primary" aria-hidden="true" />
          <span>{captures.length} erfasste Momente im 1:1 Zeitverlauf</span>
        </div>

        <PhotoLightbox
          capture={selected}
          open={Boolean(selected)}
          onOpenChange={(open) => {
            if (!open) setSelected(null);
          }}
        />
      </CardContent>
    </Card>
  );
}
