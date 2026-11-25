import { describe, expect, it } from "vitest";

import {
  formatGameProgress,
  gameNextActionLabel,
  type OverviewGameShape,
  remainingCountdownSeconds,
  sortActiveGames,
} from "./game-overview.ts";

function sampleGame(
  overrides: Partial<OverviewGameShape> & { id: string },
): OverviewGameShape {
  return {
    status: "Lobby",
    needsAction: false,
    playableTileCount: 9,
    playerCount: 3,
    serverNow: "2026-11-20T00:00:00.000Z",
    createdAt: "2026-11-19T20:00:00.000Z",
    ...overrides,
  };
}

describe("game-overview", () => {
  it("remainingCountdownSeconds derives remaining seconds from serverNow", () => {
    const remaining = remainingCountdownSeconds(
      "2026-11-20T00:01:00.000Z",
      "2026-11-20T00:00:15.000Z",
      10_200,
    );
    expect(remaining).toBe(35);
    expect(
      remainingCountdownSeconds(
        "2026-11-20T00:01:00.000Z",
        "2026-11-20T00:00:15.000Z",
        50_000,
      ),
    ).toBe(0);
    expect(
      remainingCountdownSeconds(null, "2026-11-20T00:00:15.000Z", 0),
    ).toBeNull();
  });

  it("sortActiveGames puts needsAction first, then Active, then waiting games newest-first", () => {
    const lobbyOld = sampleGame({
      id: "lobby-old",
      status: "Lobby",
      needsAction: false,
      createdAt: "2026-11-18T10:00:00.000Z",
    });
    const activeNew = sampleGame({
      id: "active-new",
      status: "Active",
      needsAction: false,
      startedAt: "2026-11-19T22:00:00.000Z",
    });
    const activeOld = sampleGame({
      id: "active-old",
      status: "Active",
      needsAction: false,
      startedAt: "2026-11-19T12:00:00.000Z",
    });
    const draftingReady = sampleGame({
      id: "draft-waiting",
      status: "Drafting",
      needsAction: false,
      myDraftLocked: true,
      createdAt: "2026-11-19T23:00:00.000Z",
    });
    const draftingNeedsAction = sampleGame({
      id: "draft-action",
      status: "Drafting",
      needsAction: true,
      myDraftLocked: false,
      createdAt: "2026-11-19T15:00:00.000Z",
    });

    const sorted = sortActiveGames([
      lobbyOld,
      activeOld,
      draftingReady,
      activeNew,
      draftingNeedsAction,
    ]);
    expect(sorted.map((g) => g.id)).toEqual([
      "draft-action",
      "active-new",
      "active-old",
      "draft-waiting",
      "lobby-old",
    ]);
  });

  it("formatGameProgress and gameNextActionLabel reflect phase and countdown", () => {
    const drafting = sampleGame({
      id: "d1",
      status: "Drafting",
      needsAction: true,
      myDraftFilled: 4,
      myDraftLocked: false,
      readyPlayerCount: 1,
      playerCount: 3,
    });
    expect(formatGameProgress(drafting, null)).toBe("Entwurf 4/9 · 1/3 bereit");
    expect(gameNextActionLabel(drafting)).toBe("Entwurf fortsetzen");

    const countdown = sampleGame({
      id: "d2",
      status: "Drafting",
      myDraftLocked: true,
      draftClosesAt: "2026-11-20T00:01:00.000Z",
    });
    expect(formatGameProgress(countdown, 42)).toBe("Start in 42s");
    expect(gameNextActionLabel(countdown)).toBe("Status ansehen");

    const active = sampleGame({
      id: "a1",
      status: "Active",
      myCapturedTiles: 5,
      playableTileCount: 9,
    });
    expect(formatGameProgress(active, null)).toBe("5 / 9 Felder");
    expect(gameNextActionLabel(active)).toBe("Weiter jagen");
  });
});
