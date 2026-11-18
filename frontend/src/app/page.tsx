"use client";

import {
  ArrowRight,
  Award,
  Camera,
  CheckCircle2,
  Plus,
  Sparkles,
  Ticket,
  Trophy,
  Users,
} from "lucide-react";
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
import { Wordmark } from "@/components/brand/wordmark";
import { PlayerBadgeLink } from "@/components/friendarazzi/player-avatar";
import { PageHeader } from "@/components/layout/page-header";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import {
  CalloutNote,
  CodeTag,
  SparkleIcon,
  StickerBand,
} from "@/components/ui/brutal-elements";
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
import { Select } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { gameHref, joinHref, paths } from "@/lib/navigation";

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

  if (phase.status === "anonymous") {
    return (
      <div className="mx-auto w-full max-w-6xl space-y-8 p-4 sm:p-6 lg:p-8">
        <GuestLanding />
      </div>
    );
  }

  return (
    <div className="mx-auto w-full max-w-5xl space-y-6 p-4 sm:p-6">
      <PageHeader
        register={["Friendarazzi", "Zentrale"]}
        title="Spiele & Lobbys"
        description="Starte eine neue Runde, tritt per Code bei oder sieh dir vergangene Bingo-Matches im Rückblick an."
      />

      {phase.status === "authenticated" ? (
        <AuthenticatedHome />
      ) : (
        <HomeSkeleton />
      )}
    </div>
  );
}

const DEMO_TILES = [
  { target: "Lena", text: "Bestellt noch ein Getränk", captured: true },
  { target: "Max", text: "Schaut aufs falsche Handy", captured: false },
  { target: "Sam", text: "Macht ein Gruppen-Selfie", captured: true },
  { target: "Jule", text: "Lacht über eigenen Witz", captured: false },
  {
    target: "BINGO",
    text: "Foto-Jagd im Gange!",
    captured: true,
    center: true,
  },
  { target: "Tom", text: "Erklärt etwas mit Händen", captured: false },
  { target: "Lea", text: "Findet einen Hund zum Streicheln", captured: true },
  { target: "Ben", text: "Bestellt Extra-Pommes", captured: false },
  { target: "Kim", text: "Macht doppelten Peace-Move", captured: false },
] as const;

