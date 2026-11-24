import { describe, expect, it, vi } from "vitest";

import {
  computeScaledDimensions,
  normalizeImageForUpload,
  supportsMobileCameraCapture,
} from "./media-capture.ts";

describe("supportsMobileCameraCapture", () => {
  it("returns true on mobile touch devices with capture API", () => {
    expect(
      supportsMobileCameraCapture({
        hasCaptureAttr: true,
        isCoarsePointer: true,
        maxTouchPoints: 5,
        userAgent:
          "Mozilla/5.0 (Linux; Android 15; Pixel 9) AppleWebKit/537.36",
      }),
    ).toBe(true);
    expect(
      supportsMobileCameraCapture({
        hasCaptureAttr: true,
        isCoarsePointer: false,
        maxTouchPoints: 5,
        userAgent: "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X)",
        userAgentMobile: true,
      }),
    ).toBe(true);
  });

  it("returns false on desktop browsers even when HTMLInputElement.capture exists", () => {
    expect(
      supportsMobileCameraCapture({
        hasCaptureAttr: true,
        isCoarsePointer: false,
        maxTouchPoints: 0,
        userAgent: "Mozilla/5.0 (X11; Linux x86_64) Chrome/131.0.0.0",
      }),
    ).toBe(false);
    expect(
      supportsMobileCameraCapture({
        hasCaptureAttr: true,
        isCoarsePointer: false,
        maxTouchPoints: 10,
        userAgent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/131.0.0.0",
        userAgentMobile: false,
      }),
    ).toBe(false);
  });

  it("returns false when HTML Media Capture attribute is unavailable", () => {
    expect(
      supportsMobileCameraCapture({
        hasCaptureAttr: false,
        isCoarsePointer: true,
        maxTouchPoints: 5,
        userAgent: "Mozilla/5.0 (Linux; Android 15)",
        userAgentMobile: true,
      }),
    ).toBe(false);
  });
});

describe("computeScaledDimensions", () => {
  it("preserves portrait aspect ratio when scaling down to 1600px max edge", () => {
    expect(computeScaledDimensions(3024, 4032, 1600)).toEqual({
      width: 1200,
      height: 1600,
    });
    expect(computeScaledDimensions(800, 1200, 1600)).toEqual({
      width: 800,
      height: 1200,
    });
  });
});

describe("normalizeImageForUpload", () => {
  function makeMockBitmap(width: number, height: number) {
    return {
      width,
      height,
      close: vi.fn(),
    } as unknown as ImageBitmap;
  }

  function makeMockCanvas() {
    const drawImage = vi.fn();
    const toBlob = vi.fn((cb: BlobCallback, type?: string) => {
      cb(new Blob(["normalized-bytes"], { type: type ?? "image/jpeg" }));
    });
    const canvas = {
      width: 0,
      height: 0,
      getContext: vi.fn(() => ({ drawImage })),
      toBlob,
    } as unknown as HTMLCanvasElement;
    return { canvas, drawImage, toBlob };
  }

  it("passes native JPEG/PNG/WebP files through untouched without browser decoding", async () => {
    for (const [name, type] of [
      ["camera-shot.jpg", "image/jpeg"],
      ["shot.png", "image/png"],
      ["shot.webp", "image/webp"],
      ["unknown.jpg", ""],
    ]) {
      const original = new File(["raw-bytes"], name, { type });
      const createBitmap = vi.fn();
      const createCanvas = vi.fn();

      const result = await normalizeImageForUpload(original, {
        createBitmap: createBitmap as unknown as typeof createImageBitmap,
        createCanvas,
      });

      expect(result).toBe(original);
      expect(createBitmap).not.toHaveBeenCalled();
      expect(createCanvas).not.toHaveBeenCalled();
    }
  });

  it("transcodes non-native formats like HEIC and empty-MIME .heic into bounded JPEG", async () => {
    for (const [name, type] of [
      ["photo.heic", "image/heic"],
      ["photo.HEIC", ""],
      ["photo.avif", ""],
    ]) {
      const original = new File(["heic-bytes"], name, {
        type,
        lastModified: 123456,
      });
      const bitmap = makeMockBitmap(3024, 4032);
      const createBitmap = vi.fn().mockResolvedValueOnce(bitmap);
      const { canvas, drawImage } = makeMockCanvas();

      const result = await normalizeImageForUpload(original, {
        createBitmap: createBitmap as unknown as typeof createImageBitmap,
        createCanvas: () => canvas,
      });

      expect(createBitmap).toHaveBeenCalledWith(original, {
        imageOrientation: "from-image",
      });
      expect(canvas.width).toBe(1200);
      expect(canvas.height).toBe(1600);
      expect(drawImage).toHaveBeenCalledWith(bitmap, 0, 0, 1200, 1600);
      expect(bitmap.close).toHaveBeenCalledOnce();
      expect(result).not.toBe(original);
      expect(result.name).toBe("photo.jpg");
      expect(result.type).toBe("image/jpeg");
      expect(result.lastModified).toBe(123456);
    }
  });

  it("falls back to original non-native file when createImageBitmap is unavailable or throws", async () => {
    const original = new File(["raw"], "shot.heic", { type: "image/heic" });
    expect(await normalizeImageForUpload(original)).toBe(original);

    const failing = vi.fn().mockRejectedValue(new Error("unsupported"));
    expect(
      await normalizeImageForUpload(original, {
        createBitmap: failing as unknown as typeof createImageBitmap,
        createCanvas: () => makeMockCanvas().canvas,
      }),
    ).toBe(original);
  });
});
