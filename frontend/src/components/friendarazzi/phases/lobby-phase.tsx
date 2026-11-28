"use client";

import { Check, Eye, EyeOff, Loader2, Sparkles, Users } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

import { apiErrorMessage } from "@/api/errors";
import { type GameDetail, VisibilityMode } from "@/api/friendarazzi-api";
import { useBenutzerQuery, useFriendarazziMutations } from "@/api/hooks";
import { InviteShareCard } from "@/components/friendarazzi/invite-share-card";
import { PlayerBadgeLink } from "@/components/friendarazzi/player-avatar";
import { Alert } from "@/components/ui/alert";
import { CalloutNote } from "@/components/ui/brutal-elements";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Field } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/cn";

import type { FlushPendingSaveFn } from "./shared";

const LOBBY_GRIDS = [
  { size: "3", title: "3 × 3", sub: "Empfohlen · 1–2 Std." },
  { size: "4", title: "4 × 4", sub: "Abend (16 Felder)" },
  { size: "5", title: "5 × 5", sub: "Wochenende (25)" },
] as const;

const LOBBY_MODES = [
  {
    mode: VisibilityMode.Undercover,
    title: "Undercover",
    desc: "Bis Bingo geheim",
    icon: EyeOff,
  },
  {
    mode: VisibilityMode.BlurredTeaser,
    title: "Blurred Teaser",
    desc: "Verpixelt ohne Ziel",
    icon: Sparkles,
  },
  {
    mode: VisibilityMode.LiveFeed,
    title: "Live-Feed",
    desc: "Sofort im Feed",
    icon: Eye,
  },
] as const;

