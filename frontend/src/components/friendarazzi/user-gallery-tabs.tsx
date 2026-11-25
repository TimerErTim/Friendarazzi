"use client";

import { Award, Camera, Trophy } from "lucide-react";
import Link from "next/link";
import { useState } from "react";

import type { CaptureFeedItem, GalleryGameGroup } from "@/api/friendarazzi-api";
import { Badge } from "@/components/ui/badge";
import { CalloutNote } from "@/components/ui/brutal-elements";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { gameHref } from "@/lib/navigation";

import { CaptureGalleryCard } from "./capture-gallery-card";
import { PhotoLightbox } from "./photo-lightbox";

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
      <CalloutNote label="Galerie leer">
        <span className="inline-flex items-center gap-2">
          <Camera className="size-4 text-ink" aria-hidden="true" />
          {emptyMessage}
        </span>
      </CalloutNote>
    );
  }

  return (
    <div className="space-y-6">
      {groups.map((group) => (
        <section key={group.gameId} className="space-y-3">
          <div className="sticky top-0 z-10 flex flex-wrap items-center justify-between gap-2 border-b-2 border-strong bg-canvas/95 py-2">
            <div className="min-w-0">
              <Link
                href={gameHref(group.gameId)}
                className="inline-flex min-h-11 items-center font-display text-base text-ink hover:underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
              >
                {group.gameTitle}
              </Link>
              <p className="font-mono text-xs text-secondary tabular-nums">
                {group.endedAt
                  ? new Date(group.endedAt).toLocaleDateString("de-AT")
                  : group.startedAt
                    ? new Date(group.startedAt).toLocaleDateString("de-AT")
                    : "Aktive Runde"}
              </p>
            </div>
            {group.winner ? (
              <Badge variant="info" className="gap-1">
                <Trophy className="size-3" aria-hidden="true" />
                {group.winner.displayName}
                <Award className="size-3" aria-hidden="true" />
              </Badge>
            ) : null}
          </div>

          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
            {group.photos.map((photo) => (
              <CaptureGalleryCard
                key={photo.id}
                capture={photo}
                onSelect={onSelect}
              />
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}
