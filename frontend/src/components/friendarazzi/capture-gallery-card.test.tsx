import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { CaptureFeedItem } from "@/api/friendarazzi-api";

import { CaptureGalleryCard } from "./capture-gallery-card";

vi.mock("./media-image", () => ({
  AuthenticatedImage: ({ alt }: { alt: string }) => (
    <div data-testid="auth-image" data-alt={alt} />
  ),
}));

vi.mock("./player-avatar", () => ({
  PlayerAvatar: ({ user }: { user: { displayName: string } }) => (
    <span>{user.displayName.slice(0, 2).toUpperCase()}</span>
  ),
}));

describe("CaptureGalleryCard", () => {
  it("pairs the target display name badge with the target-agnostic prompt text", () => {
    const capture: CaptureFeedItem = {
      id: "cap-1",
      tileId: "tile-1",
      catcher: {
        id: "u-1",
        displayName: "Alice",
        goldMedalCount: 0,
        effectiveTitle: "Rookie",
        draftLocked: true,
        isHost: true,
      },
      target: {
        id: "u-2",
        displayName: "Bob",
        goldMedalCount: 0,
        effectiveTitle: "Rookie",
        draftLocked: true,
        isHost: false,
      },
      promptText: "Bestellt noch eine Runde für alle",
      imageStorageKey: "captures/1.webp",
      thumbnailStorageKey: "captures/1-thumb.webp",
      isBlurred: false,
      capturedAt: "2026-11-27T12:00:00Z",
    };

    render(<CaptureGalleryCard capture={capture} onSelect={vi.fn()} />);

    expect(screen.getByText("Bob")).toBeInTheDocument();
    expect(
      screen.getByText("Bestellt noch eine Runde für alle"),
    ).toBeInTheDocument();
  });
});