function GuestLanding() {
  return (
    <div className="space-y-8">
      {/* Impactful Hero Section */}
      <section className="brutal-shadow-lg relative overflow-hidden rounded-md border-[2.5px] border-strong bg-surface p-6 sm:p-8 lg:p-10">
        <div className="grid items-center gap-8 lg:grid-cols-[1.15fr_0.85fr]">
          <div className="space-y-6">
            <div className="flex flex-wrap items-center gap-2.5">
              <Badge variant="info">The Candid Photo-Hunt Bingo</Badge>
              <SparkleIcon tone="primary" />
              <SparkleIcon tone="accent" className="size-5" />
            </div>

            <div className="space-y-3">
              <h1 className="sr-only">
                Friendarazzi — Das soziale Foto-Jagd-Bingo
              </h1>
              <div aria-hidden="true">
                <Wordmark size="hero" />
              </div>
              <p className="font-display text-xl tracking-tight text-ink uppercase sm:text-2xl">
                Erwische deine Freunde in flagranti!
              </p>
              <p className="max-w-xl text-base font-medium text-secondary">
                Jede Person schreibt lustige Beobachtungen und typische
                Angewohnheiten über den Rest der Gruppe. Sobald alle einloggen,
                mischt Friendarazzi geheime Bingo-Karten – dann zählt jeder
                Schnappschuss!
              </p>
            </div>

            {/* Easy Getting Started CTAs */}
            <div className="flex flex-wrap items-center gap-3 pt-1">
              <Button size="lg" asChild>
                <Link href="/anmeldung">
                  Anmelden &amp; Loslegen
                  <ArrowRight aria-hidden="true" />
                </Link>
              </Button>
              <Button variant="secondary" size="lg" asChild>
                <Link href="/registrierung">Konto erstellen</Link>
              </Button>
              <Button variant="ghost" size="lg" asChild>
                <Link href={paths.join}>
                  <Ticket aria-hidden="true" />
                  Code einlösen
                </Link>
              </Button>
            </div>
          </div>

          {/* Demonstrative Live Bingo Card Preview */}
          <div className="relative">
            <div className="brutal-shadow rounded-md border-[2.5px] border-strong bg-primary-subtle p-4">
              <div className="mb-3 flex items-center justify-between gap-2">
                <span className="inline-flex items-center gap-1.5 rounded-sm border-2 border-strong bg-cyan px-2 py-0.5 font-mono text-xs font-bold text-cyan-foreground uppercase">
                  <Camera className="size-3.5" aria-hidden="true" />
                  Tap to Capture
                </span>
                <span className="brutal-shadow-sm rounded-sm border-2 border-strong bg-pink px-2.5 py-0.5 font-display text-xs text-pink-foreground uppercase">
                  Bingo!
                </span>
              </div>

              <section
                aria-label="Beispiel einer 3 mal 3 Friendarazzi Bingo-Karte"
                className="grid grid-cols-3 gap-2"
              >
                {DEMO_TILES.map((tile, idx) => (
                  <div
                    key={`${tile.target}-${idx}`}
                    className={
                      "center" in tile && tile.center
                        ? "brutal-shadow-sm flex min-h-20 flex-col justify-between rounded-sm border-2 border-strong bg-primary p-2 text-primary-foreground"
                        : tile.captured
                          ? "brutal-shadow-sm flex min-h-20 flex-col justify-between rounded-sm border-2 border-strong bg-cyan-subtle p-2 text-ink"
                          : "flex min-h-20 flex-col justify-between rounded-sm border-2 border-strong bg-surface p-2 text-ink"
                    }
                  >
                    <div className="flex items-center justify-between gap-1">
                      <span className="rounded-sm border border-strong bg-surface px-1 py-0.5 font-mono text-[9px] font-bold uppercase">
                        {tile.target}
                      </span>
                      {tile.captured ? (
                        <CheckCircle2
                          className="size-3.5 shrink-0 text-success"
                          aria-hidden="true"
                        />
                      ) : null}
                    </div>
                    <p className="mt-1 line-clamp-2 text-[11px] leading-tight font-bold">
                      {tile.text}
                    </p>
                  </div>
                ))}
              </section>
            </div>
          </div>
        </div>
      </section>

      {/* 3-Step Demonstrative Loop */}
      <section
        aria-label="So funktioniert Friendarazzi"
        className="grid gap-4 md:grid-cols-3"
      >
        <Card>
          <CardHeader className="bg-primary text-primary-foreground">
            <div className="flex items-center justify-between">
              <CodeTag tone="surface">01</CodeTag>
              <Users className="size-5" aria-hidden="true" />
            </div>
            <CardTitle className="pt-1 text-primary-foreground">
              Lobby &amp; Code
            </CardTitle>
          </CardHeader>
          <CardContent className="pt-4 text-sm font-medium text-secondary">
            Wähle <strong className="text-ink">3×3</strong>,{" "}
            <strong className="text-ink">4×4</strong> oder{" "}
            <strong className="text-ink">5×5</strong>, stelle den
            Sichtbarkeitsmodus ein und teile den kurzen Einladungscode im
            Gruppenchat.
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="bg-pink text-pink-foreground">
            <div className="flex items-center justify-between">
              <CodeTag tone="surface">02</CodeTag>
              <Sparkles className="size-5" aria-hidden="true" />
            </div>
            <CardTitle className="pt-1 text-pink-foreground">
              Aufträge würfeln
            </CardTitle>
          </CardHeader>
          <CardContent className="pt-4 text-sm font-medium text-secondary">
            Schreibe eigene Prompts oder nutze die{" "}
            <strong className="text-ink">4 Quick-Start Decks</strong> (Night
            Out, Urlaub, WG, Büro). Niemand zieht eigene oder selbst-zielende
            Felder!
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="bg-cyan text-cyan-foreground">
            <div className="flex items-center justify-between">
              <CodeTag tone="surface">03</CodeTag>
              <Trophy className="size-5" aria-hidden="true" />
            </div>
            <CardTitle className="pt-1 text-cyan-foreground">
              Knipsen &amp; Bingo
            </CardTitle>
          </CardHeader>
          <CardContent className="pt-4 text-sm font-medium text-secondary">
            Beweisfotos im Browser hochladen –{" "}
            <strong className="text-ink">Undercover</strong>,{" "}
            <strong className="text-ink">Blurred Teaser</strong> oder{" "}
            <strong className="text-ink">Live-Feed</strong> – inklusive 1:1
            Zeitachse &amp; Goldmedaillen.
          </CardContent>
        </Card>
      </section>
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
  const [inviteCode, setInviteCode] = useState("");

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    const created = await mutations.createGame.mutateAsync({
      title,
      gridSize: Number(gridSize),
      visibilityMode,
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
      {/* Quick Launch: Host or Join */}
      <Card className="bg-primary-subtle">
        <CardContent className="flex flex-wrap items-end justify-between gap-4 pt-6">
          <div className="space-y-2">
            <p className="font-mono text-xs font-bold tracking-wider text-secondary uppercase">
              Schnellstart
            </p>
            <Button size="lg" onClick={() => setCreateOpen(true)}>
              <Plus aria-hidden="true" />
              Neues Spiel hosten
            </Button>
          </div>

          <form
            onSubmit={handleJoinSubmit}
            className="flex flex-wrap items-end gap-2"
          >
            <Field label="Einladungscode">
              <Input
                value={inviteCode}
                onChange={(e) => setInviteCode(e.target.value.toUpperCase())}
                placeholder="z.B. A7K9P2"
                className="w-40 font-mono uppercase"
                maxLength={16}
              />
            </Field>
            <Button type="submit" variant="secondary" size="field">
              <Ticket aria-hidden="true" />
              Beitreten
            </Button>
          </form>
        </CardContent>
      </Card>

      {/* Active Games */}
      <section className="space-y-3">
        <div className="flex items-center gap-3">
          <h2 className="font-display text-lg tracking-tight text-ink">
            Laufende Spiele
          </h2>
          <CodeTag tone="yellow">{active.length}</CodeTag>
        </div>

        {active.length === 0 ? (
          <CalloutNote label="Keine aktive Runde" tone="cyan">
            Erstelle oben ein neues Spiel oder gib einen Einladungscode deiner
            Freunde ein.
          </CalloutNote>
        ) : (
          <div className="grid gap-4 sm:grid-cols-2">
            {active.map((game) => (
              <ActiveGameCard key={game.id} game={game} />
            ))}
          </div>
        )}
      </section>

      <StickerBand checker />

      {/* Completed Games */}
      <section className="space-y-3">
        <div className="flex items-center gap-3">
          <h2 className="font-display text-lg tracking-tight text-ink">
            Vergangene Spiele
          </h2>
          <CodeTag tone="cyan">{completed.length}</CodeTag>
        </div>

        {completed.length === 0 ? (
          <CalloutNote label="Noch kein Archiv">
            Abgeschlossene Spiele inklusive 1:1 Zeitachse und Fotogalerie
            erscheinen hier.
          </CalloutNote>
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
                    <div className="font-bold text-ink">{game.title}</div>
                    <div className="font-mono text-xs text-secondary">
                      {game.gridSize}×{game.gridSize} · {game.playerCount}{" "}
                      Spieler
                    </div>
                  </TableCell>
                  <TableCell className="font-mono text-xs text-secondary tabular-nums">
                    {game.endedAt
                      ? new Date(game.endedAt).toLocaleDateString("de-AT")
                      : new Date(game.createdAt).toLocaleDateString("de-AT")}
                  </TableCell>
                  <TableCell>
                    {game.winner ? (
                      <PlayerBadgeLink user={game.winner} compact />
                    ) : (
                      <Badge variant="outline">Beendet</Badge>
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
          <div className="min-w-0 space-y-1">
            <CardTitle className="truncate">{game.title}</CardTitle>
            <CodeTag tone="cyan">Code: {game.inviteCode}</CodeTag>
          </div>
          <Badge variant="primary">{statusLabel(game.status)}</Badge>
        </div>
      </CardHeader>
      <CardContent className="flex flex-wrap items-center justify-between gap-3 pt-4">
        <div className="flex flex-wrap items-center gap-3 font-mono text-xs font-semibold text-secondary">
          <span className="inline-flex items-center gap-1">
            <Camera className="size-3.5 text-ink" aria-hidden="true" />
            {game.gridSize}×{game.gridSize}
          </span>
          <span className="inline-flex items-center gap-1">
            <Users className="size-3.5 text-ink" aria-hidden="true" />
            {game.playerCount} Spieler
          </span>
          {game.winner ? (
            <span className="inline-flex items-center gap-1 text-ink">
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
      <Skeleton className="h-24 w-full" />
      <div className="grid gap-4 sm:grid-cols-2">
        <Skeleton className="h-36 w-full" />
        <Skeleton className="h-36 w-full" />
      </div>
    </div>
  );
}
