import { describe, expect, it } from "vitest";

import {
  authHrefWithRedirect,
  inviteUrl,
  normalizeBasePath,
  safeRedirectPath,
  withBasePath,
} from "./invite-url.ts";

describe("invite-url", () => {
  it("inviteUrl handles empty basePath and custom domain", () => {
    expect(
      inviteUrl("ab12cd", { origin: "https://friendarazzi.app", basePath: "" }),
    ).toBe("https://friendarazzi.app/join/?code=AB12CD");
  });

  it("inviteUrl handles GitHub Pages basePath with or without trailing slash", () => {
    expect(
      inviteUrl("A7K9P2", {
        origin: "https://timerertim.github.io/",
        basePath: "/Friendarazzi/",
      }),
    ).toBe("https://timerertim.github.io/Friendarazzi/join/?code=A7K9P2");
  });

  it("inviteUrl encodes special characters cleanly", () => {
    expect(
      inviteUrl("a+b/c", { origin: "http://localhost:3000", basePath: "" }),
    ).toBe("http://localhost:3000/join/?code=A%2BB%2FC");
  });

  it("normalizeBasePath and withBasePath avoid double prefixes", () => {
    expect(normalizeBasePath("Friendarazzi/")).toBe("/Friendarazzi");
    expect(normalizeBasePath("/")).toBe("");
    expect(withBasePath("/Friendarazzi/join", "/Friendarazzi")).toBe(
      "/Friendarazzi/join",
    );
  });

  it("safeRedirectPath rejects external/protocol-relative redirects", () => {
    expect(safeRedirectPath("/join?code=XYZ")).toBe("/join?code=XYZ");
    expect(safeRedirectPath("https://evil.example")).toBe("/");
    expect(safeRedirectPath("//evil.example/join")).toBe("/");
    expect(safeRedirectPath("/\\evil.example")).toBe("/");
    expect(safeRedirectPath("javascript:alert(1)")).toBe("/");
  });

  it("authHrefWithRedirect preserves in-app join redirects", () => {
    expect(authHrefWithRedirect("/anmeldung", "/join?code=A7K9P2")).toBe(
      "/anmeldung?redirect=%2Fjoin%3Fcode%3DA7K9P2",
    );
    expect(authHrefWithRedirect("/registrierung", "https://evil.example")).toBe(
      "/registrierung",
    );
  });
});
