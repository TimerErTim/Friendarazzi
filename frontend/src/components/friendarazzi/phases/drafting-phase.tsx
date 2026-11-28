"use client";

import { useQueryClient } from "@tanstack/react-query";
import {
  Check,
  CheckCircle2,
  Clock,
  Dices,
  Loader2,
  Lock,
  LockOpen,
  LogOut,
  Shield,
  Sparkles,
  Timer,
  UserMinus,
  Users,
  XCircle,
  Zap,
} from "lucide-react";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { apiErrorMessage } from "@/api/errors";
import {
  type DraftPromptRequestItem,
  type GameDetail,
  type PlayerBadge,
  PromptSource,
} from "@/api/friendarazzi-api";
import {
  useBenutzerQuery,
  useDecksQuery,
  useErfolgsmeldung,
  useFriendarazziMutations,
} from "@/api/hooks";
import { friendarazziKeys } from "@/api/query-keys";
import { formatDeadlineShort } from "@/components/friendarazzi/draft-deadline-picker";
import {
  PlayerAvatar,
  PlayerBadgeLink,
} from "@/components/friendarazzi/player-avatar";
import { Alert } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { Progress } from "@/components/ui/progress";
import { Select } from "@/components/ui/select";
import { cn } from "@/lib/cn";
import {
  ALL_DECKS_SLUG,
  autofillEmptySlots,
  mergeDeckTemplates,
  nextPromptSuggestions,
  PROMPT_MAX_CHARS,
  PROMPT_WARN_CHARS,
  rerollSingleSlot,
} from "@/lib/draft-prompts";
import { paths } from "@/lib/navigation";
import { useGameCountdown } from "@/lib/use-game-countdown";

import type { FlushPendingSaveFn } from "./shared";

type DraftSlot = {
  key: string;
  targetUserId: string;
  targetName: string;
  text: string;
  source: PromptSource;
};

function formatRemainingShort(seconds: number): string {
  if (seconds < 60) return `${seconds} Sek.`;
  const mins = Math.ceil(seconds / 60);
  if (mins < 60) return `${mins} Min.`;
  const hrs = Math.floor(mins / 60);
  const remMins = mins % 60;
  return remMins > 0 ? `${hrs} Std. ${remMins} Min.` : `${hrs} Std.`;
}

