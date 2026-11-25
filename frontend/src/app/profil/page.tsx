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
import { normalizeImageForUpload } from "@/lib/media-capture";

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
    const normalized = await normalizeImageForUpload(file);
    await mutations.uploadAvatar.mutateAsync(normalized);
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
            <div className="flex flex-wrap items-center gap-4 border-b-2 border-strong bg-primary p-6 text-primary-foreground">
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
                  <Badge variant="info">{profile.effectiveTitle}</Badge>
                </div>
                <p className="font-mono text-xs font-bold">
                  {profile.unlockedTitles.length}{" "}
                  {profile.unlockedTitles.length === 1
                    ? "Rang-Titel freigeschaltet"
                    : "Rang-Titel freigeschaltet"}
                </p>
              </div>
            </div>

            {/* 4 Stat Tiles */}
            <div className="grid grid-cols-2 divide-x-2 divide-y-2 divide-strong bg-surface sm:grid-cols-4 sm:divide-y-0">
              <div className="bg-cyan-subtle p-4 text-center">
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

          {/* Galleries & Grouped Profile Editor */}
          <Tabs defaultValue="moments">
            <TabsList>
              <TabsTrigger value="moments">Fotomomente</TabsTrigger>
              <TabsTrigger value="settings">Profil bearbeiten</TabsTrigger>
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
                <CardContent className="space-y-6 pt-5">
                  <div className="flex flex-wrap items-center justify-between gap-4 rounded-sm border-2 border-strong bg-surface-muted p-4">
                    <div className="flex items-center gap-3">
                      <PlayerAvatar
                        user={profile}
                        size="md"
                        linkToProfile={false}
                        showMedalBadge={false}
                      />
                      <div>
                        <p className="text-sm font-bold text-ink">Profilbild</p>
                        <p className="text-xs font-medium text-secondary">
                          Wird in Lobbys, Brettern und Zeitachsen angezeigt.
                        </p>
                      </div>
                    </div>
                    <label className="brutal-press inline-flex min-h-11 cursor-pointer items-center gap-2 rounded-sm border-2 border-control bg-surface px-4 text-sm font-bold text-ink hover:bg-canvas">
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

                  {mutations.uploadAvatar.isError ? (
                    <Alert variant="error">
                      {apiErrorMessage(mutations.uploadAvatar.error, {
                        fallback: "Profilbild konnte nicht hochgeladen werden.",
                      })}
                    </Alert>
                  ) : null}

                  <form
                    className="space-y-4 border-t-2 border-subtle pt-5"
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
