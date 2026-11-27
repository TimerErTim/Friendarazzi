"use client";

import { useQueryClient } from "@tanstack/react-query";
import {
  Check,
  CheckCircle2,
  Dices,
  Loader2,
  Lock,
  LockOpen,
  Timer,
  Users,
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
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { Progress } from "@/components/ui/progress";
import { Select } from "@/components/ui/select";
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
  const readyCount = game.players.filter((p) => p.draftLocked).length;

  return (
    <div className="space-y-4">
      {countdownSeconds !== null ? (
        <section
          aria-live="polite"
          className="brutal-shadow space-y-2.5 rounded-md border-[2.5px] border-strong bg-primary-subtle p-4"
        >
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div className="flex items-center gap-2.5">
              <span className="flex size-9 items-center justify-center rounded-sm border-2 border-strong bg-primary text-primary-foreground">
                <Timer className="size-4" aria-hidden="true" />
              </span>
              <div>
                <h2 className="font-display text-base text-ink sm:text-lg">
                  Alle bereit – Start in{" "}
                  <span className="font-mono tabular-nums">
                    {countdownSeconds}s
                  </span>
                </h2>
                <p className="text-xs font-medium text-secondary">
                  Bretter werden verteilt, sobald 60 Sek. nichts mehr geändert
                  wird.
                </p>
              </div>
            </div>
            {isLocked ? (
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
                  : "Zurückziehen"}
              </Button>
            ) : null}
          </div>
          <Progress value={countdownPct} aria-label="Start-Countdown" />
        </section>
      ) : null}

      {/* Single compact toolbar above prompt fields: Deck Quick-Fill + Crew Status Popover */}
      <div className="brutal-shadow-sm flex flex-wrap items-center justify-between gap-2 rounded-md border-2 border-strong bg-cyan-subtle p-2.5">
        <div className="flex min-w-0 flex-1 flex-wrap items-center gap-2">
          <div className="min-w-44 flex-1 sm:max-w-56">
            <Select
              aria-label="Quick-Start Deck wählen"
              disabled={isLocked || (decks.data ?? []).length === 0}
              value={selectedDeckSlug}
              onChange={(e) => {
                setSelectedDeckSlug(e.target.value);
                setDeckNotice(null);
              }}
            >
              {(decks.data ?? []).map((deck) => (
                <option key={deck.id} value={deck.slug}>
                  Deck: {deck.name}
                </option>
              ))}
            </Select>
          </div>
          <Button
            type="button"
            variant="secondary"
            size="sm"
            disabled={isLocked || !activeDeck}
            onClick={autofillEmpty}
          >
            <Dices className="size-4" aria-hidden="true" />
            Leere füllen
          </Button>
        </div>

        <Popover>
          <PopoverTrigger asChild>
            <Button type="button" variant="secondary" size="sm">
              <Users className="size-4" aria-hidden="true" />
              <span className="font-mono tabular-nums">
                {readyCount}/{game.players.length} bereit
              </span>
            </Button>
          </PopoverTrigger>
          <PopoverContent align="end" className="w-64 p-3">
            <p className="mb-2 font-mono text-[11px] font-bold tracking-wider text-secondary uppercase">
              Crew-Bereitschaft ({readyCount}/{game.players.length})
            </p>
            <ul className="divide-y divide-subtle">
              {game.players.map((p) => (
                <li
                  key={p.id}
                  className="flex items-center justify-between gap-2 py-1.5 text-xs font-bold first:pt-0 last:pb-0"
                >
                  <span className="flex min-w-0 items-center gap-2">
                    <PlayerAvatar
                      user={p}
                      size="sm"
                      linkToProfile={false}
                      showMedalBadge={false}
                    />
                    <span className="truncate text-ink">{p.displayName}</span>
                  </span>
                  {p.draftLocked ? (
                    <CheckCircle2
                      className="size-4 shrink-0 text-success"
                      aria-label="Bereit"
                    />
                  ) : (
                    <span className="font-mono text-[11px] text-secondary">
                      offen
                    </span>
                  )}
                </li>
              ))}
            </ul>
          </PopoverContent>
        </Popover>
      </div>

      {deckNotice ? <Alert variant="info">{deckNotice}</Alert> : null}

      {/* Grouped Prompt Workshop — right at the top on mobile */}
      <div className="space-y-4">
        {game.myQuotas.map((quota) => {
          const targetSlots = slots.filter(
            (s) => s.targetUserId === quota.targetUser.id,
          );
          const targetFilled = targetSlots.filter((s) => s.text.trim()).length;

          return (
            <Card key={quota.targetUser.id} className="overflow-hidden">
              <CardHeader className="bg-primary-subtle px-4 py-2.5 sm:px-6">
                <div className="flex items-center justify-between gap-2">
                  <div className="flex min-w-0 items-center gap-2">
                    <span className="shrink-0 rounded-sm border border-strong bg-primary px-1.5 py-0.5 font-mono text-[10px] font-bold tracking-wider text-primary-foreground uppercase">
                      Ziel
                    </span>
                    <PlayerBadgeLink user={quota.targetUser} compact />
                  </div>
                  <span
                    className={cn(
                      "shrink-0 rounded-sm border border-strong px-2 py-0.5 font-mono text-xs font-bold tabular-nums",
                      targetFilled === quota.requiredCount
                        ? "border-success bg-success/15 text-ink"
                        : "bg-surface text-ink",
                    )}
                  >
                    {targetFilled}/{quota.requiredCount}
                  </span>
                </div>
              </CardHeader>
              <CardContent className="px-4 pt-3.5 pb-4 sm:px-6">
                <div className="grid gap-2.5 md:grid-cols-2">
                  {targetSlots.map((slot, index) => (
                    <div key={slot.key} className="flex items-center gap-2">
                      <span className="w-7 shrink-0 rounded-sm border border-strong bg-primary-subtle py-1 text-center font-mono text-xs font-bold text-ink tabular-nums">
                        {index + 1}
                      </span>
                      <Input
                        value={slot.text}
                        disabled={isLocked}
                        placeholder="z.B. Bestellt noch ein Getränk"
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
              </CardContent>
            </Card>
          );
        })}
      </div>

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
              Aufträge eingeloggt ({filledCount}/{slots.length}). Der
              60-Sekunden-Countdown startet, sobald alle bereit sind.
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
                : "Zurückziehen"}
            </Button>
          </div>
        </Alert>
      ) : !isLocked ? (
        <div className="brutal-shadow-lg sticky bottom-20 z-20 flex flex-wrap items-center justify-between gap-3 rounded-md border-[2.5px] border-strong bg-surface p-3.5 md:bottom-4">
          <div className="flex flex-wrap items-center gap-3">
            <span className="font-mono text-sm font-bold text-ink tabular-nums">
              {filledCount} / {slots.length} ausgefüllt
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
