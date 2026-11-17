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
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { HatchBand, MarginNote } from "@/components/ui/paper-rule";
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
      <div className="mx-auto w-full max-w-4xl space-y-6 p-6">
        <Skeleton className="h-10 w-56" />
        <Skeleton className="h-32 w-full" />
        <Skeleton className="h-64 w-full" />
      </div>
    );
  }

  if (!id) {
    return (
      <div className="mx-auto w-full max-w-3xl space-y-4 p-6">
        <MarginNote label="Kein Spieler gewählt">
          Wähle einen Spieler über dessen Avatar aus.
        </MarginNote>
        <Button asChild>
          <Link href={paths.home}>Zur Übersicht</Link>
        </Button>
      </div>
    );
  }

  if (gallery.isError || !gallery.data) {
    return (
      <div className="mx-auto w-full max-w-3xl space-y-4 p-6">
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
    <div className="mx-auto w-full max-w-4xl space-y-6 p-6">
      <SetBreadcrumbLabel label={profile.displayName} />
      <PageHeader
        register={["Spieler", "Profil"]}
        title={profile.displayName}
        description={`@${profile.username} · ${profile.effectiveTitle}`}
      />

      <Card>
        <CardContent className="flex flex-wrap items-center justify-between gap-4 pt-6">
          <div className="flex items-center gap-4">
            <PlayerAvatar user={profile} size="lg" linkToProfile={false} />
            <div className="space-y-1">
              <div className="flex flex-wrap items-center gap-2">
                <Badge variant="warning" className="gap-1">
                  <Award className="size-3.5 text-warning" aria-hidden="true" />
                  {profile.goldMedalCount} Goldmedaillen
                </Badge>
                <Badge variant="primary">{profile.effectiveTitle}</Badge>
              </div>
              <p className="text-sm text-secondary tabular-nums">
                {profile.gamesPlayed} Spiele · {profile.capturesMade} Fotos
                geschossen · {profile.timesCaught}× erwischt
              </p>
            </div>
          </div>
        </CardContent>
      </Card>

      <HatchBand />

      <section className="space-y-3">
        <h2 className="font-serif text-lg font-semibold text-ink">
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
