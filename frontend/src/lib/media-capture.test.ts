import { describe, expect, it } from "vitest";

import { supportsMobileCameraCapture } from "./media-capture.ts";

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
