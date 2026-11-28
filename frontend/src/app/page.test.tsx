import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  type GameDetail,
  GameStatus,
  type GameSummary,
  VisibilityMode,
} from "@/api/friendarazzi-api";
import { loadPinnedTiles, savePinnedTiles } from "@/lib/board-grouping";

import HomePage from "./page";

const pushMock = vi.fn();

vi.mock("next/navigation", () => ({
  useRouter: () => ({
    push: pushMock,
    replace: vi.fn(),
  }),
}));

vi.mock("@/components/friendarazzi/media-image", () => ({
  AuthenticatedImage: ({
    alt,
    storageKey,
  }: {
    alt: string;
    storageKey: string;
  }) => (
    <div data-testid="auth-image" data-storage-key={storageKey}>
      {alt}
    </div>
  ),
}));

vi.mock("@/components/friendarazzi/player-avatar", () => ({
  PlayerAvatar: ({ user }: { user: { displayName: string } }) => (
    <span>{user.displayName}</span>
  ),
  PlayerBadgeLink: ({ user }: { user: { displayName: string } }) => (
    <span>{user.displayName}</span>
  ),
}));

const gamesSummary: GameSummary[] = [
  {
    id: "game-1",
    inviteCode: "CODE01",
    title: "Freitagabend",
    gridSize: 3,
    visibilityMode: VisibilityMode.Undercover,
    status: GameStatus.Active,
    watching: true,
    playableTileCount: 9,
    playerCount: 4,
    myCapturedTiles: 1,
    needsAction: false,
    serverNow: "2026-11-28T18:00:00Z",
    createdAt: "2026-11-28T17:00:00Z",
  },
  {
    id: "game-2",
    inviteCode: "CODE02",
    title: "Urlaubs-Jagd",
    gridSize: 4,
    visibilityMode: VisibilityMode.LiveFeed,
    status: GameStatus.Active,
    watching: true,
    playableTileCount: 16,
    playerCount: 5,
    myCapturedTiles: 2,
    needsAction: false,
    serverNow: "2026-11-28T18:00:00Z",
    createdAt: "2026-11-28T16:00:00Z",
  },
];

const detailsByGameId: Record<string, GameDetail> = {
  "game-1": {
    id: "game-1",
    inviteCode: "CODE01",
    hostUserId: "u-me",
    title: "Freitagabend",
    gridSize: 3,
    visibilityMode: VisibilityMode.Undercover,
    status: GameStatus.Active,
    watching: true,
    playableTileCount: 9,
    draftClosesAt: null,
    serverNow: "2026-11-28T18:00:00Z",
    startedAt: "2026-11-28T17:30:00Z",
    endedAt: null,
    winner: null,
    createdAt: "2026-11-28T17:00:00Z",
    players: [],
    myQuotas: [],
    myDrafts: [],
    myBoard: {
      id: "b-1",
      hasBingo: false,
      owner: {
        id: "u-me",
        displayName: "Ich",
        goldMedalCount: 0,
        effectiveTitle: "Neuling",
        draftLocked: true,
        isHost: true,
      },
      tiles: [
        {
          id: "tile-1",
          row: 0,
          col: 0,
          promptText: "Bestellt Espresso",
          isCaptured: false,
          targetUser: {
            id: "u-lena",
            displayName: "Lena",
            goldMedalCount: 0,
            effectiveTitle: "Neuling",
            draftLocked: true,
            isHost: false,
          },
        },
        {
          id: "tile-done",
          row: 0,
          col: 1,
          promptText: "Schon erledigt",
          isCaptured: true,
          targetUser: {
            id: "u-lena",
            displayName: "Lena",
            goldMedalCount: 0,
            effectiveTitle: "Neuling",
            draftLocked: true,
            isHost: false,
          },
        },
      ],
    },
    allBoards: [],
    feed: [],
    superlatives: [],
  },
  "game-2": {
    id: "game-2",
    inviteCode: "CODE02",
    hostUserId: "u-me",
    title: "Urlaubs-Jagd",
    gridSize: 4,
    visibilityMode: VisibilityMode.LiveFeed,
    status: GameStatus.Active,
    watching: true,
    playableTileCount: 16,
    draftClosesAt: null,
    serverNow: "2026-11-28T18:00:00Z",
    startedAt: "2026-11-28T16:30:00Z",
    endedAt: null,
    winner: null,
    createdAt: "2026-11-28T16:00:00Z",
    players: [],
    myQuotas: [],
    myDrafts: [],
    myBoard: {
      id: "b-2",
      hasBingo: false,
      owner: {
        id: "u-me",
        displayName: "Ich",
        goldMedalCount: 0,
        effectiveTitle: "Neuling",
        draftLocked: true,
        isHost: true,
      },
      tiles: [
        {
          id: "tile-2",
          row: 0,
          col: 0,
          promptText: "Vergisst Sonnenbrille",
          isCaptured: false,
          targetUser: {
            id: "u-max",
            displayName: "Max",
            goldMedalCount: 0,
            effectiveTitle: "Neuling",
            draftLocked: true,
            isHost: false,
          },
        },
      ],
    },
    allBoards: [],
    feed: [],
    superlatives: [],
  },
};

