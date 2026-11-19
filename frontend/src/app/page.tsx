"use client";

import {
  ArrowRight,
  Award,
  Camera,
  CheckCircle2,
  Crown,
  Dices,
  Eye,
  EyeOff,
  Lock,
  Plus,
  Sparkles,
  Ticket,
  Trophy,
  Users,
  Zap,
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
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import {
  CalloutNote,
  CodeTag,
  SparkleIcon,
} from "@/components/ui/brutal-elements";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
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
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/cn";
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
    return <GuestLanding />;
  }

  if (phase.status === "checking") {
    return (
      <div className="mx-auto w-full max-w-6xl space-y-6 p-4 sm:p-6">
        <HomeSkeleton />
      </div>
    );
  }

  return <AuthenticatedHome />;
}

// ------------------------------------------------------------------ Guest Landing Page

const INITIAL_DEMO_TILES = [
  { id: 0, target: "Lena", text: "Bestellt noch ein Dessert", checked: true },
  { id: 1, target: "Max", text: "Schaut aufs falsche Handy", checked: false },
  { id: 2, target: "Sam", text: "Macht Gruppen-Selfie", checked: true },
  { id: 3, target: "Jule", text: "Lacht über eigenen Witz", checked: false },
  {
    id: 4,
    target: "FREIFELD",
    text: "Tap to Capture!",
    checked: true,
    center: true,
  },
  { id: 5, target: "Tom", text: "Erklärt wild mit Händen", checked: false },
  { id: 6, target: "Lea", text: "Streichelt fremden Hund", checked: false },
  { id: 7, target: "Ben", text: "Ordert Extra-Pommes", checked: false },
  { id: 8, target: "Kim", text: "Doppelter Peace-Move", checked: true },
] as const;

const WIN_LINES = [
  [0, 1, 2],
  [3, 4, 5],
  [6, 7, 8],
  [0, 3, 6],
  [1, 4, 7],
  [2, 5, 8],
  [0, 4, 8],
  [2, 4, 6],
];

