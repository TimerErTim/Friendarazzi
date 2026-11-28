import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { CaptureFeedItem } from "@/api/friendarazzi-api";

import { PhotoLightbox } from "./photo-lightbox";
import { QuickCaptureSheet } from "./quick-capture-sheet";

vi.mock("@/components/friendarazzi/media-image", () => ({
  AuthenticatedImage: ({ alt }: { alt: string }) => <div>{alt}</div>,
}));

vi.mock("@/components/friendarazzi/player-avatar", () => ({
  PlayerBadgeLink: ({ user }: { user: { displayName: string } }) => (
    <span>{user.displayName}</span>
  ),
}));

const sampleCapture: CaptureFeedItem = {
  id: "cap-1",
  tileId: "tile-1",
  catcher: {
    id: "u-alice",
    displayName: "Alice",
    goldMedalCount: 0,
    effectiveTitle: "Rookie",
    draftLocked: false,
    isHost: true,
  },
  target: {
    id: "u-bob",
    displayName: "Bob",
    goldMedalCount: 0,
    effectiveTitle: "Rookie",
    draftLocked: false,
    isHost: false,
  },
  promptText: "Trinkt Wasser",
  imageStorageKey: "cap.webp",
  thumbnailStorageKey: "thumb.webp",
  isBlurred: false,
  capturedAt: "2026-11-28T12:00:00Z",
};

describe("PhotoLightbox retract action", () => {
  it("requires confirmation before invoking onRetract", async () => {
    const onRetract = vi.fn().mockResolvedValue(undefined);
    render(
      <PhotoLightbox
        capture={sampleCapture}
        open
        canRetract
        onRetract={onRetract}
        onOpenChange={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /Foto zurückziehen/i }));
    expect(onRetract).not.toHaveBeenCalled();

    fireEvent.click(
      screen.getByRole("button", { name: /Endgültig entfernen/i }),
    );
    await waitFor(() => {
      expect(onRetract).toHaveBeenCalledWith(sampleCapture);
    });
  });
});

describe("QuickCaptureSheet", () => {
  it("assigns a tray photo to a selected open tile", async () => {
    const onAssign = vi.fn().mockResolvedValue(undefined);
    render(
      <QuickCaptureSheet
        open
        onOpenChange={vi.fn()}
        openTiles={[
          {
            id: "tile-2",
            promptText: "Bestellt Snacks",
            targetUserId: "u-bob",
            targetDisplayName: "Bob",
            isCaptured: false,
          },
        ]}
        trayItems={[
          {
            id: "tray-1",
            gameId: "game-1",
            tileId: null,
            blob: new Blob(["snap"], { type: "image/jpeg" }),
            fileName: "snap.jpg",
            mimeType: "image/jpeg",
            createdAt: "2026-11-28T12:00:00Z",
            status: "tray",
          },
        ]}
        onAddTrayPhoto={vi.fn()}
        onAssignTrayItem={onAssign}
        onDiscardTrayItem={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /Bestellt Snacks/i }));
    await waitFor(() => {
      expect(onAssign).toHaveBeenCalledWith("tray-1", "tile-2");
    });
  });
});
