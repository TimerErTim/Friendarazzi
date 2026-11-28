"use client";

import { ArrowRight, Bell, BellOff } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";

import { useAuth } from "@/api/auth-provider";
import { apiErrorMessage } from "@/api/errors";
import { GameStatus } from "@/api/friendarazzi-api";
import {
  useFriendarazziMutations,
  useGameDetailQuery,
  useGamesQuery,
} from "@/api/hooks";
import {
  ActivePlayPhase,
  CompletedPhase,
  DraftingPhase,
  LobbyPhase,
} from "@/components/friendarazzi/game-phases";
import { GameRulesDialog } from "@/components/friendarazzi/game-rules-dialog";
import { SetBreadcrumbLabel } from "@/components/layout/page-chrome";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { CalloutNote } from "@/components/ui/brutal-elements";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/cn";
import {
  formatGameProgress,
  gameNextActionLabel,
  sortActiveGames,
} from "@/lib/game-overview";
import {
  authHrefWithRedirect,
  captureHref,
  gameHref,
  paths,
} from "@/lib/navigation";
import { useGameCountdown } from "@/lib/use-game-countdown";
import { useQueryParam } from "@/lib/use-query-param";

const LAST_GAME_KEY = "friendarazzi:last_game_id";

const PHASES = [
  { status: GameStatus.Lobby, num: "1", label: "Lobby" },
  { status: GameStatus.Drafting, num: "2", label: "Entwurf" },
  { status: GameStatus.Active, num: "3", label: "Jagd" },
  { status: GameStatus.Completed, num: "4", label: "Finale" },
] as const;

function shortStatusLabel(status: GameStatus): string {
  switch (status) {
    case GameStatus.Lobby:
      return "Lobby";
    case GameStatus.Drafting:
      return "Entwurf";
    case GameStatus.Active:
      return "Läuft";
    case GameStatus.Completed:
      return "Beendet";
  }
}

