import { describe, expect, it } from "vitest";

import type {
  BoardTileView,
  BoardView,
  CaptureFeedItem,
  PlayerBadge,
} from "@/api/friendarazzi-api";

import {
  aggregateMatchByTarget,
  groupTilesByTarget,
  loadPinnedTiles,
  savePinnedTiles,
  togglePinnedTile,
} from "./board-grouping";

function player(id: string, displayName: string): PlayerBadge {
  return {
    id,
    displayName,
    goldMedalCount: 0,
    effectiveTitle: "Neuling",
    draftLocked: true,
    isHost: false,
  };
}

function tile(
  id: string,
  row: number,
  col: number,
  target: PlayerBadge,
  promptText: string,
  isCaptured = false,
): BoardTileView {
  return {
    id,
    row,
    col,
    targetUser: target,
    promptText,
    isCaptured,
    captureId: isCaptured ? `cap-${id}` : undefined,
    imageStorageKey: isCaptured ? `captures/${id}.webp` : undefined,
    capturedAt: isCaptured ? "2026-11-28T12:00:00Z" : undefined,
  };
}

describe("board-grouping", () => {
  const max = player("u-max", "Max");
  const lena = player("u-lena", "Lena");
  const sam = player("u-sam", "Sam");

  const tiles: BoardTileView[] = [
    tile("t1", 0, 0, max, "Bestellt Dessert", true),
    tile("t2", 0, 1, lena, "Schaut aufs Handy", false),
    tile("t3", 0, 2, lena, "Lacht laut", false),
    tile("t4", 1, 0, sam, "Trägt Sonnenbrille", false),
    tile("t5", 1, 1, max, "Winkt jemandem zu", false),
  ];

  it("groups tiles by target, sorting groups and open tiles with pinned tasks first", () => {
    const groups = groupTilesByTarget(tiles, {
      pinnedTileIds: new Set(["t4", "t3"]),
    });

    expect(groups.map((g) => g.target.id)).toEqual([
      "u-lena",
      "u-sam",
      "u-max",
    ]);
    expect(groups[0]?.hasPinnedTile).toBe(true);
    expect(groups[0]?.tiles.map((t) => t.id)).toEqual(["t3", "t2"]);
    expect(groups[1]?.hasPinnedTile).toBe(true);
    expect(groups[2]?.hasPinnedTile).toBe(false);
    expect(groups[2]?.tiles.map((t) => t.id)).toEqual(["t5", "t1"]);
    expect(groups[2]?.capturedCount).toBe(1);
    expect(groups[2]?.openCount).toBe(1);
  });

  it("filters out captured tiles and matches search text across target and prompt", () => {
    const openOnly = groupTilesByTarget(tiles, { hideCaptured: true });
    expect(
      openOnly.find((g) => g.target.id === "u-max")?.tiles.map((t) => t.id),
    ).toEqual(["t5"]);

    const byTargetName = groupTilesByTarget(tiles, { query: "lena" });
    expect(byTargetName).toHaveLength(1);
    expect(byTargetName[0]?.target.displayName).toBe("Lena");

    const byPrompt = groupTilesByTarget(tiles, { query: "sonnenbrille" });
    expect(byPrompt).toHaveLength(1);
    expect(byPrompt[0]?.tiles.map((t) => t.id)).toEqual(["t4"]);
  });

  it("toggles and persists pinned tiles per game without evicting earlier pins", () => {
    window.localStorage.clear();
    let pins: string[] = [];
    pins = togglePinnedTile(pins, "t1");
    pins = togglePinnedTile(pins, "t2");
    pins = togglePinnedTile(pins, "t3");
    pins = togglePinnedTile(pins, "t4");
    expect(pins).toEqual(["t1", "t2", "t3", "t4"]);

    pins = togglePinnedTile(pins, "t3");
    expect(pins).toEqual(["t1", "t2", "t4"]);

    savePinnedTiles("game-1", pins);
    expect(loadPinnedTiles("game-1")).toEqual(["t1", "t2", "t4"]);

    savePinnedTiles("game-1", []);
    expect(loadPinnedTiles("game-1")).toEqual([]);
  });

  it("aggregates completed match boards by target person", () => {
    const boards: BoardView[] = [
      {
        id: "b1",
        owner: lena,
        hasBingo: true,
        tiles: [
          tile("t1", 0, 0, max, "Bestellt Dessert", true),
          tile("t2", 0, 1, sam, "Winkt", false),
        ],
      },
      {
        id: "b2",
        owner: sam,
        hasBingo: false,
        tiles: [tile("t3", 0, 0, max, "Schaut auf die Uhr", false)],
      },
    ];
    const feed: CaptureFeedItem[] = [
      {
        id: "cap-t1",
        tileId: "t1",
        catcher: lena,
        target: max,
        promptText: "Bestellt Dessert",
        imageStorageKey: "captures/t1.webp",
        thumbnailStorageKey: "thumbs/t1.webp",
        isBlurred: false,
        capturedAt: "2026-11-28T12:00:00Z",
      },
    ];

    const summary = aggregateMatchByTarget(boards, feed);
    expect(summary.map((s) => s.target.id)).toEqual(["u-max", "u-sam"]);
    expect(summary[0]?.capturedCount).toBe(1);
    expect(summary[0]?.totalCount).toBe(2);
    expect(summary[0]?.entries[0]?.boardOwner.id).toBe("u-lena");
    expect(summary[0]?.entries[0]?.capture?.id).toBe("cap-t1");
  });
});
