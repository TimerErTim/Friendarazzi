import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import {
  type GameDetail,
  GameStatus,
  type GameSummary,
  type UserGallery,
  VisibilityMode,
} from "@/api/friendarazzi-api";

import CollectionsPage from "./page";

vi.mock("next/navigation", () => ({
  useRouter: () => ({
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
}));

const me = {
  id: "u-me",
  displayName: "Ich",
  goldMedalCount: 2,
  effectiveTitle: "Paparazzi",
  draftLocked: true,
  isHost: false,
};
const lena = {
  id: "u-lena",
  displayName: "Lena",
  goldMedalCount: 1,
  effectiveTitle: "Neuling",
  draftLocked: true,
  isHost: false,
};
const max = {
  id: "u-max",
  displayName: "Max",
  goldMedalCount: 0,
  effectiveTitle: "Neuling",
  draftLocked: true,
  isHost: false,
};

const galleryFixture: UserGallery = {
  user: {
    ...me,
    unlockedTitles: ["Neuling", "Paparazzi"],
    gamesPlayed: 2,
    capturesMade: 1,
    timesCaught: 1,
  },
  captured: [
    {
      gameId: "g-1",
      gameTitle: "Freitagabend",
      photos: [
        {
          id: "cap-old",
          tileId: "t-1",
          catcher: me,
          target: lena,
          promptText: "Älteres Foto",
          imageStorageKey: "cap-old.webp",
          thumbnailStorageKey: "cap-old.webp",
          isBlurred: false,
          capturedAt: "2026-11-27T20:00:00Z",
        },
      ],
    },
  ],
  caught: [
    {
      gameId: "g-2",
      gameTitle: "Urlaub",
      photos: [
        {
          id: "cap-new",
          tileId: "t-2",
          catcher: lena,
          target: me,
          promptText: "Neueres Foto",
          imageStorageKey: "cap-new.webp",
          thumbnailStorageKey: "cap-new.webp",
          isBlurred: false,
          capturedAt: "2026-11-28T18:00:00Z",
        },
      ],
    },
  ],
};

const gamesList: GameSummary[] = [
  {
    id: "g-2",
    inviteCode: "DONE02",
    title: "Urlaub",
    gridSize: 3,
    visibilityMode: VisibilityMode.Undercover,
    status: GameStatus.Completed,
    watching: true,
    playableTileCount: 9,
    playerCount: 3,
    myCapturedTiles: 2,
    needsAction: false,
    serverNow: "2026-11-28T19:00:00Z",
    createdAt: "2026-11-28T10:00:00Z",
  },
];

const completedDetail: GameDetail = {
  id: "g-2",
  inviteCode: "DONE02",
  hostUserId: "u-lena",
  title: "Urlaub",
  gridSize: 3,
  visibilityMode: VisibilityMode.Undercover,
  status: GameStatus.Completed,
  watching: true,
  playableTileCount: 9,
  draftClosesAt: null,
  serverNow: "2026-11-28T19:00:00Z",
  startedAt: "2026-11-28T11:00:00Z",
  endedAt: "2026-11-28T19:00:00Z",
  winner: lena,
  createdAt: "2026-11-28T10:00:00Z",
  players: [me, lena, max],
  myQuotas: [],
  myDrafts: [],
  myBoard: null,
  allBoards: [],
  feed: [
    {
      id: "cap-new",
      tileId: "t-2",
      catcher: lena,
      target: me,
      promptText: "Neueres Foto",
      imageStorageKey: "cap-new.webp",
      thumbnailStorageKey: "cap-new.webp",
      isBlurred: false,
      capturedAt: "2026-11-28T18:00:00Z",
    },
    {
      id: "cap-others",
      tileId: "t-3",
      catcher: lena,
      target: max,
      promptText: "Lena erwischt Max",
      imageStorageKey: "cap-others.webp",
      thumbnailStorageKey: "cap-others.webp",
      isBlurred: false,
      capturedAt: "2026-11-28T19:00:00Z",
    },
  ],
  superlatives: [],
};

vi.mock("@/api/auth-provider", () => ({
  useAuth: () => ({ status: "authenticated" }),
}));

vi.mock("@/api/hooks", () => ({
  useBenutzerQuery: () => ({ data: me }),
  useUserGalleryQuery: () => ({ isLoading: false, data: galleryFixture }),
  useGamesQuery: () => ({ isLoading: false, data: gamesList }),
  useGameDetailsQueries: (ids: readonly string[]) =>
    ids.map((id) => ({
      isLoading: false,
      data: id === "g-2" ? completedDetail : undefined,
    })),
}));

describe("CollectionsPage", () => {
  it("shows all accessible photos including revealed captures between other players, sorted newest first", () => {
    render(<CollectionsPage />);

    const images = screen.getAllByTestId("auth-image");
    expect(images.map((el) => el.textContent)).toEqual([
      "Lena erwischt Max",
      "Neueres Foto",
      "Älteres Foto",
    ]);

    expect(screen.queryByText("Freitagabend")).not.toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Fotos gruppieren"), {
      target: { value: "game" },
    });
    expect(screen.getByText("Urlaub")).toBeInTheDocument();
    expect(screen.getByText("Freitagabend")).toBeInTheDocument();
  });
});
