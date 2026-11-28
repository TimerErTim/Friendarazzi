import type {
  BoardTileView,
  BoardView,
  CaptureFeedItem,
  PlayerBadge,
} from "@/api/friendarazzi-api";

export type BoardViewMode = "grid" | "list";

const VIEW_MODE_KEY = "friendarazzi:board_view_mode";
const PINNED_TILES_PREFIX = "friendarazzi:pinned_tiles:";
const COACH_DISMISSED_KEY = "friendarazzi:capture_coach_dismissed";

export type TargetGroup = {
  target: PlayerBadge;
  tiles: BoardTileView[];
  totalCount: number;
  capturedCount: number;
  openCount: number;
  hasPinnedTile: boolean;
};

export type CrossGameTileEntry = {
  gameId: string;
  gameTitle: string;
  tile: BoardTileView;
  isPinned: boolean;
};

export type CrossGameTargetGroup = {
  target: PlayerBadge;
  entries: CrossGameTileEntry[];
  totalCount: number;
  capturedCount: number;
  openCount: number;
  hasPinnedTile: boolean;
};

export type MatchTargetAggregate = {
  target: PlayerBadge;
  entries: Array<{
    boardOwner: PlayerBadge;
    tile: BoardTileView;
    capture: CaptureFeedItem | null;
  }>;
  totalCount: number;
  capturedCount: number;
};

export function tileToCaptureFeedItem(
  tile: BoardTileView,
  boardOwner: PlayerBadge,
  feed: readonly CaptureFeedItem[],
): CaptureFeedItem | null {
  if (!tile.isCaptured) return null;
  if (tile.captureId) {
    const fromFeed = feed.find((item) => item.id === tile.captureId);
    if (fromFeed) {
      return {
        ...fromFeed,
        imageStorageKey:
          fromFeed.imageStorageKey ?? tile.imageStorageKey ?? null,
        promptText: fromFeed.promptText ?? tile.promptText,
        target: fromFeed.target ?? tile.targetUser,
      };
    }
  }
  const key = tile.imageStorageKey ?? null;
  if (!key) return null;
  return {
    id: tile.captureId ?? tile.id,
    tileId: tile.id,
    catcher: boardOwner,
    target: tile.targetUser,
    promptText: tile.promptText,
    imageStorageKey: key,
    thumbnailStorageKey: key,
    isBlurred: false,
    capturedAt: tile.capturedAt ?? new Date().toISOString(),
  };
}

export function filterTileMatches(
  tile: BoardTileView,
  options: { hideCaptured?: boolean; query?: string },
): boolean {
  if (options.hideCaptured && tile.isCaptured) {
    return false;
  }
  const normalizedQuery =
    options.query?.trim().toLocaleLowerCase("de-DE") ?? "";
  if (!normalizedQuery) {
    return true;
  }
  const targetMatch = tile.targetUser.displayName
    .toLocaleLowerCase("de-DE")
    .includes(normalizedQuery);
  const promptMatch = tile.promptText
    .toLocaleLowerCase("de-DE")
    .includes(normalizedQuery);
  return targetMatch || promptMatch;
}

