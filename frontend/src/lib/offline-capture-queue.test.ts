import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  clearOfflineMemoryForTest,
  flushGameCaptureQueue,
  listQueuedCaptures,
  loadBoardSnapshot,
  putQueuedCapture,
  saveBoardSnapshot,
} from "./offline-capture-queue";

describe("offline-capture-queue", () => {
  beforeEach(async () => {
    await clearOfflineMemoryForTest();
  });

  it("uploads pending items and removes them on success", async () => {
    await putQueuedCapture({
      id: "c1",
      gameId: "g1",
      tileId: "t1",
      blob: new Blob(["img"], { type: "image/jpeg" }),
      fileName: "snap.jpg",
      mimeType: "image/jpeg",
      createdAt: "2026-11-28T10:00:00Z",
      status: "pending",
    });

    const upload = vi.fn().mockResolvedValue(undefined);
    await flushGameCaptureQueue("g1", upload, undefined, () => true);

    expect(upload).toHaveBeenCalledTimes(1);
    expect(await listQueuedCaptures("g1")).toEqual([]);
  });

  it("preserves photo locally with conflict_ended and stops retrying on HTTP 409", async () => {
    await putQueuedCapture({
      id: "c409",
      gameId: "g1",
      tileId: "t1",
      blob: new Blob(["late-photo"], { type: "image/jpeg" }),
      fileName: "buzzer.jpg",
      mimeType: "image/jpeg",
      createdAt: "2026-11-28T10:00:00Z",
      status: "pending",
    });

    const upload = vi.fn().mockRejectedValue({ status: 409 });
    await flushGameCaptureQueue("g1", upload, undefined, () => true);

    const items = await listQueuedCaptures("g1");
    expect(items).toHaveLength(1);
    expect(items[0]?.status).toBe("conflict_ended");
    expect(items[0]?.errorMessage).toContain("Spiel bereits beendet");

    // Subsequent flushes must not retry a conflict_ended item automatically.
    await flushGameCaptureQueue("g1", upload, undefined, () => true);
    expect(upload).toHaveBeenCalledTimes(1);
  });

  it("keeps tray items untouched until a tileId is assigned", async () => {
    await putQueuedCapture({
      id: "tray-1",
      gameId: "g1",
      tileId: null,
      blob: new Blob(["tray"], { type: "image/jpeg" }),
      fileName: "tray.jpg",
      mimeType: "image/jpeg",
      createdAt: "2026-11-28T10:00:00Z",
      status: "tray",
    });

    const upload = vi.fn().mockResolvedValue(undefined);
    await flushGameCaptureQueue("g1", upload, undefined, () => true);

    expect(upload).not.toHaveBeenCalled();
    expect(await listQueuedCaptures("g1")).toHaveLength(1);
  });

  it("saves and loads offline board snapshots per game", async () => {
    await saveBoardSnapshot({
      gameId: "g1",
      title: "Nacht",
      updatedAt: "2026-11-28T10:00:00Z",
      tiles: [
        {
          id: "t1",
          promptText: "Trinkt Wasser",
          targetUserId: "u2",
          targetDisplayName: "Bob",
          isCaptured: false,
        },
      ],
    });

    const loaded = await loadBoardSnapshot("g1");
    expect(loaded?.title).toBe("Nacht");
    expect(loaded?.tiles[0]?.targetDisplayName).toBe("Bob");
  });
});
