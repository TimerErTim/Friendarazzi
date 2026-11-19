"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import { useAuth } from "@/api/auth-provider";
import { apiErrorMessage } from "@/api/errors";
import { useAuthMutations } from "@/api/hooks";
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
import { paths } from "@/lib/navigation";

export default function OptionsPage() {
  const phase = useAuth();
  const auth = useAuthMutations();
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
      "Das Konto konnte nicht gelöscht werden. Bitte versuchen Sie es später erneut.",
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

  return (
    <div className="mx-auto w-full max-w-2xl space-y-6 p-4 sm:p-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="font-display text-2xl tracking-tight text-ink sm:text-3xl">
            Konto-Optionen
          </h1>
          <p className="text-sm font-medium text-secondary">
            Verwalte deinen Account und deine Sitzung.
          </p>
        </div>
        <Button variant="secondary" size="sm" asChild>
          <Link href={paths.profil}>Zum Profil</Link>
        </Button>
      </div>

      <Card>
        <CardHeader className="bg-impact/10">
          <CardTitle>Gefahrenzone</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4 pt-5">
          <div className="space-y-1">
            <h2 className="text-sm font-bold text-ink">Konto löschen</h2>
            <p className="text-sm font-medium text-secondary">
              Ihr Konto, Ihr Profil und Ihre persönlichen Daten werden entfernt.
              Dieser Vorgang kann nicht rückgängig gemacht werden.
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
              Geben Sie zur Bestätigung Ihr Passwort ein. Ihre Sitzungen enden
              sofort; diese Aktion kann nicht rückgängig gemacht werden.
            </DialogDescription>
          </DialogHeader>
          <Field
            label="Passwort"
            hint="Geben Sie zur Bestätigung Ihr aktuelles Passwort ein."
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
