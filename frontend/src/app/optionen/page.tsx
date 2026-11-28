"use client";

import { Bell, KeyRound, Link2, ShieldAlert } from "lucide-react";
import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { type FormEvent, Suspense, useEffect, useRef, useState } from "react";

import { useAuth } from "@/api/auth-provider";
import { apiErrorMessage } from "@/api/errors";
import { OidcAccountActionMode } from "@/api/friendarazzi-api";
import {
  useAuthMutations,
  useErfolgsmeldung,
  useFriendarazziMutations,
  useLoginMethodsQuery,
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
import { PasswordInput } from "@/components/ui/password-input";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { paths } from "@/lib/navigation";

const PASSWORD_MIN = 8;
const PASSWORD_MAX = 128;

function oidcQueryErrorMessage(code: string): string {
  switch (code) {
    case "already_linked":
      return "Dieser Anbieter-Account ist bereits mit einem anderen Konto verknüpft.";
    case "reauth_mismatch":
      return "Die erneute Bestätigung passt nicht zu deinem angemeldeten Konto.";
    case "cancelled":
      return "Der Vorgang beim Anbieter wurde abgebrochen.";
    default:
      return "Der OIDC-Vorgang konnte nicht abgeschlossen werden. Bitte versuche es erneut.";
  }
}

function OptionsContent() {
  const phase = useAuth();
  const auth = useAuthMutations();
  const prefs = useNotificationPreferencesQuery();
  const loginMethods = useLoginMethodsQuery();
  const mutations = useFriendarazziMutations();
  const feedback = useErfolgsmeldung();
  const router = useRouter();
  const searchParams = useSearchParams();

  const [dialogOpen, setDialogOpen] = useState(false);
  const [password, setPassword] = useState("");
  const [reauthCode, setReauthCode] = useState<string | null>(null);
  const [oidcStatusError, setOidcStatusError] = useState<string | null>(null);

  const [passwordDialogOpen, setPasswordDialogOpen] = useState(false);
  const [newPassword, setNewPassword] = useState("");
  const [newPasswordConfirm, setNewPasswordConfirm] = useState("");
  const handledParamsRef = useRef<string | null>(null);

  const oidcLinkedParam = searchParams.get("oidc_linked");
  const oidcErrorParam = searchParams.get("oidc_error");
  const deleteReauthCodeParam = searchParams.get("delete_reauth_code");

  useEffect(() => {
    if (phase.status === "anonymous") router.replace("/anmeldung");
  }, [phase.status, router]);

  useEffect(() => {
    const key = [
      oidcLinkedParam ?? "",
      oidcErrorParam ?? "",
      deleteReauthCodeParam ?? "",
    ].join("|");
    if (key === "||" || handledParamsRef.current === key) return;
    handledParamsRef.current = key;

    if (oidcLinkedParam) {
      feedback.melden(`Anbieter „${oidcLinkedParam}“ erfolgreich verknüpft.`);
      setOidcStatusError(null);
    }
    if (oidcErrorParam) {
      setOidcStatusError(oidcQueryErrorMessage(oidcErrorParam));
    }
    if (deleteReauthCodeParam) {
      setReauthCode(deleteReauthCodeParam);
      setDialogOpen(true);
    }
    router.replace(paths.optionen);
  }, [
    deleteReauthCodeParam,
    feedback,
    oidcErrorParam,
    oidcLinkedParam,
    router,
  ]);

  const deleteErrorMessage = apiErrorMessage(auth.deleteAccount.error, {
    unauthorized: "Die Bestätigung ist ungültig oder das Passwort ist falsch.",
    badRequest: "Bitte bestätige die Löschung vollständig.",
    fallback:
      "Das Konto konnte nicht gelöscht werden. Bitte versuche es später erneut.",
  });

  const actionErrorMessage = apiErrorMessage(
    auth.unlinkExternalIdentity.error ?? auth.startOidcAccountAction.error,
    {
      conflict:
        "Die letzte verbleibende Anmeldemethode kann nicht entfernt werden.",
      tooManyRequests:
        "Zu viele Anfragen. Bitte warte kurz und versuche es erneut.",
      fallback: "Die Anmeldemethode konnte nicht aktualisiert werden.",
    },
  );

  const setPasswordErrorMessage = apiErrorMessage(
    auth.setInitialPassword.error,
    {
      conflict: "Für dieses Konto ist bereits ein Passwort festgelegt.",
      badRequest: "Das Passwort muss 8 bis 128 Zeichen lang sein.",
      fallback: "Das Passwort konnte nicht gespeichert werden.",
    },
  );

  const hasPassword = loginMethods.data?.hasPassword ?? true;
  const linkedIdentities = loginMethods.data?.externalIdentities ?? [];
  const availableProviders = loginMethods.data?.availableProviders ?? [];
  const linkedProviderIds = new Set(linkedIdentities.map((i) => i.providerId));
  const unlinkedProviders = availableProviders.filter(
    (p) => !linkedProviderIds.has(p.id),
  );
  const totalMethods = (hasPassword ? 1 : 0) + linkedIdentities.length;

  function startOidcAction(providerId: string, mode: OidcAccountActionMode) {
    setOidcStatusError(null);
    auth.startOidcAccountAction.mutate(
      { providerId, mode },
      {
        onSuccess: (authorizationUrl) => {
          window.location.assign(authorizationUrl);
        },
      },
    );
  }

  function submitInitialPassword(event: FormEvent) {
    event.preventDefault();
    if (newPassword !== newPasswordConfirm) return;
    auth.setInitialPassword.mutate(newPassword, {
      onSuccess: () => {
        setPasswordDialogOpen(false);
        setNewPassword("");
        setNewPasswordConfirm("");
        feedback.melden("Passwort erfolgreich festgelegt.");
      },
    });
  }

  function deleteAccount() {
    auth.deleteAccount.mutate(
      hasPassword
        ? { passwort: password, bestaetigt: true }
        : { reauthCode: reauthCode ?? undefined, bestaetigt: true },
      {
        onSuccess: () => {
          setDialogOpen(false);
          setPassword("");
          setReauthCode(null);
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

  const passwordMismatch =
    newPassword.length > 0 &&
    newPasswordConfirm.length > 0 &&
    newPassword !== newPasswordConfirm;

  return (
    <div className="mx-auto w-full max-w-2xl space-y-6 p-4 sm:p-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="font-display text-2xl tracking-tight text-ink sm:text-3xl">
            Konto-Optionen
          </h1>
          <p className="text-sm font-medium text-secondary">
            Verwalte Benachrichtigungen, Anmeldemethoden und deinen Account.
          </p>
        </div>
        <Button variant="secondary" size="sm" asChild>
          <Link href={paths.profil}>Zum Profil</Link>
        </Button>
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
        <CardHeader className="bg-pink-subtle">
          <CardTitle className="flex items-center gap-2">
            <KeyRound className="size-5" aria-hidden="true" />
            Anmeldemethoden
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4 pt-5">
          {loginMethods.isLoading ? (
            <div className="space-y-3">
              <Skeleton className="h-12 w-full" />
              <Skeleton className="h-12 w-full" />
            </div>
          ) : loginMethods.isError || !loginMethods.data ? (
            <Alert variant="error">
              Anmeldemethoden konnten nicht geladen werden.
            </Alert>
          ) : (
            <>
              <div className="flex flex-wrap items-center justify-between gap-3 rounded-sm border-2 border-strong bg-surface-muted p-4">
                <div className="space-y-0.5">
                  <p className="text-sm font-bold text-ink">
                    {hasPassword ? "Passwort aktiv" : "Kein Passwort gesetzt"}
                  </p>
                  <p className="text-xs font-medium text-secondary">
                    {hasPassword
                      ? "Du kannst dich mit deiner E-Mail-Adresse und deinem Passwort anmelden."
                      : "Lege optional ein Passwort fest, um dich auch ohne Anbieter anzumelden."}
                  </p>
                </div>
                {!hasPassword ? (
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => {
                      auth.setInitialPassword.reset();
                      setNewPassword("");
                      setNewPasswordConfirm("");
                      setPasswordDialogOpen(true);
                    }}
                  >
                    Passwort festlegen
                  </Button>
                ) : null}
              </div>

              {linkedIdentities.length > 0 ? (
                <div className="space-y-2 border-t-2 border-subtle pt-4">
                  <h2 className="font-mono text-xs font-bold tracking-wider text-secondary uppercase">
                    Verknüpfte Anbieter
                  </h2>
                  <div className="space-y-2">
                    {linkedIdentities.map((identity) => (
                      <div
                        key={identity.id}
                        className="flex flex-wrap items-center justify-between gap-3 rounded-sm border-2 border-strong bg-surface p-3"
                      >
                        <div className="space-y-0.5">
                          <p className="text-sm font-bold text-ink">
                            {identity.providerDisplayName}
                          </p>
                          <p className="text-xs font-medium text-secondary">
                            {identity.providerEmail ??
                              "Externe Identität verknüpft"}
                          </p>
                        </div>
                        <Button
                          variant="secondary"
                          size="sm"
                          disabled={
                            totalMethods <= 1 ||
                            auth.unlinkExternalIdentity.isPending
                          }
                          title={
                            totalMethods <= 1
                              ? "Letzte verbleibende Anmeldemethode kann nicht entfernt werden."
                              : undefined
                          }
                          onClick={() =>
                            auth.unlinkExternalIdentity.mutate(identity.id, {
                              onSuccess: () =>
                                feedback.melden(
                                  `Verknüpfung mit ${identity.providerDisplayName} entfernt.`,
                                ),
                            })
                          }
                        >
                          Trennen
                        </Button>
                      </div>
                    ))}
                  </div>
                </div>
              ) : null}

              {unlinkedProviders.length > 0 ? (
                <div className="space-y-2 border-t-2 border-subtle pt-4">
                  <h2 className="font-mono text-xs font-bold tracking-wider text-secondary uppercase">
                    Anbieter verknüpfen
                  </h2>
                  <div className="flex flex-wrap gap-2">
                    {unlinkedProviders.map((provider) => (
                      <Button
                        key={provider.id}
                        variant="secondary"
                        size="sm"
                        disabled={auth.startOidcAccountAction.isPending}
                        onClick={() =>
                          startOidcAction(
                            provider.id,
                            OidcAccountActionMode.link,
                          )
                        }
                      >
                        <Link2 className="mr-1.5 size-4" aria-hidden="true" />
                        Mit {provider.displayName} verknüpfen
                      </Button>
                    ))}
                  </div>
                </div>
              ) : null}

              {oidcStatusError || actionErrorMessage ? (
                <Alert variant="error">
                  {oidcStatusError ?? actionErrorMessage}
                </Alert>
              ) : null}
            </>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="bg-impact/10">
          <CardTitle className="flex items-center gap-2">
            <ShieldAlert className="size-5" aria-hidden="true" />
            Gefahrenzone
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4 pt-5">
          <div className="space-y-1">
            <h2 className="text-sm font-bold text-ink">Konto löschen</h2>
            <p className="text-sm font-medium text-secondary">
              Dein Konto, dein Profil und deine persönlichen Daten werden
              entfernt. Dieser Vorgang kann nicht rückgängig gemacht werden.
            </p>
          </div>
          {deleteErrorMessage ? (
            <Alert variant="error">
              <p>{deleteErrorMessage}</p>
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

      <Dialog open={passwordDialogOpen} onOpenChange={setPasswordDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Passwort festlegen</DialogTitle>
            <DialogDescription>
              Lege ein lokales Passwort fest, damit du dich zusätzlich mit
              E-Mail-Adresse und Passwort anmelden kannst.
            </DialogDescription>
          </DialogHeader>
          <form className="space-y-4" onSubmit={submitInitialPassword}>
            <Field label="Neues Passwort" hint="8 bis 128 Zeichen.">
              <PasswordInput
                autoComplete="new-password"
                minLength={PASSWORD_MIN}
                maxLength={PASSWORD_MAX}
                required
                value={newPassword}
                onChange={(event) => setNewPassword(event.target.value)}
              />
            </Field>
            <Field
              label="Passwort bestätigen"
              error={
                passwordMismatch
                  ? "Die Passwörter stimmen nicht überein."
                  : undefined
              }
            >
              <PasswordInput
                autoComplete="new-password"
                minLength={PASSWORD_MIN}
                maxLength={PASSWORD_MAX}
                required
                value={newPasswordConfirm}
                onChange={(event) => setNewPasswordConfirm(event.target.value)}
              />
            </Field>
            {setPasswordErrorMessage ? (
              <Alert variant="error">
                <p>{setPasswordErrorMessage}</p>
              </Alert>
            ) : null}
            <DialogFooter>
              <Button
                type="button"
                variant="secondary"
                onClick={() => setPasswordDialogOpen(false)}
                disabled={auth.setInitialPassword.isPending}
              >
                Abbrechen
              </Button>
              <Button
                type="submit"
                disabled={
                  !newPassword ||
                  passwordMismatch ||
                  auth.setInitialPassword.isPending
                }
              >
                {auth.setInitialPassword.isPending
                  ? "Wird gespeichert …"
                  : "Passwort speichern"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Konto endgültig löschen?</DialogTitle>
            <DialogDescription>
              {hasPassword
                ? "Gib zur Bestätigung dein Passwort ein. Deine Sitzungen enden sofort; diese Aktion kann nicht rückgängig gemacht werden."
                : "Bestätige deine Identität erneut über deinen verknüpften Anbieter, um das Konto endgültig zu löschen."}
            </DialogDescription>
          </DialogHeader>
          {hasPassword ? (
            <Field
              label="Passwort"
              hint="Gib zur Bestätigung dein aktuelles Passwort ein."
            >
              <Input
                type="password"
                autoComplete="current-password"
                value={password}
                onChange={(event) => setPassword(event.target.value)}
                minLength={PASSWORD_MIN}
                maxLength={PASSWORD_MAX}
                required
              />
            </Field>
          ) : reauthCode ? (
            <Alert variant="success">
              Identität bestätigt. Du kannst dein Konto jetzt endgültig löschen.
            </Alert>
          ) : (
            <div className="space-y-2">
              {linkedIdentities.map((identity) => (
                <Button
                  key={identity.id}
                  variant="secondary"
                  className="w-full"
                  disabled={auth.startOidcAccountAction.isPending}
                  onClick={() =>
                    startOidcAction(
                      identity.providerId,
                      OidcAccountActionMode.deleteReauth,
                    )
                  }
                >
                  Erneut mit {identity.providerDisplayName} bestätigen
                </Button>
              ))}
            </div>
          )}
          {deleteErrorMessage ? (
            <Alert variant="error">
              <p>{deleteErrorMessage}</p>
            </Alert>
          ) : null}
          <DialogFooter>
            <Button
              variant="secondary"
              onClick={() => {
                setDialogOpen(false);
                setReauthCode(null);
                auth.deleteAccount.reset();
              }}
              disabled={auth.deleteAccount.isPending}
            >
              Abbrechen
            </Button>
            <Button
              variant="destructive"
              onClick={deleteAccount}
              disabled={
                (hasPassword ? !password : !reauthCode) ||
                auth.deleteAccount.isPending
              }
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

export default function OptionsPage() {
  return (
    <Suspense fallback={null}>
      <OptionsContent />
    </Suspense>
  );
}
