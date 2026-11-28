import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  type GameDetail,
  GameStatus,
  VisibilityMode,
} from "@/api/friendarazzi-api";

import { DraftingPhase } from "./drafting-phase";

const savePromptsMutate = vi.fn().mockResolvedValue({});
const lockPromptsMutate = vi.fn().mockResolvedValue({});
const unlockPromptsMutate = vi.fn().mockResolvedValue({});
const hostFillDraftsMutate = vi.fn().mockResolvedValue({});

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
      {
        id: "deck-2",
        slug: "office-uni",
        name: "Office & Uni",
        description: "Alltag",
        prompts: ["Klappt spontan den Laptop zu"],
      },
    ],
  }),
  useErfolgsmeldung: () => ({
    meldung: null,
    melden: vi.fn(),
    verwerfen: vi.fn(),
  }),
  useFriendarazziMutations: () => ({
    savePrompts: {
      mutateAsync: savePromptsMutate,
      isPending: false,
      isError: false,
    },
    lockPrompts: {
      mutateAsync: lockPromptsMutate,
      isPending: false,
      isError: false,
    },
    unlockPrompts: {
      mutateAsync: unlockPromptsMutate,
      isPending: false,
      isError: false,
    },
    hostFillDrafts: {
      mutateAsync: hostFillDraftsMutate,
      isPending: false,
      isError: false,
    },
    setDraftDeadline: {
      mutateAsync: vi.fn(),
      isPending: false,
      isError: false,
    },
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
    draftDeadlineAt: null,
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
  beforeEach(() => {
    vi.clearAllMocks();
  });

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
    expect(bobInput).toHaveAttribute("maxLength", "80");
    expect(carolInput).toHaveAttribute(
      "placeholder",
      "z.B. Bestellt noch ein Getränk",
    );
    expect(screen.getAllByText("Ziel")).toHaveLength(2);
    expect(screen.getByText("Bob")).toBeInTheDocument();
    expect(screen.getByText("Carol")).toBeInTheDocument();
    expect(screen.getAllByText("0/80")).toHaveLength(2);
    expect(
      screen.getByRole("option", { name: "Alle Decks mischen" }),
    ).toBeInTheDocument();
  });

  it("updates character counter and supports quick-pick suggestions", async () => {
    const queryClient = new QueryClient();
    render(
      <QueryClientProvider client={queryClient}>
        <DraftingPhase game={makeDraftingGame()} />
      </QueryClientProvider>,
    );

    const carolInput = screen.getByLabelText("Auftrag 1 für Carol");
    fireEvent.focus(carolInput);
    fireEvent.click(
      screen.getByRole("button", {
        name: /\+ Klappt spontan den Laptop zu/i,
      }),
    );

    expect(carolInput).toHaveValue("Klappt spontan den Laptop zu");
    expect(screen.getByText("28/80")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Auftrag 1 für Bob"), {
      target: { value: "x".repeat(65) },
    });
    expect(screen.getByText("65/80")).toBeInTheDocument();
  });

  it("supports one-tap autofill and lock as well as host auto-fill dialog", async () => {
    const queryClient = new QueryClient();
    render(
      <QueryClientProvider client={queryClient}>
        <DraftingPhase game={makeDraftingGame()} />
      </QueryClientProvider>,
    );

    fireEvent.click(
      screen.getByRole("button", { name: /Leere füllen & bereit/i }),
    );
    await waitFor(() => {
      expect(savePromptsMutate).toHaveBeenCalledTimes(1);
      expect(lockPromptsMutate).toHaveBeenCalledWith("game-1");
    });

    fireEvent.click(screen.getByRole("button", { name: /0\/3 bereit/i }));
    fireEvent.click(
      screen.getByRole("button", {
        name: /Offene Plätze automatisch füllen/i,
      }),
    );
    fireEvent.click(
      screen.getByRole("button", { name: /Füllen & sofort starten/i }),
    );
    await waitFor(() => {
      expect(hostFillDraftsMutate).toHaveBeenCalledWith({
        gameId: "game-1",
        startImmediately: true,
      });
    });
  });
});
