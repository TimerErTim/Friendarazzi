"use client";

import { Award, Camera, Sparkles, Trophy } from "lucide-react";
import { useRouter } from "next/navigation";
import { useEffect } from "react";

import { useAuth } from "@/api/auth-provider";
import { useBenutzerQuery, useUserGalleryQuery } from "@/api/hooks";
import { UserGalleryTabs } from "@/components/friendarazzi/user-gallery-tabs";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { CalloutNote } from "@/components/ui/brutal-elements";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { authHrefWithRedirect, paths } from "@/lib/navigation";

export default function CollectionsPage() {
  const phase = useAuth();
  const router = useRouter();
  const benutzer = useBenutzerQuery();
  const gallery = useUserGalleryQuery(benutzer.data?.id ?? null);

  useEffect(() => {
    if (phase.status === "anonymous") {
      router.replace(authHrefWithRedirect("/anmeldung", paths.collections));
    }
  }, [phase.status, router]);

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
  const totalPhotos =
    gallery.data.caught.reduce((sum, g) => sum + g.photos.length, 0) +
    gallery.data.captured.reduce((sum, g) => sum + g.photos.length, 0);

  return (
    <div className="mx-auto w-full max-w-5xl space-y-6 p-4 sm:p-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="font-display text-2xl tracking-tight text-ink sm:text-3xl">
            Collections
          </h1>
          <p className="text-sm font-medium text-secondary">
            Deine persönliche Fotogalerie, freigeschaltete Ränge und kommende
            Erfolge.
          </p>
        </div>
        <div className="flex items-center gap-2 font-mono text-xs font-bold text-ink">
          <span className="inline-flex items-center gap-1.5 rounded-sm border-2 border-strong bg-surface px-2.5 py-1.5 tabular-nums">
            <Camera className="size-3.5" aria-hidden="true" />
            {totalPhotos} Fotos
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
            <span>Galerie ({totalPhotos})</span>
          </TabsTrigger>
          <TabsTrigger value="achievements">
            <Trophy className="size-4" aria-hidden="true" />
            <span>Erfolge &amp; Ränge</span>
          </TabsTrigger>
        </TabsList>

        <TabsContent value="gallery" className="pt-2">
          <UserGalleryTabs
            caught={gallery.data.caught}
            captured={gallery.data.captured}
          />
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
    </div>
  );
}
