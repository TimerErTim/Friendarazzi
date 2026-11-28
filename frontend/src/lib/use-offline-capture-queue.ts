"use client";

import { useCallback, useEffect, useRef, useState } from "react";

import type { GameDetail } from "@/api/friendarazzi-api";
import { useFriendarazziMutations } from "@/api/hooks";
import { normalizeImageForUpload } from "@/lib/media-capture";
import {
  type BoardSnapshot,
  flushGameCaptureQueue,
  listQueuedCaptures,
  loadBoardSnapshot,
  putQueuedCapture,
  type QueuedCaptureItem,
  queuedCaptureToFile,
  removeQueuedCapture,
  saveBoardSnapshot,
  triggerCaptureDownload,
  updateQueuedCapture,
} from "@/lib/offline-capture-queue";

function createQueueId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return `cap-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

export function useOfflineCaptureQueue(game: GameDetail) {
  const mutations = useFriendarazziMutations();
  const submitCaptureAsync = mutations.submitCapture.mutateAsync;
  const [items, setItems] = useState<QueuedCaptureItem[]>([]);
  const [cachedSnapshot, setCachedSnapshot] = useState<BoardSnapshot | null>(
    null,
  );
  const [isOnline, setIsOnline] = useState(() =>
    typeof navigator === "undefined" ? true : navigator.onLine,
  );
  const flushingRef = useRef(false);

  const reload = useCallback(async () => {
    const list = await listQueuedCaptures(game.id);
    setItems(list);
  }, [game.id]);

  // Cache caller's current board tiles locally so offline assignment still knows open targets.
  useEffect(() => {
    if (!game.myBoard) return;
    const snapshot: BoardSnapshot = {
      gameId: game.id,
      title: game.title,
      updatedAt: new Date().toISOString(),
      tiles: game.myBoard.tiles.map((t) => ({
        id: t.id,
        promptText: t.promptText,
        targetUserId: t.targetUser.id,
        targetDisplayName: t.targetUser.displayName,
        isCaptured: t.isCaptured,
      })),
    };
    void saveBoardSnapshot(snapshot).then(() => setCachedSnapshot(snapshot));
  }, [game.id, game.title, game.myBoard]);

  useEffect(() => {
    void reload();
    void loadBoardSnapshot(game.id).then((snap) => {
      if (snap) setCachedSnapshot((prev) => prev ?? snap);
    });
  }, [game.id, reload]);

  const flushNow = useCallback(async () => {
    if (flushingRef.current) return;
    flushingRef.current = true;
    try {
      await flushGameCaptureQueue(
        game.id,
        async (item) => {
          if (!item.tileId) return;
          const file = queuedCaptureToFile(item);
          const normalized = await normalizeImageForUpload(file);
          await submitCaptureAsync({
            gameId: item.gameId,
            tileId: item.tileId,
            file: normalized,
          });
        },
        reload,
      );
    } finally {
      flushingRef.current = false;
    }
  }, [game.id, submitCaptureAsync, reload]);

  useEffect(() => {
    const handleOnline = () => {
      setIsOnline(true);
      void flushNow();
    };
    const handleOffline = () => setIsOnline(false);
    const handleFocus = () => {
      if (typeof navigator === "undefined" || navigator.onLine) {
        void flushNow();
      }
    };

    window.addEventListener("online", handleOnline);
    window.addEventListener("offline", handleOffline);
    window.addEventListener("focus", handleFocus);
    return () => {
      window.removeEventListener("online", handleOnline);
      window.removeEventListener("offline", handleOffline);
      window.removeEventListener("focus", handleFocus);
    };
  }, [flushNow]);

  const enqueueTrayPhoto = useCallback(
    async (file: File): Promise<QueuedCaptureItem> => {
      const normalized = await normalizeImageForUpload(file);
      const item: QueuedCaptureItem = {
        id: createQueueId(),
        gameId: game.id,
        tileId: null,
        blob: normalized,
        fileName: normalized.name || "schnappschuss.jpg",
        mimeType: normalized.type || "image/jpeg",
        createdAt: new Date().toISOString(),
        status: "tray",
      };
      await putQueuedCapture(item);
      await reload();
      return item;
    },
    [game.id, reload],
  );

  const enqueueForTile = useCallback(
    async (tileId: string, file: File): Promise<QueuedCaptureItem> => {
      const normalized = await normalizeImageForUpload(file);
      const item: QueuedCaptureItem = {
        id: createQueueId(),
        gameId: game.id,
        tileId,
        blob: normalized,
        fileName: normalized.name || "beweis.jpg",
        mimeType: normalized.type || "image/jpeg",
        createdAt: new Date().toISOString(),
        status: "pending",
      };
      await putQueuedCapture(item);
      await reload();
      void flushNow();
      return item;
    },
    [game.id, reload, flushNow],
  );

  const assignAndEnqueue = useCallback(
    async (itemId: string, tileId: string) => {
      await updateQueuedCapture(itemId, {
        tileId,
        status: "pending",
        errorMessage: undefined,
      });
      await reload();
      void flushNow();
    },
    [reload, flushNow],
  );

  const retryItem = useCallback(
    async (itemId: string) => {
      await updateQueuedCapture(itemId, {
        status: "pending",
        errorMessage: undefined,
      });
      await reload();
      void flushNow();
    },
    [reload, flushNow],
  );

  const removeItem = useCallback(
    async (itemId: string) => {
      await removeQueuedCapture(itemId);
      await reload();
    },
    [reload],
  );

  const downloadItem = useCallback((item: QueuedCaptureItem) => {
    triggerCaptureDownload(item);
  }, []);

  return {
    items,
    cachedSnapshot,
    isOnline,
    enqueueTrayPhoto,
    enqueueForTile,
    assignAndEnqueue,
    retryItem,
    removeItem,
    downloadItem,
    flushNow,
  };
}
