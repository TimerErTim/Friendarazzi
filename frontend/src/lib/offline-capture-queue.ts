export type PendingCaptureStatus =
  | "tray"
  | "pending"
  | "uploading"
  | "conflict_ended"
  | "error";

export interface QueuedCaptureItem {
  id: string;
  gameId: string;
  tileId: string | null;
  blob: Blob;
  fileName: string;
  mimeType: string;
  createdAt: string;
  status: PendingCaptureStatus;
  errorMessage?: string;
}

export interface CachedBoardTile {
  id: string;
  promptText: string;
  targetUserId: string;
  targetDisplayName: string;
  isCaptured: boolean;
}

export interface BoardSnapshot {
  gameId: string;
  title: string;
  updatedAt: string;
  tiles: CachedBoardTile[];
}

const DB_NAME = "friendarazzi-offline-v1";
const DB_VERSION = 1;
const STORE_CAPTURES = "pending_captures";
const STORE_SNAPSHOTS = "board_snapshots";

const memoryCaptures = new Map<string, QueuedCaptureItem>();
const memorySnapshots = new Map<string, BoardSnapshot>();

function openDb(): Promise<IDBDatabase | null> {
  if (typeof indexedDB === "undefined") {
    return Promise.resolve(null);
  }
  return new Promise((resolve) => {
    try {
      const req = indexedDB.open(DB_NAME, DB_VERSION);
      req.onupgradeneeded = () => {
        const db = req.result;
        if (!db.objectStoreNames.contains(STORE_CAPTURES)) {
          const store = db.createObjectStore(STORE_CAPTURES, {
            keyPath: "id",
          });
          store.createIndex("by_game", "gameId", { unique: false });
        }
        if (!db.objectStoreNames.contains(STORE_SNAPSHOTS)) {
          db.createObjectStore(STORE_SNAPSHOTS, { keyPath: "gameId" });
        }
      };
      req.onsuccess = () => resolve(req.result);
      req.onerror = () => resolve(null);
    } catch {
      resolve(null);
    }
  });
}

function txPromise<T>(req: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

export async function listQueuedCaptures(
  gameId?: string,
): Promise<QueuedCaptureItem[]> {
  const db = await openDb();
  if (!db) {
    const all = Array.from(memoryCaptures.values());
    const filtered = gameId ? all.filter((i) => i.gameId === gameId) : all;
    return filtered.sort((a, b) => a.createdAt.localeCompare(b.createdAt));
  }
  try {
    const tx = db.transaction(STORE_CAPTURES, "readonly");
    const store = tx.objectStore(STORE_CAPTURES);
    const rows = gameId
      ? await txPromise<QueuedCaptureItem[]>(
          store.index("by_game").getAll(gameId),
        )
      : await txPromise<QueuedCaptureItem[]>(store.getAll());
    return rows.sort((a, b) => a.createdAt.localeCompare(b.createdAt));
  } finally {
    db.close();
  }
}

export async function putQueuedCapture(
  item: QueuedCaptureItem,
): Promise<QueuedCaptureItem> {
  const db = await openDb();
  if (!db) {
    memoryCaptures.set(item.id, item);
    return item;
  }
  try {
    const tx = db.transaction(STORE_CAPTURES, "readwrite");
    await txPromise(tx.objectStore(STORE_CAPTURES).put(item));
    return item;
  } finally {
    db.close();
  }
}

export async function updateQueuedCapture(
  id: string,
  patch: Partial<Omit<QueuedCaptureItem, "id" | "gameId">>,
): Promise<QueuedCaptureItem | null> {
  const existing = (await listQueuedCaptures()).find((i) => i.id === id);
  if (!existing) return null;
  const next: QueuedCaptureItem = { ...existing, ...patch };
  return putQueuedCapture(next);
}

export async function removeQueuedCapture(id: string): Promise<void> {
  const db = await openDb();
  if (!db) {
    memoryCaptures.delete(id);
    return;
  }
  try {
    const tx = db.transaction(STORE_CAPTURES, "readwrite");
    await txPromise(tx.objectStore(STORE_CAPTURES).delete(id));
  } finally {
    db.close();
  }
}

export async function saveBoardSnapshot(
  snapshot: BoardSnapshot,
): Promise<void> {
  const db = await openDb();
  if (!db) {
    memorySnapshots.set(snapshot.gameId, snapshot);
    return;
  }
  try {
    const tx = db.transaction(STORE_SNAPSHOTS, "readwrite");
    await txPromise(tx.objectStore(STORE_SNAPSHOTS).put(snapshot));
  } finally {
    db.close();
  }
}

export async function loadBoardSnapshot(
  gameId: string,
): Promise<BoardSnapshot | null> {
  const db = await openDb();
  if (!db) {
    return memorySnapshots.get(gameId) ?? null;
  }
  try {
    const tx = db.transaction(STORE_SNAPSHOTS, "readonly");
    const row = await txPromise<BoardSnapshot | undefined>(
      tx.objectStore(STORE_SNAPSHOTS).get(gameId),
    );
    return row ?? null;
  } finally {
    db.close();
  }
}

export async function clearOfflineMemoryForTest(): Promise<void> {
  memoryCaptures.clear();
  memorySnapshots.clear();
}

export function isConflictStatus(error: unknown): boolean {
  if (typeof error !== "object" || error === null) return false;
  const maybe = error as { status?: unknown };
  return maybe.status === 409;
}

export function queuedCaptureToFile(item: QueuedCaptureItem): File {
  return new File([item.blob], item.fileName, {
    type: item.mimeType || "image/jpeg",
  });
}

export async function flushGameCaptureQueue(
  gameId: string,
  upload: (item: QueuedCaptureItem) => Promise<void>,
  onUpdated?: () => void | Promise<void>,
  isOnline: () => boolean = () =>
    typeof navigator === "undefined" ? true : navigator.onLine,
): Promise<void> {
  if (!isOnline()) return;

  const queue = await listQueuedCaptures(gameId);
  const ready = queue.filter(
    (item) => item.status === "pending" && item.tileId !== null,
  );

  for (const item of ready) {
    if (!isOnline()) break;
    await updateQueuedCapture(item.id, {
      status: "uploading",
      errorMessage: undefined,
    });
    await onUpdated?.();

    try {
      await upload(item);
      await removeQueuedCapture(item.id);
      await onUpdated?.();
    } catch (error) {
      if (isConflictStatus(error)) {
        // Option A: preserve photo locally on post-buzzer / completed conflict and stop retrying.
        await updateQueuedCapture(item.id, {
          status: "conflict_ended",
          errorMessage:
            "Spiel bereits beendet – Foto bleibt lokal gespeichert.",
        });
      } else if (!isOnline()) {
        await updateQueuedCapture(item.id, {
          status: "pending",
          errorMessage: undefined,
        });
      } else {
        const msg =
          error instanceof Error && error.message
            ? error.message
            : "Upload fehlgeschlagen.";
        await updateQueuedCapture(item.id, {
          status: "error",
          errorMessage: msg,
        });
      }
      await onUpdated?.();
    }
  }
}

export function triggerCaptureDownload(item: QueuedCaptureItem): void {
  if (typeof document === "undefined") return;
  const url = URL.createObjectURL(item.blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = item.fileName || "friendarazzi-foto.jpg";
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
