import assert from "node:assert/strict";
import test from "node:test";

import { supportsMobileCameraCapture } from "./media-capture.ts";

test("returns true on mobile touch devices with capture API", () => {
  assert.equal(
    supportsMobileCameraCapture({
      hasCaptureAttr: true,
      isCoarsePointer: true,
      maxTouchPoints: 5,
      userAgent: "Mozilla/5.0 (Linux; Android 15; Pixel 9) AppleWebKit/537.36",
    }),
    true,
  );
  assert.equal(
    supportsMobileCameraCapture({
      hasCaptureAttr: true,
      isCoarsePointer: false,
      maxTouchPoints: 5,
      userAgent: "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X)",
      userAgentMobile: true,
    }),
    true,
  );
});

test("returns false on desktop browsers even when HTMLInputElement.capture exists", () => {
  assert.equal(
    supportsMobileCameraCapture({
      hasCaptureAttr: true,
      isCoarsePointer: false,
      maxTouchPoints: 0,
      userAgent: "Mozilla/5.0 (X11; Linux x86_64) Chrome/131.0.0.0",
    }),
    false,
  );
  assert.equal(
    supportsMobileCameraCapture({
      hasCaptureAttr: true,
      isCoarsePointer: false,
      maxTouchPoints: 10,
      userAgent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/131.0.0.0",
      userAgentMobile: false,
    }),
    false,
  );
});

test("returns false when HTML Media Capture attribute is unavailable", () => {
  assert.equal(
    supportsMobileCameraCapture({
      hasCaptureAttr: false,
      isCoarsePointer: true,
      maxTouchPoints: 5,
      userAgent: "Mozilla/5.0 (Linux; Android 15)",
      userAgentMobile: true,
    }),
    false,
  );
});