function GuestLanding() {
  const router = useRouter();
  const [quickCode, setQuickCode] = useState("");
  const [checkedSet, setCheckedSet] = useState<Set<number>>(
    () => new Set(INITIAL_DEMO_TILES.filter((t) => t.checked).map((t) => t.id)),
  );

  const hasDemoBingo = WIN_LINES.some((line) =>
    line.every((idx) => checkedSet.has(idx)),
  );

  function toggleDemoTile(id: number) {
    if (id === 4) return;
    setCheckedSet((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  function handleQuickCode(e: React.FormEvent) {
    e.preventDefault();
    const cleaned = quickCode.trim().toUpperCase();
    if (!cleaned) {
      router.push(paths.join);
      return;
    }
    router.push(joinHref(cleaned));
  }

  return (
    <div className="flex flex-col">
      {/* 1. HERO STAGE */}
      <section className="mx-auto w-full max-w-6xl px-4 pt-8 pb-14 sm:px-6 lg:pt-12 lg:pb-16">
        <div className="grid items-center gap-10 lg:grid-cols-[1.1fr_0.9fr]">
          {/* Left Hero Pitch & Easy Getting Started */}
          <div className="space-y-6">
            <div className="flex flex-wrap items-center gap-2.5">
              <span className="brutal-shadow-sm inline-flex items-center gap-1.5 rounded-sm border-2 border-strong bg-cyan px-3 py-1 font-mono text-xs font-bold text-cyan-foreground uppercase">
                <Camera className="size-3.5" aria-hidden="true" />
                The Candid Photo-Hunt Bingo
              </span>
              <SparkleIcon tone="primary" />
              <SparkleIcon tone="accent" className="size-5" />
            </div>

            <div className="space-y-4">
              <h1 className="sr-only">
                Friendarazzi — Das soziale Foto-Jagd-Bingo für Freundesgruppen
              </h1>
              <div aria-hidden="true">
                <Wordmark size="hero" />
              </div>
              <p className="font-display text-2xl leading-tight tracking-tight text-ink uppercase sm:text-3xl">
                Erwische deine Freunde in flagranti!
              </p>
              <p className="max-w-xl text-base leading-relaxed font-medium text-secondary sm:text-lg">
                Jede Person schreibt lustige Macken und Insider über die
                anderen. Friendarazzi mischt daraus geheime Bingo-Karten – wer
                seine Freunde zuerst auf Kamera erwischt, holt Gold!
              </p>
            </div>

            {/* Getting Started Controls */}
            <div className="brutal-shadow rounded-md border-[2.5px] border-strong bg-surface p-4 sm:p-5">
              <div className="flex flex-wrap items-center gap-3">
                <Button size="lg" asChild>
                  <Link href="/anmeldung">
                    Anmelden &amp; Loslegen
                    <ArrowRight aria-hidden="true" />
                  </Link>
                </Button>
                <Button variant="secondary" size="lg" asChild>
                  <Link href="/registrierung">Kostenlos registrieren</Link>
                </Button>
              </div>

              <form
                onSubmit={handleQuickCode}
                className="mt-4 flex flex-wrap items-center gap-2 border-t-2 border-subtle pt-4"
              >
                <label htmlFor="landing-code" className="sr-only">
                  Einladungscode eingeben
                </label>
                <Input
                  id="landing-code"
                  value={quickCode}
                  onChange={(e) => setQuickCode(e.target.value.toUpperCase())}
                  placeholder="Code eingeben (z.B. A7K9P2)"
                  className="min-w-44 flex-1 font-mono uppercase"
                  maxLength={16}
                />
                <Button type="submit" variant="secondary" size="field">
                  <Ticket aria-hidden="true" />
                  Lobby beitreten
                </Button>
              </form>
            </div>
          </div>

          {/* Right Interactive Demo Stage + Polaroids */}
          <div className="relative">
            {/* Decorative floating Polaroid card behind */}
            <div
              aria-hidden="true"
              className="brutal-shadow absolute -top-5 -right-3 hidden w-36 rotate-6 rounded-sm border-2 border-strong bg-surface p-2 sm:block"
            >
              <div className="tape-strip mx-auto -mt-4 mb-1.5 h-3.5 w-14 -rotate-3" />
              <div className="flex aspect-square items-center justify-center rounded-sm border-2 border-strong bg-pink">
                <Camera
                  className="size-10 text-pink-foreground"
                  aria-hidden="true"
                />
              </div>
              <p className="mt-1.5 text-center font-mono text-[10px] font-bold text-ink uppercase">
                Click! Erwischt
              </p>
            </div>

            {/* Interactive 3x3 Demo Bingo Board */}
            <div className="brutal-shadow-lg relative rounded-md border-[2.5px] border-strong bg-surface p-4 sm:p-5">
              <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
                <span className="inline-flex items-center gap-1.5 rounded-sm border-2 border-strong bg-primary px-2.5 py-1 font-mono text-xs font-bold text-primary-foreground uppercase">
                  <Zap className="size-3.5" aria-hidden="true" />
                  Klick eine Kachel!
                </span>
                {hasDemoBingo ? (
                  <span className="brutal-shadow-sm rounded-sm border-2 border-strong bg-pink px-3 py-1 font-display text-xs text-pink-foreground uppercase">
                    ★ BINGO! ★
                  </span>
                ) : (
                  <span className="rounded-sm border-2 border-strong bg-cyan px-2.5 py-0.5 font-mono text-xs font-bold text-cyan-foreground uppercase">
                    {checkedSet.size} / 9 Treffer
                  </span>
                )}
              </div>

              <div className="grid grid-cols-3 gap-2">
                {INITIAL_DEMO_TILES.map((tile) => {
                  const isChecked = checkedSet.has(tile.id);
                  const isCenter = "center" in tile && tile.center;
                  return (
                    <button
                      key={tile.id}
                      type="button"
                      onClick={() => toggleDemoTile(tile.id)}
                      className={cn(
                        "brutal-press flex min-h-22 cursor-pointer flex-col justify-between rounded-sm border-2 border-strong p-2 text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus sm:min-h-24",
                        isCenter
                          ? "bg-primary text-primary-foreground"
                          : isChecked
                            ? "bg-cyan-subtle text-ink"
                            : "bg-canvas text-ink hover:bg-primary-subtle",
                      )}
                    >
                      <div className="flex items-center justify-between gap-1">
                        <span className="rounded-sm border border-strong bg-surface px-1 py-0.5 font-mono text-[9px] font-bold text-ink uppercase">
                          {tile.target}
                        </span>
                        {isChecked ? (
                          <CheckCircle2
                            className="size-4 shrink-0 text-success"
                            aria-hidden="true"
                          />
                        ) : (
                          <Camera
                            className="size-3.5 shrink-0 text-secondary"
                            aria-hidden="true"
                          />
                        )}
                      </div>
                      <span className="mt-1.5 line-clamp-2 text-[11px] leading-tight font-bold sm:text-xs">
                        {tile.text}
                      </span>
                    </button>
                  );
                })}
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* 2. HIGH-CONTRAST FEATURE RIBBON */}
      <div className="border-y-[2.5px] border-strong bg-primary py-3 text-primary-foreground">
        <div className="mx-auto flex max-w-6xl flex-wrap items-center justify-center gap-x-6 gap-y-2 px-4 font-display text-xs tracking-wider uppercase sm:text-sm">
          <span>3×3 · 4×4 · 5×5 BRETTER</span>
          <span aria-hidden="true">★</span>
          <span>4 QUICK-START DECKS</span>
          <span aria-hidden="true">★</span>
          <span>UNDERCOVER &amp; LIVE-FEED</span>
          <span aria-hidden="true">★</span>
          <span>1:1 SPIEL-ZEITACHSE</span>
          <span aria-hidden="true">★</span>
          <span>GOLDMEDAILLEN &amp; RÄNGE</span>
        </div>
      </div>

      {/* 3. HOW IT PLAYS — 4-STEP STORYBOARD */}
      <section
        id="so-funktionierts"
        className="mx-auto w-full max-w-6xl space-y-8 px-4 py-14 sm:px-6 lg:py-16"
      >
        <div className="max-w-2xl space-y-2">
          <Badge variant="primary">In 4 Schritten zum Bingo</Badge>
          <h2 className="font-display text-2xl tracking-tight text-ink sm:text-4xl">
            So läuft eine Runde Friendarazzi
          </h2>
          <p className="text-base font-medium text-secondary">
            Kein App-Store, kein kompliziertes Setup – einfach Link in den
            Gruppenchat schicken und direkt im mobilen Browser starten.
          </p>
        </div>

        <div className="grid gap-5 sm:grid-cols-2 lg:grid-cols-4">
          <div className="brutal-shadow flex flex-col justify-between rounded-md border-[2.5px] border-strong bg-surface p-5">
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <CodeTag tone="yellow">01</CodeTag>
                <Users className="size-5 text-ink" aria-hidden="true" />
              </div>
              <h3 className="font-display text-lg text-ink">
                Lobby &amp; Einladung
              </h3>
              <p className="text-sm font-medium text-secondary">
                Host wählt Rastergröße (3×3 bis 5×5) und teilt den kurzen
                Einladungscode per Link.
              </p>
            </div>
            <div className="mt-5 rounded-sm border-2 border-strong bg-primary-subtle px-3 py-2 font-mono text-xs font-bold text-ink">
              CODE: A7K9P2
            </div>
          </div>

          <div className="brutal-shadow flex flex-col justify-between rounded-md border-[2.5px] border-strong bg-surface p-5">
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <CodeTag tone="pink">02</CodeTag>
                <Dices className="size-5 text-ink" aria-hidden="true" />
              </div>
              <h3 className="font-display text-lg text-ink">
                Prompts &amp; Decks
              </h3>
              <p className="text-sm font-medium text-secondary">
                Eigene Beobachtungen schreiben oder 4 Quick-Start Decks (*Night
                Out*, *Urlaub*, *WG*, *Büro*) auswürfeln.
              </p>
            </div>
            <div className="mt-5 rounded-sm border-2 border-strong bg-pink-subtle px-3 py-2 font-mono text-xs font-bold text-ink">
              Konfliktfrei gemischt
            </div>
          </div>

          <div className="brutal-shadow flex flex-col justify-between rounded-md border-[2.5px] border-strong bg-surface p-5">
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <CodeTag tone="cyan">03</CodeTag>
                <Camera className="size-5 text-ink" aria-hidden="true" />
              </div>
              <h3 className="font-display text-lg text-ink">Die Foto-Jagd</h3>
              <p className="text-sm font-medium text-secondary">
                Kachel antippen, Beweisfoto direkt mit der Handykamera knipsen
                und unauffällig hochladen.
              </p>
            </div>
            <div className="mt-5 rounded-sm border-2 border-strong bg-cyan-subtle px-3 py-2 font-mono text-xs font-bold text-ink">
              Kamera im Browser
            </div>
          </div>

          <div className="brutal-shadow flex flex-col justify-between rounded-md border-[2.5px] border-strong bg-surface p-5">
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <CodeTag tone="yellow">04</CodeTag>
                <Trophy className="size-5 text-ink" aria-hidden="true" />
              </div>
              <h3 className="font-display text-lg text-ink">
                Bingo &amp; Rückblick
              </h3>
              <p className="text-sm font-medium text-secondary">
                Reihe voll? Goldmedaille kassieren, Superlatives feiern und über
                die 1:1 Zeitachse scrollen!
              </p>
            </div>
            <div className="mt-5 rounded-sm border-2 border-strong bg-surface-muted px-3 py-2 font-mono text-xs font-bold text-ink">
              +1 Goldmedaille
            </div>
          </div>
        </div>
      </section>

      {/* 4. VISIBILITY MODES SHOWCASE */}
      <section
        id="modi"
        className="border-y-[2.5px] border-strong bg-surface py-14 lg:py-16"
      >
        <div className="mx-auto w-full max-w-6xl space-y-8 px-4 sm:px-6">
          <div className="max-w-2xl space-y-2">
            <Badge variant="info">Stealth-Level</Badge>
            <h2 className="font-display text-2xl tracking-tight text-ink sm:text-4xl">
              Wähle deinen Modus
            </h2>
            <p className="text-base font-medium text-secondary">
              Von maximal geheim bis zum offenen Live-Trash-Talk – der Host legt
              fest, was Freunde während des Spiels sehen.
            </p>
          </div>

          <div className="grid gap-6 md:grid-cols-3">
            <div className="brutal-shadow flex flex-col justify-between rounded-md border-[2.5px] border-strong bg-primary-subtle p-6">
              <div className="space-y-3">
                <div className="inline-flex items-center gap-2 rounded-sm border-2 border-strong bg-primary px-2.5 py-1 font-mono text-xs font-bold text-primary-foreground uppercase">
                  <EyeOff className="size-4" aria-hidden="true" />
                  Undercover (Standard)
                </div>
                <h3 className="font-display text-xl text-ink">
                  Maximale Geheimhaltung
                </h3>
                <p className="text-sm font-medium text-ink">
                  Alle Schnappschüsse bleiben streng geheim bis jemand Bingo
                  ruft – ideal für eine große Enthüllung am Ende.
                </p>
              </div>
              <div className="mt-6 flex items-center gap-2 rounded-sm border-2 border-strong bg-surface p-3 font-mono text-xs font-bold text-ink">
                <Lock className="size-4 shrink-0" aria-hidden="true" />
                Enthüllung erst bei Bingo!
              </div>
            </div>

            <div className="brutal-shadow flex flex-col justify-between rounded-md border-[2.5px] border-strong bg-pink-subtle p-6">
              <div className="space-y-3">
                <div className="inline-flex items-center gap-2 rounded-sm border-2 border-strong bg-pink px-2.5 py-1 font-mono text-xs font-bold text-pink-foreground uppercase">
                  <Sparkles className="size-4" aria-hidden="true" />
                  Blurred Teaser
                </div>
                <h3 className="font-display text-xl text-ink">
                  Spannung ohne Spoiler
                </h3>
                <p className="text-sm font-medium text-ink">
                  Die Gruppe sieht stark verpixelte Vorschaubilder samt
                  Zeitstempel – niemand weiß, wer erwischt wurde!
                </p>
              </div>
              <div className="mt-6 flex items-center gap-2 rounded-sm border-2 border-strong bg-surface p-3 font-mono text-xs font-bold text-ink">
                <Eye className="size-4 shrink-0" aria-hidden="true" />
                Pixel-Teaser + Zeitstempel
              </div>
            </div>

            <div className="brutal-shadow flex flex-col justify-between rounded-md border-[2.5px] border-strong bg-cyan-subtle p-6">
              <div className="space-y-3">
                <div className="inline-flex items-center gap-2 rounded-sm border-2 border-strong bg-cyan px-2.5 py-1 font-mono text-xs font-bold text-cyan-foreground uppercase">
                  <Eye className="size-4" aria-hidden="true" />
                  Live-Feed
                </div>
                <h3 className="font-display text-xl text-ink">
                  Sofortige Action
                </h3>
                <p className="text-sm font-medium text-ink">
                  Jedes Beweisfoto landet augenblicklich im Live-Stream –
                  perfekt für direkte Reaktionen und Rivalitäten.
                </p>
              </div>
              <div className="mt-6 flex items-center gap-2 rounded-sm border-2 border-strong bg-surface p-3 font-mono text-xs font-bold text-ink">
                <Camera className="size-4 shrink-0" aria-hidden="true" />
                Live-Stream &amp; Alerts
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* 5. FINAL CTA BANNER */}
      <section
        id="loslegen"
        className="mx-auto w-full max-w-6xl px-4 py-14 sm:px-6 lg:py-16"
      >
        <div className="brutal-shadow-lg flex flex-col items-start justify-between gap-6 rounded-md border-[2.5px] border-strong bg-primary p-6 text-primary-foreground sm:p-10 md:flex-row md:items-center">
          <div className="space-y-2">
            <p className="font-mono text-xs font-bold tracking-wider uppercase">
              Bereit für den nächsten Abend?
            </p>
            <h2 className="font-display text-2xl tracking-tight sm:text-4xl">
              Lobby öffnen &amp; Freunde jagen!
            </h2>
            <p className="max-w-xl text-sm font-bold sm:text-base">
              Melde dich an, erstelle in Sekunden ein Konto oder spring direkt
              mit einem Einladungscode in eine laufende Runde.
            </p>
          </div>
          <div className="flex flex-wrap gap-3">
            <Button variant="secondary" size="lg" asChild>
              <Link href="/anmeldung">Anmelden</Link>
            </Button>
            <Button variant="secondary" size="lg" asChild>
              <Link href="/registrierung">Konto erstellen</Link>
            </Button>
          </div>
        </div>
      </section>

      {/* FOOTER */}
      <footer className="border-t-[2.5px] border-strong bg-surface py-6">
        <div className="mx-auto flex max-w-6xl flex-wrap items-center justify-between gap-4 px-4 sm:px-6">
          <Wordmark size="sm" showMark />
          <p className="font-mono text-xs font-bold text-secondary">
            THE CANDID PHOTO-HUNT BINGO · OPEN SOURCE (AGPL-3.0)
          </p>
        </div>
      </footer>
    </div>
  );
}

// ------------------------------------------------------------------ Authenticated Cockpit

const GRID_OPTIONS = [
  { size: "3", title: "3 × 3", subtitle: "Schnell (9 Felder)" },
  { size: "4", title: "4 × 4", subtitle: "Mittel (16 Felder)" },
  { size: "5", title: "5 × 5", subtitle: "Marathon (25 Felder)" },
] as const;

const MODE_OPTIONS = [
  {
    mode: VisibilityMode.Undercover,
    label: "Undercover",
    hint: "Bis Bingo geheim",
    icon: EyeOff,
  },
  {
    mode: VisibilityMode.BlurredTeaser,
    label: "Blurred Teaser",
    hint: "Verpixelter Hinweis",
    icon: Sparkles,
  },
  {
    mode: VisibilityMode.LiveFeed,
    label: "Live-Feed",
    hint: "Sofort sichtbar",
    icon: Eye,
  },
] as const;

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

  async function handleJoinSubmit(e: React.FormEvent) {
    e.preventDefault();
    const trimmed = inviteCode.trim().toUpperCase();
    if (!trimmed) {
      router.push(paths.join);
      return;
    }
    try {
      const joined = await mutations.joinGame.mutateAsync(trimmed);
      router.push(gameHref(joined.id));
    } catch {
      router.push(joinHref(trimmed));
    }
  }

  if (games.isLoading) {
    return (
      <div className="mx-auto w-full max-w-6xl space-y-6 p-4 sm:p-6">
        <HomeSkeleton />
      </div>
    );
  }

  const all = games.data ?? [];
  const active = all.filter((g) => g.status !== GameStatus.Completed);
  const completed = all.filter((g) => g.status === GameStatus.Completed);

  return (
    <div className="mx-auto w-full max-w-6xl space-y-8 p-4 sm:p-6">
      {/* Quick Launch Cockpit: Host or Join */}
      <section className="brutal-shadow-lg rounded-md border-[2.5px] border-strong bg-surface p-5 sm:p-7">
        <div className="grid items-center gap-6 lg:grid-cols-[1.15fr_0.85fr]">
          <div className="space-y-3">
            <div className="flex flex-wrap items-center gap-2">
              <Badge variant="primary">Spielzentrale</Badge>
              <SparkleIcon tone="accent" className="size-5" />
            </div>
            <h1 className="font-display text-2xl tracking-tight text-ink sm:text-3xl">
              Nächste Runde starten oder beitreten
            </h1>
            <p className="max-w-xl text-sm font-medium text-secondary">
              Eröffne eine neue Foto-Jagd-Lobby als Host oder gib direkt den
              Einladungscode deiner Freunde ein.
            </p>
            <div className="pt-1">
              <Button size="lg" onClick={() => setCreateOpen(true)}>
                <Plus aria-hidden="true" />
                Neues Spiel hosten
              </Button>
            </div>
          </div>

          <form
            onSubmit={(e) => void handleJoinSubmit(e)}
            className="brutal-shadow-sm space-y-3 rounded-md border-2 border-strong bg-cyan-subtle p-4"
          >
            <p className="font-mono text-xs font-bold tracking-wider text-ink uppercase">
              Mit Einladungscode beitreten
            </p>
            <div className="flex flex-wrap gap-2">
              <label htmlFor="cockpit-invite-code" className="sr-only">
                Einladungscode
              </label>
              <Input
                id="cockpit-invite-code"
                value={inviteCode}
                onChange={(e) => setInviteCode(e.target.value.toUpperCase())}
                placeholder="CODE (z.B. A7K9P2)"
                className="min-w-36 flex-1 font-mono font-bold tracking-wider uppercase"
                maxLength={16}
              />
              <Button
                type="submit"
                variant="secondary"
                size="field"
                disabled={mutations.joinGame.isPending}
              >
                <Ticket aria-hidden="true" />
                {mutations.joinGame.isPending ? "Tritt bei …" : "Beitreten"}
              </Button>
            </div>
          </form>
        </div>
      </section>

      {/* Active Games */}
      <section className="space-y-4">
        <div className="flex items-center justify-between gap-3">
          <div className="flex items-center gap-3">
            <h2 className="font-display text-xl tracking-tight text-ink">
              Laufende Spiele
            </h2>
            <CodeTag tone="yellow">{active.length}</CodeTag>
          </div>
        </div>

        {active.length === 0 ? (
          <CalloutNote label="Keine aktive Runde" tone="yellow">
            Erstelle oben ein neues Spiel oder tritt über einen Einladungscode
            einer bestehenden Lobby bei.
          </CalloutNote>
        ) : (
          <div className="grid gap-4 md:grid-cols-2">
            {active.map((game) => (
              <ActiveGameCard key={game.id} game={game} />
            ))}
          </div>
        )}
      </section>

      {/* Completed Games as Memory Cards (replaces dry table) */}
      <section className="space-y-4">
        <div className="flex items-center gap-3">
          <h2 className="font-display text-xl tracking-tight text-ink">
            Vergangene Matches &amp; Erinnerungen
          </h2>
          <CodeTag tone="cyan">{completed.length}</CodeTag>
        </div>

        {completed.length === 0 ? (
          <CalloutNote label="Noch kein Archiv">
            Abgeschlossene Runden mit 1:1 Zeitachse, Superlatives und
            Fotogalerien landen hier.
          </CalloutNote>
        ) : (
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {completed.map((game) => (
              <CompletedGameCard key={game.id} game={game} />
            ))}
          </div>
        )}
      </section>

      {/* Visual Create Game Dialog */}
      <Dialog open={createOpen} onOpenChange={setCreateOpen}>
        <DialogContent className="max-w-xl">
          <DialogHeader>
            <DialogTitle>Neues Friendarazzi-Spiel</DialogTitle>
            <DialogDescription>
              Wähle Titel, Spielfeld-Größe und Stealth-Modus – in der Lobby
              kannst du alles weiter anpassen.
            </DialogDescription>
          </DialogHeader>
          <form className="space-y-5" onSubmit={(e) => void handleCreate(e)}>
            <Field label="Spieltitel">
              <Input
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                required
              />
            </Field>

            <fieldset className="space-y-2">
              <legend className="text-xs font-bold tracking-[0.06em] text-ink uppercase">
                Rastergröße
              </legend>
              <div className="grid grid-cols-3 gap-2">
                {GRID_OPTIONS.map((opt) => {
                  const selected = gridSize === opt.size;
                  return (
                    <button
                      key={opt.size}
                      type="button"
                      aria-pressed={selected}
                      onClick={() => setGridSize(opt.size)}
                      className={cn(
                        "brutal-press flex min-h-16 cursor-pointer flex-col items-start justify-center rounded-sm border-2 border-control p-3 text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus",
                        selected
                          ? "bg-primary text-primary-foreground"
                          : "bg-surface text-ink hover:bg-surface-muted",
                      )}
                    >
                      <span className="font-display text-base">
                        {opt.title}
                      </span>
                      <span className="text-[11px] font-semibold">
                        {opt.subtitle}
                      </span>
                    </button>
                  );
                })}
              </div>
            </fieldset>

            <fieldset className="space-y-2">
              <legend className="text-xs font-bold tracking-[0.06em] text-ink uppercase">
                Sichtbarkeitsmodus
              </legend>
              <div className="grid gap-2 sm:grid-cols-3">
                {MODE_OPTIONS.map((opt) => {
                  const Icon = opt.icon;
                  const selected = visibilityMode === opt.mode;
                  return (
                    <button
                      key={opt.mode}
                      type="button"
                      aria-pressed={selected}
                      onClick={() => setVisibilityMode(opt.mode)}
                      className={cn(
                        "brutal-press flex min-h-16 cursor-pointer flex-col items-start justify-center gap-1 rounded-sm border-2 border-control p-3 text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus",
                        selected
                          ? "bg-cyan text-cyan-foreground"
                          : "bg-surface text-ink hover:bg-surface-muted",
                      )}
                    >
                      <span className="inline-flex items-center gap-1.5 text-xs font-bold uppercase">
                        <Icon className="size-3.5" aria-hidden="true" />
                        {opt.label}
                      </span>
                      <span className="text-[11px] font-medium">
                        {opt.hint}
                      </span>
                    </button>
                  );
                })}
              </div>
            </fieldset>

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
  const ctaLabel =
    game.status === GameStatus.Active
      ? "Brett öffnen"
      : game.status === GameStatus.Drafting
        ? "Zum Entwurf"
        : "Zur Lobby";

  return (
    <Card className="flex flex-col justify-between overflow-hidden">
      <div className="flex items-start justify-between gap-3 border-b-2 border-strong bg-primary-subtle px-5 py-4">
        <div className="min-w-0 space-y-1">
          <h3 className="truncate font-display text-lg text-ink">
            {game.title}
          </h3>
          <div className="flex flex-wrap items-center gap-1.5">
            <CodeTag tone="surface">Code: {game.inviteCode}</CodeTag>
          </div>
        </div>
        <Badge variant="primary">{statusLabel(game.status)}</Badge>
      </div>
      <CardContent className="flex flex-wrap items-center justify-between gap-3 pt-4 pb-4">
        <div className="flex flex-wrap items-center gap-3 font-mono text-xs font-bold text-secondary">
          <span className="inline-flex items-center gap-1">
            <Camera className="size-3.5 text-ink" aria-hidden="true" />
            {game.gridSize}×{game.gridSize}
          </span>
          <span className="inline-flex items-center gap-1">
            <Users className="size-3.5 text-ink" aria-hidden="true" />
            {game.playerCount} Spieler
          </span>
        </div>
        <Button size="sm" asChild>
          <Link href={gameHref(game.id)}>
            {ctaLabel}
            <ArrowRight aria-hidden="true" />
          </Link>
        </Button>
      </CardContent>
    </Card>
  );
}

function CompletedGameCard({ game }: { game: GameSummary }) {
  const dateStr = game.endedAt
    ? new Date(game.endedAt).toLocaleDateString("de-AT")
    : new Date(game.createdAt).toLocaleDateString("de-AT");

  return (
    <Card className="flex flex-col justify-between overflow-hidden">
      <div className="border-b-2 border-strong bg-surface-muted px-5 py-3.5">
        <div className="flex items-center justify-between gap-2">
          <span className="font-mono text-xs font-bold text-secondary tabular-nums">
            {dateStr}
          </span>
          <CodeTag tone="surface">
            {game.gridSize}×{game.gridSize} · {game.playerCount}P
          </CodeTag>
        </div>
        <h3 className="mt-1 truncate font-display text-base text-ink">
          {game.title}
        </h3>
      </div>

      <CardContent className="flex flex-col justify-between gap-4 pt-4 pb-4">
        {game.winner ? (
          <div className="flex items-center gap-2">
            <Crown
              className="size-4 shrink-0 text-warning"
              aria-hidden="true"
            />
            <div className="min-w-0 flex-1">
              <PlayerBadgeLink user={game.winner} compact />
            </div>
          </div>
        ) : (
          <span className="inline-flex items-center gap-1.5 font-mono text-xs font-bold text-secondary">
            <Award className="size-4" aria-hidden="true" />
            Runde beendet
          </span>
        )}
        <Button variant="secondary" size="sm" className="w-full" asChild>
          <Link href={gameHref(game.id)}>Rückblick ansehen</Link>
        </Button>
      </CardContent>
    </Card>
  );
}

function HomeSkeleton() {
  return (
    <div aria-busy="true" className="space-y-4">
      <Skeleton className="h-36 w-full" />
      <div className="grid gap-4 md:grid-cols-2">
        <Skeleton className="h-36 w-full" />
        <Skeleton className="h-36 w-full" />
      </div>
    </div>
  );
}
