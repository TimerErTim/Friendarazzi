import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  type GameDetail,
  GameStatus,
  VisibilityMode,
} from "@/api/friendarazzi-api";

import { ActivePlayPhase } from "./active-play-phase";

vi.mock("@/api/hooks", () => ({
  useFriendarazziMutations: () => ({
    submitCapture: {
      mutateAsync: vi.fn(),
      reset: vi.fn(),
      isPending: false,
      isError: false,
      error: null,
    },
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

function makeActiveGame(): GameDetail {
  const alice = {
    id: "u-alice",
    displayName: "Alice",
    goldMedalCount: 0,
    effectiveTitle: "Neuling",
    draftLocked: true,
    isHost: true,
  };
  const bob = {
    id: "u-bob",
    displayName: "Bob",
    goldMedalCount: 0,
    effectiveTitle: "Neuling",
    draftLocked: true,
    isHost: false,
  };
  const carol = {
    id: "u-carol",
    displayName: "Carol",
    goldMedalCount: 0,
    effectiveTitle: "Neuling",
    draftLocked: true,
    isHost: false,
  };

  return {
    id: "game-active-1",
    inviteCode: "HUNT12",
    hostUserId: "u-alice",
    title: "Kiez Jagd",
    gridSize: 3,
    visibilityMode: VisibilityMode.LiveFeed,
    status: GameStatus.Active,
    watching: true,
    playableTileCount: 9,
    draftClosesAt: null,
    serverNow: "2026-11-28T12:00:00Z",
    startedAt: "2026-11-28T11:00:00Z",
    endedAt: null,
    winner: null,
    createdAt: "2026-11-28T10:00:00Z",
    players: [alice, bob, carol],
    myQuotas: [],
    myDrafts: [],
    myBoard: {
      id: "board-1",
      owner: alice,
      hasBingo: false,
      tiles: [
        {
          id: "t-bob",
          row: 0,
          col: 0,
          targetUser: bob,
          promptText: "Trinkt Wasser",
          isCaptured: false,
        },
        {
          id: "t-bob-2",
          row: 0,
          col: 2,
          targetUser: bob,
          promptText: "Lacht laut",
          isCaptured: false,
        },
        {
          id: "t-carol",
          row: 0,
          col: 1,
          targetUser: carol,
          promptText: "Schaut auf die Uhr",
          isCaptured: true,
          captureId: "cap-1",
          imageStorageKey: "captures/cap-1.webp",
          capturedAt: "2026-11-28T11:30:00Z",
        },
      ],
    },
    allBoards: [],
    feed: [],
    superlatives: [],
  };
}

describe("ActivePlayPhase", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it("switches views cleanly, hides captured tasks in Nach Ziel, pins individual tasks, and toggles search", () => {
    render(<ActivePlayPhase game={makeActiveGame()} />);

    expect(
      screen.queryByRole("button", { name: /Nur offene/i }),
    ).not.toBeInTheDocument();

    expect(
      screen.getByRole("radio", { name: "Raster-Ansicht" }),
    ).toHaveAttribute("aria-checked", "true");
    expect(
      screen.getAllByText("Schaut auf die Uhr").length,
    ).toBeGreaterThanOrEqual(1);

    fireEvent.click(
      screen.getByRole("radio", { name: "Liste nach Zielperson" }),
    );
    expect(
      screen.getByRole("radio", { name: "Liste nach Zielperson" }),
    ).toHaveAttribute("aria-checked", "true");

    expect(screen.getByText("Trinkt Wasser")).toBeInTheDocument();
    expect(screen.queryByText("Schaut auf die Uhr")).not.toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("button", {
        name: "Auftrag „Trinkt Wasser“ anheften",
      }),
    );
    expect(
      screen.getByRole("button", { name: "Fokus (1)" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", {
        name: "Auftrag „Trinkt Wasser“ aus Fokus entfernen",
      }),
    ).toHaveAttribute("aria-pressed", "true");

    fireEvent.click(
      screen.getByRole("button", {
        name: "Aufträge oder Ziele durchsuchen",
      }),
    );
    expect(
      screen.getByRole("searchbox", {
        name: "Suchfeld für Aufträge oder Ziele",
      }),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("radio", { name: "Raster-Ansicht" }));
    const pinnedGridTile = screen.getByRole("button", {
      name: /Bob: Trinkt Wasser \(Foto aufnehmen, angeheftet\)/,
    });
    expect(
      pinnedGridTile.querySelector("[data-testid='pinned-tile-icon']"),
    ).not.toBeNull();
  });
});
