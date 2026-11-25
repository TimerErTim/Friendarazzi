export type OverviewGameShape = {
  id: string;
  status: "Lobby" | "Drafting" | "Active" | "Completed";
  needsAction: boolean;
  playableTileCount: number;
  playerCount: number;
  myCapturedTiles?: number | null;
  myDraftFilled?: number | null;
  myDraftLocked?: boolean | null;
  readyPlayerCount?: number | null;
  draftClosesAt?: string | null;
  serverNow: string;
  startedAt?: string | null;
  createdAt: string;
};

/** Compute remaining countdown seconds using server clock anchor to resist client clock skew. */
export function remainingCountdownSeconds(
  draftClosesAt: string | null | undefined,
  serverNow: string | null | undefined,
  elapsedSinceFetchMs: number,
): number | null {
  if (!draftClosesAt || !serverNow) return null;
  const deadlineMs = Date.parse(draftClosesAt);
  const serverMs = Date.parse(serverNow);
  if (Number.isNaN(deadlineMs) || Number.isNaN(serverMs)) return null;
  const remainingMs =
    deadlineMs - (serverMs + Math.max(0, elapsedSinceFetchMs));
  return Math.max(0, Math.ceil(remainingMs / 1000));
}

function statusPriority(game: OverviewGameShape): number {
  if (game.needsAction) return 0;
  switch (game.status) {
    case "Active":
      return 1;
    case "Drafting":
      return 2;
    case "Lobby":
      return 3;
    case "Completed":
      return 4;
  }
}

function activityTimestampMs(game: OverviewGameShape): number {
  const raw = game.startedAt ?? game.createdAt;
  const parsed = Date.parse(raw);
  return Number.isNaN(parsed) ? 0 : parsed;
}

export function sortActiveGames<T extends OverviewGameShape>(
  games: readonly T[],
): T[] {
  return [...games].sort((a, b) => {
    const priorityDiff = statusPriority(a) - statusPriority(b);
    if (priorityDiff !== 0) return priorityDiff;
    const timeDiff = activityTimestampMs(b) - activityTimestampMs(a);
    if (timeDiff !== 0) return timeDiff;
    return a.id.localeCompare(b.id);
  });
}

export function gameNextActionLabel(game: OverviewGameShape): string {
  switch (game.status) {
    case "Active":
      return "Weiter jagen";
    case "Drafting":
      return game.myDraftLocked ? "Status ansehen" : "Entwurf fortsetzen";
    case "Lobby":
      return "Lobby öffnen";
    case "Completed":
      return "Rückblick ansehen";
  }
}

export function formatGameProgress(
  game: OverviewGameShape,
  countdownSeconds: number | null,
): string {
  switch (game.status) {
    case "Active":
      return `${game.myCapturedTiles ?? 0} / ${game.playableTileCount} Felder`;
    case "Drafting": {
      if (countdownSeconds !== null) {
        return `Start in ${countdownSeconds}s`;
      }
      const filled = game.myDraftFilled ?? 0;
      const ready = game.readyPlayerCount ?? 0;
      return `Entwurf ${filled}/${game.playableTileCount} · ${ready}/${game.playerCount} bereit`;
    }
    case "Lobby":
      return `${game.playerCount} Spieler`;
    case "Completed":
      return `${game.myCapturedTiles ?? 0} / ${game.playableTileCount} Felder`;
  }
}
