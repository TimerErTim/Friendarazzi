"use client";

import {
  type Bell,
  Camera,
  Check,
  CheckCheck,
  Clock,
  Crown,
  Flame,
  Play,
  Settings2,
} from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect } from "react";

import { useAuth } from "@/api/auth-provider";
import { apiErrorMessage } from "@/api/errors";
import {
  NotificationKind,
  type NotificationView,
} from "@/api/friendarazzi-api";
import { useFriendarazziMutations, useNotificationsQuery } from "@/api/hooks";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { CalloutNote, CodeTag } from "@/components/ui/brutal-elements";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/cn";
import {
  authHrefWithRedirect,
  captureHref,
  gameHref,
  paths,
} from "@/lib/navigation";

function notificationHref(item: NotificationView): string {
  if (item.kind === NotificationKind.CapturePosted && item.captureId) {
    return captureHref(item.gameId, item.captureId);
  }
  return gameHref(item.gameId);
}

function notificationSummary(item: NotificationView): {
  title: string;
  detail: string;
  badgeLabel: string;
  icon: typeof Bell;
  accentClass: string;
} {
  switch (item.kind) {
    case NotificationKind.GameStarted:
      return {
        title: `Spiel gestartet: „${item.gameTitle}“`,
        detail:
          "Die Entwurfsphase ist vorbei und alle Bingo-Bretter wurden ausgeteilt.",
        badgeLabel: "Spielstart",
        icon: Play,
        accentClass: "bg-cyan-subtle",
      };
    case NotificationKind.CapturePosted: {
      const catcher = item.actor?.displayName ?? "Jemand";
      if (item.isBlurred) {
        return {
          title: `${catcher} hat ein Teaser-Foto hochgeladen`,
          detail: `Spiel „${item.gameTitle}“`,
          badgeLabel: "Teaser",
          icon: Camera,
          accentClass: "bg-pink-subtle",
        };
      }
      const target = item.target?.displayName ?? "einen Mitspieler";
      const prompt = item.promptText ? `„${item.promptText}“` : "einem Auftrag";
      return {
        title: `${catcher} hat ${target} erwischt!`,
        detail: `${prompt} · Spiel „${item.gameTitle}“`,
        badgeLabel: "Schnappschuss",
        icon: Camera,
        accentClass: "bg-primary-subtle",
      };
    }
    case NotificationKind.NearBingo: {
      const leader = item.actor?.displayName ?? "Ein Mitspieler";
      return {
        title: `Fast Bingo! ${leader} fehlt nur noch 1 Feld`,
        detail: `Endspurt in „${item.gameTitle}“ – schnell die letzten Fotos sichern!`,
        badgeLabel: "Fast Bingo",
        icon: Flame,
        accentClass: "bg-pink-subtle",
      };
    }
    case NotificationKind.DraftReminder:
      return {
        title: `Entwurfsfrist läuft ab: „${item.gameTitle}“`,
        detail:
          "Bitte logge deine letzten Aufträge ein, bevor die Runde automatisch startet.",
        badgeLabel: "Entwurfsfrist",
        icon: Clock,
        accentClass: "bg-cyan-subtle",
      };
    case NotificationKind.GameCompleted: {
      if (!item.actor) {
        return {
          title: `Runde beendet: „${item.gameTitle}“`,
          detail:
            "Alle Spielbretter, Fotos und Auszeichnungen sind jetzt im Rückblick sichtbar.",
          badgeLabel: "Beendet",
          icon: Crown,
          accentClass: "bg-primary-subtle",
        };
      }
      const winner = item.actor.displayName;
      return {
        title: `Bingo! ${winner} hat „${item.gameTitle}“ gewonnen`,
        detail:
          "Alle Spielbretter, Fotos und Auszeichnungen sind jetzt freigeschaltet.",
        badgeLabel: "Bingo-Sieg",
        icon: Crown,
        accentClass: "bg-primary-subtle",
      };
    }
  }
}

