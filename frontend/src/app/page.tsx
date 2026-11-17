"use client";

import { Award, Camera, Plus, Ticket, Trophy, Users } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";

import { useAuth } from "@/api/auth-provider";
import { apiErrorMessage } from "@/api/errors";
import {
  GameStatus,
  type GameSummary,
  VisibilityMode,
} from "@/api/friendarazzi-api";
import { useFriendarazziMutations, useGamesQuery } from "@/api/hooks";
import { PlayerBadgeLink } from "@/components/friendarazzi/player-avatar";
import { PageHeader } from "@/components/layout/page-header";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
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
import {
  HatchBand,
  MarginNote,
  SheetNumber,
  StampMark,
} from "@/components/ui/paper-rule";
import { Select } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { gameHref, joinHref } from "@/lib/navigation";

function statusLabel(status: GameStatus): string {
  switch (status) {
    case GameStatus.Lobby:
      return "Lobby";
    case GameStatus.Drafting:
      return "Entwurf";
    case GameStatus.Active:
      return "Aktiv";
    case GameStatus.Completed:
      return "Beendet";
  }
}

export default function HomePage() {
  const phase = useAuth();

  return (
    <div className="mx-auto w-full max-w-5xl space-y-6 p-6">
      <PageHeader
        register={["Friendarazzi"]}
        title="Spiele & Lobbys"
        description="Starte ein neues Foto-Jagd-Bingo mit deinen Freunden, tritt über einen Einladungscode bei oder schau dir vergangene Runden im Rückblick an."
      />

      {phase.status === "authenticated" ? (
        <AuthenticatedHome />
      ) : phase.status === "anonymous" ? (
        <GuestLanding />
      ) : (
        <HomeSkeleton />
      )}
    </div>
  );
}

