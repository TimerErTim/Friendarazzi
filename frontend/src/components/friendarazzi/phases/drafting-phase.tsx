"use client";

import { useQueryClient } from "@tanstack/react-query";
import {
  Check,
  CheckCircle2,
  Dices,
  Loader2,
  Lock,
  LockOpen,
  Sparkles,
  Timer,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { apiErrorMessage } from "@/api/errors";
import {
  type DraftPromptRequestItem,
  type GameDetail,
  PromptSource,
} from "@/api/friendarazzi-api";
import {
  useBenutzerQuery,
  useDecksQuery,
  useErfolgsmeldung,
  useFriendarazziMutations,
} from "@/api/hooks";
import { friendarazziKeys } from "@/api/query-keys";
import {
  PlayerAvatar,
  PlayerBadgeLink,
} from "@/components/friendarazzi/player-avatar";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { CodeTag } from "@/components/ui/brutal-elements";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Progress } from "@/components/ui/progress";
import { cn } from "@/lib/cn";
import { autofillEmptySlots, rerollSingleSlot } from "@/lib/draft-prompts";
import { useGameCountdown } from "@/lib/use-game-countdown";

import type { FlushPendingSaveFn } from "./shared";

type DraftSlot = {
  key: string;
  targetUserId: string;
  targetName: string;
  text: string;
  source: PromptSource;
};

function toPromptPayload(list: DraftSlot[]): DraftPromptRequestItem[] {
  return list
    .filter((s) => s.text.trim().length > 0)
    .map((s) => ({
      targetUserId: s.targetUserId,
      text: s.text.trim(),
      source: s.source,
    }));
}