export function groupTilesByTarget(
  tiles: readonly BoardTileView[],
  options: {
    pinnedTileIds?: ReadonlySet<string>;
    hideCaptured?: boolean;
    query?: string;
  } = {},
): TargetGroup[] {
  const byTarget = new Map<
    string,
    { target: PlayerBadge; allTiles: BoardTileView[]; visible: BoardTileView[] }
  >();

  for (const tile of tiles) {
    let bucket = byTarget.get(tile.targetUser.id);
    if (!bucket) {
      bucket = {
        target: tile.targetUser,
        allTiles: [],
        visible: [],
      };
      byTarget.set(tile.targetUser.id, bucket);
    }
    bucket.allTiles.push(tile);
    if (filterTileMatches(tile, options)) {
      bucket.visible.push(tile);
    }
  }

  const pinnedSet = options.pinnedTileIds;
  const groups: TargetGroup[] = [];

  for (const bucket of byTarget.values()) {
    if (bucket.visible.length === 0) continue;

    const sortedTiles = [...bucket.visible].sort((a, b) => {
      if (a.isCaptured !== b.isCaptured) {
        return a.isCaptured ? 1 : -1;
      }
      const aPinned = !a.isCaptured && (pinnedSet?.has(a.id) ?? false);
      const bPinned = !b.isCaptured && (pinnedSet?.has(b.id) ?? false);
      if (aPinned !== bPinned) {
        return aPinned ? -1 : 1;
      }
      if (a.row !== b.row) return a.row - b.row;
      return a.col - b.col;
    });

    const capturedCount = bucket.allTiles.filter((t) => t.isCaptured).length;
    const totalCount = bucket.allTiles.length;
    const hasPinnedTile = sortedTiles.some(
      (t) => !t.isCaptured && (pinnedSet?.has(t.id) ?? false),
    );
    groups.push({
      target: bucket.target,
      tiles: sortedTiles,
      totalCount,
      capturedCount,
      openCount: totalCount - capturedCount,
      hasPinnedTile,
    });
  }

  groups.sort((a, b) => {
    if (a.hasPinnedTile !== b.hasPinnedTile) {
      return a.hasPinnedTile ? -1 : 1;
    }
    if (a.openCount !== b.openCount) {
      return b.openCount - a.openCount;
    }
    return a.target.displayName.localeCompare(b.target.displayName, "de");
  });

  return groups;
}

export function groupActiveGamesByTarget(
  games: ReadonlyArray<{
    id: string;
    title: string;
    tiles: readonly BoardTileView[];
    pinnedTileIds?: ReadonlySet<string>;
  }>,
  query = "",
): CrossGameTargetGroup[] {
  const byTarget = new Map<
    string,
    {
      target: PlayerBadge;
      totalCount: number;
      capturedCount: number;
      entries: CrossGameTileEntry[];
    }
  >();

  for (const game of games) {
    for (const tile of game.tiles) {
      let bucket = byTarget.get(tile.targetUser.id);
      if (!bucket) {
        bucket = {
          target: tile.targetUser,
          totalCount: 0,
          capturedCount: 0,
          entries: [],
        };
        byTarget.set(tile.targetUser.id, bucket);
      }

      bucket.totalCount += 1;
      if (tile.isCaptured) {
        bucket.capturedCount += 1;
        continue;
      }

      if (filterTileMatches(tile, { hideCaptured: true, query })) {
        bucket.entries.push({
          gameId: game.id,
          gameTitle: game.title,
          tile,
          isPinned: game.pinnedTileIds?.has(tile.id) ?? false,
        });
      }
    }
  }

  const groups: CrossGameTargetGroup[] = [];
  for (const bucket of byTarget.values()) {
    if (bucket.entries.length === 0) continue;

    bucket.entries.sort((a, b) => {
      if (a.isPinned !== b.isPinned) {
        return a.isPinned ? -1 : 1;
      }
      const gameDiff = a.gameTitle.localeCompare(b.gameTitle, "de");
      if (gameDiff !== 0) return gameDiff;
      if (a.tile.row !== b.tile.row) return a.tile.row - b.tile.row;
      return a.tile.col - b.tile.col;
    });

    groups.push({
      target: bucket.target,
      entries: bucket.entries,
      totalCount: bucket.totalCount,
      capturedCount: bucket.capturedCount,
      openCount: bucket.totalCount - bucket.capturedCount,
      hasPinnedTile: bucket.entries.some((entry) => entry.isPinned),
    });
  }

  groups.sort((a, b) => {
    if (a.hasPinnedTile !== b.hasPinnedTile) {
      return a.hasPinnedTile ? -1 : 1;
    }
    if (a.openCount !== b.openCount) {
      return b.openCount - a.openCount;
    }
    return a.target.displayName.localeCompare(b.target.displayName, "de");
  });

  return groups;
}

