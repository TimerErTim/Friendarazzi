"use client";

import { CheckCircle2, MailX } from "lucide-react";
import Link from "next/link";
import { useEffect, useRef } from "react";

import { apiErrorMessage } from "@/api/errors";
import { NotificationKind } from "@/api/friendarazzi-api";
import { useFriendarazziMutations } from "@/api/hooks";
import { Alert } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { paths } from "@/lib/navigation";
import { useQueryParam } from "@/lib/use-query-param";

function kindLabel(kind: NotificationKind): string {
  switch (kind) {
    case NotificationKind.GameStarted:
      return "E-Mails zum Spielstart";
    case NotificationKind.CapturePosted:
      return "E-Mails zu neuen Beweisfotos";
    case NotificationKind.GameCompleted:
      return "E-Mails zum Spielende & Bingo-Sieg";
  }
}

export default function UnsubscribePage() {
  const token = useQueryParam("token");
  const { unsubscribeNotification } = useFriendarazziMutations();
  const { mutate, isPending, isSuccess, isError, data, error } =
    unsubscribeNotification;
  const triggeredForRef = useRef<string | null>(null);

  useEffect(() => {
    if (!token || triggeredForRef.current === token) return;
    triggeredForRef.current = token;
    mutate(token);
  }, [token, mutate]);

  return (
    <div className="mx-auto w-full max-w-xl px-4 py-10 sm:px-6 lg:py-16">
      <Card>
        <CardHeader className="bg-cyan-subtle">
          <CardTitle className="flex items-center gap-2">
            <MailX className="size-5" aria-hidden="true" />
            E-Mail-Benachrichtigungen
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4 pt-5">
          <h1 className="font-display text-2xl tracking-tight text-ink">
            Benachrichtigung abbestellen
          </h1>

          {token === null || isPending ? (
            <Skeleton className="h-16 w-full" />
          ) : !token ? (
            <Alert variant="error">
              Der Abmelde-Link ist unvollständig oder ungültig.
            </Alert>
          ) : isError ? (
            <Alert variant="error">
              {apiErrorMessage(error, {
                badRequest: "Der Abmelde-Link ist ungültig oder abgelaufen.",
                fallback:
                  "Die E-Mail-Einstellung konnte nicht geändert werden.",
              })}
            </Alert>
          ) : isSuccess && data ? (
            <Alert
              variant="success"
              icon={<CheckCircle2 className="size-4" aria-hidden="true" />}
            >
              <p>
                <strong>{kindLabel(data.kind)}</strong> wurden erfolgreich
                abbestellt. Deine In-App-Benachrichtigungen bleiben davon
                unberührt.
              </p>
            </Alert>
          ) : null}

          <div className="flex flex-wrap gap-3 pt-2">
            <Button variant="secondary" asChild>
              <Link href={paths.optionen}>Zu den Konto-Optionen</Link>
            </Button>
            <Button variant="ghost" asChild>
              <Link href={paths.home}>Zur Startseite</Link>
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
