"use client";

import { Plus } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect } from "react";

import { useAuth } from "@/api/auth-provider";
import { apiErrorMessage } from "@/api/errors";
import { GameStatus } from "@/api/friendarazzi-api";
import { useGameDetailQuery, useGamesQuery } from "@/api/hooks";
import {
  ActivePlayPhase,
  CompletedPhase,
  DraftingPhase,
  LobbyPhase,
} from "@/components/friendarazzi/game-phases";
import { SetBreadcrumbLabel } from "@/components/layout/page-chrome";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { CalloutNote, CodeTag } from "@/components/ui/brutal-elements";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/cn";
import { authHrefWithRedirect, gameHref, paths } from "@/lib/navigation";
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

  useEffect(() => {
    if (phase.status === "anonymous") {
      router.replace(
        authHrefWithRedirect("/anmeldung", id ? gameHref(id) : paths.games),
      );
    }
  }, [phase.status, id, router]);

  useEffect(() => {
    if (!id || typeof window === "undefined") return;
    try {
      window.sessionStorage.setItem(LAST_GAME_KEY, id);
    } catch {
      // Ignore storage errors
    }
  }, [id]);

  useEffect(() => {
    if (id === "" && gamesList.data && gamesList.data.length > 0) {
      let lastId: string | null = null;
      if (typeof window !== "undefined") {
        try {
          lastId = window.sessionStorage.getItem(LAST_GAME_KEY);
        } catch {
          lastId = null;
        }
      }
      const target =
        (lastId ? gamesList.data.find((g) => g.id === lastId) : undefined) ??
        gamesList.data.find((g) => g.status !== GameStatus.Completed) ??
        gamesList.data[0];
      if (target) {
        router.replace(gameHref(target.id));
      }
    }
  }, [id, gamesList.data, router]);

  if (id === null || detail.isLoading || (id === "" && gamesList.isLoading)) {
    return (
      <div className="mx-auto w-full max-w-6xl space-y-6 p-4 sm:p-6">
        <Skeleton className="h-20 w-full" />
        <Skeleton className="h-64 w-full" />
      </div>
    );
  }

  if (!id) {
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
  const runningGames = (gamesList.data ?? []).filter(
    (g) => g.status !== GameStatus.Completed,
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
              <Link
                key={item.id}
                href={gameHref(item.id)}
                aria-current={isCurrent ? "page" : undefined}
                className={cn(
                  "inline-flex min-h-11 items-center gap-2 rounded-sm border-2 border-strong px-3 py-1.5 text-xs font-bold transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus",
                  isCurrent
                    ? "brutal-shadow-sm bg-primary text-primary-foreground"
                    : "bg-surface text-ink hover:bg-surface-muted",
                )}
              >
                <span className="max-w-36 truncate">{item.title}</span>
                <CodeTag tone={isCurrent ? "surface" : "cyan"}>
                  {shortStatusLabel(item.status)}
                </CodeTag>
              </Link>
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

      {/* Match Stage Bar & 4-Phase Stepper */}
      <section className="brutal-shadow rounded-md border-[2.5px] border-strong bg-surface p-4 sm:p-5">
        <div className="flex flex-wrap items-center justify-between gap-4">
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
              {game.players.length} Spieler
            </p>
          </div>
          <Badge variant="primary">{shortStatusLabel(game.status)}</Badge>
        </div>

        {/* Visual Phase Stepper */}
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
        <LobbyPhase key={game.id} game={game} />
      ) : game.status === GameStatus.Drafting ? (
        <DraftingPhase key={game.id} game={game} />
      ) : game.status === GameStatus.Active ? (
        <ActivePlayPhase key={game.id} game={game} />
      ) : (
        <CompletedPhase key={game.id} game={game} />
      )}
    </div>
  );
}
