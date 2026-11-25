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

const MAX_UPLOAD_IMAGE_EDGE = 1600;
const BACKEND_NATIVE_IMAGE_TYPES = new Set([
  "image/jpeg",
  "image/png",
  "image/webp",
]);
const NON_NATIVE_IMAGE_EXT_RE = /\.(heic|heif|avif)$/i;

export function computeScaledDimensions(
  width: number,
  height: number,
  maxEdge = MAX_UPLOAD_IMAGE_EDGE,
): { width: number; height: number } {
  if (width <= 0 || height <= 0) return { width: 1, height: 1 };
  const longest = Math.max(width, height);
  if (longest <= maxEdge) return { width, height };
  const scale = maxEdge / longest;
  return {
    width: Math.max(1, Math.round(width * scale)),
    height: Math.max(1, Math.round(height * scale)),
  };
}

export interface NormalizeImageDeps {
  createBitmap?: typeof createImageBitmap;
  createCanvas?: () => HTMLCanvasElement;
}

function replaceExtension(filename: string, ext: string): string {
  const dot = filename.lastIndexOf(".");
  const base = dot > 0 ? filename.slice(0, dot) : filename;
  return `${base}${ext}`;
}

function isBackendNativeImage(file: File): boolean {
  const mime = file.type.toLowerCase();
  if (mime !== "") {
    return BACKEND_NATIVE_IMAGE_TYPES.has(mime);
  }
  return !NON_NATIVE_IMAGE_EXT_RE.test(file.name);
}

function canvasToBlob(
  canvas: HTMLCanvasElement,
  mimeType: string,
  quality: number,
): Promise<Blob | null> {
  return new Promise((resolve) => {
    canvas.toBlob((blob) => resolve(blob), mimeType, quality);
  });
}

/**
 * Pass backend-native files (JPEG/PNG/WebP) through untouched so the server
 * receives genuine raw camera bytes and EXIF/XMP metadata, while transcoding
 * browser-only formats like HEIC/AVIF into an upright JPEG capped at 1600px.
 */
export async function normalizeImageForUpload(
  file: File,
  deps?: NormalizeImageDeps,
): Promise<File> {
  if (isBackendNativeImage(file)) {
    return file;
  }

  const createBitmap =
    deps?.createBitmap ??
    (typeof createImageBitmap === "function" ? createImageBitmap : undefined);
  const createCanvas =
    deps?.createCanvas ??
    (typeof document !== "undefined"
      ? () => document.createElement("canvas")
      : undefined);

  if (!createBitmap || !createCanvas) {
    return file;
  }

  let bitmap: ImageBitmap | null = null;
  try {
    bitmap = await createBitmap(file, { imageOrientation: "from-image" });
    const target = computeScaledDimensions(bitmap.width, bitmap.height);
    const canvas = createCanvas();
    canvas.width = target.width;
    canvas.height = target.height;
    const ctx = canvas.getContext("2d");
    if (!ctx) {
      return file;
    }

    ctx.drawImage(bitmap, 0, 0, target.width, target.height);
    bitmap.close();
    bitmap = null;

    const blob = await canvasToBlob(canvas, "image/jpeg", 0.92);
    if (!blob) {
      return file;
    }

    return new File([blob], replaceExtension(file.name, ".jpg"), {
      type: "image/jpeg",
      lastModified: file.lastModified,
    });
  } catch {
    return file;
  } finally {
    bitmap?.close();
  }
}
