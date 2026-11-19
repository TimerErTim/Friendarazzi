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
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Field } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

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
    <div className="mx-auto w-full max-w-5xl space-y-6 p-4 sm:p-6">
      {gallery.isLoading || !gallery.data || !profile ? (
        <div className="space-y-4">
          <Skeleton className="h-44 w-full" />
          <Skeleton className="h-64 w-full" />
        </div>
      ) : (
        <>
          {/* Player Trading Card Hero */}
          <section className="brutal-shadow-lg overflow-hidden rounded-md border-[2.5px] border-strong bg-surface">
            <div className="flex flex-wrap items-center justify-between gap-6 border-b-2 border-strong bg-primary p-6 text-primary-foreground">
              <div className="flex items-center gap-4">
                <PlayerAvatar user={profile} size="lg" linkToProfile={false} />
                <div className="space-y-1.5">
                  <div className="flex flex-wrap items-center gap-2">
                    <h1 className="font-display text-2xl tracking-tight sm:text-3xl">
                      {profile.displayName}
                    </h1>
                    <Badge variant="outline" className="gap-1">
                      <Award className="size-3.5" aria-hidden="true" />
                      {profile.goldMedalCount} Gold
                    </Badge>
                    <Badge variant="info">{profile.effectiveTitle}</Badge>
                  </div>
                  <p className="font-mono text-xs font-bold">
                    Dein Friendarazzi-Paparazzi-Pass
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
            </div>

            {/* 3 Stat Tiles */}
            <div className="grid grid-cols-3 divide-x-2 divide-strong bg-surface">
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

          {mutations.uploadAvatar.isError ? (
            <Alert variant="error">
              {apiErrorMessage(mutations.uploadAvatar.error, {
                fallback: "Profilbild konnte nicht hochgeladen werden.",
              })}
            </Alert>
          ) : null}

          {/* Galleries First, Settings in Tab */}
          <Tabs defaultValue="moments">
            <TabsList>
              <TabsTrigger value="moments">Fotomomente</TabsTrigger>
              <TabsTrigger value="settings">
                Profil &amp; Titel anpassen
              </TabsTrigger>
            </TabsList>

            <TabsContent value="moments" className="pt-2">
              <UserGalleryTabs
                caught={gallery.data.caught}
                captured={gallery.data.captured}
              />
            </TabsContent>

            <TabsContent value="settings" className="pt-2">
              <Card>
                <CardHeader>
                  <CardTitle>Profil bearbeiten</CardTitle>
                </CardHeader>
                <CardContent className="pt-5">
                  <form
                    className="space-y-4"
                    onSubmit={(e) => void saveProfile(e)}
                  >
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
            </TabsContent>
          </Tabs>
        </>
      )}
    </div>
  );
}