vi.mock("@/api/auth-provider", () => ({
  useAuth: () => ({ status: "authenticated" }),
}));

vi.mock("@/api/hooks", () => ({
  useGamesQuery: () => ({
    isLoading: false,
    data: gamesSummary,
    dataUpdatedAt: Date.now(),
  }),
  useGameDetailsQueries: (gameIds: readonly string[]) =>
    gameIds.map((id) => ({
      isLoading: false,
      data: detailsByGameId[id],
    })),
  useFriendarazziMutations: () => ({
    createGame: { mutateAsync: vi.fn(), isPending: false, isError: false },
    joinGame: { mutateAsync: vi.fn(), isPending: false, isError: false },
    submitCapture: {
      mutateAsync: vi.fn(),
      reset: vi.fn(),
      isPending: false,
      isError: false,
      error: null,
    },
  }),
}));

describe("HomePage", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it("renders all running boards with Raster/Nach Ziel toggle and cross-game pinned tasks", () => {
    savePinnedTiles("game-1", ["tile-1", "tile-done"]);
    savePinnedTiles("game-2", ["tile-2"]);

    render(<HomePage />);

    expect(screen.queryByText(/Laufende Spiele/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/Vergangene Matches/i)).not.toBeInTheDocument();

    expect(screen.getByText("Aktive Bretter (2)")).toBeInTheDocument();
    expect(screen.getByText("Angeheftete Aufgaben (2)")).toBeInTheDocument();
    expect(screen.getAllByText("Bestellt Espresso").length).toBeGreaterThan(1);
    expect(screen.getAllByText("Vergisst Sonnenbrille").length).toBeGreaterThan(
      1,
    );
    expect(screen.getByText("Schon erledigt")).toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("radio", { name: "Liste nach Zielperson" }),
    );
    expect(screen.getByText("Offene Ziele")).toBeInTheDocument();
    expect(screen.queryByText("Schon erledigt")).not.toBeInTheDocument();
    // In Nach Ziel view, open tasks from both games appear under their buddy cards without game-board splitting
    expect(screen.getAllByText("Freitagabend").length).toBeGreaterThanOrEqual(
      1,
    );
    expect(screen.getAllByText("Urlaubs-Jagd").length).toBeGreaterThanOrEqual(
      1,
    );

    const unpinButton = screen.getAllByRole("button", {
      name: "Auftrag „Bestellt Espresso“ aus Fokus entfernen",
    })[0];
    expect(unpinButton).toBeDefined();
    if (unpinButton) {
      fireEvent.click(unpinButton);
    }

    expect(screen.getByText("Angeheftete Aufgaben (1)")).toBeInTheDocument();
    expect(loadPinnedTiles("game-1")).toEqual(["tile-done"]);
  });

  it("orders grid size options as Abend (9), Mittel (16), Wochen (25)", () => {
    render(<HomePage />);

    fireEvent.click(
      screen.getByRole("button", { name: /Neues Spiel hosten/i }),
    );

    expect(screen.getByText("Abend (9 Felder)")).toBeInTheDocument();
    expect(screen.getByText("Mittel (16 Felder)")).toBeInTheDocument();
    expect(screen.getByText("Wochen (25 Felder)")).toBeInTheDocument();
  });
});