export default function GamePage() {
  const phase = useAuth();
  const router = useRouter();
  const id = useQueryParam("id");
  const captureParam = useQueryParam("capture");
  const tileParam = useQueryParam("tile");
  const gamesList = useGamesQuery();
  const detail = useGameDetailQuery(id);
  const mutations = useFriendarazziMutations();
  const [lastGameHintId, setLastGameHintId] = useState<string | null>(null);

  useEffect(() => {
    if (phase.status === "anonymous") {
      const target = id
        ? captureParam
          ? captureHref(id, captureParam)
          : gameHref(id)
        : paths.games;
      router.replace(authHrefWithRedirect("/anmeldung", target));
    }
  }, [phase.status, id, captureParam, router]);

  useEffect(() => {
    if (typeof window === "undefined") return;
    try {
      if (id) {
        window.sessionStorage.setItem(LAST_GAME_KEY, id);
        setLastGameHintId(id);
      } else {
        setLastGameHintId(window.sessionStorage.getItem(LAST_GAME_KEY));
      }
    } catch {
      // Ignore storage errors
    }
  }, [id]);

  if (id === null || detail.isLoading || (id === "" && gamesList.isLoading)) {
    return (
      <div className="mx-auto w-full max-w-6xl space-y-6 p-4 sm:p-6">
        <Skeleton className="h-20 w-full" />
        <Skeleton className="h-64 w-full" />
      </div>
    );
  }

  if (!id) {
    const allGames = gamesList.data ?? [];
    if (allGames.length === 0) {
      return (
        <div className="mx-auto w-full max-w-2xl space-y-4 p-4 sm:p-6">
          <CalloutNote label="Keine Lobbys" tone="yellow">
            Du bist aktuell in keinem Spiel. Erstelle auf Home ein neues Spiel
            oder tritt über einen Einladungscode bei.
          </CalloutNote>
          <Button asChild>
            <Link href={paths.home}>Zu Home</Link>
          </Button>
        </div>
      );
    }

    const active = sortActiveGames(
      allGames.filter((g) => g.status !== GameStatus.Completed),
    );
    const completed = allGames.filter((g) => g.status === GameStatus.Completed);

    return (
      <div className="mx-auto w-full max-w-5xl space-y-6 p-4 sm:p-6">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h1 className="font-display text-2xl tracking-tight text-ink">
              Lobbys &amp; Matches
            </h1>
            <p className="text-sm font-medium text-secondary">
              Alle beigetretenen Lobbys, Entwürfe, laufenden Runden und
              vergangenen Matches.
            </p>
          </div>
          <Button variant="secondary" asChild>
            <Link href={paths.home}>Zu Home</Link>
          </Button>
        </div>

        {active.length > 0 ? (
          <section className="space-y-3">
            <h2 className="font-display text-lg text-ink">Laufende Runden</h2>
            <div className="grid gap-4 md:grid-cols-2">
              {active.map((item) => (
                <GameOverviewCard
                  key={item.id}
                  game={item}
                  dataUpdatedAt={gamesList.dataUpdatedAt}
                  isLastVisited={item.id === lastGameHintId}
                />
              ))}
            </div>
          </section>
        ) : null}

        {completed.length > 0 ? (
          <section className="space-y-3">
            <h2 className="font-display text-lg text-ink">
              Abgeschlossene Runden
            </h2>
            <div className="grid gap-4 md:grid-cols-2">
              {completed.map((item) => (
                <GameOverviewCard
                  key={item.id}
                  game={item}
                  dataUpdatedAt={gamesList.dataUpdatedAt}
                  isLastVisited={item.id === lastGameHintId}
                />
              ))}
            </div>
          </section>
        ) : null}
      </div>
    );
  }

  if (detail.isError || !detail.data) {
    return (
      <div className="mx-auto w-full max-w-2xl space-y-4 p-4 sm:p-6">
        <Alert variant="error">
          {apiErrorMessage(detail.error, {
            unauthorized: "Bitte melde dich an.",
            fallback: "Das Spiel konnte nicht geladen werden.",
          })}
        </Alert>
        <Button variant="secondary" asChild>
          <Link href={paths.home}>Zurück zu Home</Link>
        </Button>
      </div>
    );
  }

  const game = detail.data;
  const activeIdx = PHASES.findIndex((p) => p.status === game.status);

  return (
    <div className="mx-auto w-full max-w-6xl space-y-3 px-3 pt-3 pb-6 sm:space-y-4 sm:p-6">
      <SetBreadcrumbLabel label={game.title} />

      {/* Single-line match header */}
      <header className="flex items-center justify-between gap-2 border-b-2 border-strong pb-2.5">
        <div className="flex min-w-0 items-center gap-2">
          <h1 className="truncate font-display text-lg tracking-tight text-ink sm:text-2xl">
            {game.title}
          </h1>
          <Badge variant="primary" className="shrink-0">
            {shortStatusLabel(game.status)}
          </Badge>
          <ol
            aria-label="Spielphasen"
            className="hidden items-center gap-0.5 sm:inline-flex"
          >
            {PHASES.map((step, idx) => {
              const isCurrent = idx === activeIdx;
              const isDone = idx < activeIdx;
              return (
                <li
                  key={step.status}
                  aria-current={isCurrent ? "step" : undefined}
                  title={`${step.num}. ${step.label}`}
                  className={cn(
                    "h-2 w-4 rounded-[1px] border border-strong",
                    isCurrent
                      ? "bg-primary"
                      : isDone
                        ? "bg-cyan"
                        : "bg-surface-muted",
                  )}
                >
                  <span className="sr-only">
                    {step.num}. {step.label}
                    {isCurrent ? " (aktuell)" : isDone ? " (erledigt)" : ""}
                  </span>
                </li>
              );
            })}
          </ol>
        </div>

        <div className="flex shrink-0 items-center gap-1.5">
          <GameRulesDialog
            status={game.status}
            visibilityMode={game.visibilityMode}
          />
          <Button
            type="button"
            variant={game.watching ? "secondary" : "ghost"}
            size="sm"
            aria-pressed={game.watching}
            aria-label={
              game.watching
                ? "Benachrichtigungen für dieses Spiel stummschalten"
                : "Benachrichtigungen für dieses Spiel aktivieren"
            }
            title={
              game.watching
                ? "Benachrichtigungen aktiv"
                : "Benachrichtigungen stumm"
            }
            disabled={mutations.setGameWatch.isPending}
            onClick={() =>
              mutations.setGameWatch.mutate({
                gameId: game.id,
                watching: !game.watching,
              })
            }
            className="shrink-0 gap-1.5"
          >
            {game.watching ? (
              <Bell className="size-4" aria-hidden="true" />
            ) : (
              <BellOff className="size-4" aria-hidden="true" />
            )}
            <span className="hidden sm:inline">
              {game.watching ? "Benachrichtigungen" : "Stumm"}
            </span>
          </Button>
        </div>
      </header>

      {game.status === GameStatus.Lobby ? (
        <LobbyPhase key={game.id} game={game} />
      ) : game.status === GameStatus.Drafting ? (
        <DraftingPhase
          key={game.id}
          game={game}
          detailUpdatedAt={detail.dataUpdatedAt}
        />
      ) : game.status === GameStatus.Active ? (
        <ActivePlayPhase
          key={game.id}
          game={game}
          initialCaptureId={captureParam || null}
          onConsumeInitialCapture={() => router.replace(gameHref(game.id))}
          initialTileId={tileParam || null}
          onConsumeInitialTile={() => router.replace(gameHref(game.id))}
        />
      ) : (
        <CompletedPhase
          key={game.id}
          game={game}
          initialCaptureId={captureParam || null}
          onConsumeInitialCapture={() => router.replace(gameHref(game.id))}
        />
      )}
    </div>
  );
}

