"use client";

import { Award, Camera, Trophy } from "lucide-react";
import Link from "next/link";
import { useState } from "react";

import type { CaptureFeedItem, GalleryGameGroup } from "@/api/friendarazzi-api";
import { Badge } from "@/components/ui/badge";
import { MarginNote } from "@/components/ui/paper-rule";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { gameHref } from "@/lib/navigation";

import { AuthenticatedImage } from "./media-image";
import { PhotoLightbox } from "./photo-lightbox";

/**
 * Native phone-style photo tabs ("Erwischt" / "Geschossen") grouped under sticky
 * game section headers.
 */
export function UserGalleryTabs({
  caught,
  captured,
}: {
  caught: GalleryGameGroup[];
  captured: GalleryGameGroup[];
}) {
  const [selected, setSelected] = useState<CaptureFeedItem | null>(null);

  return (
    <>
      <Tabs defaultValue="caught">
        <TabsList>
          <TabsTrigger value="caught">
            Erwischt ({caught.reduce((sum, g) => sum + g.photos.length, 0)})
          </TabsTrigger>
          <TabsTrigger value="captured">
            Geschossen ({captured.reduce((sum, g) => sum + g.photos.length, 0)})
          </TabsTrigger>
        </TabsList>

        <TabsContent value="caught" className="space-y-6 pt-2">
          <GalleryGroups
            groups={caught}
            emptyMessage="Noch keine Fotos, auf denen diese Person erwischt wurde."
            onSelect={setSelected}
          />
        </TabsContent>

        <TabsContent value="captured" className="space-y-6 pt-2">
          <GalleryGroups
            groups={captured}
            emptyMessage="Noch keine selbst geschossenen Beweisfotos."
            onSelect={setSelected}
          />
        </TabsContent>
      </Tabs>

      <PhotoLightbox
        capture={selected}
        open={Boolean(selected)}
        onOpenChange={(open) => {
          if (!open) setSelected(null);
        }}
      />
    </>
  );
}

function GalleryGroups({
  groups,
  emptyMessage,
  onSelect,
}: {
  groups: GalleryGameGroup[];
  emptyMessage: string;
  onSelect: (item: CaptureFeedItem) => void;
}) {
  if (groups.length === 0) {
    return (
      <MarginNote label="Galerie leer">
        <span className="inline-flex items-center gap-1.5">
          <Camera className="size-4 text-secondary" aria-hidden="true" />
          {emptyMessage}
        </span>
      </MarginNote>
    );
  }

  return (
    <div className="space-y-6">
      {groups.map((group) => (
        <section key={group.gameId} className="space-y-3">
          {/* Sticky mobile-gallery-style game section header */}
          <div className="sticky top-0 z-10 flex flex-wrap items-center justify-between gap-2 border-b border-subtle bg-canvas/95 py-2">
            <div className="min-w-0">
              <Link
                href={gameHref(group.gameId)}
                className="inline-flex min-h-11 items-center font-serif text-base font-semibold text-ink hover:underline"
              >
                {group.gameTitle}
              </Link>
              <p className="text-xs text-secondary tabular-nums">
                {group.endedAt
                  ? new Date(group.endedAt).toLocaleDateString("de-AT")
                  : group.startedAt
                    ? new Date(group.startedAt).toLocaleDateString("de-AT")
                    : "Aktive Runde"}
              </p>
            </div>
            {group.winner ? (
              <Badge variant="warning" className="gap-1">
                <Trophy className="size-3 text-warning" aria-hidden="true" />
                {group.winner.displayName}
                <Award className="size-3 text-warning" aria-hidden="true" />
              </Badge>
            ) : null}
          </div>

          <div className="grid grid-cols-2 gap-2 sm:grid-cols-3 md:grid-cols-4">
            {group.photos.map((photo) => (
              <button
                key={photo.id}
                type="button"
                onClick={() => onSelect(photo)}
                className="group relative aspect-square overflow-hidden rounded-md border border-subtle bg-surface-muted text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
              >
                <AuthenticatedImage
                  storageKey={
                    photo.imageStorageKey ?? photo.thumbnailStorageKey
                  }
                  alt={photo.promptText ?? "Beweisfoto"}
                  className="h-full w-full transition-transform group-hover:scale-105"
                />
                <span className="absolute inset-x-0 bottom-0 bg-ink/75 px-2 py-1 text-[11px] leading-tight text-surface">
                  <span className="line-clamp-1 font-medium">
                    {photo.promptText}
                  </span>
                </span>
              </button>
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}
