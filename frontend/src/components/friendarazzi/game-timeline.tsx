"use client";

import { Clock } from "lucide-react";
import { useState } from "react";

import type { CaptureFeedItem } from "@/api/friendarazzi-api";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { cn } from "@/lib/cn";

import { AuthenticatedImage } from "./media-image";
import { PhotoLightbox } from "./photo-lightbox";

/**
 * 1:1 linear-scaled timeline proportional to total match duration (`startedAt` .. `endedAt`).
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
    return null;
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
      <CardHeader className="bg-cyan-subtle">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <CardTitle>1:1 Spiel-Zeitachse</CardTitle>
          <span className="inline-flex items-center gap-1.5 rounded-sm border-2 border-strong bg-surface px-2 py-0.5 font-mono text-xs font-bold text-ink tabular-nums">
            <Clock className="size-3.5" aria-hidden="true" />
            {formatDuration(totalMs)}
          </span>
        </div>
      </CardHeader>
      <CardContent className="pt-5">
        <div className="overflow-x-auto pb-2">
          <div className="relative min-w-[540px] pt-12 pb-8">
            <div className="brutal-grid absolute inset-x-4 top-6 h-12 rounded-sm border-2 border-strong bg-canvas" />
            <div className="absolute inset-x-4 top-12 h-0.5 -translate-y-1/2 bg-strong" />

            <div className="absolute top-0 left-4 font-mono text-[11px] font-bold text-ink tabular-nums">
              Start:{" "}
              {new Date(startMs).toLocaleTimeString("de-AT", {
                hour: "2-digit",
                minute: "2-digit",
              })}
            </div>
            <div className="absolute top-0 right-4 font-mono text-[11px] font-bold text-ink tabular-nums">
              Bingo:{" "}
              {new Date(endMs).toLocaleTimeString("de-AT", {
                hour: "2-digit",
                minute: "2-digit",
              })}
            </div>

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
                      "group absolute -translate-x-1/2 cursor-pointer focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus",
                      staggerTop,
                    )}
                    title={`${cap.catcher.displayName}: ${cap.promptText ?? "Foto"} (${new Date(cap.capturedAt).toLocaleTimeString("de-AT")})`}
                    aria-label={`Foto von ${cap.catcher.displayName} um ${new Date(cap.capturedAt).toLocaleTimeString("de-AT")}`}
                  >
                    <span className="flex size-11 items-center justify-center">
                      <span className="brutal-shadow-sm size-9 overflow-hidden rounded-full border-2 border-strong bg-surface transition-transform group-hover:scale-110">
                        <AuthenticatedImage
                          storageKey={
                            cap.imageStorageKey ?? cap.thumbnailStorageKey
                          }
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
