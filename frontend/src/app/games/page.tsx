"use client";

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
import { PageHeader } from "@/components/layout/page-header";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { CalloutNote } from "@/components/ui/brutal-elements";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { gameHref, paths } from "@/lib/navigation";
import { useQueryParam } from "@/lib/use-query-param";

function statusLabel(status: GameStatus): string {
  switch (status) {
    case GameStatus.Lobby:
      return "Lobby";
    case GameStatus.Drafting:
      return "Entwurfsphase";
    case GameStatus.Active:
      return "Aktive Jagd";
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
      router.replace("/anmeldung");
    }
  }, [phase.status, router]);

  useEffect(() => {
    if (id === "" && gamesList.data && gamesList.data.length > 0) {
      const target =
        gamesList.data.find((g) => g.status !== GameStatus.Completed) ??
        gamesList.data[0];
      if (target) {
        router.replace(gameHref(target.id));
      }
    }
  }, [id, gamesList.data, router]);

  if (id === null || detail.isLoading || (id === "" && gamesList.isLoading)) {
    return (
      <div className="mx-auto w-full max-w-5xl space-y-6 p-4 sm:p-6">
        <Skeleton className="h-12 w-64" />
        <Skeleton className="h-48 w-full" />
        <Skeleton className="h-64 w-full" />
      </div>
    );
  }

  if (!id) {
    return (
      <div className="mx-auto w-full max-w-3xl space-y-4 p-4 sm:p-6">
        <CalloutNote label="Kein Spiel gewählt" tone="yellow">
          Du bist aktuell in keinem Spiel. Starte ein neues Spiel auf der
          Übersichtsseite.
        </CalloutNote>
        <Button asChild>
          <Link href={paths.home}>Zur Übersicht</Link>
        </Button>
      </div>
    );
  }

  if (detail.isError || !detail.data) {
    return (
      <div className="mx-auto w-full max-w-3xl space-y-4 p-4 sm:p-6">
        <Alert variant="error">
          {apiErrorMessage(detail.error, {
            unauthorized: "Bitte melde dich an.",
            fallback: "Das Spiel konnte nicht geladen werden.",
          })}
        </Alert>
        <Button variant="secondary" asChild>
          <Link href={paths.home}>Zurück zur Übersicht</Link>
        </Button>
      </div>
    );
  }

  const game = detail.data;

  return (
    <div className="mx-auto w-full max-w-5xl space-y-6 p-4 sm:p-6">
      <SetBreadcrumbLabel label={game.title} />
      <PageHeader
        register={["Friendarazzi", "Match"]}
        title={game.title}
        plate={game.inviteCode}
        description={`${game.gridSize}×${game.gridSize} Bingo · ${game.players.length} Spieler`}
        actions={
          <Badge
            variant={
              game.status === GameStatus.Completed ? "warning" : "primary"
            }
          >
            {statusLabel(game.status)}
          </Badge>
        }
      />

      {game.status === GameStatus.Lobby ? (
        <LobbyPhase game={game} />
      ) : game.status === GameStatus.Drafting ? (
        <DraftingPhase game={game} />
      ) : game.status === GameStatus.Active ? (
        <ActivePlayPhase game={game} />
      ) : (
        <CompletedPhase game={game} />
      )}
    </div>
  );
}
