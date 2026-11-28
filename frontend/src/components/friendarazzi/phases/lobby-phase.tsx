"use client";

import {
  Check,
  Eye,
  EyeOff,
  Loader2,
  LogOut,
  Shield,
  Sparkles,
  UserMinus,
  Users,
  XCircle,
} from "lucide-react";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useRef, useState } from "react";

import { apiErrorMessage } from "@/api/errors";
import {
  type GameDetail,
  type PlayerBadge,
  VisibilityMode,
} from "@/api/friendarazzi-api";
import { useBenutzerQuery, useFriendarazziMutations } from "@/api/hooks";
import {
  type DeadlinePresetKey,
  DraftDeadlinePicker,
  formatDeadlineShort,
  toLocalDateTimeInputValue,
} from "@/components/friendarazzi/draft-deadline-picker";
import { InviteShareCard } from "@/components/friendarazzi/invite-share-card";
import { PlayerBadgeLink } from "@/components/friendarazzi/player-avatar";
import { Alert } from "@/components/ui/alert";
import { CalloutNote } from "@/components/ui/brutal-elements";
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
import { cn } from "@/lib/cn";
import { paths } from "@/lib/navigation";

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
  const router = useRouter();
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
  const [draftDeadlineAt, setDraftDeadlineAt] = useState<string | null>(
    game.draftDeadlineAt ?? null,
  );
  const [deadlinePreset, setDeadlinePreset] = useState<DeadlinePresetKey>(
    game.draftDeadlineAt ? "custom" : "none",
  );
  const [deadlineCustomLocal, setDeadlineCustomLocal] = useState(() =>
    toLocalDateTimeInputValue(game.draftDeadlineAt),
  );
  const [endsAt, setEndsAt] = useState<string | null>(game.endsAt ?? null);
  const [endsAtPreset, setEndsAtPreset] = useState<DeadlinePresetKey>(
    game.endsAt ? "custom" : "none",
  );
  const [endsAtCustomLocal, setEndsAtCustomLocal] = useState(() =>
    toLocalDateTimeInputValue(game.endsAt),
  );
  const [leaveConfirmOpen, setLeaveConfirmOpen] = useState(false);
  const [abandonConfirmOpen, setAbandonConfirmOpen] = useState(false);
  const [kickTarget, setKickTarget] = useState<PlayerBadge | null>(null);
  const [transferTarget, setTransferTarget] = useState<PlayerBadge | null>(
    null,
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
    draftDeadlineAt: game.draftDeadlineAt ?? null,
    endsAt: game.endsAt ?? null,
  });
  const savedSettingsRef = useRef({
    title: game.title,
    gridSize: String(game.gridSize),
    visibilityMode: game.visibilityMode,
    draftDeadlineAt: game.draftDeadlineAt ?? null,
    endsAt: game.endsAt ?? null,
  });
  const lastSavedRef = useRef(savedSettingsRef.current);

  useEffect(() => {
    const serverDeadline = game.draftDeadlineAt ?? null;
    const serverEndsAt = game.endsAt ?? null;
    if (!isEditingTitleRef.current) {
      setTitle(game.title);
      latestValuesRef.current.title = game.title;
    }
    setGridSize(String(game.gridSize));
    setVisibilityMode(game.visibilityMode);
    setDraftDeadlineAt(serverDeadline);
    setDeadlinePreset((prev) =>
      serverDeadline === null ? "none" : prev === "none" ? "custom" : prev,
    );
    setDeadlineCustomLocal(toLocalDateTimeInputValue(serverDeadline));
    setEndsAt(serverEndsAt);
    setEndsAtPreset((prev) =>
      serverEndsAt === null ? "none" : prev === "none" ? "custom" : prev,
    );
    setEndsAtCustomLocal(toLocalDateTimeInputValue(serverEndsAt));
    latestValuesRef.current.gridSize = String(game.gridSize);
    latestValuesRef.current.visibilityMode = game.visibilityMode;
    latestValuesRef.current.draftDeadlineAt = serverDeadline;
    latestValuesRef.current.endsAt = serverEndsAt;
    savedSettingsRef.current = {
      title: game.title,
      gridSize: String(game.gridSize),
      visibilityMode: game.visibilityMode,
      draftDeadlineAt: serverDeadline,
      endsAt: serverEndsAt,
    };
    lastSavedRef.current = savedSettingsRef.current;
  }, [
    game.title,
    game.gridSize,
    game.visibilityMode,
    game.draftDeadlineAt,
    game.endsAt,
  ]);

  useEffect(() => {
    if (!justSaved) return;
    const timer = setTimeout(() => setJustSaved(false), 1800);
    return () => clearTimeout(timer);
  }, [justSaved]);

  const commitSettings = useCallback(
    (
      nextTitle: string,
      nextGridSize: string,
      nextMode: VisibilityMode,
      nextDeadline: string | null = latestValuesRef.current.draftDeadlineAt,
      nextEndsAt: string | null = latestValuesRef.current.endsAt,
    ) => {
      setHasPendingDebounce(false);
      const trimmedTitle = nextTitle.trim();
      if (!trimmedTitle) return settingsSaveRef.current;
      const next = {
        title: trimmedTitle,
        gridSize: nextGridSize,
        visibilityMode: nextMode,
        draftDeadlineAt: nextDeadline,
        endsAt: nextEndsAt,
      };
      const prev = lastSavedRef.current;
      if (
        prev.title === next.title &&
        prev.gridSize === next.gridSize &&
        prev.visibilityMode === next.visibilityMode &&
        prev.draftDeadlineAt === next.draftDeadlineAt &&
        prev.endsAt === next.endsAt
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
              draftDeadlineAt: next.draftDeadlineAt,
              endsAt: next.endsAt,
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
    await commitSettings(
      latest.title,
      latest.gridSize,
      latest.visibilityMode,
      latest.draftDeadlineAt,
      latest.endsAt,
    );
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
          latest.draftDeadlineAt,
          latest.endsAt,
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
              {game.players.map((player) => {
                const canManagePlayer = isHost && player.id !== me.data?.id;
                return (
                  <li
                    key={player.id}
                    className="flex items-center justify-between gap-2 py-2 first:pt-0 last:pb-0"
                  >
                    <div className="min-w-0 flex-1">
                      <PlayerBadgeLink user={player} compact />
                    </div>
                    {canManagePlayer ? (
                      <div className="flex shrink-0 items-center gap-1">
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon"
                          aria-label={`${player.displayName} zum Host machen`}
                          title="Host übergeben"
                          onClick={() => setTransferTarget(player)}
                        >
                          <Shield className="size-4" aria-hidden="true" />
                        </Button>
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon"
                          aria-label={`${player.displayName} entfernen`}
                          title="Aus Lobby entfernen"
                          onClick={() => setKickTarget(player)}
                        >
                          <UserMinus className="size-4" aria-hidden="true" />
                        </Button>
                      </div>
                    ) : null}
                  </li>
                );
              })}
            </ul>
            {!canStart ? (
              <p className="border-t-2 border-subtle pt-3 font-mono text-xs font-bold text-secondary">
                Noch {openSlots} weitere(r) Mitspieler nötig (min. 3).
              </p>
            ) : null}
            <div className="flex flex-wrap gap-2 border-t-2 border-subtle pt-3">
              <Button
                type="button"
                variant="secondary"
                size="sm"
                className="flex-1"
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
                  className="flex-1 text-error hover:bg-error-subtle"
                  onClick={() => setAbandonConfirmOpen(true)}
                >
                  <XCircle className="size-4" aria-hidden="true" />
                  Spiel abbrechen
                </Button>
              ) : null}
            </div>
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

              <DraftDeadlinePicker
                preset={deadlinePreset}
                customLocalValue={deadlineCustomLocal}
                currentIso={draftDeadlineAt}
                onSelectPreset={(preset, nextIso) => {
                  setDeadlinePreset(preset);
                  setDraftDeadlineAt(nextIso);
                  latestValuesRef.current.draftDeadlineAt = nextIso;
                  if (nextIso || preset === "none") {
                    void commitSettings(
                      title,
                      gridSize,
                      visibilityMode,
                      nextIso,
                    );
                  }
                }}
                onCustomChange={(local, nextIso) => {
                  setDeadlineCustomLocal(local);
                  setDraftDeadlineAt(nextIso);
                  latestValuesRef.current.draftDeadlineAt = nextIso;
                  if (nextIso) {
                    void commitSettings(
                      title,
                      gridSize,
                      visibilityMode,
                      nextIso,
                    );
                  }
                }}
              />

              <DraftDeadlinePicker
                legend="Spiel endet spätestens am (optional)"
                description="Wenn bis dahin niemand Bingo hat, gewinnt die Person mit den meisten ausgefüllten Feldern."
                preset={endsAtPreset}
                customLocalValue={endsAtCustomLocal}
                currentIso={endsAt}
                onSelectPreset={(preset, nextIso) => {
                  setEndsAtPreset(preset);
                  setEndsAt(nextIso);
                  latestValuesRef.current.endsAt = nextIso;
                  if (nextIso || preset === "none") {
                    void commitSettings(
                      title,
                      gridSize,
                      visibilityMode,
                      latestValuesRef.current.draftDeadlineAt,
                      nextIso,
                    );
                  }
                }}
                onCustomChange={(local, nextIso) => {
                  setEndsAtCustomLocal(local);
                  setEndsAt(nextIso);
                  latestValuesRef.current.endsAt = nextIso;
                  if (nextIso) {
                    void commitSettings(
                      title,
                      gridSize,
                      visibilityMode,
                      latestValuesRef.current.draftDeadlineAt,
                      nextIso,
                    );
                  }
                }}
              />

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
          {game.draftDeadlineAt
            ? ` Entwurfsfrist: ${formatDeadlineShort(game.draftDeadlineAt)}.`
            : ""}
          {game.endsAt
            ? ` Spielzeitlimit: ${formatDeadlineShort(game.endsAt)}.`
            : ""}
        </CalloutNote>
      )}

      <Dialog open={leaveConfirmOpen} onOpenChange={setLeaveConfirmOpen}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Runde verlassen?</DialogTitle>
            <DialogDescription>
              {isHost
                ? game.players.length > 1
                  ? "Als Host überträgst du beim Verlassen automatisch die Host-Rolle an die nächste Person in der Lobby."
                  : "Du bist die letzte Person in dieser Lobby – beim Verlassen wird sie aufgelöst."
                : "Du verlässt diese Lobby. Über den Einladungscode kannst du später wieder beitreten."}
            </DialogDescription>
          </DialogHeader>
          {mutations.leaveGame.isError ? (
            <Alert variant="error">
              {apiErrorMessage(mutations.leaveGame.error, {
                fallback: "Lobby konnte nicht verlassen werden.",
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
            <DialogTitle>
              {kickTarget?.displayName} aus der Lobby entfernen?
            </DialogTitle>
            <DialogDescription>
              Der Mitspieler wird aus der Lobby entfernt und kann über den
              Einladungscode später bei Bedarf wieder beitreten.
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
              Danach kann {transferTarget?.displayName} Spieleinstellungen
              ändern und die Entwurfsphase starten.
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
              vergeben wird. Diese Aktion kann nicht rückgängig gemacht werden.
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
