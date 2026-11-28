import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { UserGallery } from "@/api/friendarazzi-api";

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

vi.mock("@/api/auth-provider", () => ({
  useAuth: () => ({ status: "authenticated" }),
}));

vi.mock("@/api/hooks", () => ({
  useBenutzerQuery: () => ({ data: me }),
  useUserGalleryQuery: () => ({ isLoading: false, data: galleryFixture }),
}));

describe("CollectionsPage", () => {
  it("shows all related photos flat and sorted newest first, with optional grouping", () => {
    render(<CollectionsPage />);

    const images = screen.getAllByTestId("auth-image");
    expect(images.map((el) => el.textContent)).toEqual([
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
