"use client";

import {
  ArrowRight,
  Camera,
  CheckCircle2,
  Dices,
  Eye,
  EyeOff,
  LayoutGrid,
  List,
  Lock,
  Pin,
  Plus,
  Sparkles,
  Ticket,
  Trophy,
  Users,
  X,
  Zap,
} from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useMemo, useState } from "react";

import { useAuth } from "@/api/auth-provider";
import { apiErrorMessage } from "@/api/errors";
import {
  type BoardTileView,
  type CaptureFeedItem,
  GameStatus,
  type GameSummary,
  VisibilityMode,
} from "@/api/friendarazzi-api";
import {
  useFriendarazziMutations,
  useGameDetailsQueries,
  useGamesQuery,
} from "@/api/hooks";
import { Wordmark } from "@/components/brand/wordmark";
import { BingoBoard } from "@/components/friendarazzi/bingo-board";
import { CaptureTileDialog } from "@/components/friendarazzi/capture-tile-dialog";
import { PhotoLightbox } from "@/components/friendarazzi/photo-lightbox";
import { TargetGroupedBoard } from "@/components/friendarazzi/target-grouped-board";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import {
  CalloutNote,
  CodeTag,
  SparkleIcon,
} from "@/components/ui/brutal-elements";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
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
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import {
  type BoardViewMode,
  loadBoardViewMode,
  loadPinnedTiles,
  saveBoardViewMode,
  savePinnedTiles,
  togglePinnedTile,
} from "@/lib/board-grouping";
import { cn } from "@/lib/cn";
import { sortActiveGames } from "@/lib/game-overview";
import { gameHref, joinHref, paths } from "@/lib/navigation";

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
          <div className="brutal-shadow rounded-md border-[2.5px] border-strong bg-surface p-5">
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
          </div>

          <div className="brutal-shadow rounded-md border-[2.5px] border-strong bg-surface p-5">
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
          </div>

          <div className="brutal-shadow rounded-md border-[2.5px] border-strong bg-surface p-5">
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
          </div>

          <div className="brutal-shadow rounded-md border-[2.5px] border-strong bg-surface p-5">
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
  { size: "3", title: "3 × 3", subtitle: "Abend (9 Felder)" },
  { size: "4", title: "4 × 4", subtitle: "Mittel (16 Felder)" },
  { size: "5", title: "5 × 5", subtitle: "Wochen (25 Felder)" },
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
  const [boardViewMode, setBoardViewMode] = useState<BoardViewMode>(() =>
    loadBoardViewMode(),
  );
  const [pinnedByGame, setPinnedByGame] = useState<Record<string, string[]>>(
    {},
  );
  const [captureTarget, setCaptureTarget] = useState<{
    gameId: string;
    tile: BoardTileView;
  } | null>(null);
  const [lightboxCapture, setLightboxCapture] =
    useState<CaptureFeedItem | null>(null);

  const all = games.data ?? [];
  const active = useMemo(
    () => sortActiveGames(all.filter((g) => g.status !== GameStatus.Completed)),
    [all],
  );
  const runningActiveGames = useMemo(
    () => active.filter((g) => g.status === GameStatus.Active),
    [active],
  );
  const activeGameIds = useMemo(
    () => runningActiveGames.map((g) => g.id),
    [runningActiveGames],
  );
  const activeGameIdsKey = activeGameIds.join(",");

  const refreshPinnedFromStorage = useCallback(() => {
    const ids = activeGameIdsKey ? activeGameIdsKey.split(",") : [];
    const next: Record<string, string[]> = {};
    for (const id of ids) {
      const pins = loadPinnedTiles(id);
      if (pins.length > 0) {
        next[id] = pins;
      }
    }
    setPinnedByGame(next);
  }, [activeGameIdsKey]);

  useEffect(() => {
    refreshPinnedFromStorage();
    if (typeof window === "undefined") return;
    window.addEventListener("focus", refreshPinnedFromStorage);
    window.addEventListener("storage", refreshPinnedFromStorage);
    return () => {
      window.removeEventListener("focus", refreshPinnedFromStorage);
      window.removeEventListener("storage", refreshPinnedFromStorage);
    };
  }, [refreshPinnedFromStorage]);

  const activeDetailsQueries = useGameDetailsQueries(activeGameIds);

  const pinnedTasks: Array<{ game: GameSummary; tile: BoardTileView }> = [];
  let isLoadingPinned = false;
  runningActiveGames.forEach((game, index) => {
    const pinIds = pinnedByGame[game.id] ?? [];
    if (pinIds.length === 0) return;
    const query = activeDetailsQueries[index];
    if (!query) return;
    if (query.isLoading && !query.data) {
      isLoadingPinned = true;
      return;
    }
    const board = query.data?.myBoard;
    if (!board) return;
    const tileById = new Map(board.tiles.map((t) => [t.id, t]));
    for (const tileId of pinIds) {
      const tile = tileById.get(tileId);
      if (tile && !tile.isCaptured) {
        pinnedTasks.push({ game, tile });
      }
    }
  });

  function handleViewModeChange(next: string) {
    if (next !== "grid" && next !== "list") return;
    setBoardViewMode(next);
    saveBoardViewMode(next);
  }

  function handleTogglePinTile(gameId: string, tileId: string) {
    setPinnedByGame((prev) => {
      const current = prev[gameId] ?? loadPinnedTiles(gameId);
      const nextForGame = togglePinnedTile(current, tileId);
      savePinnedTiles(gameId, nextForGame);
      return { ...prev, [gameId]: nextForGame };
    });
  }

  function handleTileCaptured(gameId: string, tileId: string) {
    setPinnedByGame((prev) => {
      const current = prev[gameId] ?? loadPinnedTiles(gameId);
      if (!current.includes(tileId)) return prev;
      const nextForGame = current.filter((id) => id !== tileId);
      savePinnedTiles(gameId, nextForGame);
      return { ...prev, [gameId]: nextForGame };
    });
  }

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

  const hasPendingLobbies = active.length > runningActiveGames.length;

  return (
    <div className="mx-auto w-full max-w-6xl space-y-5 px-3 py-4 sm:space-y-6 sm:p-6">
      {/* Compact Spielzentrale card: create CTA + single-line join form straight beneath */}
      <section className="brutal-shadow space-y-3 rounded-md border-[2.5px] border-strong bg-surface p-4">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h1 className="font-display text-xl tracking-tight text-ink sm:text-2xl">
            Spielzentrale
          </h1>
          <Button size="md" onClick={() => setCreateOpen(true)}>
            <Plus aria-hidden="true" />
            Neues Spiel hosten
          </Button>
        </div>

        <form
          onSubmit={(e) => void handleJoinSubmit(e)}
          className="flex items-center gap-2"
        >
          <label htmlFor="cockpit-invite-code" className="sr-only">
            Mit Einladungscode beitreten
          </label>
          <Input
            id="cockpit-invite-code"
            value={inviteCode}
            onChange={(e) => setInviteCode(e.target.value.toUpperCase())}
            placeholder="Code (z.B. A7K9P2)"
            className="min-w-0 flex-1 font-mono font-bold tracking-wider uppercase"
            maxLength={16}
          />
          <Button
            type="submit"
            variant="secondary"
            size="field"
            className="shrink-0"
            disabled={mutations.joinGame.isPending}
          >
            <Ticket aria-hidden="true" />
            {mutations.joinGame.isPending ? "Tritt bei …" : "Beitreten"}
          </Button>
        </form>
      </section>

      {/* Pinned Tasks Across All Running Games */}
      {pinnedTasks.length > 0 || isLoadingPinned ? (
        <section className="space-y-2.5">
          <div className="flex items-center gap-2">
            <Pin className="size-4 rotate-45 text-ink" aria-hidden="true" />
            <h2 className="font-display text-lg tracking-tight text-ink sm:text-xl">
              Angeheftete Aufgaben ({pinnedTasks.length})
            </h2>
          </div>

          {pinnedTasks.length === 0 ? (
            <div className="grid gap-3 md:grid-cols-2">
              <Skeleton className="h-20 w-full" />
            </div>
          ) : (
            <div className="grid gap-3 md:grid-cols-2">
              {pinnedTasks.map(({ game, tile }) => (
                <Card
                  key={`${game.id}-${tile.id}`}
                  className="flex items-stretch overflow-hidden"
                >
                  <button
                    type="button"
                    onClick={() => setCaptureTarget({ gameId: game.id, tile })}
                    className="flex min-h-14 min-w-0 flex-1 cursor-pointer items-center justify-between gap-3 px-4 py-3 text-left transition-colors hover:bg-primary-subtle/40 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-focus"
                  >
                    <div className="min-w-0 flex-1 space-y-1">
                      <div className="flex flex-wrap items-center gap-1.5">
                        <span className="rounded-[2px] border border-strong bg-primary px-1.5 py-0.5 font-mono text-[10px] leading-none font-bold text-primary-foreground">
                          {tile.targetUser.displayName}
                        </span>
                        <span className="truncate font-mono text-[11px] font-bold text-secondary">
                          {game.title}
                        </span>
                      </div>
                      <p className="text-sm leading-snug font-bold text-ink">
                        {tile.promptText}
                      </p>
                    </div>
                    <Camera
                      className="size-4 shrink-0 text-secondary"
                      aria-hidden="true"
                    />
                  </button>
                  <button
                    type="button"
                    aria-label={`Auftrag „${tile.promptText}“ aus Fokus entfernen`}
                    title="Aus Fokus entfernen"
                    onClick={() => handleTogglePinTile(game.id, tile.id)}
                    className="inline-flex min-h-11 min-w-11 shrink-0 cursor-pointer items-center justify-center border-l-2 border-strong bg-surface-muted px-3 text-ink transition-colors hover:bg-surface focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-focus"
                  >
                    <X className="size-4" aria-hidden="true" />
                  </button>
                </Card>
              ))}
            </div>
          )}
        </section>
      ) : null}

      {/* Central Playing Hub: All Running Bingo Boards */}
      <section className="space-y-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h2 className="font-display text-lg tracking-tight text-ink sm:text-xl">
            Aktive Bretter ({runningActiveGames.length})
          </h2>
          {runningActiveGames.length > 0 ? (
            <ToggleGroup
              type="single"
              value={boardViewMode}
              onValueChange={handleViewModeChange}
              aria-label="Brett-Ansicht auf Home wählen"
            >
              <ToggleGroupItem value="grid" aria-label="Raster-Ansicht">
                <LayoutGrid className="size-4" aria-hidden="true" />
                <span>Raster</span>
              </ToggleGroupItem>
              <ToggleGroupItem value="list" aria-label="Liste nach Zielperson">
                <List className="size-4" aria-hidden="true" />
                <span>Nach Ziel</span>
              </ToggleGroupItem>
            </ToggleGroup>
          ) : null}
        </div>

        {runningActiveGames.length === 0 ? (
          <CalloutNote
            label={
              hasPendingLobbies
                ? "Noch keine laufende Jagd"
                : "Keine aktive Runde"
            }
            tone={hasPendingLobbies ? "cyan" : "yellow"}
          >
            <div className="flex flex-wrap items-center justify-between gap-3">
              <p>
                {hasPendingLobbies
                  ? "Du hast offene Lobbys oder Entwürfe – starte sie unter Lobbys, sobald alle bereit sind."
                  : "Erstelle oben ein neues Spiel oder tritt über einen Einladungscode einer Lobby bei."}
              </p>
              {hasPendingLobbies ? (
                <Button variant="secondary" size="sm" asChild>
                  <Link href={paths.games}>
                    Zu den Lobbys
                    <ArrowRight aria-hidden="true" />
                  </Link>
                </Button>
              ) : null}
            </div>
          </CalloutNote>
        ) : (
          <div className="space-y-4">
            {runningActiveGames.map((game, index) => {
              const query = activeDetailsQueries[index];
              const detail = query?.data;
              const board = detail?.myBoard;

              if (!detail || !board) {
                return (
                  <div key={game.id} className="space-y-2">
                    <div className="flex items-center justify-between px-1">
                      <Link
                        href={gameHref(game.id)}
                        className="font-display text-base text-ink hover:underline"
                      >
                        {game.title}
                      </Link>
                      <span className="font-mono text-xs font-bold text-secondary">
                        Lädt …
                      </span>
                    </div>
                    <Skeleton className="h-64 w-full" />
                  </div>
                );
              }

              const capturedCount = board.tiles.filter(
                (t) => t.isCaptured,
              ).length;
              const totalCount = board.tiles.length;
              const gamePinnedSet = new Set(
                (pinnedByGame[game.id] ?? []).filter((id) =>
                  board.tiles.some((t) => t.id === id && !t.isCaptured),
                ),
              );

              return (
                <div key={game.id} className="space-y-2">
                  <div className="flex items-center justify-between gap-2 px-0.5">
                    <Link
                      href={gameHref(detail.id)}
                      className="inline-flex min-h-11 min-w-0 items-center gap-1.5 font-display text-base text-ink hover:underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus sm:text-lg"
                    >
                      <span className="truncate">{detail.title}</span>
                      <ArrowRight
                        className="size-4 shrink-0"
                        aria-hidden="true"
                      />
                    </Link>
                    <span className="shrink-0 font-mono text-xs font-bold text-secondary tabular-nums">
                      {capturedCount}/{totalCount}
                    </span>
                  </div>

                  {boardViewMode === "list" ? (
                    <TargetGroupedBoard
                      board={board}
                      feed={detail.feed}
                      pinnedTileIds={gamePinnedSet}
                      onTogglePinTile={(tileId) =>
                        handleTogglePinTile(detail.id, tileId)
                      }
                      hideCaptured
                      onEmptyTileSelect={(tile) =>
                        setCaptureTarget({ gameId: detail.id, tile })
                      }
                      onViewCapture={(cap) => setLightboxCapture(cap)}
                    />
                  ) : (
                    <BingoBoard
                      board={board}
                      gridSize={detail.gridSize}
                      feed={detail.feed}
                      pinnedTileIds={gamePinnedSet}
                      onEmptyTileSelect={(tile) =>
                        setCaptureTarget({ gameId: detail.id, tile })
                      }
                      onViewCapture={(cap) => setLightboxCapture(cap)}
                    />
                  )}
                </div>
              );
            })}
          </div>
        )}
      </section>

      <CaptureTileDialog
        gameId={captureTarget?.gameId ?? ""}
        tile={captureTarget?.tile ?? null}
        isPinned={
          captureTarget
            ? (pinnedByGame[captureTarget.gameId] ?? []).includes(
                captureTarget.tile.id,
              )
            : false
        }
        onTogglePin={(tileId) => {
          if (captureTarget) {
            handleTogglePinTile(captureTarget.gameId, tileId);
          }
        }}
        onClose={() => setCaptureTarget(null)}
        onCaptured={(tileId) => {
          if (captureTarget) {
            handleTileCaptured(captureTarget.gameId, tileId);
          }
        }}
      />

      <PhotoLightbox
        capture={lightboxCapture}
        open={Boolean(lightboxCapture)}
        onOpenChange={(open) => {
          if (!open) {
            setLightboxCapture(null);
          }
        }}
      />

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