function GameOverviewCard({
  game,
  dataUpdatedAt,
  isLastVisited,
}: {
  game: NonNullable<ReturnType<typeof useGamesQuery>["data"]>[number];
  dataUpdatedAt: number;
  isLastVisited: boolean;
}) {
  const countdown = useGameCountdown(
    game.draftClosesAt,
    game.serverNow,
    dataUpdatedAt,
  );

  return (
    <Card className="overflow-hidden">
      <div
        className={cn(
          "flex items-start justify-between gap-2 border-b-2 border-strong px-5 py-3.5",
          game.status === GameStatus.Completed
            ? "bg-surface-muted"
            : game.needsAction || game.status === GameStatus.Active
              ? "bg-primary-subtle"
              : game.status === GameStatus.Drafting
                ? "bg-cyan-subtle"
                : "bg-pink-subtle",
        )}
      >
        <div className="min-w-0 space-y-1">
          <h3 className="truncate font-display text-lg text-ink">
            {game.title}
          </h3>
          {game.needsAction || isLastVisited ? (
            <div className="flex flex-wrap items-center gap-2">
              {game.needsAction ? (
                <Badge variant="warning">Aktion nötig</Badge>
              ) : null}
              {isLastVisited ? (
                <span className="font-mono text-[11px] font-bold text-secondary">
                  Zuletzt geöffnet
                </span>
              ) : null}
            </div>
          ) : null}
        </div>
        <Badge variant="info">{shortStatusLabel(game.status)}</Badge>
      </div>
      <CardContent className="flex items-center justify-between gap-3 pt-4 pb-4">
        <p className="font-mono text-xs font-bold text-secondary">
          {formatGameProgress(game, countdown)}
        </p>
        <Button variant="secondary" size="sm" asChild>
          <Link href={gameHref(game.id)}>
            {gameNextActionLabel(game)}
            <ArrowRight aria-hidden="true" />
          </Link>
        </Button>
      </CardContent>
    </Card>
  );
}
