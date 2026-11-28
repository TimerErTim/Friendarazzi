"use client";

export interface BeforeInstallPromptEvent extends Event {
  prompt(): Promise<void>;
  userChoice: Promise<{ outcome: "accepted" | "dismissed" }>;
}

let deferredPrompt: BeforeInstallPromptEvent | null = null;
let serviceWorkerRegistered = false;
const listeners = new Set<() => void>();

function notify() {
  for (const listener of listeners) {
    listener();
  }
}

export function registerPwaSupport(): void {
  if (typeof window === "undefined") return;

  if (!serviceWorkerRegistered && "serviceWorker" in navigator) {
    serviceWorkerRegistered = true;
    const basePath = (
      process.env.NEXT_PUBLIC_FRIENDARAZZI_BASE_PATH ?? ""
    ).replace(/\/+$/, "");
    void navigator.serviceWorker
      .register(`${basePath}/sw.js`)
      .catch(() => undefined);
  }

  window.addEventListener("beforeinstallprompt", (event) => {
    event.preventDefault();
    deferredPrompt = event as BeforeInstallPromptEvent;
    notify();
  });

  window.addEventListener("appinstalled", () => {
    deferredPrompt = null;
    notify();
  });
}

export function subscribeInstallPrompt(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function getInstallPromptAvailable(): boolean {
  return deferredPrompt !== null;
}

export async function triggerInstallPrompt(): Promise<boolean> {
  if (!deferredPrompt) return false;
  const promptEvent = deferredPrompt;
  deferredPrompt = null;
  notify();
  await promptEvent.prompt();
  const choice = await promptEvent.userChoice;
  return choice.outcome === "accepted";
}
