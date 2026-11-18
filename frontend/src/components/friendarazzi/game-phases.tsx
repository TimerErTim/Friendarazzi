"use client";

import {
  Award,
  Camera,
  Check,
  CheckCircle2,
  Copy,
  Crown,
  Dices,
  Eye,
  EyeOff,
  Flame,
  Ghost,
  Lock,
  Sparkles,
  Target,
  Timer,
  Upload,
  Users,
  Zap,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

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
  MarginNote,
  Perforation,
  SheetNumber,
} from "@/components/ui/paper-rule";
import { Select } from "@/components/ui/select";
import { cn } from "@/lib/cn";
import { absoluteUrl, joinHref } from "@/lib/navigation";

import { GameTimeline } from "./game-timeline";
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

// ------------------------------------------------------------------ Lobby Phase

export function LobbyPhase({ game }: { game: GameDetail }) {
  const me = useBenutzerQuery();
  const mutations = useFriendarazziMutations();
  const feedback = useErfolgsmeldung();
  const isHost = me.data?.id === game.hostUserId;

  const [title, setTitle] = useState(game.title);
  const [gridSize, setGridSize] = useState(String(game.gridSize));
  const [visibilityMode, setVisibilityMode] = useState<VisibilityMode>(
    game.visibilityMode,
  );
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    setTitle(game.title);
    setGridSize(String(game.gridSize));
    setVisibilityMode(game.visibilityMode);
  }, [game.title, game.gridSize, game.visibilityMode]);

  const inviteUrl = absoluteUrl(joinHref(game.inviteCode));

  const canStart = game.players.length >= 3;

  function copyInvite() {
    if (typeof navigator !== "undefined" && navigator.clipboard) {
      void navigator.clipboard.writeText(inviteUrl);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  }

  return (
    <div className="space-y-6">
      {/* Invite Card */}
      <Card>
        <CardHeader>
          <CardTitle>Einladung teilen</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex flex-wrap items-center justify-between gap-4 rounded-md border border-subtle bg-surface-muted p-4">
            <div>
              <p className="text-xs font-semibold tracking-wider text-secondary uppercase">
                Einladungscode
              </p>
              <p className="font-mono text-2xl font-bold tracking-widest text-ink">
                {game.inviteCode}
              </p>
            </div>
            <Button variant="secondary" onClick={copyInvite}>
              {copied ? (
                <>
                  <Check className="size-4 text-success" aria-hidden="true" />
                  Link kopiert
                </>
              ) : (
                <>
                  <Copy className="size-4" aria-hidden="true" />
                  Einladungslink kopieren
                </>
              )}
            </Button>
          </div>
          <p className="font-mono text-xs break-all text-secondary">
            {inviteUrl}
          </p>
        </CardContent>
      </Card>

      {/* Player Roster */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between gap-2">
            <CardTitle className="flex items-center gap-2">
              <Users className="size-4 text-primary" aria-hidden="true" />
              Spieler ({game.players.length})
            </CardTitle>
            <Badge variant={canStart ? "success" : "warning"}>
              {canStart
                ? "Bereit (3+)"
                : `Noch ${3 - game.players.length} nötig`}
            </Badge>
          </div>
        </CardHeader>
        <CardContent>
          <ul className="grid gap-2 sm:grid-cols-2">
            {game.players.map((player) => (
              <li
                key={player.id}
                className="rounded-md border border-subtle bg-elevated p-2"
              >
                <PlayerBadgeLink user={player} />
              </li>
            ))}
          </ul>
        </CardContent>
      </Card>

      {/* Host Settings or Guest Read-Only Summary */}
      <Card>
        <CardHeader>
          <CardTitle>Spieleinstellungen</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          {isHost ? (
            <form
              className="space-y-4"
              onSubmit={(e) => {
                e.preventDefault();
                feedback.verwerfen();
                mutations.updateSettings.mutate(
                  {
                    gameId: game.id,
                    input: {
                      title,
                      gridSize: Number(gridSize),
                      visibilityMode,
                    },
                  },
                  {
                    onSuccess: () =>
                      feedback.melden("Einstellungen gespeichert."),
                  },
                );
              }}
            >
              <div className="grid gap-4 sm:grid-cols-2">
                <Field label="Spieltitel">
                  <Input
                    value={title}
                    onChange={(e) => setTitle(e.target.value)}
                    required
                  />
                </Field>
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
              </div>

              <Field label="Sichtbarkeitsmodus">
                <Select
                  value={visibilityMode}
                  onChange={(e) =>
                    setVisibilityMode(e.target.value as VisibilityMode)
                  }
                >
                  <option value={VisibilityMode.Undercover}>
                    Undercover – Fotos bis Bingo verborgen
                  </option>
                  <option value={VisibilityMode.LiveFeed}>
                    Live-Feed – Fotos erscheinen sofort
                  </option>
                  <option value={VisibilityMode.BlurredTeaser}>
                    Blurred Teaser – Verpixelter Hinweis ohne Ziel
                  </option>
                </Select>
              </Field>

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

              <div className="flex flex-wrap items-center justify-between gap-3 pt-2">
                <Button
                  type="submit"
                  variant="secondary"
                  disabled={mutations.updateSettings.isPending}
                >
                  Einstellungen speichern
                </Button>
                <Button
                  type="button"
                  disabled={!canStart || mutations.startDrafting.isPending}
                  onClick={() => mutations.startDrafting.mutate(game.id)}
                >
                  {mutations.startDrafting.isPending
                    ? "Startet …"
                    : "Entwurfsphase starten"}
                </Button>
              </div>
            </form>
          ) : (
            <div className="space-y-3">
              <dl className="grid gap-3 text-sm sm:grid-cols-2">
                <div>
                  <dt className="text-xs text-secondary uppercase">Raster</dt>
                  <dd className="font-semibold text-ink">
                    {game.gridSize} × {game.gridSize}
                  </dd>
                </div>
                <div>
                  <dt className="text-xs text-secondary uppercase">Modus</dt>
                  <dd className="font-semibold text-ink">
                    {visibilityLabel(game.visibilityMode)}
                  </dd>
                </div>
              </dl>
              <MarginNote label="Warten auf Host">
                Der Host kann die Entwurfsphase starten, sobald mindestens 3
                Freunde beigetreten sind.
              </MarginNote>
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
  const feedback = useErfolgsmeldung();

  const myPlayer = game.players.find((p) => p.id === me.data?.id);
  const isLocked = myPlayer?.draftLocked ?? false;

  const [selectedDeckSlug, setSelectedDeckSlug] = useState<string>("night-out");
  const [slots, setSlots] = useState<DraftSlot[]>([]);
  const initializedRef = useRef(false);

  // Build deterministic slot list from quotas + existing saved drafts.
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
    }
  }, [game.myQuotas, game.myDrafts, slots.length]);

  const activeDeck = useMemo(() => {
    const list = decks.data ?? [];
    return list.find((d) => d.slug === selectedDeckSlug) ?? list[0];
  }, [decks.data, selectedDeckSlug]);

  function fillTemplate(template: string, name: string): string {
    return template.replaceAll("{name}", name);
  }

  function autofillEmpty() {
    if (!activeDeck || activeDeck.prompts.length === 0) return;
    const used = new Set(slots.map((s) => s.text.trim()).filter(Boolean));
    const pool = [...activeDeck.prompts].sort(() => Math.random() - 0.5);
    let cursor = 0;

    setSlots((prev) =>
      prev.map((slot) => {
        if (slot.text.trim()) return slot;
        let candidate = "";
        for (let tries = 0; tries < pool.length; tries += 1) {
          const tpl = pool[(cursor + tries) % pool.length] ?? "";
          const filled = fillTemplate(tpl, slot.targetName);
          if (!used.has(filled)) {
            candidate = filled;
            used.add(filled);
            cursor = (cursor + tries + 1) % pool.length;
            break;
          }
        }
        if (!candidate) {
          const tpl =
            pool[cursor % pool.length] ?? "{name} macht etwas Lustiges";
          candidate = fillTemplate(tpl, slot.targetName);
          cursor += 1;
        }
        return {
          ...slot,
          text: candidate,
          source: PromptSource.StarterDeck,
        };
      }),
    );
  }

  function rerollSlot(key: string) {
    if (!activeDeck || activeDeck.prompts.length === 0) return;
    const pool = activeDeck.prompts;
    setSlots((prev) =>
      prev.map((slot) => {
        if (slot.key !== key) return slot;
        const idx = Math.floor(Math.random() * pool.length);
        const tpl = pool[idx] ?? "{name} lacht laut";
        return {
          ...slot,
          text: fillTemplate(tpl, slot.targetName),
          source: PromptSource.StarterDeck,
        };
      }),
    );
  }

  const allFilled =
    slots.length > 0 && slots.every((s) => s.text.trim().length > 0);
  const filledCount = slots.filter((s) => s.text.trim().length > 0).length;

  async function handleSaveAndLock(lockAfter: boolean) {
    feedback.verwerfen();
    const payload: DraftPromptRequestItem[] = slots
      .filter((s) => s.text.trim().length > 0)
      .map((s) => ({
        targetUserId: s.targetUserId,
        text: s.text.trim(),
        source: s.source,
      }));

    await mutations.savePrompts.mutateAsync({
      gameId: game.id,
      prompts: payload,
    });
    if (lockAfter) {
      await mutations.lockPrompts.mutateAsync(game.id);
      feedback.melden("Aufträge eingeloggt!");
    } else {
      feedback.melden("Entwurf gespeichert.");
    }
  }

  return (
    <div className="space-y-6">
      {/* Player Lock Progress */}
      <Card>
        <CardHeader>
          <div className="flex flex-wrap items-center justify-between gap-2">
            <CardTitle>Entwurfsphase – Status</CardTitle>
            <SheetNumber>
              {game.players.filter((p) => p.draftLocked).length} /{" "}
              {game.players.length} bereit
            </SheetNumber>
          </div>
        </CardHeader>
        <CardContent>
          <ul className="flex flex-wrap gap-2">
            {game.players.map((p) => (
              <li
                key={p.id}
                className={cn(
                  "inline-flex items-center gap-2 rounded-sm border px-2.5 py-1.5 text-xs font-medium",
                  p.draftLocked
                    ? "border-success/40 bg-success/10 text-ink"
                    : "border-subtle bg-surface-muted text-secondary",
                )}
              >
                <PlayerAvatar user={p} size="sm" showMedalBadge={false} />
                <span>{p.displayName}</span>
                {p.draftLocked ? (
                  <CheckCircle2
                    className="size-3.5 text-success"
                    aria-hidden="true"
                  />
                ) : null}
              </li>
            ))}
          </ul>
        </CardContent>
      </Card>

      {/* Quick-Start Deck Toolbar */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Sparkles className="size-4 text-primary" aria-hidden="true" />
            Quick-Start Decks
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <p className="text-sm text-secondary">
            Wähle ein Paket und fülle leere Felder mit Namen deiner Freunde –
            oder würfle einzelne Felder neu aus.
          </p>
          <div className="flex flex-wrap items-end gap-3">
            <Field label="Starter-Paket" className="min-w-52 flex-1">
              <Select
                value={selectedDeckSlug}
                onChange={(e) => setSelectedDeckSlug(e.target.value)}
                disabled={isLocked}
              >
                {(decks.data ?? []).map((deck) => (
                  <option key={deck.id} value={deck.slug}>
                    {deck.name} – {deck.description}
                  </option>
                ))}
              </Select>
            </Field>
            <Button
              type="button"
              variant="secondary"
              size="field"
              disabled={isLocked || !activeDeck}
              onClick={autofillEmpty}
            >
              <Dices className="size-4" aria-hidden="true" />
              Leere Felder füllen
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Per-Friend Equal-Distribution Slots */}
      <div className="space-y-4">
        {game.myQuotas.map((quota) => {
          const targetSlots = slots.filter(
            (s) => s.targetUserId === quota.targetUser.id,
          );
          const targetFilled = targetSlots.filter((s) => s.text.trim()).length;

          return (
            <Card key={quota.targetUser.id}>
              <CardHeader>
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
              <CardContent className="space-y-3">
                {targetSlots.map((slot, index) => (
                  <div key={slot.key} className="flex items-center gap-2">
                    <SheetNumber className="w-6 shrink-0 text-center">
                      {index + 1}
                    </SheetNumber>
                    <Input
                      value={slot.text}
                      disabled={isLocked}
                      placeholder={`z.B. ${quota.targetUser.displayName} bestellt noch ein Getränk`}
                      aria-label={`Auftrag ${index + 1} für ${quota.targetUser.displayName}`}
                      onChange={(e) => {
                        const val = e.target.value;
                        setSlots((prev) =>
                          prev.map((item) =>
                            item.key === slot.key
                              ? {
                                  ...item,
                                  text: val,
                                  source: PromptSource.Custom,
                                }
                              : item,
                          ),
                        );
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
        <Alert variant="success">
          Deine Aufträge ({filledCount}/{slots.length}) sind eingeloggt. Sobald
          alle anderen bereit sind, werden die Bingo-Bretter automatisch
          ausgeteilt!
        </Alert>
      ) : (
        <div className="flex flex-wrap items-center justify-between gap-3 rounded-md border border-subtle bg-surface p-4">
          <span className="text-sm text-secondary tabular-nums">
            {filledCount} von {slots.length} Aufträgen ausgefüllt
          </span>
          <div className="flex flex-wrap gap-3">
            <Button
              type="button"
              variant="secondary"
              disabled={
                mutations.savePrompts.isPending ||
                mutations.lockPrompts.isPending
              }
              onClick={() => void handleSaveAndLock(false)}
            >
              Zwischenspeichern
            </Button>
            <Button
              type="button"
              disabled={
                !allFilled ||
                mutations.savePrompts.isPending ||
                mutations.lockPrompts.isPending
              }
              onClick={() => void handleSaveAndLock(true)}
            >
              <Lock className="size-4" aria-hidden="true" />
              {mutations.lockPrompts.isPending
                ? "Wird eingeloggt …"
                : "Einloggen & Bereit melden"}
            </Button>
          </div>
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
      <MarginNote label="Brett wird geladen">
        Dein Bingo-Brett wird vorbereitet …
      </MarginNote>
    );
  }

  const gridColsClass =
    game.gridSize === 3
      ? "grid-cols-3"
      : game.gridSize === 4
        ? "grid-cols-4"
        : "grid-cols-5";

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
    <div className="space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <Badge variant="primary">Aktive Jagd</Badge>
          <Badge variant="outline">
            {game.visibilityMode === VisibilityMode.Undercover ? (
              <EyeOff className="size-3" aria-hidden="true" />
            ) : (
              <Eye className="size-3" aria-hidden="true" />
            )}
            {visibilityLabel(game.visibilityMode)}
          </Badge>
        </div>
        <span className="text-xs text-secondary tabular-nums">
          {board.tiles.filter((t) => t.isCaptured).length} /{" "}
          {board.tiles.length} Felder
        </span>
      </div>

      {/* Bingo Grid */}
      <div className={cn("grid gap-2", gridColsClass)}>
        {board.tiles.map((tile) => {
          return (
            <button
              key={tile.id}
              type="button"
              disabled={tile.isCaptured}
              onClick={() => {
                if (!tile.isCaptured) {
                  mutations.submitCapture.reset();
                  setSelectedFile(null);
                  setSelectedTile(tile);
                }
              }}
              className={cn(
                "relative flex min-h-24 flex-col justify-between overflow-hidden rounded-md border p-2 text-left transition-colors sm:min-h-28 sm:p-2.5",
                "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary",
                tile.isCaptured
                  ? "border-success/50 bg-success/10"
                  : "border-control bg-surface hover:bg-surface-muted",
              )}
            >
              {tile.imageStorageKey ? (
                <div className="absolute inset-0 opacity-25">
                  <AuthenticatedImage
                    storageKey={tile.imageStorageKey}
                    alt={tile.promptText}
                    className="h-full w-full"
                  />
                </div>
              ) : null}

              <div className="relative z-10 flex items-start justify-between gap-1">
                <span className="inline-flex items-center gap-1 rounded-sm bg-elevated/90 px-1.5 py-0.5 text-[10px] font-semibold text-ink">
                  {tile.targetUser.displayName}
                </span>
                {tile.isCaptured ? (
                  <CheckCircle2
                    className="size-4 shrink-0 text-success"
                    aria-hidden="true"
                  />
                ) : (
                  <Camera
                    className="size-4 shrink-0 text-secondary"
                    aria-hidden="true"
                  />
                )}
              </div>

              <p className="relative z-10 mt-1 line-clamp-3 text-xs leading-snug font-medium text-ink sm:text-[13px]">
                {tile.promptText}
              </p>
            </button>
          );
        })}
      </div>

      {/* Player Roster with Crosslinked Avatars */}
      <Card>
        <CardHeader>
          <CardTitle>Mitspieler</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex flex-wrap gap-3">
            {game.players.map((p) => (
              <div
                key={p.id}
                className="rounded-md border border-subtle bg-elevated px-2 py-1"
              >
                <PlayerBadgeLink user={p} compact />
              </div>
            ))}
          </div>
        </CardContent>
      </Card>

      <Perforation />

      {/* Activity Feed according to VisibilityMode */}
      <section className="space-y-3">
        <div className="flex items-center justify-between">
          <h2 className="font-serif text-lg font-semibold text-ink">
            Gruppen-Aktivität
          </h2>
          <SheetNumber>{game.feed.length}</SheetNumber>
        </div>

        {game.visibilityMode === VisibilityMode.Undercover ? (
          <MarginNote label="Undercover-Modus">
            Alle Beweisfotos bleiben geheim, bis jemand Bingo ruft. Deine
            eigenen Schnappschüsse siehst du direkt auf deinem Brett.
          </MarginNote>
        ) : game.feed.length === 0 ? (
          <MarginNote label="Noch ruhig">
            Noch keine Beweisfotos hochgeladen – schnapp dir dein erstes Motiv!
          </MarginNote>
        ) : (
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {game.feed.map((item) => (
              <button
                key={item.id}
                type="button"
                onClick={() => setLightboxCapture(item)}
                className="group overflow-hidden rounded-md border border-subtle bg-surface text-left transition-colors hover:bg-surface-muted focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
              >
                <div className="aspect-video overflow-hidden bg-surface-muted">
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
                <div className="space-y-1 p-3">
                  <div className="flex items-center justify-between gap-2 text-xs text-secondary">
                    <span className="font-semibold text-ink">
                      {item.catcher.displayName}
                    </span>
                    <span className="tabular-nums">
                      {new Date(item.capturedAt).toLocaleTimeString("de-AT", {
                        hour: "2-digit",
                        minute: "2-digit",
                      })}
                    </span>
                  </div>
                  {item.isBlurred ? (
                    <Badge variant="warning">Ziel & Auftrag verborgen</Badge>
                  ) : (
                    <p className="line-clamp-2 text-xs text-ink">
                      {item.promptText}
                    </p>
                  )}
                </div>
              </button>
            ))}
          </div>
        )}
      </section>

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
          <DialogHeader>
            <DialogTitle>Beweisfoto hochladen</DialogTitle>
            <DialogDescription>
              {selectedTile?.promptText}
              {selectedTile?.targetUser
                ? ` · Ziel: ${selectedTile.targetUser.displayName}`
                : ""}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4">
            <label className="flex min-h-11 cursor-pointer flex-col items-center justify-center gap-2 rounded-md border border-dashed border-control bg-surface-muted p-6 text-center transition-colors hover:bg-surface">
              <Camera className="size-6 text-primary" aria-hidden="true" />
              <span className="text-sm font-medium text-ink">
                Kamera öffnen oder Foto auswählen
              </span>
              <span className="text-xs text-secondary">
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
              <div className="overflow-hidden rounded-md border border-subtle bg-surface-muted">
                {/* biome-ignore lint/performance/noImgElement: local File preview blob URL */}
                <img
                  src={previewUrl}
                  alt="Vorschau"
                  className="max-h-64 w-full object-contain"
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
      {/* Winner Crown & Gold Medal Banner */}
      {game.winner ? (
        <Card className="border-warning/60 bg-warning/10">
          <CardContent className="flex flex-wrap items-center justify-between gap-4 py-6">
            <div className="flex items-center gap-4">
              <PlayerAvatar user={game.winner} size="lg" />
              <div>
                <div className="flex items-center gap-2">
                  <Badge variant="warning" className="gap-1">
                    <Crown className="size-3 text-warning" aria-hidden="true" />
                    BINGO-SIEGER
                  </Badge>
                  <Badge variant="warning" className="gap-1">
                    <Award className="size-3 text-warning" aria-hidden="true" />
                    +1 Goldmedaille
                  </Badge>
                </div>
                <h2 className="mt-1 font-serif text-2xl font-semibold text-ink">
                  {game.winner.displayName} gewinnt „{game.title}“!
                </h2>
                <p className="text-sm text-secondary">
                  {game.winner.effectiveTitle} · {game.winner.goldMedalCount}{" "}
                  Goldmedaillen gesamt
                </p>
              </div>
            </div>
            {game.startedAt && game.endedAt ? (
              <div className="flex items-center gap-1.5 text-xs text-secondary tabular-nums">
                <Timer className="size-4" aria-hidden="true" />
                Beendet am {new Date(game.endedAt).toLocaleString("de-AT")}
              </div>
            ) : null}
          </CardContent>
        </Card>
      ) : null}

      {/* Automated Post-Game Superlatives */}
      {game.superlatives.length > 0 ? (
        <section className="space-y-3">
          <h2 className="font-serif text-lg font-semibold text-ink">
            Auszeichnungen
          </h2>
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            {game.superlatives.map((sup) => {
              const meta = superlativeMeta(sup.kind, sup.metricValue);
              const Icon = meta.icon;
              return (
                <Card key={sup.kind}>
                  <CardContent className="space-y-2 pt-4">
                    <div className="flex items-center gap-2 text-xs font-semibold tracking-wide text-secondary uppercase">
                      <Icon
                        className="size-4 text-primary"
                        aria-hidden="true"
                      />
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

      {/* Full Game Photo Gallery */}
      <section className="space-y-3">
        <div className="flex items-center justify-between">
          <h2 className="font-serif text-lg font-semibold text-ink">
            Spiel-Galerie
          </h2>
          <SheetNumber>{game.feed.length} Fotos</SheetNumber>
        </div>

        {game.feed.length === 0 ? (
          <MarginNote label="Keine Fotos">
            In diesem Spiel wurden keine Fotos eingereicht.
          </MarginNote>
        ) : (
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
            {game.feed.map((cap) => (
              <div
                key={cap.id}
                className="overflow-hidden rounded-md border border-subtle bg-surface"
              >
                <button
                  type="button"
                  onClick={() => setSelectedCapture(cap)}
                  className="aspect-square w-full overflow-hidden bg-surface-muted focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
                >
                  <AuthenticatedImage
                    storageKey={cap.imageStorageKey ?? cap.thumbnailStorageKey}
                    alt={cap.promptText ?? "Beweisfoto"}
                    className="h-full w-full transition-transform hover:scale-105"
                  />
                </button>
                <div className="space-y-1.5 p-2.5">
                  <p className="line-clamp-2 text-xs font-medium text-ink">
                    {cap.promptText}
                  </p>
                  <div className="flex items-center justify-between gap-1">
                    <PlayerAvatar user={cap.catcher} size="sm" />
                    <span className="text-[11px] text-secondary tabular-nums">
                      {new Date(cap.capturedAt).toLocaleTimeString("de-AT", {
                        hour: "2-digit",
                        minute: "2-digit",
                      })}
                    </span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </section>

      <Perforation />

      {/* All Unlocked Boards */}
      <section className="space-y-4">
        <h2 className="font-serif text-lg font-semibold text-ink">
          Alle aufgedeckten Bingo-Bretter
        </h2>
        <div className="grid gap-6 lg:grid-cols-2">
          {game.allBoards.map((board) => (
            <Card key={board.id}>
              <CardHeader>
                <div className="flex items-center justify-between gap-2">
                  <PlayerBadgeLink user={board.owner} compact />
                  {board.hasBingo ? (
                    <Badge variant="warning">Bingo!</Badge>
                  ) : null}
                </div>
              </CardHeader>
              <CardContent>
                <div
                  className={cn(
                    "grid gap-1.5",
                    game.gridSize === 3
                      ? "grid-cols-3"
                      : game.gridSize === 4
                        ? "grid-cols-4"
                        : "grid-cols-5",
                  )}
                >
                  {board.tiles.map((tile) => (
                    <div
                      key={tile.id}
                      className={cn(
                        "relative flex min-h-16 flex-col justify-between overflow-hidden rounded-sm border p-1.5 text-[11px]",
                        tile.isCaptured
                          ? "border-success/50 bg-success/10"
                          : "border-subtle bg-surface-muted",
                      )}
                    >
                      {tile.imageStorageKey ? (
                        <div className="absolute inset-0 opacity-30">
                          <AuthenticatedImage
                            storageKey={tile.imageStorageKey}
                            alt={tile.promptText}
                            className="h-full w-full"
                          />
                        </div>
                      ) : null}
                      <span className="relative z-10 line-clamp-3 leading-tight font-medium text-ink">
                        {tile.promptText}
                      </span>
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      </section>

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
