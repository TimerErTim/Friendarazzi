import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  type GameDetail,
  GameStatus,
  VisibilityMode,
} from "@/api/friendarazzi-api";

import { CompletedPhase } from "./completed-phase";

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

function makeCompletedGame(): GameDetail {
  const alice = {
    id: "u-alice",
    displayName: "Alice",
    goldMedalCount: 1,
    effectiveTitle: "Schnappschuss-Jäger",
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

  return {
    id: "game-done-1",
    inviteCode: "DONE12",
    hostUserId: "u-alice",
    title: "Finale Runde",
    gridSize: 3,
    visibilityMode: VisibilityMode.Undercover,
    status: GameStatus.Completed,
    watching: true,
    playableTileCount: 9,
    draftClosesAt: null,
    serverNow: "2026-11-28T13:00:00Z",
    startedAt: "2026-11-28T11:00:00Z",
    endedAt: "2026-11-28T12:30:00Z",
    winner: alice,
    createdAt: "2026-11-28T10:00:00Z",
    players: [alice, bob],
    myQuotas: [],
    myDrafts: [],
    myBoard: null,
    allBoards: [
      {
        id: "board-a",
        owner: alice,
        hasBingo: true,
        tiles: [
          {
            id: "t-1",
            row: 0,
            col: 0,
            targetUser: bob,
            promptText: "Bestellt Espresso",
            isCaptured: true,
            captureId: "cap-1",
            imageStorageKey: "captures/cap-1.webp",
            capturedAt: "2026-11-28T11:20:00Z",
          },
        ],
      },
    ],
    feed: [
      {
        id: "cap-1",
        tileId: "t-1",
        catcher: alice,
        target: bob,
        promptText: "Bestellt Espresso",
        imageStorageKey: "captures/cap-1.webp",
        thumbnailStorageKey: "thumbs/cap-1.webp",
        isBlurred: false,
        capturedAt: "2026-11-28T11:20:00Z",
      },
    ],
    superlatives: [],
  };
}

describe("CompletedPhase", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it("exposes a target-grouped match overview tab", () => {
    render(<CompletedPhase game={makeCompletedGame()} />);

    const byTargetTab = screen.getByRole("tab", { name: /Nach Ziel \(1\)/i });
    fireEvent.mouseDown(byTargetTab, { button: 0 });
    fireEvent.click(byTargetTab);

    expect(screen.getByText("1/1 erwischt")).toBeInTheDocument();
    expect(screen.getByText("Brett: Alice")).toBeInTheDocument();
    expect(
      screen.getAllByText("Bestellt Espresso").length,
    ).toBeGreaterThanOrEqual(1);
  });
});