export function DraftingPhase({
  game,
  detailUpdatedAt = 0,
  registerFlush,
}: {
  game: GameDetail;
  detailUpdatedAt?: number;
  registerFlush?: (fn: FlushPendingSaveFn | null) => void;
}) {
  const me = useBenutzerQuery();
  const decks = useDecksQuery();
  const queryClient = useQueryClient();
  const mutations = useFriendarazziMutations();
  const { mutateAsync: savePromptsAsync, isPending: isSavingDraft } =
    mutations.savePrompts;
  const feedback = useErfolgsmeldung();

  const myPlayer = game.players.find((p) => p.id === me.data?.id);
  const isLocked = myPlayer?.draftLocked ?? false;

  const [selectedDeckSlug, setSelectedDeckSlug] = useState<string>("night-out");
  const [slots, setSlots] = useState<DraftSlot[]>([]);
  const [justSavedDraft, setJustSavedDraft] = useState(false);
  const [hasPendingDebounce, setHasPendingDebounce] = useState(false);
  const [deckNotice, setDeckNotice] = useState<string | null>(null);

  const initializedRef = useRef(false);
  const draftDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastDigestRef = useRef<string>("");
  const latestSlotsRef = useRef<DraftSlot[]>([]);
  const saveChainRef = useRef<Promise<boolean>>(Promise.resolve(true));
  const isLockedRef = useRef(isLocked);
  isLockedRef.current = isLocked;

  const countdownSeconds = useGameCountdown(
    game.draftClosesAt,
    game.serverNow,
    detailUpdatedAt,
    () => {
      void queryClient.invalidateQueries({
        queryKey: friendarazziKeys.games.detail(game.id),
      });
      void queryClient.invalidateQueries({
        queryKey: friendarazziKeys.games.all(),
      });
    },
  );

  useEffect(() => {
    if (initializedRef.current && slots.length > 0) return;
    const savedByTarget = new Map<
      string,
      Array<{ text: string; source: PromptSource }>
    >();
    for (const draft of game.myDrafts) {
      const list = savedByTarget.get(draft.targetUserId) ?? [];
      list.push({ text: draft.text, source: draft.source });
      savedByTarget.set(draft.targetUserId, list);
    }

    const built: DraftSlot[] = [];
    for (const quota of game.myQuotas) {
      const existing = savedByTarget.get(quota.targetUser.id) ?? [];
      for (let i = 0; i < quota.requiredCount; i += 1) {
        const saved = existing[i];
        built.push({
          key: `${quota.targetUser.id}-${i}`,
          targetUserId: quota.targetUser.id,
          targetName: quota.targetUser.displayName,
          text: saved?.text ?? "",
          source: saved?.source ?? PromptSource.Custom,
        });
      }
    }
    setSlots(built);
    latestSlotsRef.current = built;
    if (built.length > 0) {
      initializedRef.current = true;
      lastDigestRef.current = JSON.stringify(toPromptPayload(built));
    }
  }, [game.myQuotas, game.myDrafts, slots.length]);

  useEffect(() => {
    if (!justSavedDraft) return;
    const timer = setTimeout(() => setJustSavedDraft(false), 1800);
    return () => clearTimeout(timer);
  }, [justSavedDraft]);

  const commitDraft = useCallback(
    (nextSlots: DraftSlot[]): Promise<boolean> => {
      setHasPendingDebounce(false);
      if (isLockedRef.current) return saveChainRef.current;
      const payload = toPromptPayload(nextSlots);
      const digest = JSON.stringify(payload);
      if (digest === lastDigestRef.current) return saveChainRef.current;

      const prevDigest = lastDigestRef.current;
      lastDigestRef.current = digest;
      saveChainRef.current = saveChainRef.current.then(async () => {
        try {
          await savePromptsAsync({ gameId: game.id, prompts: payload });
          setJustSavedDraft(true);
          return true;
        } catch {
          if (lastDigestRef.current === digest) {
            lastDigestRef.current = prevDigest;
          }
          return false;
        }
      });
      return saveChainRef.current;
    },
    [game.id, savePromptsAsync],
  );

  const flushPending = useCallback(async () => {
    if (draftDebounceRef.current) {
      clearTimeout(draftDebounceRef.current);
      draftDebounceRef.current = null;
    }
    await commitDraft(latestSlotsRef.current);
  }, [commitDraft]);

  useEffect(() => {
    registerFlush?.(flushPending);
    return () => {
      registerFlush?.(null);
      if (draftDebounceRef.current) {
        clearTimeout(draftDebounceRef.current);
        draftDebounceRef.current = null;
        void commitDraft(latestSlotsRef.current);
      }
    };
  }, [registerFlush, flushPending, commitDraft]);

  const hasUnsaved = hasPendingDebounce || isSavingDraft;
  useEffect(() => {
    if (isLocked || !hasUnsaved) return;
    const handler = (event: BeforeUnloadEvent) => {
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", handler);
    return () => window.removeEventListener("beforeunload", handler);
  }, [isLocked, hasUnsaved]);

  function scheduleDraftSave(nextSlots: DraftSlot[]) {
    if (isLocked) return;
    setHasPendingDebounce(true);
    if (draftDebounceRef.current) clearTimeout(draftDebounceRef.current);
    draftDebounceRef.current = setTimeout(() => {
      draftDebounceRef.current = null;
      void commitDraft(nextSlots);
    }, 600);
  }

  const activeDeck = useMemo(() => {
    const list = decks.data ?? [];
    return list.find((d) => d.slug === selectedDeckSlug) ?? list[0];
  }, [decks.data, selectedDeckSlug]);

  function autofillEmpty() {
    if (!activeDeck || activeDeck.prompts.length === 0) return;
    const {
      slots: nextSlots,
      filledCount: addedCount,
      remainingEmptyCount,
    } = autofillEmptySlots(slots, activeDeck.prompts, (slot, text) => ({
      ...slot,
      text,
      source: PromptSource.StarterDeck,
    }));

    if (addedCount > 0) {
      setSlots(nextSlots);
      latestSlotsRef.current = nextSlots;
      if (draftDebounceRef.current) {
        clearTimeout(draftDebounceRef.current);
        draftDebounceRef.current = null;
      }
      void commitDraft(nextSlots);
    }

    if (remainingEmptyCount > 0) {
      setDeckNotice(
        addedCount > 0
          ? `Es sind keine weiteren eindeutigen Vorschläge im Deck „${activeDeck.name}“ übrig. Wechsle das Deck oder formuliere die übrigen ${remainingEmptyCount} Felder selbst.`
          : `Im Deck „${activeDeck.name}“ gibt es keine unbenutzten Vorschläge mehr. Wechsle das Deck oder schreibe eigene Aufträge.`,
      );
    } else {
      setDeckNotice(null);
    }
  }

  function rerollSlot(key: string) {
    if (!activeDeck || activeDeck.prompts.length === 0) return;
    const { slots: nextSlots, changed } = rerollSingleSlot(
      slots,
      key,
      activeDeck.prompts,
      (slot, text) => ({
        ...slot,
        text,
        source: PromptSource.StarterDeck,
      }),
    );

    if (!changed) {
      setDeckNotice(
        `Im Deck „${activeDeck.name}“ gibt es für dieses Feld keinen weiteren eindeutigen Vorschlag. Wechsle das Deck oder passe den Text manuell an.`,
      );
      return;
    }

    setDeckNotice(null);
    setSlots(nextSlots);
    latestSlotsRef.current = nextSlots;
    if (draftDebounceRef.current) {
      clearTimeout(draftDebounceRef.current);
      draftDebounceRef.current = null;
    }
    void commitDraft(nextSlots);
  }

  const allFilled =
    slots.length > 0 && slots.every((s) => s.text.trim().length > 0);
  const filledCount = slots.filter((s) => s.text.trim().length > 0).length;

  async function handleLock() {
    feedback.verwerfen();
    if (draftDebounceRef.current) {
      clearTimeout(draftDebounceRef.current);
      draftDebounceRef.current = null;
    }
    const saved = await commitDraft(latestSlotsRef.current);
    if (!saved) return;
    try {
      await mutations.lockPrompts.mutateAsync(game.id);
      feedback.melden("Aufträge eingeloggt!");
    } catch {
      // Mutation error state is rendered via mutations.lockPrompts.error.
    }
  }

  async function handleUnlock() {
    feedback.verwerfen();
    try {
      await mutations.unlockPrompts.mutateAsync(game.id);
      feedback.melden(
        "Bereitschaft zurückgezogen – du kannst wieder bearbeiten.",
      );
    } catch {
      // Mutation error state is rendered via mutations.unlockPrompts.error.
    }
  }

  const countdownPct =
    countdownSeconds !== null
      ? Math.max(0, Math.min(100, Math.round((countdownSeconds / 60) * 100)))
      : 0;

  return (
    <div className="space-y-6">
      {countdownSeconds !== null ? (
        <section
          aria-live="polite"
          className="brutal-shadow-lg space-y-3 rounded-md border-[2.5px] border-strong bg-primary-subtle p-5"
        >
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div className="flex items-center gap-3">
              <span className="flex size-10 items-center justify-center rounded-sm border-2 border-strong bg-primary text-primary-foreground">
                <Timer className="size-5" aria-hidden="true" />
              </span>
              <div>
                <h2 className="font-display text-lg text-ink">
                  Alle bereit – Spielstart in{" "}
                  <span className="font-mono tabular-nums">
                    {countdownSeconds}s
                  </span>
                </h2>
                <p className="text-xs font-medium text-secondary">
                  Die Bingo-Bretter werden ausgeteilt, sobald eine volle Minute
                  ohne Änderung abläuft.
                </p>
              </div>
            </div>
            {isLocked ? (
              <Button
                type="button"
                variant="secondary"
                disabled={mutations.unlockPrompts.isPending}
                onClick={() => void handleUnlock()}
              >
                <LockOpen className="size-4" aria-hidden="true" />
                {mutations.unlockPrompts.isPending
                  ? "Wird freigegeben …"
                  : "Bereitschaft zurückziehen"}
              </Button>
            ) : null}
          </div>
          <Progress value={countdownPct} aria-label="Start-Countdown" />
        </section>
      ) : null}

      {/* Crew Status + Quick-Start Deck Picker */}
      <div className="grid gap-6 lg:grid-cols-[0.85fr_1.15fr]">
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between gap-2">
              <CardTitle>Crew-Status</CardTitle>
              <CodeTag tone="yellow">
                {game.players.filter((p) => p.draftLocked).length} /{" "}
                {game.players.length} bereit
              </CodeTag>
            </div>
          </CardHeader>
          <CardContent className="pt-4">
            <ul className="flex flex-wrap gap-2">
              {game.players.map((p) => (
                <li
                  key={p.id}
                  className={cn(
                    "inline-flex items-center gap-2 rounded-sm border-2 border-strong px-2.5 py-1 text-xs font-bold",
                    p.draftLocked
                      ? "bg-success/15 text-ink"
                      : "bg-surface-muted text-secondary",
                  )}
                >
                  <PlayerAvatar user={p} size="sm" showMedalBadge={false} />
                  <span>{p.displayName}</span>
                  {p.draftLocked ? (
                    <CheckCircle2
                      className="size-4 text-success"
                      aria-hidden="true"
                    />
                  ) : null}
                </li>
              ))}
            </ul>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="bg-cyan-subtle">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <CardTitle className="flex items-center gap-2">
                <Sparkles className="size-5" aria-hidden="true" />
                Quick-Start Decks
              </CardTitle>
              <Button
                type="button"
                variant="secondary"
                size="sm"
                disabled={isLocked || !activeDeck}
                onClick={autofillEmpty}
              >
                <Dices className="size-4" aria-hidden="true" />
                Leere Felder füllen
              </Button>
            </div>
          </CardHeader>
          <CardContent className="space-y-3 pt-4">
            <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
              {(decks.data ?? []).map((deck) => {
                const selected = deck.slug === selectedDeckSlug;
                return (
                  <button
                    key={deck.id}
                    type="button"
                    disabled={isLocked}
                    aria-pressed={selected}
                    onClick={() => {
                      setSelectedDeckSlug(deck.slug);
                      setDeckNotice(null);
                    }}
                    className={cn(
                      "brutal-press flex min-h-12 cursor-pointer flex-col justify-center rounded-sm border-2 border-control px-2.5 py-2 text-left disabled:cursor-not-allowed disabled:opacity-50",
                      "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus",
                      selected
                        ? "bg-primary text-primary-foreground"
                        : "bg-surface text-ink hover:bg-surface-muted",
                    )}
                  >
                    <span className="truncate text-xs font-bold">
                      {deck.name}
                    </span>
                  </button>
                );
              })}
            </div>
            {activeDeck ? (
              <p className="text-xs font-medium text-secondary">
                {activeDeck.description}
              </p>
            ) : null}
            {deckNotice ? <Alert variant="info">{deckNotice}</Alert> : null}
          </CardContent>
        </Card>
      </div>

      {/* Grouped Prompt Workshop */}
      <Card>
        <CardHeader className="bg-surface-muted">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <CardTitle>Aufträge für deine Mitspieler</CardTitle>
            <span className="font-mono text-xs font-bold text-ink tabular-nums">
              {filledCount} / {slots.length} ausgefüllt
            </span>
          </div>
        </CardHeader>
        <CardContent className="divide-y-2 divide-subtle pt-2">
          {game.myQuotas.map((quota) => {
            const targetSlots = slots.filter(
              (s) => s.targetUserId === quota.targetUser.id,
            );
            const targetFilled = targetSlots.filter((s) =>
              s.text.trim(),
            ).length;

            return (
              <section key={quota.targetUser.id} className="space-y-3 py-5">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <PlayerBadgeLink user={quota.targetUser} />
                  <Badge
                    variant={
                      targetFilled === quota.requiredCount
                        ? "success"
                        : "default"
                    }
                  >
                    {targetFilled} / {quota.requiredCount} Aufträge
                  </Badge>
                </div>
                <div className="grid gap-3 md:grid-cols-2">
                  {targetSlots.map((slot, index) => (
                    <div key={slot.key} className="flex items-center gap-2">
                      <CodeTag tone="yellow" className="w-8 justify-center">
                        {index + 1}
                      </CodeTag>
                      <Input
                        value={slot.text}
                        disabled={isLocked}
                        placeholder={`z.B. ${quota.targetUser.displayName} bestellt noch ein Getränk`}
                        aria-label={`Auftrag ${index + 1} für ${quota.targetUser.displayName}`}
                        onChange={(e) => {
                          const val = e.target.value;
                          const nextSlots = slots.map((item) =>
                            item.key === slot.key
                              ? {
                                  ...item,
                                  text: val,
                                  source: PromptSource.Custom,
                                }
                              : item,
                          );
                          setDeckNotice(null);
                          setSlots(nextSlots);
                          latestSlotsRef.current = nextSlots;
                          scheduleDraftSave(nextSlots);
                        }}
                        onBlur={() => {
                          if (draftDebounceRef.current) {
                            clearTimeout(draftDebounceRef.current);
                            draftDebounceRef.current = null;
                          }
                          void commitDraft(latestSlotsRef.current);
                        }}
                      />
                      <Button
                        type="button"
                        variant="secondary"
                        size="icon"
                        disabled={isLocked || !activeDeck}
                        aria-label={`Zufälliger Vorschlag für ${quota.targetUser.displayName}`}
                        title="Aus Deck neu auswürfeln"
                        onClick={() => rerollSlot(slot.key)}
                      >
                        <Dices className="size-4" aria-hidden="true" />
                      </Button>
                    </div>
                  ))}
                </div>
              </section>
            );
          })}
        </CardContent>
      </Card>

      {mutations.savePrompts.isError ? (
        <Alert variant="error">
          {apiErrorMessage(mutations.savePrompts.error, {
            fallback: "Aufträge konnten nicht gespeichert werden.",
          })}
        </Alert>
      ) : null}

      {mutations.lockPrompts.isError ? (
        <Alert variant="error">
          {apiErrorMessage(mutations.lockPrompts.error, {
            fallback: "Aufträge konnten nicht eingeloggt werden.",
          })}
        </Alert>
      ) : null}

      {mutations.unlockPrompts.isError ? (
        <Alert variant="error">
          {apiErrorMessage(mutations.unlockPrompts.error, {
            fallback: "Bereitschaft konnte nicht zurückgezogen werden.",
          })}
        </Alert>
      ) : null}

      {isLocked && countdownSeconds === null ? (
        <Alert variant="success" icon={<CheckCircle2 className="size-4" />}>
          <div className="flex flex-wrap items-center justify-between gap-3">
            <span>
              Deine Aufträge ({filledCount}/{slots.length}) sind eingeloggt.
              Sobald alle bereit sind, startet ein 60-Sekunden-Countdown vor
              Spielbeginn.
            </span>
            <Button
              type="button"
              variant="secondary"
              size="sm"
              disabled={mutations.unlockPrompts.isPending}
              onClick={() => void handleUnlock()}
            >
              <LockOpen className="size-4" aria-hidden="true" />
              {mutations.unlockPrompts.isPending
                ? "Wird freigegeben …"
                : "Bereitschaft zurückziehen"}
            </Button>
          </div>
        </Alert>
      ) : !isLocked ? (
        <div className="brutal-shadow-lg sticky bottom-20 z-20 flex flex-wrap items-center justify-between gap-3 rounded-md border-[2.5px] border-strong bg-surface p-4 md:bottom-4">
          <div className="flex flex-wrap items-center gap-3">
            <CodeTag tone="cyan">
              {filledCount} / {slots.length}
            </CodeTag>
            <span className="text-sm font-bold text-ink">
              Aufträge ausgefüllt
            </span>
            <output
              aria-live="polite"
              className="inline-flex items-center gap-1 font-mono text-[11px] font-bold text-secondary"
            >
              {isSavingDraft ? (
                <>
                  <Loader2
                    className="size-3.5 animate-spin"
                    aria-hidden="true"
                  />
                  Speichert …
                </>
              ) : hasPendingDebounce ? (
                <span>Nicht gespeichert</span>
              ) : justSavedDraft ? (
                <>
                  <Check className="size-3.5 text-success" aria-hidden="true" />
                  Gespeichert
                </>
              ) : null}
            </output>
          </div>
          <Button
            type="button"
            disabled={
              !allFilled ||
              mutations.savePrompts.isPending ||
              mutations.lockPrompts.isPending
            }
            onClick={() => void handleLock()}
          >
            <Lock className="size-4" aria-hidden="true" />
            {mutations.lockPrompts.isPending
              ? "Wird eingeloggt …"
              : "Einloggen & Bereit melden"}
          </Button>
        </div>
      ) : null}
    </div>
  );
}
