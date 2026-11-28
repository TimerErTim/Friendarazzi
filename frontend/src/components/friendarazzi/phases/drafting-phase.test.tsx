import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import {
  type GameDetail,
  GameStatus,
  VisibilityMode,
} from "@/api/friendarazzi-api";

import { DraftingPhase } from "./drafting-phase";

vi.mock("@/api/hooks", () => ({
  useBenutzerQuery: () => ({
    data: { id: "user-alice", displayName: "Alice" },
  }),
  useDecksQuery: () => ({
    data: [
      {
        id: "deck-1",
        slug: "night-out",
        name: "Night Out",
        description: "Party",
        prompts: ["Bestellt noch eine Runde für alle"],
      },
    ],
  }),
  useErfolgsmeldung: () => ({
    meldung: null,
    melden: vi.fn(),
    verwerfen: vi.fn(),
  }),
  useFriendarazziMutations: () => ({
    savePrompts: { mutateAsync: vi.fn(), isPending: false, isError: false },
    lockPrompts: { mutateAsync: vi.fn(), isPending: false, isError: false },
    unlockPrompts: { mutateAsync: vi.fn(), isPending: false, isError: false },
  }),
}));

vi.mock("@/components/friendarazzi/player-avatar", () => ({
  PlayerAvatar: ({ user }: { user: { displayName: string } }) => (
    <span>{user.displayName}</span>
  ),
  PlayerBadgeLink: ({ user }: { user: { displayName: string } }) => (
    <span>{user.displayName}</span>
  ),
}));

function makeDraftingGame(): GameDetail {
  return {
    id: "game-1",
    inviteCode: "ABC123",
    hostUserId: "user-alice",
    title: "Kiez Tour",
    gridSize: 3,
    visibilityMode: VisibilityMode.Undercover,
    status: GameStatus.Drafting,
    watching: true,
    playableTileCount: 9,
    draftClosesAt: null,
    serverNow: "2026-11-27T12:00:00Z",
    startedAt: null,
    endedAt: null,
    winner: null,
    createdAt: "2026-11-27T11:00:00Z",
    players: [
      {
        id: "user-alice",
        displayName: "Alice",
        goldMedalCount: 0,
        effectiveTitle: "Rookie",
        draftLocked: false,
        isHost: true,
      },
      {
        id: "user-bob",
        displayName: "Bob",
        goldMedalCount: 0,
        effectiveTitle: "Rookie",
        draftLocked: false,
        isHost: false,
      },
      {
        id: "user-carol",
        displayName: "Carol",
        goldMedalCount: 0,
        effectiveTitle: "Rookie",
        draftLocked: false,
        isHost: false,
      },
    ],
    myQuotas: [
      {
        targetUser: {
          id: "user-bob",
          displayName: "Bob",
          goldMedalCount: 0,
          effectiveTitle: "Rookie",
          draftLocked: false,
          isHost: false,
        },
        requiredCount: 1,
      },
      {
        targetUser: {
          id: "user-carol",
          displayName: "Carol",
          goldMedalCount: 0,
          effectiveTitle: "Rookie",
          draftLocked: false,
          isHost: false,
        },
        requiredCount: 1,
      },
    ],
    myDrafts: [],
    myBoard: null,
    allBoards: [],
    feed: [],
    superlatives: [],
  };
}

describe("DraftingPhase", () => {
  it("uses target-agnostic placeholders and groups each slot under its target card", () => {
    const queryClient = new QueryClient();
    render(
      <QueryClientProvider client={queryClient}>
        <DraftingPhase game={makeDraftingGame()} />
      </QueryClientProvider>,
    );

    const bobInput = screen.getByLabelText("Auftrag 1 für Bob");
    const carolInput = screen.getByLabelText("Auftrag 1 für Carol");

    expect(bobInput).toHaveAttribute(
      "placeholder",
      "z.B. Bestellt noch ein Getränk",
    );
    expect(carolInput).toHaveAttribute(
      "placeholder",
      "z.B. Bestellt noch ein Getränk",
    );
    expect(screen.getAllByText("Ziel")).toHaveLength(2);
    expect(screen.getByText("Bob")).toBeInTheDocument();
    expect(screen.getByText("Carol")).toBeInTheDocument();
  });
});
