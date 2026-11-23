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

  it("bakes rotated portrait orientation into canvas pixels when browser swaps dimensions", async () => {
    const original = new File(["jpeg-exif6"], "camera-shot.jpg", {
      type: "image/jpeg",
      lastModified: 123456,
    });
    const fromImage = makeMockBitmap(3024, 4032);
    const raw = makeMockBitmap(4032, 3024);
    const createBitmap = vi
      .fn()
      .mockResolvedValueOnce(fromImage)
      .mockResolvedValueOnce(raw);
    const { canvas, drawImage } = makeMockCanvas();

    const result = await normalizeImageForUpload(original, {
      createBitmap: createBitmap as unknown as typeof createImageBitmap,
      createCanvas: () => canvas,
    });

    expect(canvas.width).toBe(1200);
    expect(canvas.height).toBe(1600);
    expect(drawImage).toHaveBeenCalledWith(fromImage, 0, 0, 1200, 1600);
    expect(fromImage.close).toHaveBeenCalledOnce();
    expect(raw.close).toHaveBeenCalledOnce();
    expect(result).not.toBe(original);
    expect(result.name).toBe("camera-shot.jpg");
    expect(result.type).toBe("image/jpeg");
    expect(result.lastModified).toBe(123456);
  });

  it("returns original native file untouched when browser did not rotate dimensions", async () => {
    const original = new File(["jpeg-bytes"], "portrait.jpg", {
      type: "image/jpeg",
    });
    const fromImage = makeMockBitmap(4032, 3024);
    const raw = makeMockBitmap(4032, 3024);
    const createBitmap = vi
      .fn()
      .mockResolvedValueOnce(fromImage)
      .mockResolvedValueOnce(raw);
    const createCanvas = vi.fn();

    const result = await normalizeImageForUpload(original, {
      createBitmap: createBitmap as unknown as typeof createImageBitmap,
      createCanvas,
    });

    expect(result).toBe(original);
    expect(createCanvas).not.toHaveBeenCalled();
    expect(fromImage.close).toHaveBeenCalledOnce();
    expect(raw.close).toHaveBeenCalledOnce();
  });

  it("transcodes non-native formats like HEIC into upright JPEG", async () => {
    const original = new File(["heic-bytes"], "photo.heic", {
      type: "image/heic",
    });
    const fromImage = makeMockBitmap(1200, 1600);
    const raw = makeMockBitmap(1200, 1600);
    const createBitmap = vi
      .fn()
      .mockResolvedValueOnce(fromImage)
      .mockResolvedValueOnce(raw);
    const { canvas } = makeMockCanvas();

    const result = await normalizeImageForUpload(original, {
      createBitmap: createBitmap as unknown as typeof createImageBitmap,
      createCanvas: () => canvas,
    });

    expect(result).not.toBe(original);
    expect(result.name).toBe("photo.jpg");
    expect(result.type).toBe("image/jpeg");
  });

  it("falls back to original file when createImageBitmap is unavailable or throws", async () => {
    const original = new File(["raw"], "shot.jpg", { type: "image/jpeg" });
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
