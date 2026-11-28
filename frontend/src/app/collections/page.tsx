"use client";

import { Award, Camera, Sparkles, Trophy } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect, useMemo, useState } from "react";

import { useAuth } from "@/api/auth-provider";
import {
  type CaptureFeedItem,
  type GalleryGameGroup,
  GameStatus,
  VisibilityMode,
} from "@/api/friendarazzi-api";
import {
  useBenutzerQuery,
  useGameDetailsQueries,
  useGamesQuery,
  useUserGalleryQuery,
} from "@/api/hooks";
import { CaptureGalleryCard } from "@/components/friendarazzi/capture-gallery-card";
import { PhotoLightbox } from "@/components/friendarazzi/photo-lightbox";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { CalloutNote } from "@/components/ui/brutal-elements";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Select } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { authHrefWithRedirect, gameHref, paths } from "@/lib/navigation";

type RoleFilter = "all" | "captured" | "caught" | "others";
type GroupMode = "none" | "game" | "person";

type CollectionItem = {
  capture: CaptureFeedItem;
  gameId: string;
  gameTitle: string;
  isCaught: boolean;
  isCaptured: boolean;
  personId: string;
  personName: string;
};

function compareCaptureNewestFirst(
  a: CollectionItem,
  b: CollectionItem,
): number {
  const diff =
    Date.parse(b.capture.capturedAt) - Date.parse(a.capture.capturedAt);
  if (diff !== 0 && !Number.isNaN(diff)) return diff;
  return b.capture.id.localeCompare(a.capture.id);
}

