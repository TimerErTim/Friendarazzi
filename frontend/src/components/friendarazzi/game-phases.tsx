"use client";

import {
  Award,
  Camera,
  Check,
  CheckCircle2,
  Crosshair,
  Crown,
  Dices,
  Eye,
  EyeOff,
  Flame,
  Ghost,
  Loader2,
  Lock,
  Sparkles,
  Target,
  Timer,
  Upload,
  Users,
  Zap,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { apiErrorMessage } from "@/api/errors";
import {
  type BoardTileView,
  type CaptureFeedItem,
  type DraftPromptRequestItem,
  type GameDetail,
  PromptSource,
  SuperlativeKind,
  VisibilityMode,
} from "@/api/friendarazzi-api";
import {
  useBenutzerQuery,
  useDecksQuery,
  useErfolgsmeldung,
  useFriendarazziMutations,
} from "@/api/hooks";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { CalloutNote, CodeTag } from "@/components/ui/brutal-elements";
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
import { Progress } from "@/components/ui/progress";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { cn } from "@/lib/cn";
import { autofillEmptySlots, rerollSingleSlot } from "@/lib/draft-prompts";

import { BingoBoard } from "./bingo-board";
import { CaptureGalleryCard } from "./capture-gallery-card";
import { GameTimeline } from "./game-timeline";
import { InviteShareCard } from "./invite-share-card";
import { AuthenticatedImage } from "./media-image";
import { PhotoLightbox } from "./photo-lightbox";
import { PlayerAvatar, PlayerBadgeLink } from "./player-avatar";

export function visibilityLabel(mode: VisibilityMode): string {
  switch (mode) {
    case VisibilityMode.Undercover:
      return "Undercover (verdeckt bis Bingo)";
    case VisibilityMode.LiveFeed:
      return "Live-Feed (sofort sichtbar)";
    case VisibilityMode.BlurredTeaser:
      return "Blurred Teaser (verpixelt)";
  }
}

const LOBBY_GRIDS = [
  { size: "3", title: "3 × 3", sub: "Schnell" },
  { size: "4", title: "4 × 4", sub: "Mittel" },
  { size: "5", title: "5 × 5", sub: "Marathon" },
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

// ------------------------------------------------------------------ Lobby Phase

export function LobbyPhase({ game }: { game: GameDetail }) {
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

  const isEditingTitleRef = useRef(false);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const startDraftingRef = useRef(false);
  const settingsSaveRef = useRef<Promise<boolean>>(Promise.resolve(true));
  const savedSettingsRef = useRef({
    title: game.title,
    gridSize: String(game.gridSize),
    visibilityMode: game.visibilityMode,
  });
  const lastSavedRef = useRef(savedSettingsRef.current);

  useEffect(() => {
    if (!isEditingTitleRef.current) {
      setTitle(game.title);
    }
    setGridSize(String(game.gridSize));
    setVisibilityMode(game.visibilityMode);
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

  useEffect(() => {
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, []);

  const canStart = game.players.length >= 3;
  const openSlots = Math.max(0, 3 - game.players.length);

  function handleTitleChange(val: string) {
    isEditingTitleRef.current = true;
    setTitle(val);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      debounceRef.current = null;
      isEditingTitleRef.current = false;
      void commitSettings(val, gridSize, visibilityMode);
    }, 500);
  }

  async function handleStartDrafting() {
    if (startDraftingRef.current) return;
    startDraftingRef.current = true;
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = null;
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
      {/* Left Column: Consolidated Invite Card + Crew Roster */}
      <div className="space-y-6">
        <InviteShareCard inviteCode={game.inviteCode} />

        <Card>
          <CardHeader>
            <div className="flex items-center justify-between gap-2">
              <CardTitle className="flex items-center gap-2">
                <Users className="size-5" aria-hidden="true" />
                Crew ({game.players.length})
              </CardTitle>
              <Badge variant={canStart ? "success" : "warning"}>
                {canStart ? "Bereit (3+)" : `Noch ${openSlots} nötig`}
              </Badge>
            </div>
          </CardHeader>
          <CardContent className="pt-5">
            <ul className="grid gap-3 sm:grid-cols-2">
              {game.players.map((player) => (
                <li
                  key={player.id}
                  className="brutal-shadow-sm rounded-md border-2 border-strong bg-surface-muted p-2"
                >
                  <PlayerBadgeLink user={player} />
                </li>
              ))}
              {["slot-1", "slot-2", "slot-3"]
                .slice(0, openSlots)
                .map((slotId) => (
                  <li
                    key={slotId}
                    className="flex min-h-14 items-center justify-center rounded-md border-2 border-dashed border-strong bg-canvas px-3 py-2 font-mono text-xs font-bold text-secondary"
                  >
                    Warte auf Mitspieler …
                  </li>
                ))}
            </ul>
          </CardContent>
        </Card>
      </div>

      {/* Right Column: Host Console or Guest Rules Card */}
      <Card>
        <CardHeader className="bg-cyan-subtle">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <CardTitle>
              {isHost ? "Host-Regiepult" : "Regeln dieser Runde"}
            </CardTitle>
            {isHost ? (
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
            ) : null}
          </div>
        </CardHeader>
        <CardContent className="space-y-5 pt-5">
          {isHost ? (
            <div className="space-y-5">
              <Field label="Spieltitel">
                <Input
                  value={title}
                  onChange={(e) => handleTitleChange(e.target.value)}
                  onBlur={() => {
                    isEditingTitleRef.current = false;
                    if (debounceRef.current) clearTimeout(debounceRef.current);
                    commitSettings(title, gridSize, visibilityMode);
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
                          commitSettings(title, opt.size, visibilityMode);
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
                          commitSettings(title, gridSize, opt.mode);
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
          ) : (
            <div className="space-y-4">
              <dl className="grid gap-3 text-sm sm:grid-cols-2">
                <div className="rounded-sm border-2 border-strong bg-surface-muted p-3">
                  <dt className="font-mono text-xs font-bold text-secondary uppercase">
                    Raster
                  </dt>
                  <dd className="font-display text-lg text-ink">
                    {game.gridSize} × {game.gridSize}
                  </dd>
                </div>
                <div className="rounded-sm border-2 border-strong bg-surface-muted p-3">
                  <dt className="font-mono text-xs font-bold text-secondary uppercase">
                    Modus
                  </dt>
                  <dd className="font-bold text-ink">
                    {visibilityLabel(game.visibilityMode)}
                  </dd>
                </div>
              </dl>
              <CalloutNote label="Warten auf Host" tone="yellow">
                Der Host startet die Entwurfsphase, sobald mindestens 3 Freunde
                beigetreten sind.
              </CalloutNote>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

// ------------------------------------------------------------------ Drafting Phase

type DraftSlot = {
  key: string;
  targetUserId: string;
  targetName: string;
  text: string;
  source: PromptSource;
};

export function DraftingPhase({ game }: { game: GameDetail }) {
  const me = useBenutzerQuery();
  const decks = useDecksQuery();
  const mutations = useFriendarazziMutations();
  const {
    mutate: savePrompts,
    mutateAsync: savePromptsAsync,
    isPending: isSavingDraft,
  } = mutations.savePrompts;
  const feedback = useErfolgsmeldung();

  const myPlayer = game.players.find((p) => p.id === me.data?.id);
  const isLocked = myPlayer?.draftLocked ?? false;

  const [selectedDeckSlug, setSelectedDeckSlug] = useState<string>("night-out");
  const [slots, setSlots] = useState<DraftSlot[]>([]);
  const [justSavedDraft, setJustSavedDraft] = useState(false);
  const [deckNotice, setDeckNotice] = useState<string | null>(null);

  const initializedRef = useRef(false);
  const draftDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastDigestRef = useRef<string>("");

  function toPromptPayload(list: DraftSlot[]): DraftPromptRequestItem[] {
    return list
      .filter((s) => s.text.trim().length > 0)
      .map((s) => ({
        targetUserId: s.targetUserId,
        text: s.text.trim(),
        source: s.source,
      }));
  }

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
    if (built.length > 0) {
      initializedRef.current = true;
      lastDigestRef.current = JSON.stringify(
        built
          .filter((s) => s.text.trim().length > 0)
          .map((s) => ({
            targetUserId: s.targetUserId,
            text: s.text.trim(),
            source: s.source,
          })),
      );
    }
  }, [game.myQuotas, game.myDrafts, slots.length]);

  useEffect(() => {
    if (!justSavedDraft) return;
    const timer = setTimeout(() => setJustSavedDraft(false), 1800);
    return () => clearTimeout(timer);
  }, [justSavedDraft]);

  useEffect(() => {
    return () => {
      if (draftDebounceRef.current) clearTimeout(draftDebounceRef.current);
    };
  }, []);

  function commitDraft(nextSlots: DraftSlot[]) {
    if (isLocked) return;
    const payload = toPromptPayload(nextSlots);
    const digest = JSON.stringify(payload);
    if (digest === lastDigestRef.current) return;
    lastDigestRef.current = digest;
    savePrompts(
      { gameId: game.id, prompts: payload },
      { onSuccess: () => setJustSavedDraft(true) },
    );
  }

  function scheduleDraftSave(nextSlots: DraftSlot[]) {
    if (isLocked) return;
    if (draftDebounceRef.current) clearTimeout(draftDebounceRef.current);
    draftDebounceRef.current = setTimeout(() => {
      commitDraft(nextSlots);
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
      if (draftDebounceRef.current) clearTimeout(draftDebounceRef.current);
      commitDraft(nextSlots);
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
    if (draftDebounceRef.current) clearTimeout(draftDebounceRef.current);
    commitDraft(nextSlots);
  }

  const allFilled =
    slots.length > 0 && slots.every((s) => s.text.trim().length > 0);
  const filledCount = slots.filter((s) => s.text.trim().length > 0).length;

  async function handleLock() {
    feedback.verwerfen();
    if (draftDebounceRef.current) clearTimeout(draftDebounceRef.current);
    const payload = toPromptPayload(slots);
    lastDigestRef.current = JSON.stringify(payload);
    await savePromptsAsync({
      gameId: game.id,
      prompts: payload,
    });
    await mutations.lockPrompts.mutateAsync(game.id);
    feedback.melden("Aufträge eingeloggt!");
  }

  return (
    <div className="space-y-6">
      {/* Group Lock Strip + Quick-Start Deck Picker */}
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

      {/* Prompt Workshop Grid */}
      <div className="grid gap-5 lg:grid-cols-2">
        {game.myQuotas.map((quota) => {
          const targetSlots = slots.filter(
            (s) => s.targetUserId === quota.targetUser.id,
          );
          const targetFilled = targetSlots.filter((s) => s.text.trim()).length;

          return (
            <Card key={quota.targetUser.id}>
              <CardHeader className="bg-surface-muted">
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
              </CardHeader>
              <CardContent className="space-y-3 pt-4">
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
                        scheduleDraftSave(nextSlots);
                      }}
                      onBlur={() => {
                        if (draftDebounceRef.current) {
                          clearTimeout(draftDebounceRef.current);
                        }
                        commitDraft(slots);
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

      {isLocked ? (
        <Alert variant="success" icon={<CheckCircle2 className="size-4" />}>
          Deine Aufträge ({filledCount}/{slots.length}) sind eingeloggt. Sobald
          alle anderen bereit sind, werden die Bingo-Bretter automatisch
          ausgeteilt!
        </Alert>
      ) : (
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
              ) : justSavedDraft ? (
                <>
                  <Check className="size-3.5 text-success" aria-hidden="true" />
                  Entwurf gespeichert
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
      )}
    </div>
  );
}

// ------------------------------------------------------------------ Active Play Phase

export function ActivePlayPhase({ game }: { game: GameDetail }) {
  const mutations = useFriendarazziMutations();
  const [selectedTile, setSelectedTile] = useState<BoardTileView | null>(null);
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [lightboxCapture, setLightboxCapture] =
    useState<CaptureFeedItem | null>(null);

  useEffect(() => {
    if (!selectedFile) {
      setPreviewUrl(null);
      return;
    }
    const url = URL.createObjectURL(selectedFile);
    setPreviewUrl(url);
    return () => URL.revokeObjectURL(url);
  }, [selectedFile]);

  const board = game.myBoard;
  if (!board) {
    return (
      <CalloutNote label="Brett wird geladen" tone="yellow">
        Dein Bingo-Brett wird vorbereitet …
      </CalloutNote>
    );
  }

  const capturedCount = board.tiles.filter((t) => t.isCaptured).length;
  const totalCount = board.tiles.length;
  const pct =
    totalCount > 0 ? Math.round((capturedCount / totalCount) * 100) : 0;

  async function submitTilePhoto() {
    if (!selectedTile || !selectedFile) return;
    await mutations.submitCapture.mutateAsync({
      gameId: game.id,
      tileId: selectedTile.id,
      file: selectedFile,
    });
    setSelectedTile(null);
    setSelectedFile(null);
  }

  return (
    <div className="grid items-start gap-6 lg:grid-cols-[1.35fr_0.65fr]">
      {/* Left: Hero Bingo Board */}
      <div className="space-y-4">
        <div className="brutal-shadow rounded-md border-[2.5px] border-strong bg-surface p-4">
          <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
            <div className="flex flex-wrap items-center gap-2">
              <Badge variant="primary">Aktive Jagd</Badge>
              <Badge variant="info">
                {game.visibilityMode === VisibilityMode.Undercover ? (
                  <EyeOff className="size-3" aria-hidden="true" />
                ) : (
                  <Eye className="size-3" aria-hidden="true" />
                )}
                {visibilityLabel(game.visibilityMode)}
              </Badge>
            </div>
            <CodeTag tone="yellow">
              {capturedCount} / {totalCount} Felder
            </CodeTag>
          </div>
          <Progress value={pct} aria-label="Spielfortschritt" />
        </div>

        <BingoBoard
          board={board}
          gridSize={game.gridSize}
          feed={game.feed}
          onEmptyTileSelect={(tile) => {
            mutations.submitCapture.reset();
            setSelectedFile(null);
            setSelectedTile(tile);
          }}
          onViewCapture={(cap) => setLightboxCapture(cap)}
        />
      </div>

      {/* Right: Live Crew + Activity Stream */}
      <div className="space-y-6">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Users className="size-5" aria-hidden="true" />
              Mitspieler
            </CardTitle>
          </CardHeader>
          <CardContent className="pt-4">
            <div className="flex flex-wrap gap-2">
              {game.players.map((p) => (
                <div
                  key={p.id}
                  className="brutal-shadow-sm rounded-md border-2 border-strong bg-surface-muted px-2 py-1"
                >
                  <PlayerBadgeLink user={p} compact />
                </div>
              ))}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="bg-pink-subtle">
            <div className="flex items-center justify-between gap-2">
              <CardTitle>Live-Aktivität</CardTitle>
              <CodeTag tone="cyan">{game.feed.length}</CodeTag>
            </div>
          </CardHeader>
          <CardContent className="space-y-3 pt-4">
            {game.visibilityMode === VisibilityMode.Undercover ? (
              <CalloutNote label="Undercover aktiv" tone="pink">
                Alle Beweisfotos bleiben bis zum Bingo geheim. Deine eigenen
                Treffer siehst du direkt auf deinem Brett.
              </CalloutNote>
            ) : game.feed.length === 0 ? (
              <CalloutNote label="Noch ruhig">
                Noch keine Beweisfotos hochgeladen – schnapp dir dein erstes
                Motiv!
              </CalloutNote>
            ) : (
              <div className="space-y-3">
                {game.feed.map((item) => (
                  <button
                    key={item.id}
                    type="button"
                    onClick={() => setLightboxCapture(item)}
                    className="brutal-press flex w-full cursor-pointer items-center gap-3 rounded-md border-2 border-strong bg-surface p-2 text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
                  >
                    <div className="size-16 shrink-0 overflow-hidden rounded-sm border-2 border-strong bg-surface-muted">
                      <AuthenticatedImage
                        storageKey={
                          item.isBlurred
                            ? item.thumbnailStorageKey
                            : (item.imageStorageKey ?? item.thumbnailStorageKey)
                        }
                        alt={item.promptText ?? "Teaser"}
                        blurred={item.isBlurred}
                        className="h-full w-full"
                      />
                    </div>
                    <div className="min-w-0 flex-1 space-y-1">
                      <div className="flex items-center justify-between gap-2 text-xs">
                        <span className="truncate font-bold text-ink">
                          {item.catcher.displayName}
                        </span>
                        <span className="font-mono text-secondary tabular-nums">
                          {new Date(item.capturedAt).toLocaleTimeString(
                            "de-AT",
                            { hour: "2-digit", minute: "2-digit" },
                          )}
                        </span>
                      </div>
                      {item.isBlurred ? (
                        <Badge variant="warning">Teaser verpixelt</Badge>
                      ) : (
                        <p className="line-clamp-2 text-xs font-medium text-ink">
                          {item.promptText}
                        </p>
                      )}
                    </div>
                  </button>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Camera / Upload Capture Dialog */}
      <Dialog
        open={Boolean(selectedTile)}
        onOpenChange={(open) => {
          if (!open) {
            setSelectedTile(null);
            setSelectedFile(null);
          }
        }}
      >
        <DialogContent>
          <DialogHeader className="space-y-3">
            <span className="inline-flex w-fit items-center gap-1.5 rounded-sm border-2 border-strong bg-surface-muted px-2 py-0.5 font-mono text-[11px] font-bold tracking-wider text-ink uppercase">
              <Camera className="size-3.5" aria-hidden="true" />
              Beweisfoto hochladen
            </span>
            <DialogTitle className="font-display text-2xl leading-tight tracking-tight text-ink sm:text-3xl">
              {selectedTile?.promptText}
            </DialogTitle>
            <DialogDescription className="sr-only">
              Lade ein Beweisfoto für diesen Auftrag hoch.
            </DialogDescription>
          </DialogHeader>

          {selectedTile?.targetUser ? (
            <div className="brutal-shadow-sm flex items-center gap-3 rounded-md border-2 border-strong bg-cyan-subtle p-2.5">
              <div className="relative shrink-0">
                <PlayerAvatar
                  user={selectedTile.targetUser}
                  size="md"
                  linkToProfile={false}
                  showMedalBadge={false}
                />
                <span
                  aria-hidden="true"
                  className="absolute -right-1 -bottom-1 flex size-5 items-center justify-center rounded-full border-2 border-strong bg-primary text-primary-foreground"
                >
                  <Crosshair className="size-3" />
                </span>
              </div>
              <div className="min-w-0 flex-1">
                <p className="truncate font-display text-base leading-tight text-ink">
                  {selectedTile.targetUser.displayName}
                </p>
                <p className="truncate font-mono text-[11px] font-bold text-secondary">
                  vor der Linse erwischen ·{" "}
                  {selectedTile.targetUser.effectiveTitle}
                </p>
              </div>
            </div>
          ) : null}

          <div className="space-y-4">
            <label className="brutal-press flex min-h-11 cursor-pointer flex-col items-center justify-center gap-2 rounded-md border-2 border-dashed border-control bg-primary-subtle p-6 text-center hover:bg-primary/25">
              <Camera className="size-7 text-ink" aria-hidden="true" />
              <span className="text-sm font-bold text-ink">
                Kamera öffnen oder Foto auswählen
              </span>
              <span className="font-mono text-xs text-secondary">
                JPEG, PNG oder WebP (wird automatisch optimiert)
              </span>
              <input
                type="file"
                accept="image/*"
                capture="environment"
                className="sr-only"
                onChange={(e) => {
                  const file = e.target.files?.[0] ?? null;
                  setSelectedFile(file);
                }}
              />
            </label>

            {previewUrl ? (
              <div className="brutal-shadow-sm overflow-hidden rounded-md border-2 border-strong bg-surface p-1.5">
                {/* biome-ignore lint/performance/noImgElement: local File preview blob URL */}
                <img
                  src={previewUrl}
                  alt="Vorschau"
                  className="max-h-64 w-full rounded-sm object-contain"
                />
              </div>
            ) : null}

            {mutations.submitCapture.isError ? (
              <Alert variant="error">
                {apiErrorMessage(mutations.submitCapture.error, {
                  fallback: "Das Foto konnte nicht hochgeladen werden.",
                })}
              </Alert>
            ) : null}
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="secondary"
              onClick={() => setSelectedTile(null)}
              disabled={mutations.submitCapture.isPending}
            >
              Abbrechen
            </Button>
            <Button
              type="button"
              disabled={!selectedFile || mutations.submitCapture.isPending}
              onClick={() => void submitTilePhoto()}
            >
              <Upload className="size-4" aria-hidden="true" />
              {mutations.submitCapture.isPending
                ? "Wird hochgeladen …"
                : "Beweis einreichen"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <PhotoLightbox
        capture={lightboxCapture}
        open={Boolean(lightboxCapture)}
        onOpenChange={(open) => {
          if (!open) setLightboxCapture(null);
        }}
      />
    </div>
  );
}

// ------------------------------------------------------------------ Completed Phase

function superlativeMeta(kind: SuperlativeKind, metric: number) {
  switch (kind) {
    case SuperlativeKind.MostWanted:
      return {
        label: "Meistgesucht (Most Wanted)",
        detail: `${metric}× erwischt`,
        icon: Target,
      };
    case SuperlativeKind.Ghost:
      return {
        label: "Phantom (Ghost)",
        detail: `${metric}× erwischt`,
        icon: Ghost,
      };
    case SuperlativeKind.QuickestDraw:
      return {
        label: "Schnellster Finger (Quickest Draw)",
        detail: `nach ${metric} Sek.`,
        icon: Zap,
      };
    case SuperlativeKind.Sniper:
      return {
        label: "Scharfschütze (Sniper)",
        detail: `${metric} Treffer in 1 Std.`,
        icon: Flame,
      };
  }
}

export function CompletedPhase({ game }: { game: GameDetail }) {
  const [selectedCapture, setSelectedCapture] =
    useState<CaptureFeedItem | null>(null);

  return (
    <div className="space-y-8">
      {/* Winner Podium Banner */}
      {game.winner ? (
        <div className="brutal-shadow-lg rounded-md border-[2.5px] border-strong bg-primary p-6 text-primary-foreground">
          <div className="flex flex-wrap items-center justify-between gap-4">
            <div className="flex items-center gap-4">
              <PlayerAvatar user={game.winner} size="lg" />
              <div className="space-y-1">
                <div className="flex flex-wrap items-center gap-2">
                  <Badge variant="outline" className="gap-1">
                    <Crown className="size-3.5" aria-hidden="true" />
                    BINGO-SIEGER
                  </Badge>
                  <Badge variant="info" className="gap-1">
                    <Award className="size-3.5" aria-hidden="true" />
                    +1 Goldmedaille
                  </Badge>
                </div>
                <h2 className="font-display text-2xl tracking-tight sm:text-3xl">
                  {game.winner.displayName} gewinnt „{game.title}“!
                </h2>
                <p className="font-mono text-xs font-bold">
                  {game.winner.effectiveTitle} · {game.winner.goldMedalCount}{" "}
                  Goldmedaillen gesamt
                </p>
              </div>
            </div>
            {game.startedAt && game.endedAt ? (
              <div className="inline-flex items-center gap-1.5 rounded-sm border-2 border-strong bg-surface px-3 py-1 font-mono text-xs font-bold text-ink tabular-nums">
                <Timer className="size-4" aria-hidden="true" />
                {new Date(game.endedAt).toLocaleString("de-AT")}
              </div>
            ) : null}
          </div>
        </div>
      ) : null}

      {/* Automated Post-Game Superlatives */}
      {game.superlatives.length > 0 ? (
        <section className="space-y-3">
          <h2 className="font-display text-lg tracking-tight text-ink">
            Auszeichnungen
          </h2>
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
            {game.superlatives.map((sup) => {
              const meta = superlativeMeta(sup.kind, sup.metricValue);
              const Icon = meta.icon;
              return (
                <Card key={sup.kind}>
                  <CardContent className="space-y-2 pt-4">
                    <div className="flex items-center gap-2 font-mono text-xs font-bold tracking-wide text-secondary uppercase">
                      <Icon className="size-4 text-ink" aria-hidden="true" />
                      <span>{meta.label}</span>
                    </div>
                    <PlayerBadgeLink
                      user={sup.user}
                      compact
                      subtitle={meta.detail}
                    />
                  </CardContent>
                </Card>
              );
            })}
          </div>
        </section>
      ) : null}

      {/* 1:1 Proportional Timeline */}
      <GameTimeline
        startedAt={game.startedAt}
        endedAt={game.endedAt}
        captures={game.feed}
      />

      {/* Tabbed Recap: Gallery vs All Unlocked Boards */}
      <Tabs defaultValue="gallery">
        <TabsList>
          <TabsTrigger value="gallery">
            Spiel-Galerie ({game.feed.length})
          </TabsTrigger>
          <TabsTrigger value="boards">
            Aufgedeckte Bretter ({game.allBoards.length})
          </TabsTrigger>
        </TabsList>

        <TabsContent value="gallery" className="pt-2">
          {game.feed.length === 0 ? (
            <CalloutNote label="Keine Fotos">
              In diesem Spiel wurden keine Fotos eingereicht.
            </CalloutNote>
          ) : (
            <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
              {game.feed.map((cap) => (
                <CaptureGalleryCard
                  key={cap.id}
                  capture={cap}
                  onSelect={setSelectedCapture}
                />
              ))}
            </div>
          )}
        </TabsContent>

        <TabsContent value="boards" className="pt-2">
          <div className="grid gap-6 xl:grid-cols-2">
            {game.allBoards.map((board) => (
              <div key={board.id} className="space-y-3">
                <div className="brutal-shadow-sm flex items-center justify-between gap-2 rounded-md border-2 border-strong bg-surface px-4 py-2.5">
                  <PlayerBadgeLink user={board.owner} compact />
                  {board.hasBingo ? (
                    <Badge variant="primary">Bingo!</Badge>
                  ) : null}
                </div>
                <BingoBoard
                  board={board}
                  gridSize={game.gridSize}
                  feed={game.feed}
                  onViewCapture={(cap) => setSelectedCapture(cap)}
                />
              </div>
            ))}
          </div>
        </TabsContent>
      </Tabs>

      <PhotoLightbox
        capture={selectedCapture}
        open={Boolean(selectedCapture)}
        onOpenChange={(open) => {
          if (!open) setSelectedCapture(null);
        }}
      />
    </div>
  );
}
