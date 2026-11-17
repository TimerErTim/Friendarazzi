/** Hierarchical keys for TanStack Query — invalidate at the right granularity. */
export const logandaimKeys = {
  root: ["friendarazzi"] as const,
  health: () => [...logandaimKeys.root, "health"] as const,
  benutzer: () => [...logandaimKeys.root, "benutzer"] as const,
  decks: () => [...logandaimKeys.root, "decks"] as const,
  games: {
    all: () => [...logandaimKeys.root, "games"] as const,
    detail: (id: string) => [...logandaimKeys.games.all(), id] as const,
    byCode: (code: string) =>
      [...logandaimKeys.games.all(), "by-code", code] as const,
  },
  profiles: {
    all: () => [...logandaimKeys.root, "profiles"] as const,
    detail: (id: string) => [...logandaimKeys.profiles.all(), id] as const,
    gallery: (id: string) =>
      [...logandaimKeys.profiles.all(), id, "gallery"] as const,
  },
  media: (key: string) => [...logandaimKeys.root, "media", key] as const,
};