export function aggregateMatchByTarget(
  boards: readonly BoardView[],
  feed: readonly CaptureFeedItem[],
  query = "",
): MatchTargetAggregate[] {
  const normalizedQuery = query.trim().toLocaleLowerCase("de-DE");
  const byTarget = new Map<string, MatchTargetAggregate>();

  for (const board of boards) {
    for (const tile of board.tiles) {
      if (normalizedQuery) {
        const matches =
          tile.targetUser.displayName
            .toLocaleLowerCase("de-DE")
            .includes(normalizedQuery) ||
          tile.promptText
            .toLocaleLowerCase("de-DE")
            .includes(normalizedQuery) ||
          board.owner.displayName
            .toLocaleLowerCase("de-DE")
            .includes(normalizedQuery);
        if (!matches) continue;
      }

      let group = byTarget.get(tile.targetUser.id);
      if (!group) {
        group = {
          target: tile.targetUser,
          entries: [],
          totalCount: 0,
          capturedCount: 0,
        };
        byTarget.set(tile.targetUser.id, group);
      }

      const capture = tileToCaptureFeedItem(tile, board.owner, feed);
      group.entries.push({
        boardOwner: board.owner,
        tile,
        capture,
      });
      group.totalCount += 1;
      if (tile.isCaptured) {
        group.capturedCount += 1;
      }
    }
  }

  const result = Array.from(byTarget.values());
  for (const group of result) {
    group.entries.sort((a, b) => {
      if (a.tile.isCaptured !== b.tile.isCaptured) {
        return a.tile.isCaptured ? -1 : 1;
      }
      return a.boardOwner.displayName.localeCompare(
        b.boardOwner.displayName,
        "de",
      );
    });
  }

  result.sort((a, b) => {
    if (a.capturedCount !== b.capturedCount) {
      return b.capturedCount - a.capturedCount;
    }
    return a.target.displayName.localeCompare(b.target.displayName, "de");
  });

  return result;
}

export function togglePinnedTile(
  current: readonly string[],
  tileId: string,
): string[] {
  if (current.includes(tileId)) {
    return current.filter((id) => id !== tileId);
  }
  return [...current, tileId];
}

export function loadBoardViewMode(): BoardViewMode {
  if (typeof window === "undefined") return "grid";
  try {
    const raw = window.localStorage.getItem(VIEW_MODE_KEY);
    return raw === "list" ? "list" : "grid";
  } catch {
    return "grid";
  }
}

export function saveBoardViewMode(mode: BoardViewMode): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(VIEW_MODE_KEY, mode);
  } catch {
    // Ignore storage errors
  }
}

export function loadPinnedTiles(gameId: string): string[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(`${PINNED_TILES_PREFIX}${gameId}`);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return Array.from(
      new Set(
        parsed.filter((item): item is string => typeof item === "string"),
      ),
    );
  } catch {
    return [];
  }
}

export function savePinnedTiles(
  gameId: string,
  tileIds: readonly string[],
): void {
  if (typeof window === "undefined") return;
  try {
    const unique = Array.from(new Set(tileIds));
    const key = `${PINNED_TILES_PREFIX}${gameId}`;
    if (unique.length === 0) {
      window.localStorage.removeItem(key);
      return;
    }
    window.localStorage.setItem(key, JSON.stringify(unique));
  } catch {
    // Ignore storage errors
  }
}

export function hasDismissedCaptureCoachMark(): boolean {
  if (typeof window === "undefined") return false;
  try {
    return window.localStorage.getItem(COACH_DISMISSED_KEY) === "1";
  } catch {
    return false;
  }
}

export function dismissCaptureCoachMark(): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(COACH_DISMISSED_KEY, "1");
  } catch {
    // Ignore storage errors
  }
}
