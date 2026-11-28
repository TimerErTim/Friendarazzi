"use client";

import { Bell } from "lucide-react";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";

import { useAuth } from "@/api/auth-provider";
import { apiErrorMessage } from "@/api/errors";
import {
  useAuthMutations,
  useErfolgsmeldung,
  useFriendarazziMutations,
  useNotificationPreferencesQuery,
} from "@/api/hooks";
import { Alert } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Field } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";

export default function OptionsPage() {
  const phase = useAuth();
  const auth = useAuthMutations();
  const prefs = useNotificationPreferencesQuery();
  const mutations = useFriendarazziMutations();
  const feedback = useErfolgsmeldung();
  const router = useRouter();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [password, setPassword] = useState("");

  useEffect(() => {
    if (phase.status === "anonymous") router.replace("/anmeldung");
  }, [phase.status, router]);

  const message = apiErrorMessage(auth.deleteAccount.error, {
    unauthorized: "Das Passwort ist falsch oder die Sitzung ist abgelaufen.",
    badRequest: "Das Passwort muss 8 bis 128 Zeichen lang sein.",
    fallback:
      "Das Konto konnte nicht gelöscht werden. Bitte versuche es später erneut.",
  });

  function deleteAccount() {
    auth.deleteAccount.mutate(
      { passwort: password, bestaetigt: true },
      {
        onSuccess: () => {
          setDialogOpen(false);
          setPassword("");
          router.replace("/");
        },
      },
    );
  }

  function updatePref(
    patch: Parameters<typeof mutations.updateNotificationPreferences.mutate>[0],
  ) {
    feedback.verwerfen();
    mutations.updateNotificationPreferences.mutate(patch, {
      onSuccess: () => {
        feedback.melden("Benachrichtigungs-Einstellung gespeichert.");
      },
    });
  }

  return (
    <div className="mx-auto w-full max-w-2xl space-y-6 p-4 sm:p-6">
      <div>
        <h1 className="font-display text-2xl tracking-tight text-ink sm:text-3xl">
          Konto-Optionen
        </h1>
        <p className="text-sm font-medium text-secondary">
          Verwalte Benachrichtigungen, Standard-Abos und deinen Account.
        </p>
      </div>

      <Card>
        <CardHeader className="bg-cyan-subtle">
          <CardTitle className="flex items-center gap-2">
            <Bell className="size-5" aria-hidden="true" />
            Benachrichtigungen
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-5 pt-5">
          {phase.status === "unavailable" ? (
            <Alert variant="error">
              Der Server ist derzeit nicht erreichbar.
            </Alert>
          ) : prefs.isError ? (
            <Alert variant="error">
              {apiErrorMessage(prefs.error, {
                fallback:
                  "Benachrichtigungs-Einstellungen konnten nicht geladen werden.",
              })}
            </Alert>
          ) : prefs.isLoading || !prefs.data ? (
            <div className="space-y-3">
              <Skeleton className="h-12 w-full" />
              <Skeleton className="h-12 w-full" />
              <Skeleton className="h-12 w-full" />
            </div>
          ) : (
            <>
              <div className="flex items-center justify-between gap-4 rounded-sm border-2 border-strong bg-surface-muted p-4">
                <div className="space-y-0.5">
                  <label
                    htmlFor="pref-default-watch"
                    className="block text-sm font-bold text-ink"
                  >
                    Benachrichtigungen für neue Spiele automatisch aktivieren
                  </label>
                  <p className="text-xs font-medium text-secondary">
                    Beim Erstellen oder Beitreten eines Spiels aktiv (pro Spiel
                    änderbar).
                  </p>
                </div>
                <Switch
                  id="pref-default-watch"
                  checked={prefs.data.defaultWatchGames}
                  disabled={mutations.updateNotificationPreferences.isPending}
                  onCheckedChange={(checked) =>
                    updatePref({ defaultWatchGames: checked })
                  }
                />
              </div>

              <div className="space-y-3 border-t-2 border-subtle pt-4">
                <h2 className="font-mono text-xs font-bold tracking-wider text-secondary uppercase">
                  E-Mail-Benachrichtigungen
                </h2>

                <div className="flex items-center justify-between gap-4 py-1.5">
                  <div className="space-y-0.5">
                    <label
                      htmlFor="pref-email-started"
                      className="block text-sm font-bold text-ink"
                    >
                      Spielstart (Entwurfsphase beendet)
                    </label>
                    <p className="text-xs font-medium text-secondary">
                      Wenn ein Spiel mit aktivierten Benachrichtigungen startet.
                    </p>
                  </div>
                  <Switch
                    id="pref-email-started"
                    checked={prefs.data.emailGameStarted}
                    disabled={mutations.updateNotificationPreferences.isPending}
                    onCheckedChange={(checked) =>
                      updatePref({ emailGameStarted: checked })
                    }
                  />
                </div>

                <div className="flex items-center justify-between gap-4 border-t border-subtle py-2.5">
                  <div className="space-y-0.5">
                    <label
                      htmlFor="pref-email-capture"
                      className="block text-sm font-bold text-ink"
                    >
                      Neue Beweisfotos
                    </label>
                    <p className="text-xs font-medium text-secondary">
                      Wenn in einem Spiel mit aktivierten Benachrichtigungen ein
                      Beweisfoto hochgeladen wird.
                    </p>
                  </div>
                  <Switch
                    id="pref-email-capture"
                    checked={prefs.data.emailCapturePosted}
                    disabled={mutations.updateNotificationPreferences.isPending}
                    onCheckedChange={(checked) =>
                      updatePref({ emailCapturePosted: checked })
                    }
                  />
                </div>

                <div className="flex items-center justify-between gap-4 border-t border-subtle pt-2.5">
                  <div className="space-y-0.5">
                    <label
                      htmlFor="pref-email-completed"
                      className="block text-sm font-bold text-ink"
                    >
                      Spielende & Bingo-Sieg
                    </label>
                    <p className="text-xs font-medium text-secondary">
                      Wenn jemand ein Spiel mit aktivierten Benachrichtigungen
                      gewinnt.
                    </p>
                  </div>
                  <Switch
                    id="pref-email-completed"
                    checked={prefs.data.emailGameCompleted}
                    disabled={mutations.updateNotificationPreferences.isPending}
                    onCheckedChange={(checked) =>
                      updatePref({ emailGameCompleted: checked })
                    }
                  />
                </div>
              </div>

              {mutations.updateNotificationPreferences.isError ? (
                <Alert variant="error">
                  {apiErrorMessage(
                    mutations.updateNotificationPreferences.error,
                    {
                      fallback:
                        "Einstellungen konnten nicht gespeichert werden.",
                    },
                  )}
                </Alert>
              ) : null}
            </>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="bg-impact/10">
          <CardTitle>Gefahrenzone</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4 pt-5">
          <div className="space-y-1">
            <h2 className="text-sm font-bold text-ink">Konto löschen</h2>
            <p className="text-sm font-medium text-secondary">
              Dein Konto, dein Profil und deine persönlichen Daten werden
              entfernt. Dieser Vorgang kann nicht rückgängig gemacht werden.
            </p>
          </div>
          {message ? (
            <Alert variant="error">
              <p>{message}</p>
            </Alert>
          ) : null}
          <Button
            variant="destructive"
            onClick={() => {
              auth.deleteAccount.reset();
              setPassword("");
              setDialogOpen(true);
            }}
            disabled={phase.status !== "authenticated"}
          >
            Konto löschen
          </Button>
        </CardContent>
      </Card>

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Konto endgültig löschen?</DialogTitle>
            <DialogDescription>
              Gib zur Bestätigung dein Passwort ein. Deine Sitzungen enden
              sofort; diese Aktion kann nicht rückgängig gemacht werden.
            </DialogDescription>
          </DialogHeader>
          <Field
            label="Passwort"
            hint="Gib zur Bestätigung dein aktuelles Passwort ein."
          >
            <Input
              type="password"
              autoComplete="current-password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              minLength={8}
              maxLength={128}
              required
            />
          </Field>
          {auth.deleteAccount.error ? (
            <Alert variant="error">
              <p>{message}</p>
            </Alert>
          ) : null}
          <DialogFooter>
            <Button
              variant="secondary"
              onClick={() => {
                setDialogOpen(false);
                auth.deleteAccount.reset();
              }}
              disabled={auth.deleteAccount.isPending}
            >
              Abbrechen
            </Button>
            <Button
              variant="destructive"
              onClick={deleteAccount}
              disabled={!password || auth.deleteAccount.isPending}
            >
              {auth.deleteAccount.isPending
                ? "Wird gelöscht …"
                : "Endgültig löschen"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