function deterministicUnit(seed: number): () => number {
  let state = (seed + 1) >>> 0;
  return () => {
    state = (state * 1664525 + 1013904223) >>> 0;
    return state / 0x1_0000_0000;
  };
}

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
  const router = useRouter();
  const me = useBenutzerQuery();
  const decks = useDecksQuery();
  const queryClient = useQueryClient();
  const mutations = useFriendarazziMutations();
  const { mutateAsync: savePromptsAsync, isPending: isSavingDraft } =
    mutations.savePrompts;
  const feedback = useErfolgsmeldung();

  const myPlayer = game.players.find((p) => p.id === me.data?.id);
  const isLocked = myPlayer?.draftLocked ?? false;
  const isHost = me.data?.id === game.hostUserId;

  const [selectedDeckSlug, setSelectedDeckSlug] =
    useState<string>(ALL_DECKS_SLUG);
  const [slots, setSlots] = useState<DraftSlot[]>([]);
  const [focusedSlotKey, setFocusedSlotKey] = useState<string | null>(null);
  const [suggestionSeed, setSuggestionSeed] = useState(0);
  const [hostFillOpen, setHostFillOpen] = useState(false);
  const [leaveConfirmOpen, setLeaveConfirmOpen] = useState(false);
  const [abandonConfirmOpen, setAbandonConfirmOpen] = useState(false);
  const [kickTarget, setKickTarget] = useState<PlayerBadge | null>(null);
  const [transferTarget, setTransferTarget] = useState<PlayerBadge | null>(
    null,
  );
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

  const invalidateDraftDetail = useCallback(() => {
    void queryClient.invalidateQueries({
      queryKey: friendarazziKeys.games.detail(game.id),
    });
    void queryClient.invalidateQueries({
      queryKey: friendarazziKeys.games.all(),
    });
  }, [queryClient, game.id]);

  const countdownSeconds = useGameCountdown(
    game.draftClosesAt,
    game.serverNow,
    detailUpdatedAt,
    invalidateDraftDetail,
  );

  const deadlineSeconds = useGameCountdown(
    game.draftDeadlineAt,
    game.serverNow,
    detailUpdatedAt,
    invalidateDraftDetail,
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
    if (selectedDeckSlug === ALL_DECKS_SLUG) return null;
    return list.find((d) => d.slug === selectedDeckSlug) ?? list[0] ?? null;
  }, [decks.data, selectedDeckSlug]);

  const activePromptPool = useMemo(() => {
    const list = decks.data ?? [];
    if (selectedDeckSlug === ALL_DECKS_SLUG) {
      return mergeDeckTemplates(list);
    }
    return activeDeck?.prompts ?? [];
  }, [decks.data, selectedDeckSlug, activeDeck]);

  const activeDeckLabel =
    selectedDeckSlug === ALL_DECKS_SLUG
      ? "Alle Decks gemischt"
      : (activeDeck?.name ?? "");

  const quickPickSuggestions = useMemo(
    () =>
      nextPromptSuggestions(
        activePromptPool,
        slots,
        4,
        deterministicUnit(suggestionSeed),
      ),
    [activePromptPool, slots, suggestionSeed],
  );

  function applyAutofill(): {
    nextSlots: DraftSlot[];
    allComplete: boolean;
  } | null {
    if (activePromptPool.length === 0) return null;
    const {
      slots: nextSlots,
      filledCount: addedCount,
      remainingEmptyCount,
    } = autofillEmptySlots(slots, activePromptPool, (slot, text) => ({
      ...slot,
      text: text.slice(0, PROMPT_MAX_CHARS),
      source: PromptSource.StarterDeck,
    }));

    if (addedCount > 0) {
      setSlots(nextSlots);
      latestSlotsRef.current = nextSlots;
      if (draftDebounceRef.current) {
        clearTimeout(draftDebounceRef.current);
        draftDebounceRef.current = null;
      }
      setSuggestionSeed((n) => n + 1);
    }

    if (remainingEmptyCount > 0) {
      setDeckNotice(
        addedCount > 0
          ? `Es sind keine weiteren eindeutigen Vorschläge im Deck „${activeDeckLabel}“ übrig. Wechsle das Deck oder formuliere die übrigen ${remainingEmptyCount} Felder selbst.`
          : `Im Deck „${activeDeckLabel}“ gibt es keine unbenutzten Vorschläge mehr. Wechsle das Deck oder schreibe eigene Aufträge.`,
      );
    } else {
      setDeckNotice(null);
    }

    return {
      nextSlots,
      allComplete:
        nextSlots.length > 0 &&
        nextSlots.every((s) => s.text.trim().length > 0),
    };
  }

  function autofillEmpty() {
    const outcome = applyAutofill();
    if (!outcome) return;
    void commitDraft(outcome.nextSlots);
  }

  async function autofillEmptyAndLock() {
    feedback.verwerfen();
    const outcome = applyAutofill();
    if (!outcome || !outcome.allComplete) {
      if (outcome) {
        await commitDraft(outcome.nextSlots);
      }
      return;
    }
    const saved = await commitDraft(outcome.nextSlots);
    if (!saved) return;
    try {
      await mutations.lockPrompts.mutateAsync(game.id);
      feedback.melden("Alle Lücken gefüllt & bereit gemeldet!");
    } catch {
      // Mutation error state is rendered via mutations.lockPrompts.error.
    }
  }

  function applyQuickPick(promptText: string) {
    if (isLocked) return;
    const targetSlot =
      (focusedSlotKey
        ? slots.find((s) => s.key === focusedSlotKey)
        : undefined) ?? slots.find((s) => s.text.trim().length === 0);
    if (!targetSlot) return;

    const trimmed = promptText.trim().slice(0, PROMPT_MAX_CHARS);
    const nextSlots = slots.map((item) =>
      item.key === targetSlot.key
        ? { ...item, text: trimmed, source: PromptSource.StarterDeck }
        : item,
    );
    setDeckNotice(null);
    setSlots(nextSlots);
    latestSlotsRef.current = nextSlots;
    setSuggestionSeed((n) => n + 1);
    if (draftDebounceRef.current) {
      clearTimeout(draftDebounceRef.current);
      draftDebounceRef.current = null;
    }
    void commitDraft(nextSlots);
  }

  function rerollSlot(key: string) {
    if (activePromptPool.length === 0) return;
    const { slots: nextSlots, changed } = rerollSingleSlot(
      slots,
      key,
      activePromptPool,
      (slot, text) => ({
        ...slot,
        text: text.slice(0, PROMPT_MAX_CHARS),
        source: PromptSource.StarterDeck,
      }),
    );

    if (!changed) {
      setDeckNotice(
        `Im Deck „${activeDeckLabel}“ gibt es für dieses Feld keinen weiteren eindeutigen Vorschlag. Wechsle das Deck oder passe den Text manuell an.`,
      );
      return;
    }

    setDeckNotice(null);
    setSlots(nextSlots);
    latestSlotsRef.current = nextSlots;
    setSuggestionSeed((n) => n + 1);
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

  async function handleHostFill(startImmediately: boolean) {
    feedback.verwerfen();
    try {
      await mutations.hostFillDrafts.mutateAsync({
        gameId: game.id,
        startImmediately,
      });
      setHostFillOpen(false);
      feedback.melden(
        startImmediately
          ? "Offene Plätze gefüllt & Spiel gestartet!"
          : "Offene Plätze gefüllt – 60s-Countdown läuft.",
      );
    } catch {
      // Rendered inside dialog / error alerts.
    }
  }

  const countdownPct =
    countdownSeconds !== null
      ? Math.max(0, Math.min(100, Math.round((countdownSeconds / 60) * 100)))
      : 0;
  const readyCount = game.players.filter((p) => p.draftLocked).length;

  return (
    <div className="space-y-4">
      {game.draftDeadlineAt && deadlineSeconds !== null ? (
        <Alert variant="info" icon={<Clock className="size-4" />}>
          <div className="flex flex-wrap items-center justify-between gap-2">
            <span>
              <strong>Entwurfsfrist:</strong>{" "}
              {deadlineSeconds > 0
                ? `noch ${formatRemainingShort(deadlineSeconds)} (${formatDeadlineShort(game.draftDeadlineAt)})`
                : "Frist erreicht – offene Plätze werden gleich automatisch gefüllt."}
            </span>
          </div>
        </Alert>
      ) : null}

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
                setSuggestionSeed((n) => n + 1);
                setDeckNotice(null);
              }}
            >
              {(decks.data ?? []).length > 1 ? (
                <option value={ALL_DECKS_SLUG}>Alle Decks mischen</option>
              ) : null}
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
            disabled={isLocked || activePromptPool.length === 0}
            onClick={autofillEmpty}
          >
            <Dices className="size-4" aria-hidden="true" />
            Leere füllen
          </Button>
          {!isLocked && !allFilled ? (
            <Button
              type="button"
              variant="primary"
              size="sm"
              disabled={
                activePromptPool.length === 0 ||
                mutations.savePrompts.isPending ||
                mutations.lockPrompts.isPending
              }
              onClick={() => void autofillEmptyAndLock()}
            >
              <Zap className="size-4" aria-hidden="true" />
              Leere füllen &amp; bereit
            </Button>
          ) : null}
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
          <PopoverContent align="end" className="w-72 space-y-3 p-3">
            <div>
              <p className="mb-2 font-mono text-[11px] font-bold tracking-wider text-secondary uppercase">
                Crew-Bereitschaft ({readyCount}/{game.players.length})
              </p>
              <ul className="divide-y divide-subtle">
                {game.players.map((p) => {
                  const canManage = isHost && p.id !== me.data?.id;
                  return (
                    <li
                      key={p.id}
                      className="flex items-center justify-between gap-2 py-1.5 text-xs font-bold first:pt-0 last:pb-0"
                    >
                      <span className="flex min-w-0 flex-1 items-center gap-2">
                        <PlayerAvatar
                          user={p}
                          size="sm"
                          linkToProfile={false}
                          showMedalBadge={false}
                        />
                        <span className="truncate text-ink">
                          {p.displayName}
                        </span>
                      </span>
                      <div className="flex shrink-0 items-center gap-1">
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
                        {canManage ? (
                          <>
                            <Button
                              type="button"
                              variant="ghost"
                              size="icon"
                              aria-label={`${p.displayName} zum Host machen`}
                              title="Host übergeben"
                              onClick={() => setTransferTarget(p)}
                            >
                              <Shield className="size-3.5" aria-hidden="true" />
                            </Button>
                            <Button
                              type="button"
                              variant="ghost"
                              size="icon"
                              aria-label={`${p.displayName} entfernen`}
                              title="Entfernen"
                              onClick={() => setKickTarget(p)}
                            >
                              <UserMinus
                                className="size-3.5"
                                aria-hidden="true"
                              />
                            </Button>
                          </>
                        ) : null}
                      </div>
                    </li>
                  );
                })}
              </ul>
            </div>
            {isHost && readyCount < game.players.length ? (
              <div className="border-t-2 border-subtle pt-2">
                <Button
                  type="button"
                  variant="secondary"
                  size="sm"
                  className="w-full"
                  onClick={() => setHostFillOpen(true)}
                >
                  <Sparkles className="size-4" aria-hidden="true" />
                  Offene Plätze automatisch füllen
                </Button>
              </div>
            ) : null}
            <div className="space-y-1.5 border-t-2 border-subtle pt-2">
              <Button
                type="button"
                variant="secondary"
                size="sm"
                className="w-full"
                onClick={() => setLeaveConfirmOpen(true)}
              >
                <LogOut className="size-4" aria-hidden="true" />
                Runde verlassen
              </Button>
              {isHost ? (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="w-full text-error hover:bg-error-subtle"
                  onClick={() => setAbandonConfirmOpen(true)}
                >
                  <XCircle className="size-4" aria-hidden="true" />
                  Spiel abbrechen
                </Button>
              ) : null}
            </div>
          </PopoverContent>
        </Popover>
      </div>

      {!isLocked && quickPickSuggestions.length > 0 ? (
        <section
          aria-label="Schnellauswahl-Vorschläge"
          className="flex flex-wrap items-center gap-2 rounded-md border-2 border-strong bg-surface px-3 py-2"
        >
          <span className="font-mono text-[11px] font-bold tracking-wider text-secondary uppercase">
            Schnellwahl:
          </span>
          {quickPickSuggestions.map((suggestion) => (
            <button
              key={suggestion}
              type="button"
              onClick={() => applyQuickPick(suggestion)}
              className="brutal-press min-h-11 cursor-pointer rounded-sm border-2 border-control bg-primary-subtle px-2.5 py-1 text-left text-xs font-semibold text-ink hover:bg-primary hover:text-primary-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
            >
              + {suggestion}
            </button>
          ))}
          <button
            type="button"
            aria-label="Neue Schnellvorschläge laden"
            onClick={() => setSuggestionSeed((n) => n + 1)}
            className="ml-auto inline-flex min-h-11 cursor-pointer items-center gap-1 rounded-sm border-2 border-control bg-surface px-2.5 py-1 font-mono text-[11px] font-bold text-ink hover:bg-surface-muted focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
          >
            <Dices className="size-3.5" aria-hidden="true" />
            Neu
          </button>
        </section>
      ) : null}

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
                  {targetSlots.map((slot, index) => {
                    const charCount = slot.text.length;
                    const nearLimit = charCount >= PROMPT_WARN_CHARS;
                    return (
                      <div key={slot.key} className="space-y-1">
                        <div className="flex items-center gap-2">
                          <span className="w-7 shrink-0 rounded-sm border border-strong bg-primary-subtle py-1 text-center font-mono text-xs font-bold text-ink tabular-nums">
                            {index + 1}
                          </span>
                          <Input
                            value={slot.text}
                            disabled={isLocked}
                            maxLength={PROMPT_MAX_CHARS}
                            placeholder="z.B. Bestellt noch ein Getränk"
                            aria-label={`Auftrag ${index + 1} für ${quota.targetUser.displayName}`}
                            onFocus={() => setFocusedSlotKey(slot.key)}
                            onChange={(e) => {
                              const val = e.target.value.slice(
                                0,
                                PROMPT_MAX_CHARS,
                              );
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
                            disabled={isLocked || activePromptPool.length === 0}
                            aria-label={`Zufälliger Vorschlag für ${quota.targetUser.displayName}`}
                            title="Aus Deck neu auswürfeln"
                            onClick={() => rerollSlot(slot.key)}
                          >
                            <Dices className="size-4" aria-hidden="true" />
                          </Button>
                        </div>
                        <div className="flex justify-end pr-12">
                          <span
                            className={cn(
                              "font-mono text-[11px] font-bold tabular-nums",
                              nearLimit
                                ? "rounded-xs border border-strong bg-primary-subtle px-1.5 text-ink"
                                : "text-secondary",
                            )}
                          >
                            {charCount}/{PROMPT_MAX_CHARS}
                          </span>
                        </div>
                      </div>
                    );
                  })}
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

      <Dialog open={hostFillOpen} onOpenChange={setHostFillOpen}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Offene Entwürfe automatisch füllen</DialogTitle>
            <DialogDescription>
              Füllt alle noch offenen Plätze aller nicht bereiten Mitspieler mit
              Starter-Decks und loggt sie ein. Bisher geschriebene Aufträge
              bleiben erhalten.
            </DialogDescription>
          </DialogHeader>
          {mutations.hostFillDrafts.isError ? (
            <Alert variant="error">
              {apiErrorMessage(mutations.hostFillDrafts.error, {
                fallback:
                  "Offene Plätze konnten nicht automatisch gefüllt werden.",
              })}
            </Alert>
          ) : null}
          <DialogFooter className="sm:flex-col sm:items-stretch">
            <Button
              type="button"
              variant="secondary"
              disabled={mutations.hostFillDrafts.isPending}
              onClick={() => void handleHostFill(false)}
            >
              Füllen &amp; 60s-Countdown starten
            </Button>
            <Button
              type="button"
              variant="primary"
              disabled={mutations.hostFillDrafts.isPending}
              onClick={() => void handleHostFill(true)}
            >
              Füllen &amp; sofort starten
            </Button>
            <Button
              type="button"
              variant="ghost"
              disabled={mutations.hostFillDrafts.isPending}
              onClick={() => setHostFillOpen(false)}
            >
              Abbrechen
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={leaveConfirmOpen} onOpenChange={setLeaveConfirmOpen}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Runde verlassen?</DialogTitle>
            <DialogDescription>
              {game.players.length <= 3
                ? "Da weniger als 3 Personen übrig bleiben, kehrt das Spiel zurück in die Lobby. Deine bisherigen Entwürfe werden verworfen."
                : "Deine bisherigen Entwürfe und Ziel-Zuordnungen für andere werden bereinigt."}
              {isHost && game.players.length > 1
                ? " Als Host wird deine Rolle automatisch an die nächste Person übertragen."
                : ""}
            </DialogDescription>
          </DialogHeader>
          {mutations.leaveGame.isError ? (
            <Alert variant="error">
              {apiErrorMessage(mutations.leaveGame.error, {
                fallback: "Runde konnte nicht verlassen werden.",
              })}
            </Alert>
          ) : null}
          <DialogFooter>
            <Button
              type="button"
              variant="secondary"
              disabled={mutations.leaveGame.isPending}
              onClick={() => setLeaveConfirmOpen(false)}
            >
              Abbrechen
            </Button>
            <Button
              type="button"
              variant="primary"
              disabled={mutations.leaveGame.isPending}
              onClick={() => {
                void mutations.leaveGame.mutateAsync(game.id).then(() => {
                  setLeaveConfirmOpen(false);
                  router.replace(paths.home);
                });
              }}
            >
              {mutations.leaveGame.isPending ? "Verlässt …" : "Verlassen"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={Boolean(kickTarget)}
        onOpenChange={(open) => {
          if (!open) setKickTarget(null);
        }}
      >
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>{kickTarget?.displayName} entfernen?</DialogTitle>
            <DialogDescription>
              {game.players.length <= 3
                ? "Danach sind weniger als 3 Personen im Spiel, sodass die Runde automatisch in die Lobby zurückkehrt."
                : "Entwürfe von und für diese Person werden entfernt und die Quoten für alle neu berechnet."}
            </DialogDescription>
          </DialogHeader>
          {mutations.kickPlayer.isError ? (
            <Alert variant="error">
              {apiErrorMessage(mutations.kickPlayer.error, {
                fallback: "Mitspieler konnte nicht entfernt werden.",
              })}
            </Alert>
          ) : null}
          <DialogFooter>
            <Button
              type="button"
              variant="secondary"
              disabled={mutations.kickPlayer.isPending}
              onClick={() => setKickTarget(null)}
            >
              Abbrechen
            </Button>
            <Button
              type="button"
              variant="primary"
              disabled={!kickTarget || mutations.kickPlayer.isPending}
              onClick={() => {
                if (!kickTarget) return;
                void mutations.kickPlayer
                  .mutateAsync({ gameId: game.id, userId: kickTarget.id })
                  .then(() => setKickTarget(null));
              }}
            >
              {mutations.kickPlayer.isPending ? "Entfernt …" : "Entfernen"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={Boolean(transferTarget)}
        onOpenChange={(open) => {
          if (!open) setTransferTarget(null);
        }}
      >
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>
              Host-Rolle an {transferTarget?.displayName} übergeben?
            </DialogTitle>
            <DialogDescription>
              {transferTarget?.displayName} erhält damit alle Host-Rechte in
              dieser Runde.
            </DialogDescription>
          </DialogHeader>
          {mutations.transferHost.isError ? (
            <Alert variant="error">
              {apiErrorMessage(mutations.transferHost.error, {
                fallback: "Host-Rolle konnte nicht übergeben werden.",
              })}
            </Alert>
          ) : null}
          <DialogFooter>
            <Button
              type="button"
              variant="secondary"
              disabled={mutations.transferHost.isPending}
              onClick={() => setTransferTarget(null)}
            >
              Abbrechen
            </Button>
            <Button
              type="button"
              variant="primary"
              disabled={!transferTarget || mutations.transferHost.isPending}
              onClick={() => {
                if (!transferTarget) return;
                void mutations.transferHost
                  .mutateAsync({
                    gameId: game.id,
                    newHostUserId: transferTarget.id,
                  })
                  .then(() => setTransferTarget(null));
              }}
            >
              {mutations.transferHost.isPending ? "Übergibt …" : "Übergeben"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={abandonConfirmOpen} onOpenChange={setAbandonConfirmOpen}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Spiel abbrechen?</DialogTitle>
            <DialogDescription>
              Das Spiel wird sofort beendet, ohne dass eine Goldmedaille
              vergeben wird.
            </DialogDescription>
          </DialogHeader>
          {mutations.abandonGame.isError ? (
            <Alert variant="error">
              {apiErrorMessage(mutations.abandonGame.error, {
                fallback: "Spiel konnte nicht abgebrochen werden.",
              })}
            </Alert>
          ) : null}
          <DialogFooter>
            <Button
              type="button"
              variant="secondary"
              disabled={mutations.abandonGame.isPending}
              onClick={() => setAbandonConfirmOpen(false)}
            >
              Abbrechen
            </Button>
            <Button
              type="button"
              variant="primary"
              disabled={mutations.abandonGame.isPending}
              onClick={() => {
                void mutations.abandonGame.mutateAsync(game.id).then(() => {
                  setAbandonConfirmOpen(false);
                });
              }}
            >
              {mutations.abandonGame.isPending
                ? "Bricht ab …"
                : "Spiel abbrechen"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