export default function NotificationsPage() {
  const phase = useAuth();
  const router = useRouter();
  const inbox = useNotificationsQuery();
  const mutations = useFriendarazziMutations();

  useEffect(() => {
    if (phase.status === "anonymous") {
      router.replace(
        authHrefWithRedirect("/anmeldung", paths.benachrichtigungen),
      );
    }
  }, [phase.status, router]);

  function openNotification(item: NotificationView) {
    if (!item.readAt) {
      mutations.markNotificationRead.mutate(item.id);
    }
    router.push(notificationHref(item));
  }

  if (phase.status === "unavailable") {
    return (
      <div className="mx-auto w-full max-w-3xl p-4 sm:p-6">
        <Alert variant="error">Der Server ist derzeit nicht erreichbar.</Alert>
      </div>
    );
  }

  if (phase.status !== "authenticated" || inbox.isLoading) {
    return (
      <div className="mx-auto w-full max-w-3xl space-y-4 p-4 sm:p-6">
        <Skeleton className="h-14 w-full" />
        <Skeleton className="h-24 w-full" />
        <Skeleton className="h-24 w-full" />
      </div>
    );
  }

  const data = inbox.data;
  const items = data?.notifications ?? [];
  const unreadCount = data?.unreadCount ?? 0;

  return (
    <div className="mx-auto w-full max-w-3xl space-y-6 p-4 sm:p-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="space-y-1">
          <div className="flex items-center gap-2.5">
            <h1 className="font-display text-2xl tracking-tight text-ink sm:text-3xl">
              Benachrichtigungen
            </h1>
            {unreadCount > 0 ? (
              <CodeTag tone="pink">{unreadCount}</CodeTag>
            ) : null}
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          {unreadCount > 0 ? (
            <Button
              type="button"
              size="sm"
              disabled={mutations.markAllNotificationsRead.isPending}
              onClick={() => mutations.markAllNotificationsRead.mutate()}
            >
              <CheckCheck className="size-4" aria-hidden="true" />
              Alle gelesen
            </Button>
          ) : null}
          <Button variant="secondary" size="sm" asChild>
            <Link href={paths.optionen}>
              <Settings2 className="size-4" aria-hidden="true" />
              Einstellungen
            </Link>
          </Button>
        </div>
      </div>

      {inbox.isError ? (
        <Alert variant="error">
          {apiErrorMessage(inbox.error, {
            fallback: "Benachrichtigungen konnten nicht geladen werden.",
          })}
        </Alert>
      ) : null}

      {items.length === 0 ? (
        <CalloutNote label="Postfach leer" tone="yellow">
          Sobald in deinen Spielen etwas passiert, erscheint hier ein Eintrag.
        </CalloutNote>
      ) : (
        <div className="space-y-3">
          {items.map((item) => {
            const meta = notificationSummary(item);
            const Icon = meta.icon;
            const isUnread = !item.readAt;
            return (
              <Card
                key={item.id}
                className={cn(
                  "overflow-hidden transition-colors",
                  isUnread ? "border-strong" : "opacity-85",
                )}
              >
                <CardContent className="flex items-start gap-3 p-4">
                  <button
                    type="button"
                    onClick={() => openNotification(item)}
                    className="flex min-w-0 flex-1 cursor-pointer items-start gap-3 text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
                  >
                    <span
                      className={cn(
                        "mt-0.5 flex size-10 shrink-0 items-center justify-center rounded-sm border-2 border-strong text-ink",
                        meta.accentClass,
                      )}
                    >
                      <Icon className="size-4" aria-hidden="true" />
                    </span>
                    <div className="min-w-0 flex-1 space-y-1">
                      <div className="flex flex-wrap items-center gap-2">
                        <Badge variant={isUnread ? "warning" : "outline"}>
                          {isUnread ? "Neu" : meta.badgeLabel}
                        </Badge>
                        {isUnread ? (
                          <span className="font-mono text-[11px] font-bold text-secondary uppercase">
                            {meta.badgeLabel}
                          </span>
                        ) : null}
                        <span className="font-mono text-[11px] font-bold text-secondary tabular-nums">
                          {new Date(item.createdAt).toLocaleString("de-AT", {
                            dateStyle: "short",
                            timeStyle: "short",
                          })}
                        </span>
                      </div>
                      <p className="font-display text-base leading-snug text-ink">
                        {meta.title}
                      </p>
                      <p className="text-sm font-medium text-secondary">
                        {meta.detail}
                      </p>
                    </div>
                  </button>
                  {isUnread ? (
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      aria-label="Als gelesen markieren"
                      title="Als gelesen markieren"
                      disabled={mutations.markNotificationRead.isPending}
                      onClick={() =>
                        mutations.markNotificationRead.mutate(item.id)
                      }
                    >
                      <Check className="size-4" aria-hidden="true" />
                    </Button>
                  ) : null}
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}
    </div>
  );
}
