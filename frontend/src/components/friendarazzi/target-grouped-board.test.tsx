import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { BoardView } from "@/api/friendarazzi-api";

import { TargetGroupedBoard } from "./target-grouped-board";

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

vi.mock("./player-avatar", () => ({
  PlayerBadgeLink: ({ user }: { user: { displayName: string } }) => (
    <span>{user.displayName}</span>
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
      effectiveTitle: "Neuling",
      draftLocked: true,
      isHost: false,
    },
    tiles: [
      {
        id: "tile-open-max",
        row: 0,
        col: 0,
        promptText: "Bestellt noch ein Dessert",
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
      {
        id: "tile-open-max-2",
        row: 0,
        col: 2,
        promptText: "Lacht über eigenen Witz",
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
      {
        id: "tile-cap-sam",
        row: 0,
        col: 1,
        promptText: "Macht heimlich ein Selfie",
        isCaptured: true,
        captureId: "cap-1",
        imageStorageKey: "captures/cap-1.webp",
        capturedAt: "2026-11-28T12:00:00Z",
        targetUser: {
          id: "u-sam",
          displayName: "Sam",
          goldMedalCount: 1,
          effectiveTitle: "Schnappschuss-Jäger",
          draftLocked: true,
          isHost: false,
        },
      },
    ],
  };
}

describe("TargetGroupedBoard", () => {
  it("groups prompts by target person and supports per-task pin + upload callbacks", () => {
    const onEmptyTileSelect = vi.fn();
    const onViewCapture = vi.fn();
    const onTogglePinTile = vi.fn();

    render(
      <TargetGroupedBoard
        board={makeBoard()}
        pinnedTileIds={new Set(["tile-open-max-2"])}
        onTogglePinTile={onTogglePinTile}
        onEmptyTileSelect={onEmptyTileSelect}
        onViewCapture={onViewCapture}
      />,
    );

    expect(screen.getByText("Max")).toBeInTheDocument();
    expect(screen.getByText("Sam")).toBeInTheDocument();
    expect(screen.getByText("0/2")).toBeInTheDocument();
    expect(screen.getByText("1/1")).toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("button", {
        name: "Auftrag „Bestellt noch ein Dessert“ anheften",
      }),
    );
    expect(onTogglePinTile).toHaveBeenCalledWith("tile-open-max");

    expect(
      screen.getByRole("button", {
        name: "Auftrag „Lacht über eigenen Witz“ aus Fokus entfernen",
      }),
    ).toHaveAttribute("aria-pressed", "true");

    fireEvent.click(
      screen.getByRole("button", {
        name: /Max: Bestellt noch ein Dessert \(Foto aufnehmen\)/,
      }),
    );
    expect(onEmptyTileSelect).toHaveBeenCalledWith(
      expect.objectContaining({ id: "tile-open-max" }),
    );

    fireEvent.click(
      screen.getByRole("button", {
        name: /Sam: Macht heimlich ein Selfie \(erfasst — Foto öffnen\)/,
      }),
    );
    expect(onViewCapture).toHaveBeenCalledWith(
      expect.objectContaining({ id: "cap-1", tileId: "tile-cap-sam" }),
    );
  });

  it("hides captured prompts when hideCaptured is true", () => {
    render(<TargetGroupedBoard board={makeBoard()} hideCaptured />);

    expect(screen.getByText("Bestellt noch ein Dessert")).toBeInTheDocument();
    expect(
      screen.queryByText("Macht heimlich ein Selfie"),
    ).not.toBeInTheDocument();
  });
});
