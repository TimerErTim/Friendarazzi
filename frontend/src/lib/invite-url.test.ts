import assert from "node:assert/strict";
import test from "node:test";

import {
  authHrefWithRedirect,
  inviteUrl,
  normalizeBasePath,
  safeRedirectPath,
  withBasePath,
} from "./invite-url.ts";

test("inviteUrl handles empty basePath and custom domain", () => {
  assert.equal(
    inviteUrl("ab12cd", { origin: "https://friendarazzi.app", basePath: "" }),
    "https://friendarazzi.app/join/?code=AB12CD",
  );
});

test("inviteUrl handles GitHub Pages basePath with or without trailing slash", () => {
  assert.equal(
    inviteUrl("A7K9P2", {
      origin: "https://timerertim.github.io/",
      basePath: "/Friendarazzi/",
    }),
    "https://timerertim.github.io/Friendarazzi/join/?code=A7K9P2",
  );
});

test("inviteUrl encodes special characters cleanly", () => {
  assert.equal(
    inviteUrl("a+b/c", { origin: "http://localhost:3000", basePath: "" }),
    "http://localhost:3000/join/?code=A%2BB%2FC",
  );
});

test("normalizeBasePath and withBasePath avoid double prefixes", () => {
  assert.equal(normalizeBasePath("Friendarazzi/"), "/Friendarazzi");
  assert.equal(normalizeBasePath("/"), "");
  assert.equal(
    withBasePath("/Friendarazzi/join", "/Friendarazzi"),
    "/Friendarazzi/join",
  );
});

test("safeRedirectPath rejects external/protocol-relative redirects", () => {
  assert.equal(safeRedirectPath("/join?code=XYZ"), "/join?code=XYZ");
  assert.equal(safeRedirectPath("https://evil.example"), "/");
  assert.equal(safeRedirectPath("//evil.example/join"), "/");
  assert.equal(safeRedirectPath("/\\evil.example"), "/");
  assert.equal(safeRedirectPath("javascript:alert(1)"), "/");
});

test("authHrefWithRedirect preserves in-app join redirects", () => {
  assert.equal(
    authHrefWithRedirect("/anmeldung", "/join?code=A7K9P2"),
    "/anmeldung?redirect=%2Fjoin%3Fcode%3DA7K9P2",
  );
  assert.equal(
    authHrefWithRedirect("/registrierung", "https://evil.example"),
    "/registrierung",
  );
});
