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
 * Bake browser-applied orientation into pixel data when the browser rotates a
 * sensor-landscape photo (`capture="environment"` portrait shots) or when the
 * source format (e.g. HEIC) needs transcoding before upload. Falls back to the
 * original file untouched so backend EXIF/XMP handling stays intact.
 */
export async function normalizeImageForUpload(
  file: File,
  deps?: NormalizeImageDeps,
): Promise<File> {
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

  let fromImage: ImageBitmap | null = null;
  let raw: ImageBitmap | null = null;
  try {
    fromImage = await createBitmap(file, { imageOrientation: "from-image" });
    try {
      raw = await createBitmap(file, { imageOrientation: "none" });
    } catch {
      raw = null;
    }

    const mime = file.type.toLowerCase();
    const browserRotated =
      raw !== null &&
      (fromImage.width !== raw.width || fromImage.height !== raw.height);
    const needsTranscode = mime !== "" && !BACKEND_NATIVE_IMAGE_TYPES.has(mime);

    if (!browserRotated && !needsTranscode) {
      return file;
    }

    const target = computeScaledDimensions(fromImage.width, fromImage.height);
    const canvas = createCanvas();
    canvas.width = target.width;
    canvas.height = target.height;
    const ctx = canvas.getContext("2d");
    if (!ctx) {
      return file;
    }

    ctx.drawImage(fromImage, 0, 0, target.width, target.height);
    const outputMime = mime === "image/png" ? "image/png" : "image/jpeg";
    const blob = await canvasToBlob(canvas, outputMime, 0.92);
    if (!blob) {
      return file;
    }

    const ext = outputMime === "image/png" ? ".png" : ".jpg";
    return new File([blob], replaceExtension(file.name, ext), {
      type: outputMime,
      lastModified: file.lastModified,
    });
  } catch {
    return file;
  } finally {
    fromImage?.close();
    raw?.close();
  }
}