export default function CollectionsPage() {
  const phase = useAuth();
  const router = useRouter();
  const benutzer = useBenutzerQuery();
  const gallery = useUserGalleryQuery(benutzer.data?.id ?? null);
  const games = useGamesQuery();
  const [selected, setSelected] = useState<CaptureFeedItem | null>(null);
  const [roleFilter, setRoleFilter] = useState<RoleFilter>("all");
  const [groupMode, setGroupMode] = useState<GroupMode>("none");

  useEffect(() => {
    if (phase.status === "anonymous") {
      router.replace(authHrefWithRedirect("/anmeldung", paths.collections));
    }
  }, [phase.status, router]);

  const myId = benutzer.data?.id;
  const galleryData = gallery.data;

  const accessibleGameIds = useMemo(() => {
    const list = games.data ?? [];
    return list
      .filter(
        (g) =>
          g.status === GameStatus.Completed ||
          (g.status === GameStatus.Active &&
            g.visibilityMode === VisibilityMode.LiveFeed),
      )
      .map((g) => g.id);
  }, [games.data]);

  const gameDetailsQueries = useGameDetailsQueries(accessibleGameIds);
  const loadedDetails = gameDetailsQueries.map((q) => q.data);

  const allItems = useMemo(() => {
    const byId = new Map<string, CollectionItem>();

    function upsertPhoto(
      photo: CaptureFeedItem,
      gameId: string,
      gameTitle: string,
      opts: { isCaught?: boolean; isCaptured?: boolean },
    ) {
      if (photo.isBlurred) return;
      const isMeCatcher = opts.isCaptured ?? photo.catcher.id === myId;
      const isMeTarget = opts.isCaught ?? photo.target?.id === myId;

      const existing = byId.get(photo.id);
      if (existing) {
        if (isMeCaughtOrCaptured(isMeTarget)) existing.isCaught = true;
        if (isMeCaughtOrCaptured(isMeCatcher)) existing.isCaptured = true;
        return;
      }

      const person =
        isMeTarget && !isMeCatcher
          ? photo.catcher
          : (photo.target ?? photo.catcher);

      byId.set(photo.id, {
        capture: photo,
        gameId,
        gameTitle,
        isCaught: isMeTarget,
        isCaptured: isMeCatcher,
        personId: person.id,
        personName: person.displayName,
      });
    }

    function addGalleryGroups(
      groups: readonly GalleryGameGroup[],
      kind: "caught" | "captured",
    ) {
      for (const group of groups) {
        for (const photo of group.photos) {
          upsertPhoto(photo, group.gameId, group.gameTitle, {
            isCaught: kind === "caught",
            isCaptured: kind === "captured",
          });
        }
      }
    }

    if (galleryData) {
      addGalleryGroups(galleryData.captured, "captured");
      addGalleryGroups(galleryData.caught, "caught");
    }

    for (const detail of loadedDetails) {
      if (!detail) continue;
      for (const photo of detail.feed) {
        upsertPhoto(photo, detail.id, detail.title, {});
      }
    }

    return Array.from(byId.values()).sort(compareCaptureNewestFirst);
  }, [galleryData, loadedDetails, myId]);

  const filteredItems = useMemo(() => {
    switch (roleFilter) {
      case "all":
        return allItems;
      case "captured":
        return allItems.filter((item) => item.isCaptured);
      case "caught":
        return allItems.filter((item) => item.isCaught);
      case "others":
        return allItems.filter((item) => !item.isCaptured && !item.isCaught);
    }
  }, [allItems, roleFilter]);

  const groupedItems = useMemo(() => {
    if (groupMode === "none") return null;
    const map = new Map<
      string,
      { key: string; label: string; href?: string; items: CollectionItem[] }
    >();

    for (const item of filteredItems) {
      const key = groupMode === "game" ? item.gameId : item.personId;
      const label = groupMode === "game" ? item.gameTitle : item.personName;
      let bucket = map.get(key);
      if (!bucket) {
        bucket = {
          key,
          label,
          href: groupMode === "game" ? gameHref(item.gameId) : undefined,
          items: [],
        };
        map.set(key, bucket);
      }
      bucket.items.push(item);
    }

    return Array.from(map.values()).map((bucket) => ({
      ...bucket,
      items: bucket.items.sort(compareCaptureNewestFirst),
    }));
  }, [filteredItems, groupMode]);

  if (phase.status === "unavailable") {
    return (
      <div className="mx-auto w-full max-w-5xl p-4 sm:p-6">
        <Alert variant="error">Der Server ist derzeit nicht erreichbar.</Alert>
      </div>
    );
  }

  if (phase.status !== "authenticated" || gallery.isLoading || !gallery.data) {
    return (
      <div className="mx-auto w-full max-w-5xl space-y-4 p-4 sm:p-6">
        <Skeleton className="h-14 w-64" />
        <Skeleton className="h-64 w-full" />
      </div>
    );
  }

  const profile = gallery.data.user;

  return (
    <div className="mx-auto w-full max-w-5xl space-y-5 p-4 sm:p-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="font-display text-2xl tracking-tight text-ink sm:text-3xl">
            Collections
          </h1>
          <p className="text-sm font-medium text-secondary">
            Alle zugänglichen Schnappschüsse, Ränge und kommenden Erfolge.
          </p>
        </div>
        <div className="flex items-center gap-2 font-mono text-xs font-bold text-ink">
          <span className="inline-flex items-center gap-1.5 rounded-sm border-2 border-strong bg-surface px-2.5 py-1.5 tabular-nums">
            <Camera className="size-3.5" aria-hidden="true" />
            {allItems.length} Fotos
          </span>
          <span className="inline-flex items-center gap-1.5 rounded-sm border-2 border-strong bg-primary px-2.5 py-1.5 text-primary-foreground tabular-nums">
            <Award className="size-3.5" aria-hidden="true" />
            {profile.goldMedalCount} Gold
          </span>
        </div>
      </div>

      <Tabs defaultValue="gallery">
        <TabsList>
          <TabsTrigger value="gallery">
            <Camera className="size-4" aria-hidden="true" />
            <span>Alle Fotos ({allItems.length})</span>
          </TabsTrigger>
          <TabsTrigger value="achievements">
            <Trophy className="size-4" aria-hidden="true" />
            <span>Erfolge &amp; Ränge</span>
          </TabsTrigger>
        </TabsList>

        <TabsContent value="gallery" className="space-y-4 pt-2">
          {allItems.length > 0 ? (
            <div className="flex flex-wrap items-center gap-2">
              <label htmlFor="collections-role" className="sr-only">
                Fotos filtern
              </label>
              <Select
                id="collections-role"
                aria-label="Fotos filtern"
                value={roleFilter}
                onChange={(e) => setRoleFilter(e.target.value as RoleFilter)}
                className="w-auto min-w-36"
              >
                <option value="all">Alle Fotos</option>
                <option value="captured">Selbst geschossen</option>
                <option value="caught">Selbst erwischt</option>
                <option value="others">Von Mitspielern</option>
              </Select>

              <label htmlFor="collections-group" className="sr-only">
                Fotos gruppieren
              </label>
              <Select
                id="collections-group"
                aria-label="Fotos gruppieren"
                value={groupMode}
                onChange={(e) => setGroupMode(e.target.value as GroupMode)}
                className="w-auto min-w-44"
              >
                <option value="none">Keine Gruppierung</option>
                <option value="game">Nach Spiel gruppieren</option>
                <option value="person">Nach Person gruppieren</option>
              </Select>
            </div>
          ) : null}

          {filteredItems.length === 0 ? (
            <div className="brutal-shadow-sm flex items-center gap-2 rounded-md border-2 border-strong bg-surface p-5 text-sm font-medium text-secondary">
              <Camera className="size-4 shrink-0 text-ink" aria-hidden="true" />
              <span>Noch keine passenden Beweisfotos vorhanden.</span>
            </div>
          ) : groupedItems ? (
            <div className="space-y-6">
              {groupedItems.map((group) => (
                <section key={group.key} className="space-y-3">
                  <div className="flex items-center justify-between border-b-2 border-strong pb-1.5">
                    {group.href ? (
                      <Link
                        href={group.href}
                        className="inline-flex min-h-11 items-center font-display text-base text-ink hover:underline"
                      >
                        {group.label}
                      </Link>
                    ) : (
                      <h2 className="font-display text-base text-ink">
                        {group.label}
                      </h2>
                    )}
                    <span className="font-mono text-xs font-bold text-secondary tabular-nums">
                      {group.items.length}
                    </span>
                  </div>
                  <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
                    {group.items.map((item) => (
                      <CaptureGalleryCard
                        key={item.capture.id}
                        capture={item.capture}
                        onSelect={setSelected}
                      />
                    ))}
                  </div>
                </section>
              ))}
            </div>
          ) : (
            <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
              {filteredItems.map((item) => (
                <CaptureGalleryCard
                  key={item.capture.id}
                  capture={item.capture}
                  onSelect={setSelected}
                />
              ))}
            </div>
          )}
        </TabsContent>

        <TabsContent value="achievements" className="space-y-4 pt-2">
          <Card>
            <CardHeader className="bg-primary-subtle">
              <CardTitle className="flex items-center gap-2">
                <Sparkles className="size-5" aria-hidden="true" />
                Freigeschaltete Rang-Titel ({profile.unlockedTitles.length})
              </CardTitle>
            </CardHeader>
            <CardContent className="flex flex-wrap gap-2 pt-4">
              {profile.unlockedTitles.map((title) => (
                <Badge
                  key={title}
                  variant={
                    title === profile.effectiveTitle ? "primary" : "outline"
                  }
                >
                  {title}
                </Badge>
              ))}
            </CardContent>
          </Card>

          <CalloutNote label="Bald verfügbar" tone="cyan">
            Erfolge, Meilensteine und Sammler-Abzeichen über alle Runden hinweg
            werden demnächst hier freigeschaltet.
          </CalloutNote>
        </TabsContent>
      </Tabs>

      <PhotoLightbox
        capture={selected}
        open={Boolean(selected)}
        onOpenChange={(open) => {
          if (!open) setSelected(null);
        }}
      />
    </div>
  );
}

function isMeCaughtOrCaptured(flag: boolean): boolean {
  return flag;
}
