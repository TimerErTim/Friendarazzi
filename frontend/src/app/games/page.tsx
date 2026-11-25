"use client";

import { ArrowRight, Plus } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect, useRef, useState } from "react";

import { useAuth } from "@/api/auth-provider";
import { apiErrorMessage } from "@/api/errors";
import { GameStatus } from "@/api/friendarazzi-api";
import { useGameDetailQuery, useGamesQuery } from "@/api/hooks";
import {
  ActivePlayPhase,
  CompletedPhase,
  DraftingPhase,
  type FlushPendingSaveFn,
  LobbyPhase,
  visibilityLabel,
} from "@/components/friendarazzi/game-phases";
import { SetBreadcrumbLabel } from "@/components/layout/page-chrome";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { CalloutNote, CodeTag } from "@/components/ui/brutal-elements";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/cn";
import {
  formatGameProgress,
  gameNextActionLabel,
  sortActiveGames,
} from "@/lib/game-overview";
import { authHrefWithRedirect, gameHref, paths } from "@/lib/navigation";
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
  const gamesList = useGamesQuery();
  const detail = useGameDetailQuery(id);
  const flushPendingSaveRef = useRef<FlushPendingSaveFn | null>(null);
  const [isSwitching, setIsSwitching] = useState(false);
  const [lastGameHintId, setLastGameHintId] = useState<string | null>(null);

  useEffect(() => {
    if (phase.status === "anonymous") {
      router.replace(
        authHrefWithRedirect("/anmeldung", id ? gameHref(id) : paths.games),
      );
    }
  }, [phase.status, id, router]);

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

  // Auto-open only when the player has a single game; multiple games get an explicit overview.
  useEffect(() => {
    if (id === "" && gamesList.data && gamesList.data.length === 1) {
      const single = gamesList.data[0];
      if (single) {
        router.replace(gameHref(single.id));
      }
    }
  }, [id, gamesList.data, router]);

  async function handleSwitchGame(targetId: string) {
    if (targetId === id || isSwitching) return;
    setIsSwitching(true);
    try {
      await flushPendingSaveRef.current?.();
    } finally {
      setIsSwitching(false);
      router.push(gameHref(targetId));
    }
  }

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
          <CalloutNote label="Kein Spiel gewählt" tone="yellow">
            Du bist aktuell in keinem Spiel. Starte ein neues Spiel oder tritt
            einem bestehenden Match bei.
          </CalloutNote>
          <Button asChild>
            <Link href={paths.home}>Zur Spielzentrale</Link>
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
              Deine Spiele
            </h1>
            <p className="text-sm font-medium text-secondary">
              Wähle eine laufende Runde aus oder kehre zu einem abgeschlossenen
              Spiel zurück.
            </p>
          </div>
          <Button variant="secondary" asChild>
            <Link href={paths.home}>Zur Spielzentrale</Link>
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
          <Link href={paths.home}>Zurück zur Spielzentrale</Link>
        </Button>
      </div>
    );
  }

  const game = detail.data;
  const activeIdx = PHASES.findIndex((p) => p.status === game.status);
  const runningGames = sortActiveGames(
    (gamesList.data ?? []).filter((g) => g.status !== GameStatus.Completed),
  );

  return (
    <div className="mx-auto w-full max-w-6xl space-y-6 p-4 sm:p-6">
      <SetBreadcrumbLabel label={game.title} />

      {/* Parallel Games Quick-Switch Bar */}
      {runningGames.length > 1 ? (
        <nav
          aria-label="Parallele Spiele wechseln"
          className="flex flex-wrap items-center gap-2"
        >
          {runningGames.map((item) => {
            const isCurrent = item.id === game.id;
            return (
              <button
                key={item.id}
                type="button"
                disabled={isSwitching}
                aria-current={isCurrent ? "page" : undefined}
                onClick={() => void handleSwitchGame(item.id)}
                className={cn(
                  "inline-flex min-h-11 cursor-pointer items-center gap-2 rounded-sm border-2 border-strong px-3 py-1.5 text-xs font-bold transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus disabled:opacity-60",
                  isCurrent
                    ? "brutal-shadow-sm bg-primary text-primary-foreground"
                    : "bg-surface text-ink hover:bg-surface-muted",
                )}
              >
                <span className="max-w-36 truncate">{item.title}</span>
                {item.needsAction && !isCurrent ? (
                  <CodeTag tone="yellow">Aktion</CodeTag>
                ) : (
                  <CodeTag tone={isCurrent ? "surface" : "cyan"}>
                    {shortStatusLabel(item.status)}
                  </CodeTag>
                )}
              </button>
            );
          })}
          <Link
            href={paths.home}
            className="inline-flex min-h-11 items-center gap-1 rounded-sm border-2 border-dashed border-strong bg-canvas px-3 py-1.5 text-xs font-bold text-ink hover:bg-surface-muted focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
          >
            <Plus className="size-3.5" aria-hidden="true" />
            Lobbys
          </Link>
        </nav>
      ) : null}

      {/* Match Header & 4-Phase Stepper */}
      <section className="brutal-shadow rounded-md border-[2.5px] border-strong bg-surface p-4 sm:p-5">
        <div className="min-w-0 space-y-1">
          <div className="flex flex-wrap items-center gap-2">
            <h1 className="truncate font-display text-xl tracking-tight text-ink sm:text-2xl">
              {game.title}
            </h1>
            <CodeTag tone="cyan">
              {game.gridSize}×{game.gridSize}
            </CodeTag>
          </div>
          <p className="font-mono text-xs font-bold text-secondary">
            {visibilityLabel(game.visibilityMode)} · {game.players.length}{" "}
            Spieler
          </p>
        </div>

        <ol
          aria-label="Spielphasen"
          className="mt-4 grid grid-cols-4 gap-1.5 border-t-2 border-strong pt-4"
        >
          {PHASES.map((step, idx) => {
            const isCurrent = idx === activeIdx;
            const isDone = idx < activeIdx;
            return (
              <li
                key={step.status}
                aria-current={isCurrent ? "step" : undefined}
                className={cn(
                  "flex items-center justify-center gap-1.5 rounded-sm border-2 border-strong px-2 py-1.5 text-center text-xs font-bold",
                  isCurrent
                    ? "brutal-shadow-sm bg-primary text-primary-foreground"
                    : isDone
                      ? "bg-cyan-subtle text-ink"
                      : "bg-surface-muted text-secondary",
                )}
              >
                <span className="font-mono">{step.num}.</span>
                <span className="truncate">{step.label}</span>
              </li>
            );
          })}
        </ol>
      </section>

      {game.status === GameStatus.Lobby ? (
        <LobbyPhase
          key={game.id}
          game={game}
          registerFlush={(fn) => {
            flushPendingSaveRef.current = fn;
          }}
        />
      ) : game.status === GameStatus.Drafting ? (
        <DraftingPhase
          key={game.id}
          game={game}
          detailUpdatedAt={detail.dataUpdatedAt}
          registerFlush={(fn) => {
            flushPendingSaveRef.current = fn;
          }}
        />
      ) : game.status === GameStatus.Active ? (
        <ActivePlayPhase key={game.id} game={game} />
      ) : (
        <CompletedPhase key={game.id} game={game} />
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
      <div className="flex items-start justify-between gap-2 border-b-2 border-strong bg-surface-muted px-5 py-3.5">
        <div className="min-w-0 space-y-1">
          <h3 className="truncate font-display text-lg text-ink">
            {game.title}
          </h3>
          <div className="flex flex-wrap items-center gap-1.5">
            {game.needsAction ? (
              <Badge variant="warning">Aktion nötig</Badge>
            ) : null}
            {isLastVisited ? (
              <CodeTag tone="cyan">Zuletzt geöffnet</CodeTag>
            ) : null}
          </div>
        </div>
        <Badge variant="primary">{shortStatusLabel(game.status)}</Badge>
      </div>
      <CardContent className="flex items-center justify-between gap-3 pt-4 pb-4">
        <p className="font-mono text-xs font-bold text-secondary">
          {formatGameProgress(game, countdown)}
        </p>
        <Button size="sm" asChild>
          <Link href={gameHref(game.id)}>
            {gameNextActionLabel(game)}
            <ArrowRight aria-hidden="true" />
          </Link>
        </Button>
      </CardContent>
    </Card>
  );
}