function GuestLanding() {
  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Foto-Jagd-Bingo für Freundesgruppen</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4 text-sm text-secondary">
          <p>
            Jede Person schreibt lustige Beobachtungen und typische
            Angewohnheiten über die anderen Mitspieler. Sobald alle eingeloggt
            sind, werden konfliktfreie Bingo-Karten gemischt – dann beginnt die
            Jagd nach Schnappschüssen!
          </p>
          <div className="flex flex-wrap gap-3 pt-1">
            <Button asChild>
              <Link href="/anmeldung">Anmelden</Link>
            </Button>
            <Button variant="secondary" asChild>
              <Link href="/registrierung">Konto erstellen</Link>
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

function AuthenticatedHome() {
  const router = useRouter();
  const games = useGamesQuery();
  const mutations = useFriendarazziMutations();

  const [createOpen, setCreateOpen] = useState(false);
  const [title, setTitle] = useState("Freitagabend");
  const [gridSize, setGridSize] = useState("3");
  const [visibilityMode, setVisibilityMode] = useState<VisibilityMode>(
    VisibilityMode.Undercover,
  );
  const [includeFreeSpace, setIncludeFreeSpace] = useState(true);
  const [inviteCode, setInviteCode] = useState("");

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    const created = await mutations.createGame.mutateAsync({
      title,
      gridSize: Number(gridSize),
      visibilityMode,
      includeFreeSpace,
    });
    setCreateOpen(false);
    router.push(gameHref(created.id));
  }

  function handleJoinSubmit(e: React.FormEvent) {
    e.preventDefault();
    const trimmed = inviteCode.trim().toUpperCase();
    if (!trimmed) return;
    router.push(joinHref(trimmed));
  }

  if (games.isLoading) {
    return <HomeSkeleton />;
  }

  const all = games.data ?? [];
  const active = all.filter((g) => g.status !== GameStatus.Completed);
  const completed = all.filter((g) => g.status === GameStatus.Completed);

  return (
    <div className="space-y-6">
      {/* Primary Action + Quick Join */}
      <div className="flex flex-wrap items-end justify-between gap-4">
        <Button onClick={() => setCreateOpen(true)}>
          <Plus className="size-4" aria-hidden="true" />
          Neues Spiel erstellen
        </Button>

        <form
          onSubmit={handleJoinSubmit}
          className="flex flex-wrap items-end gap-2"
        >
          <Field label="Einladungscode">
            <Input
              value={inviteCode}
              onChange={(e) => setInviteCode(e.target.value.toUpperCase())}
              placeholder="z.B. A7K9P2"
              className="w-36 font-mono uppercase"
              maxLength={16}
            />
          </Field>
          <Button type="submit" variant="secondary" size="field">
            <Ticket className="size-4" aria-hidden="true" />
            Beitreten
          </Button>
        </form>
      </div>

      {/* Active Games */}
      <section className="space-y-3">
        <div className="flex items-baseline gap-3">
          <h2 className="font-serif text-lg font-semibold text-ink">
            Laufende Spiele
          </h2>
          <SheetNumber>{active.length}</SheetNumber>
        </div>

        {active.length === 0 ? (
          <MarginNote label="Keine aktive Runde">
            Erstelle oben ein neues Spiel oder gib einen Einladungscode deiner
            Freunde ein.
          </MarginNote>
        ) : (
          <div className="grid gap-4 sm:grid-cols-2">
            {active.map((game) => (
              <ActiveGameCard key={game.id} game={game} />
            ))}
          </div>
        )}
      </section>

      <HatchBand />

      {/* Completed Games */}
      <section className="space-y-3">
        <div className="flex items-baseline gap-3">
          <h2 className="font-serif text-lg font-semibold text-ink">
            Vergangene Spiele
          </h2>
          <SheetNumber>{completed.length}</SheetNumber>
        </div>

        {completed.length === 0 ? (
          <MarginNote label="Noch kein Archiv">
            Abgeschlossene Spiele inklusive 1:1 Zeitachse und Fotogalerie
            erscheinen hier.
          </MarginNote>
        ) : (
          <Table>
            <TableHead>
              <TableRow>
                <TableHeader>Spiel</TableHeader>
                <TableHeader>Datum</TableHeader>
                <TableHeader>Sieger</TableHeader>
                <TableHeader>
                  <span className="sr-only">Öffnen</span>
                </TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {completed.map((game) => (
                <TableRow key={game.id}>
                  <TableCell>
                    <div className="font-medium text-ink">{game.title}</div>
                    <div className="text-xs text-secondary">
                      {game.gridSize}×{game.gridSize} · {game.playerCount}{" "}
                      Spieler
                    </div>
                  </TableCell>
                  <TableCell className="text-secondary tabular-nums">
                    {game.endedAt
                      ? new Date(game.endedAt).toLocaleDateString("de-AT")
                      : new Date(game.createdAt).toLocaleDateString("de-AT")}
                  </TableCell>
                  <TableCell>
                    {game.winner ? (
                      <PlayerBadgeLink user={game.winner} compact />
                    ) : (
                      <StampMark>Beendet</StampMark>
                    )}
                  </TableCell>
                  <TableCell className="text-right">
                    <Button variant="secondary" size="sm" asChild>
                      <Link href={gameHref(game.id)}>Rückblick</Link>
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </section>

      {/* Create Game Dialog */}
      <Dialog open={createOpen} onOpenChange={setCreateOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Neues Friendarazzi-Spiel</DialogTitle>
            <DialogDescription>
              Lege Rastergröße und Sichtbarkeit fest – alles lässt sich in der
              Lobby noch anpassen.
            </DialogDescription>
          </DialogHeader>
          <form className="space-y-4" onSubmit={(e) => void handleCreate(e)}>
            <Field label="Spieltitel">
              <Input
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                required
              />
            </Field>
            <div className="grid gap-4 sm:grid-cols-2">
              <Field label="Rastergröße">
                <Select
                  value={gridSize}
                  onChange={(e) => setGridSize(e.target.value)}
                >
                  <option value="3">3 × 3 (Schnell)</option>
                  <option value="4">4 × 4 (Mittel)</option>
                  <option value="5">5 × 5 (Marathon)</option>
                </Select>
              </Field>
              <Field label="Sichtbarkeit">
                <Select
                  value={visibilityMode}
                  onChange={(e) =>
                    setVisibilityMode(e.target.value as VisibilityMode)
                  }
                >
                  <option value={VisibilityMode.Undercover}>Undercover</option>
                  <option value={VisibilityMode.LiveFeed}>Live-Feed</option>
                  <option value={VisibilityMode.BlurredTeaser}>
                    Blurred Teaser
                  </option>
                </Select>
              </Field>
            </div>
            {gridSize !== "4" ? (
              <div className="flex items-center justify-between gap-4 rounded-md border border-subtle p-3">
                <span className="text-sm text-ink">
                  Mittleres „Free Space“-Feld aktivieren
                </span>
                <Switch
                  checked={includeFreeSpace}
                  onCheckedChange={setIncludeFreeSpace}
                  aria-label="Free Space aktivieren"
                />
              </div>
            ) : null}

            {mutations.createGame.isError ? (
              <Alert variant="error">
                {apiErrorMessage(mutations.createGame.error, {
                  fallback: "Das Spiel konnte nicht erstellt werden.",
                })}
              </Alert>
            ) : null}

            <DialogFooter>
              <Button
                type="button"
                variant="secondary"
                onClick={() => setCreateOpen(false)}
              >
                Abbrechen
              </Button>
              <Button type="submit" disabled={mutations.createGame.isPending}>
                {mutations.createGame.isPending
                  ? "Wird erstellt …"
                  : "Lobby eröffnen"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function ActiveGameCard({ game }: { game: GameSummary }) {
  return (
    <Card>
      <CardHeader>
        <div className="flex items-start justify-between gap-2">
          <div className="min-w-0">
            <CardTitle className="truncate">{game.title}</CardTitle>
            <p className="mt-1 font-mono text-xs text-secondary">
              Code: {game.inviteCode}
            </p>
          </div>
          <Badge variant="primary">{statusLabel(game.status)}</Badge>
        </div>
      </CardHeader>
      <CardContent className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap items-center gap-3 text-xs text-secondary">
          <span className="inline-flex items-center gap-1">
            <Camera className="size-3.5" aria-hidden="true" />
            {game.gridSize}×{game.gridSize}
          </span>
          <span className="inline-flex items-center gap-1">
            <Users className="size-3.5" aria-hidden="true" />
            {game.playerCount} Spieler
          </span>
          {game.winner ? (
            <span className="inline-flex items-center gap-1">
              <Trophy className="size-3.5 text-warning" aria-hidden="true" />
              <Award className="size-3.5 text-warning" aria-hidden="true" />
              {game.winner.displayName}
            </span>
          ) : null}
        </div>
        <Button size="sm" asChild>
          <Link href={gameHref(game.id)}>Öffnen</Link>
        </Button>
      </CardContent>
    </Card>
  );
}

function HomeSkeleton() {
  return (
    <div aria-busy="true" className="space-y-4">
      <Skeleton className="h-11 w-48" />
      <div className="grid gap-4 sm:grid-cols-2">
        <Skeleton className="h-32 w-full" />
        <Skeleton className="h-32 w-full" />
      </div>
    </div>
  );
}
