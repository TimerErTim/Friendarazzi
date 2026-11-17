/** Hierarchical keys for TanStack Query — invalidate at the right granularity. */
export const friendarazziKeys = {
  root: ["friendarazzi"] as const,
  health: () => [...friendarazziKeys.root, "health"] as const,
  benutzer: () => [...friendarazziKeys.root, "benutzer"] as const,
  decks: () => [...friendarazziKeys.root, "decks"] as const,
  games: {
    all: () => [...friendarazziKeys.root, "games"] as const,
    detail: (id: string) => [...friendarazziKeys.games.all(), id] as const,
    byCode: (code: string) =>
      [...friendarazziKeys.games.all(), "by-code", code] as const,
  },
  profiles: {
    all: () => [...friendarazziKeys.root, "profiles"] as const,
    detail: (id: string) => [...friendarazziKeys.profiles.all(), id] as const,
    gallery: (id: string) =>
      [...friendarazziKeys.profiles.all(), id, "gallery"] as const,
  },
  media: (key: string) => [...friendarazziKeys.root, "media", key] as const,
};
