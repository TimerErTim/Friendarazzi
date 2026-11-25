import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { BoardView } from "@/api/friendarazzi-api";
import { BingoBoard } from "./bingo-board";

vi.mock("./media-image", () => ({
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

function makeBoard(): BoardView {
  return {
    id: "board-1",
    hasBingo: false,
    owner: {
      id: "owner-1",
      displayName: "Lena",
      goldMedalCount: 0,
      effectiveTitle: "Rookie",
      draftLocked: true,
      isHost: false,
    },
    tiles: [
      {
        id: "tile-open",
        row: 0,
        col: 0,
        promptText: "Bestellt noch ein zweites Dessert trotz Völlegefühl",
        isCaptured: false,
        targetUser: {
          id: "target-1",
          displayName: "Max",
          goldMedalCount: 0,
          effectiveTitle: "Rookie",
          draftLocked: true,
          isHost: false,
        },
      },
      {
        id: "tile-captured",
        row: 0,
        col: 1,
        promptText: "Macht heimlich ein Spiegel-Selfie",
        isCaptured: true,
        captureId: "cap-1",
        imageStorageKey: "captures/cap-1.jpg",
        capturedAt: "2026-11-26T00:00:00Z",
        targetUser: {
          id: "target-2",
          displayName: "Sam",
          goldMedalCount: 1,
          effectiveTitle: "Pro",
          draftLocked: true,
          isHost: false,
        },
      },
    ],
  };
}

describe("BingoBoard", () => {
  it("focuses empty tiles on target and prompt without an image placeholder", () => {
    const onEmptyTileSelect = vi.fn();

    render(
      <BingoBoard
        board={makeBoard()}
        gridSize={3}
        onEmptyTileSelect={onEmptyTileSelect}
      />,
    );

    const openButton = screen.getByRole("button", {
      name: /Max: Bestellt noch ein zweites Dessert trotz Völlegefühl \(Foto aufnehmen\)/,
    });
    expect(openButton).toHaveTextContent("Max");
    expect(openButton).toHaveTextContent(
      "Bestellt noch ein zweites Dessert trotz Völlegefühl",
    );
    expect(openButton.querySelector("[data-testid='auth-image']")).toBeNull();

    fireEvent.click(openButton);
    expect(onEmptyTileSelect).toHaveBeenCalledWith(
      expect.objectContaining({ id: "tile-open" }),
    );
  });

  it("renders captured tiles full-bleed with deterministic tilt and triggers lightbox", () => {
    const onViewCapture = vi.fn();

    render(
      <BingoBoard
        board={makeBoard()}
        gridSize={3}
        onViewCapture={onViewCapture}
      />,
    );

    const capturedButton = screen.getByRole("button", {
      name: /Sam: Macht heimlich ein Spiegel-Selfie \(erfasst — Foto öffnen\)/,
    });
    const tiltedFrame = capturedButton.firstElementChild;
    expect(tiltedFrame?.className).toMatch(/-?rotate-[12]/);
    expect(
      capturedButton.querySelector("[data-testid='auth-image']"),
    ).toHaveAttribute("data-storage-key", "captures/cap-1.jpg");

    fireEvent.click(capturedButton);
    expect(onViewCapture).toHaveBeenCalledWith(
      expect.objectContaining({ id: "cap-1", tileId: "tile-captured" }),
    );
  });
});