export function LobbyPhase({
  game,
  registerFlush,
}: {
  game: GameDetail;
  registerFlush?: (fn: FlushPendingSaveFn | null) => void;
}) {
  const me = useBenutzerQuery();
  const mutations = useFriendarazziMutations();
  const { mutateAsync: updateSettingsAsync, isPending: isSavingSettings } =
    mutations.updateSettings;
  const isHost = me.data?.id === game.hostUserId;

  const [title, setTitle] = useState(game.title);
  const [gridSize, setGridSize] = useState(String(game.gridSize));
  const [visibilityMode, setVisibilityMode] = useState<VisibilityMode>(
    game.visibilityMode,
  );
  const [justSaved, setJustSaved] = useState(false);
  const [hasPendingDebounce, setHasPendingDebounce] = useState(false);

  const isEditingTitleRef = useRef(false);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const startDraftingRef = useRef(false);
  const settingsSaveRef = useRef<Promise<boolean>>(Promise.resolve(true));
  const latestValuesRef = useRef({
    title: game.title,
    gridSize: String(game.gridSize),
    visibilityMode: game.visibilityMode,
  });
  const savedSettingsRef = useRef({
    title: game.title,
    gridSize: String(game.gridSize),
    visibilityMode: game.visibilityMode,
  });
  const lastSavedRef = useRef(savedSettingsRef.current);

  useEffect(() => {
    if (!isEditingTitleRef.current) {
      setTitle(game.title);
      latestValuesRef.current.title = game.title;
    }
    setGridSize(String(game.gridSize));
    setVisibilityMode(game.visibilityMode);
    latestValuesRef.current.gridSize = String(game.gridSize);
    latestValuesRef.current.visibilityMode = game.visibilityMode;
    savedSettingsRef.current = {
      title: game.title,
      gridSize: String(game.gridSize),
      visibilityMode: game.visibilityMode,
    };
    lastSavedRef.current = savedSettingsRef.current;
  }, [game.title, game.gridSize, game.visibilityMode]);

  useEffect(() => {
    if (!justSaved) return;
    const timer = setTimeout(() => setJustSaved(false), 1800);
    return () => clearTimeout(timer);
  }, [justSaved]);

  const commitSettings = useCallback(
    (nextTitle: string, nextGridSize: string, nextMode: VisibilityMode) => {
      setHasPendingDebounce(false);
      const trimmedTitle = nextTitle.trim();
      if (!trimmedTitle) return settingsSaveRef.current;
      const next = {
        title: trimmedTitle,
        gridSize: nextGridSize,
        visibilityMode: nextMode,
      };
      const prev = lastSavedRef.current;
      if (
        prev.title === next.title &&
        prev.gridSize === next.gridSize &&
        prev.visibilityMode === next.visibilityMode
      ) {
        return settingsSaveRef.current;
      }

      lastSavedRef.current = next;
      settingsSaveRef.current = settingsSaveRef.current.then(async () => {
        try {
          await updateSettingsAsync({
            gameId: game.id,
            input: {
              title: next.title,
              gridSize: Number(next.gridSize),
              visibilityMode: next.visibilityMode,
            },
          });
          savedSettingsRef.current = next;
          setJustSaved(true);
          return true;
        } catch {
          if (lastSavedRef.current === next) {
            lastSavedRef.current = savedSettingsRef.current;
          }
          return false;
        }
      });
      return settingsSaveRef.current;
    },
    [game.id, updateSettingsAsync],
  );

  const flushPending = useCallback(async () => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
      debounceRef.current = null;
    }
    isEditingTitleRef.current = false;
    const latest = latestValuesRef.current;
    await commitSettings(latest.title, latest.gridSize, latest.visibilityMode);
  }, [commitSettings]);

  useEffect(() => {
    registerFlush?.(isHost ? flushPending : null);
    return () => {
      registerFlush?.(null);
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
        debounceRef.current = null;
        const latest = latestValuesRef.current;
        void commitSettings(
          latest.title,
          latest.gridSize,
          latest.visibilityMode,
        );
      }
    };
  }, [isHost, registerFlush, flushPending, commitSettings]);

  const hasUnsaved = hasPendingDebounce || isSavingSettings;
  useEffect(() => {
    if (!isHost || !hasUnsaved) return;
    const handler = (event: BeforeUnloadEvent) => {
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", handler);
    return () => window.removeEventListener("beforeunload", handler);
  }, [isHost, hasUnsaved]);

  const canStart = game.players.length >= 3;
  const openSlots = Math.max(0, 3 - game.players.length);

  function handleTitleChange(val: string) {
    isEditingTitleRef.current = true;
    setTitle(val);
    latestValuesRef.current.title = val;
    setHasPendingDebounce(true);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      debounceRef.current = null;
      isEditingTitleRef.current = false;
      void commitSettings(
        val,
        latestValuesRef.current.gridSize,
        latestValuesRef.current.visibilityMode,
      );
    }, 500);
  }

  async function handleStartDrafting() {
    if (startDraftingRef.current) return;
    startDraftingRef.current = true;
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
      debounceRef.current = null;
    }
    isEditingTitleRef.current = false;
    try {
      if (!(await commitSettings(title, gridSize, visibilityMode))) return;
      await mutations.startDrafting.mutateAsync(game.id);
    } catch {
      // The mutation state renders the existing error message.
    } finally {
      startDraftingRef.current = false;
    }
  }

  return (
    <div className="grid items-start gap-6 lg:grid-cols-[1.05fr_0.95fr]">
      {/* Left Column: Invite & Crew Roster */}
      <div className="space-y-6">
        <InviteShareCard inviteCode={game.inviteCode} />

        <Card>
          <CardHeader className="bg-pink-subtle py-4">
            <CardTitle className="flex items-center gap-2">
              <Users className="size-5" aria-hidden="true" />
              Mitspieler ({game.players.length})
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-3 pt-4">
            <ul className="divide-y divide-subtle">
              {game.players.map((player) => (
                <li key={player.id} className="py-2 first:pt-0 last:pb-0">
                  <PlayerBadgeLink user={player} compact />
                </li>
              ))}
            </ul>
            {!canStart ? (
              <p className="border-t-2 border-subtle pt-3 font-mono text-xs font-bold text-secondary">
                Noch {openSlots} weitere(r) Mitspieler nötig (min. 3).
              </p>
            ) : null}
          </CardContent>
        </Card>
      </div>

      {/* Right Column: Host Console or Guest Waiting Note */}
      {isHost ? (
        <Card>
          <CardHeader className="bg-cyan-subtle">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <CardTitle>Einstellungen</CardTitle>
              <output
                aria-live="polite"
                className="inline-flex items-center gap-1 font-mono text-[11px] font-bold text-secondary"
              >
                {isSavingSettings ? (
                  <>
                    <Loader2
                      className="size-3.5 animate-spin"
                      aria-hidden="true"
                    />
                    Speichert …
                  </>
                ) : hasPendingDebounce ? (
                  <span>Nicht gespeichert</span>
                ) : justSaved ? (
                  <>
                    <Check
                      className="size-3.5 text-success"
                      aria-hidden="true"
                    />
                    Gespeichert
                  </>
                ) : null}
              </output>
            </div>
          </CardHeader>
          <CardContent className="space-y-5 pt-5">
            <div className="space-y-5">
              <Field label="Spieltitel">
                <Input
                  value={title}
                  onChange={(e) => handleTitleChange(e.target.value)}
                  onBlur={() => {
                    isEditingTitleRef.current = false;
                    if (debounceRef.current) {
                      clearTimeout(debounceRef.current);
                      debounceRef.current = null;
                    }
                    void commitSettings(title, gridSize, visibilityMode);
                  }}
                  required
                />
              </Field>

              <fieldset className="space-y-2">
                <legend className="text-xs font-bold tracking-[0.06em] text-ink uppercase">
                  Rastergröße
                </legend>
                <div className="grid grid-cols-3 gap-2">
                  {LOBBY_GRIDS.map((opt) => {
                    const selected = gridSize === opt.size;
                    return (
                      <button
                        key={opt.size}
                        type="button"
                        aria-pressed={selected}
                        onClick={() => {
                          setGridSize(opt.size);
                          latestValuesRef.current.gridSize = opt.size;
                          void commitSettings(title, opt.size, visibilityMode);
                        }}
                        className={cn(
                          "brutal-press flex min-h-14 cursor-pointer flex-col items-start justify-center rounded-sm border-2 border-control p-2.5 text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus",
                          selected
                            ? "bg-primary text-primary-foreground"
                            : "bg-surface text-ink hover:bg-surface-muted",
                        )}
                      >
                        <span className="font-display text-sm">
                          {opt.title}
                        </span>
                        <span className="text-[11px] font-semibold">
                          {opt.sub}
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
                <div className="grid gap-2">
                  {LOBBY_MODES.map((opt) => {
                    const Icon = opt.icon;
                    const selected = visibilityMode === opt.mode;
                    return (
                      <button
                        key={opt.mode}
                        type="button"
                        aria-pressed={selected}
                        onClick={() => {
                          setVisibilityMode(opt.mode);
                          latestValuesRef.current.visibilityMode = opt.mode;
                          void commitSettings(title, gridSize, opt.mode);
                        }}
                        className={cn(
                          "brutal-press flex min-h-12 cursor-pointer items-center justify-between gap-3 rounded-sm border-2 border-control px-3 py-2 text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus",
                          selected
                            ? "bg-cyan text-cyan-foreground"
                            : "bg-surface text-ink hover:bg-surface-muted",
                        )}
                      >
                        <span className="inline-flex items-center gap-2 text-sm font-bold">
                          <Icon className="size-4" aria-hidden="true" />
                          {opt.title}
                        </span>
                        <span className="font-mono text-xs font-semibold">
                          {opt.desc}
                        </span>
                      </button>
                    );
                  })}
                </div>
              </fieldset>

              {mutations.updateSettings.isError ? (
                <Alert variant="error">
                  {apiErrorMessage(mutations.updateSettings.error, {
                    fallback: "Einstellungen konnten nicht gespeichert werden.",
                  })}
                </Alert>
              ) : null}

              {mutations.startDrafting.isError ? (
                <Alert variant="error">
                  {apiErrorMessage(mutations.startDrafting.error, {
                    fallback: "Entwurfsphase konnte nicht gestartet werden.",
                  })}
                </Alert>
              ) : null}

              <div className="border-t-2 border-strong pt-4">
                <Button
                  type="button"
                  size="lg"
                  className="w-full"
                  disabled={!canStart || mutations.startDrafting.isPending}
                  onClick={() => void handleStartDrafting()}
                >
                  {mutations.startDrafting.isPending
                    ? "Startet …"
                    : "Entwurfsphase starten"}
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>
      ) : (
        <CalloutNote label="Warten auf Host" tone="yellow">
          Der Host startet die Entwurfsphase, sobald mindestens 3 Mitspieler
          dabei sind.
        </CalloutNote>
      )}
    </div>
  );
}
