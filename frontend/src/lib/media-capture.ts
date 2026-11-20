import { useSyncExternalStore } from "react";

export interface CaptureEnvironment {
  hasCaptureAttr: boolean;
  isCoarsePointer: boolean;
  maxTouchPoints: number;
  userAgent: string;
  userAgentMobile?: boolean;
}

const MOBILE_UA_RE = /Android|iPhone|iPad|iPod|Mobile/i;

/**
 * True when the browser supports HTML Media Capture (`<input capture>`) on a
 * touch-first mobile device rather than a desktop with drag-and-drop.
 */
export function supportsMobileCameraCapture(env: CaptureEnvironment): boolean {
  if (!env.hasCaptureAttr) return false;
  if (env.isCoarsePointer) return true;
  return (
    env.maxTouchPoints > 0 &&
    (env.userAgentMobile === true || MOBILE_UA_RE.test(env.userAgent))
  );
}

export function readBrowserCaptureEnvironment(): CaptureEnvironment {
  if (typeof window === "undefined" || typeof navigator === "undefined") {
    return {
      hasCaptureAttr: false,
      isCoarsePointer: false,
      maxTouchPoints: 0,
      userAgent: "",
    };
  }
  const hasCaptureAttr =
    typeof HTMLInputElement !== "undefined" &&
    "capture" in HTMLInputElement.prototype;
  const isCoarsePointer =
    typeof window.matchMedia === "function" &&
    window.matchMedia("(pointer: coarse)").matches;
  const navWithUaData = navigator as Navigator & {
    userAgentData?: { mobile?: boolean };
  };
  return {
    hasCaptureAttr,
    isCoarsePointer,
    maxTouchPoints: navigator.maxTouchPoints ?? 0,
    userAgent: navigator.userAgent ?? "",
    userAgentMobile: navWithUaData.userAgentData?.mobile,
  };
}

function subscribePointerMedia(onChange: () => void): () => void {
  if (
    typeof window === "undefined" ||
    typeof window.matchMedia !== "function"
  ) {
    return () => {};
  }
  const mql = window.matchMedia("(pointer: coarse)");
  mql.addEventListener("change", onChange);
  return () => mql.removeEventListener("change", onChange);
}

function getMobileCaptureSnapshot(): boolean {
  return supportsMobileCameraCapture(readBrowserCaptureEnvironment());
}

/** Reactive client check for mobile HTML Media Capture support, safe under SSR. */
export function useSupportsMobileCameraCapture(): boolean {
  return useSyncExternalStore(
    subscribePointerMedia,
    getMobileCaptureSnapshot,
    () => false,
  );
}
