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
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { CalloutNote } from "@/components/ui/brutal-elements";
import { Button } from "@/components/ui/button";
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
      <div className="mx-auto w-full max-w-5xl space-y-6 p-4 sm:p-6">
        <Skeleton className="h-44 w-full" />
        <Skeleton className="h-64 w-full" />
      </div>
    );
  }

  if (!id) {
    return (
      <div className="mx-auto w-full max-w-2xl space-y-4 p-4 sm:p-6">
        <CalloutNote label="Kein Spieler gewählt" tone="yellow">
          Wähle einen Spieler über dessen Avatar aus.
        </CalloutNote>
        <Button asChild>
          <Link href={paths.home}>Zur Spielzentrale</Link>
        </Button>
      </div>
    );
  }

  if (gallery.isError || !gallery.data) {
    return (
      <div className="mx-auto w-full max-w-2xl space-y-4 p-4 sm:p-6">
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
    <div className="mx-auto w-full max-w-5xl space-y-6 p-4 sm:p-6">
      <SetBreadcrumbLabel label={profile.displayName} />

      <section className="brutal-shadow-lg overflow-hidden rounded-md border-[2.5px] border-strong bg-surface">
        <div className="flex flex-wrap items-center gap-4 border-b-2 border-strong bg-cyan p-6 text-cyan-foreground">
          <PlayerAvatar
            user={profile}
            size="lg"
            linkToProfile={false}
            showMedalBadge={false}
          />
          <div className="space-y-1.5">
            <div className="flex flex-wrap items-center gap-2">
              <h1 className="font-display text-2xl tracking-tight sm:text-3xl">
                {profile.displayName}
              </h1>
              <Badge variant="primary">{profile.effectiveTitle}</Badge>
            </div>
            <p className="font-mono text-xs font-bold">
              {profile.unlockedTitles.length}{" "}
              {profile.unlockedTitles.length === 1
                ? "Rang-Titel freigeschaltet"
                : "Rang-Titel freigeschaltet"}
            </p>
          </div>
        </div>

        <div className="grid grid-cols-2 divide-x-2 divide-y-2 divide-strong bg-surface sm:grid-cols-4 sm:divide-y-0">
          <div className="bg-primary-subtle p-4 text-center">
            <p className="inline-flex items-center justify-center gap-1.5 font-display text-2xl text-ink tabular-nums">
              <Award className="size-5 text-warning" aria-hidden="true" />
              {profile.goldMedalCount}
            </p>
            <p className="font-mono text-[11px] font-bold text-ink uppercase">
              Goldmedaillen
            </p>
          </div>
          <div className="p-4 text-center">
            <p className="font-display text-2xl text-ink tabular-nums">
              {profile.gamesPlayed}
            </p>
            <p className="font-mono text-[11px] font-bold text-secondary uppercase">
              Spiele
            </p>
          </div>
          <div className="p-4 text-center">
            <p className="font-display text-2xl text-ink tabular-nums">
              {profile.capturesMade}
            </p>
            <p className="font-mono text-[11px] font-bold text-secondary uppercase">
              Geschossen
            </p>
          </div>
          <div className="p-4 text-center">
            <p className="font-display text-2xl text-ink tabular-nums">
              {profile.timesCaught}
            </p>
            <p className="font-mono text-[11px] font-bold text-secondary uppercase">
              Erwischt
            </p>
          </div>
        </div>
      </section>

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
