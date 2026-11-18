"use client";

import { Award } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect } from "react";

import { useAuth } from "@/api/auth-provider";
import { apiErrorMessage } from "@/api/errors";
import { useUserGalleryQuery } from "@/api/hooks";
import { PlayerAvatar } from "@/components/friendarazzi/player-avatar";
import { UserGalleryTabs } from "@/components/friendarazzi/user-gallery-tabs";
import { SetBreadcrumbLabel } from "@/components/layout/page-chrome";
import { PageHeader } from "@/components/layout/page-header";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { CalloutNote, StickerBand } from "@/components/ui/brutal-elements";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { paths } from "@/lib/navigation";
import { useQueryParam } from "@/lib/use-query-param";

export default function UserProfilePage() {
  const phase = useAuth();
  const router = useRouter();
  const id = useQueryParam("id");
  const gallery = useUserGalleryQuery(id);

  useEffect(() => {
    if (phase.status === "anonymous") router.replace("/anmeldung");
  }, [phase.status, router]);

  if (id === null || gallery.isLoading) {
    return (
      <div className="mx-auto w-full max-w-4xl space-y-6 p-4 sm:p-6">
        <Skeleton className="h-12 w-56" />
        <Skeleton className="h-32 w-full" />
        <Skeleton className="h-64 w-full" />
      </div>
    );
  }

  if (!id) {
    return (
      <div className="mx-auto w-full max-w-3xl space-y-4 p-4 sm:p-6">
        <CalloutNote label="Kein Spieler gewählt" tone="yellow">
          Wähle einen Spieler über dessen Avatar aus.
        </CalloutNote>
        <Button asChild>
          <Link href={paths.home}>Zur Übersicht</Link>
        </Button>
      </div>
    );
  }

  if (gallery.isError || !gallery.data) {
    return (
      <div className="mx-auto w-full max-w-3xl space-y-4 p-4 sm:p-6">
        <Alert variant="error">
          {apiErrorMessage(gallery.error, {
            fallback: "Dieses Spielerprofil konnte nicht geladen werden.",
          })}
        </Alert>
      </div>
    );
  }

  const profile = gallery.data.user;

  return (
    <div className="mx-auto w-full max-w-4xl space-y-6 p-4 sm:p-6">
      <SetBreadcrumbLabel label={profile.displayName} />
      <PageHeader
        register={["Spieler", "Profil"]}
        title={profile.displayName}
        description={profile.effectiveTitle}
      />

      <Card className="bg-cyan-subtle">
        <CardContent className="flex flex-wrap items-center justify-between gap-4 pt-6">
          <div className="flex items-center gap-4">
            <PlayerAvatar user={profile} size="lg" linkToProfile={false} />
            <div className="space-y-1.5">
              <div className="flex flex-wrap items-center gap-2">
                <Badge variant="primary" className="gap-1">
                  <Award className="size-3.5" aria-hidden="true" />
                  {profile.goldMedalCount} Goldmedaillen
                </Badge>
                <Badge variant="info">{profile.effectiveTitle}</Badge>
              </div>
              <p className="font-mono text-xs font-semibold text-secondary tabular-nums">
                {profile.gamesPlayed} Spiele · {profile.capturesMade} Fotos
                geschossen · {profile.timesCaught}× erwischt
              </p>
            </div>
          </div>
        </CardContent>
      </Card>

      <StickerBand checker />

      <section className="space-y-3">
        <h2 className="font-display text-lg tracking-tight text-ink">
          Fotomomente
        </h2>
        <UserGalleryTabs
          caught={gallery.data.caught}
          captured={gallery.data.captured}
        />
      </section>
    </div>
  );
}
