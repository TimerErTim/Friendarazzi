"use client";

import { Award, Camera, Upload } from "lucide-react";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";

import { useAuth } from "@/api/auth-provider";
import { apiErrorMessage } from "@/api/errors";
import {
  useBenutzerQuery,
  useErfolgsmeldung,
  useFriendarazziMutations,
  useUserGalleryQuery,
} from "@/api/hooks";
import { PlayerAvatar } from "@/components/friendarazzi/player-avatar";
import { UserGalleryTabs } from "@/components/friendarazzi/user-gallery-tabs";
import { PageHeader } from "@/components/layout/page-header";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { StickerBand } from "@/components/ui/brutal-elements";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Field } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";

export default function ProfilePage() {
  const phase = useAuth();
  const router = useRouter();
  const benutzer = useBenutzerQuery();
  const gallery = useUserGalleryQuery(benutzer.data?.id ?? null);
  const mutations = useFriendarazziMutations();
  const feedback = useErfolgsmeldung();

  const [displayName, setDisplayName] = useState("");
  const [equippedTitle, setEquippedTitle] = useState("");

  useEffect(() => {
    if (phase.status === "anonymous") router.replace("/anmeldung");
  }, [phase.status, router]);

  useEffect(() => {
    const profile = gallery.data?.user;
    if (!profile) return;
    setDisplayName(profile.displayName);
    setEquippedTitle(profile.equippedTitle ?? "");
  }, [gallery.data?.user]);

  const profile = gallery.data?.user;

  async function saveProfile(e: React.FormEvent) {
    e.preventDefault();
    feedback.verwerfen();
    await mutations.updateProfile.mutateAsync({
      displayName,
      equippedTitle: equippedTitle || undefined,
    });
    feedback.melden("Profil gespeichert.");
  }

  async function handleAvatarChange(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    feedback.verwerfen();
    await mutations.uploadAvatar.mutateAsync(file);
    feedback.melden("Profilbild aktualisiert.");
  }

  return (
    <div className="mx-auto w-full max-w-4xl space-y-6 p-4 sm:p-6">
      <PageHeader
        register={["Konto", "Profil"]}
        title="Mein Profil"
        description="Deine Goldmedaillen, freigeschalteten Titel und deine gesammelten Fotomomente."
      />

      {gallery.isLoading || !gallery.data || !profile ? (
        <div className="space-y-4">
          <Skeleton className="h-36 w-full" />
          <Skeleton className="h-56 w-full" />
        </div>
      ) : (
        <>
          <Card className="bg-primary-subtle">
            <CardContent className="flex flex-wrap items-center justify-between gap-6 pt-6">
              <div className="flex items-center gap-4">
                <PlayerAvatar user={profile} size="lg" linkToProfile={false} />
                <div className="space-y-1.5">
                  <div className="flex flex-wrap items-center gap-2">
                    <h2 className="font-display text-xl tracking-tight text-ink">
                      {profile.displayName}
                    </h2>
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

              <label className="brutal-press inline-flex min-h-11 cursor-pointer items-center gap-2 rounded-sm border-2 border-control bg-surface px-4 text-sm font-bold text-ink hover:bg-surface-muted">
                <Camera className="size-4" aria-hidden="true" />
                {mutations.uploadAvatar.isPending
                  ? "Lädt hoch …"
                  : "Profilbild ändern"}
                <input
                  type="file"
                  accept="image/*"
                  className="sr-only"
                  disabled={mutations.uploadAvatar.isPending}
                  onChange={(e) => void handleAvatarChange(e)}
                />
              </label>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Profil bearbeiten</CardTitle>
            </CardHeader>
            <CardContent>
              <form className="space-y-4" onSubmit={(e) => void saveProfile(e)}>
                <Field
                  label="Anzeigename"
                  hint="So sehen dich deine Freunde in Lobbys."
                >
                  <Input
                    value={displayName}
                    onChange={(e) => setDisplayName(e.target.value)}
                    required
                  />
                </Field>

                <Field
                  label="Angelegter Rang-Titel"
                  hint="Weitere Titel schaltest du durch Goldmedaillen frei."
                >
                  <Select
                    value={equippedTitle}
                    onChange={(e) => setEquippedTitle(e.target.value)}
                  >
                    <option value="">
                      Automatisch ({profile.effectiveTitle})
                    </option>
                    {profile.unlockedTitles.map((t) => (
                      <option key={t} value={t}>
                        {t}
                      </option>
                    ))}
                  </Select>
                </Field>

                {mutations.updateProfile.isError ? (
                  <Alert variant="error">
                    {apiErrorMessage(mutations.updateProfile.error, {
                      fallback: "Profil konnte nicht gespeichert werden.",
                    })}
                  </Alert>
                ) : null}

                {mutations.uploadAvatar.isError ? (
                  <Alert variant="error">
                    {apiErrorMessage(mutations.uploadAvatar.error, {
                      fallback: "Profilbild konnte nicht hochgeladen werden.",
                    })}
                  </Alert>
                ) : null}

                <Button
                  type="submit"
                  disabled={mutations.updateProfile.isPending}
                >
                  <Upload className="size-4" aria-hidden="true" />
                  {mutations.updateProfile.isPending
                    ? "Wird gespeichert …"
                    : "Änderungen speichern"}
                </Button>
              </form>
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
        </>
      )}
    </div>
  );
}
