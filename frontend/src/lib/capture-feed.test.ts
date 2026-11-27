import { describe, expect, it } from "vitest";

import { newestCapturesFirst } from "./capture-feed.ts";

describe("newestCapturesFirst", () => {
  it("orders captures newest-first without mutating the source array", () => {
    const captures = [
      { id: "old", capturedAt: "2026-11-25T10:00:00.000Z" },
      { id: "new", capturedAt: "2026-11-26T10:00:00.000Z" },
      { id: "middle", capturedAt: "2026-11-25T16:00:00.000Z" },
    ];

    expect(newestCapturesFirst(captures).map((capture) => capture.id)).toEqual([
      "new",
      "middle",
      "old",
    ]);
    expect(captures.map((capture) => capture.id)).toEqual([
      "old",
      "new",
      "middle",
    ]);
  });
});
