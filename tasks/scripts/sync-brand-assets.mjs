#!/usr/bin/env node
// Brand-asset generator and verification gate.
//
// Usage:
//   node tasks/scripts/sync-brand-assets.mjs --write   # Regenerate frontend/public/ from assets/
//   node tasks/scripts/sync-brand-assets.mjs --check   # Verify files, dimensions, and manifest

import { execFileSync } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
} from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("../..", import.meta.url));
const ASSETS = join(ROOT, "assets");
const PUBLIC = join(ROOT, "frontend/public");

const STARTER_FILES = [
  "file.svg",
  "globe.svg",
  "next.svg",
  "vercel.svg",
  "window.svg",
];

const EXPECTED_PNGS = {
  "favicon-16x16.png": [16, 16],
  "favicon-32x32.png": [32, 32],
  "apple-touch-icon.png": [180, 180],
  "android-chrome-192x192.png": [192, 192],
  "android-chrome-512x512.png": [512, 512],
  "android-chrome-maskable-512x512.png": [512, 512],
};

function readPngSize(filePath) {
  const buf = readFileSync(filePath);
  if (buf.length < 24 || buf.toString("ascii", 1, 4) !== "PNG") {
    throw new Error(`Not a valid PNG: ${filePath}`);
  }
  return [buf.readUInt32BE(16), buf.readUInt32BE(20)];
}

function readJpegSize(filePath) {
  const buf = readFileSync(filePath);
  if (buf.length < 4 || buf[0] !== 0xff || buf[1] !== 0xd8) {
    throw new Error(`Not a valid JPEG: ${filePath}`);
  }
  let offset = 2;
  while (offset + 9 < buf.length) {
    if (buf[offset] !== 0xff) {
      offset += 1;
      continue;
    }
    const marker = buf[offset + 1];
    // SOF0..SOF2 carry frame dimensions.
    if (marker >= 0xc0 && marker <= 0xc2) {
      const height = buf.readUInt16BE(offset + 5);
      const width = buf.readUInt16BE(offset + 7);
      return [width, height];
    }
    const len = buf.readUInt16BE(offset + 2);
    offset += 2 + len;
  }
  throw new Error(`Could not find JPEG dimensions: ${filePath}`);
}

function writeAssets() {
  mkdirSync(PUBLIC, { recursive: true });

  for (const starter of STARTER_FILES) {
    const target = join(PUBLIC, starter);
    if (existsSync(target)) {
      rmSync(target);
    }
  }

  // Extract core icon set from the checked-in favicon zip.
  const zipPath = join(ASSETS, "favicon_io.zip");
  const zipFiles = [
    "favicon.ico",
    "favicon-16x16.png",
    "favicon-32x32.png",
    "android-chrome-192x192.png",
    "android-chrome-512x512.png",
  ];
  execFileSync("unzip", ["-o", zipPath, ...zipFiles, "-d", PUBLIC], {
    stdio: "pipe",
  });

  // Single-source vector mark.
  copyFileSync(join(ASSETS, "favicon.svg"), join(PUBLIC, "favicon.svg"));

  // Opaque 180x180 Apple Touch Icon (no alpha corners).
  execFileSync(
    "ffmpeg",
    [
      "-y",
      "-i",
      join(ASSETS, "favicon_raw.jpg"),
      "-vf",
      "scale=180:180",
      join(PUBLIC, "apple-touch-icon.png"),
    ],
    { stdio: "pipe" },
  );

  // 512x512 maskable icon keeping the 3x3 mark inside the 80% Android safe zone.
  execFileSync(
    "ffmpeg",
    [
      "-y",
      "-i",
      join(ASSETS, "favicon_raw.jpg"),
      "-vf",
      "scale=424:424,pad=512:512:(ow-iw)/2:(oh-ih)/2:#FAF902",
      join(PUBLIC, "android-chrome-maskable-512x512.png"),
    ],
    { stdio: "pipe" },
  );

  // 1200x630 social preview (Open Graph / Twitter) from landscape banner.
  execFileSync(
    "ffmpeg",
    [
      "-y",
      "-i",
      join(ASSETS, "banner_landscape.png"),
      "-vf",
      "scale=1200:-1,pad=1200:630:(ow-iw)/2:(oh-ih)/2:#F9F5EA",
      "-q:v",
      "2",
      join(PUBLIC, "og-image.jpg"),
    ],
    { stdio: "pipe" },
  );

  console.log("sync-brand-assets: generated icons and social preview in frontend/public/");
}

function checkAssets() {
  const failures = [];

  for (const starter of STARTER_FILES) {
    if (existsSync(join(PUBLIC, starter))) {
      failures.push(`starter SVG still present in frontend/public/: ${starter}`);
    }
  }

  for (const req of [
    "favicon.ico",
    "favicon.svg",
    "safari-pinned-tab.svg",
    "site.webmanifest",
    "og-image.jpg",
  ]) {
    if (!existsSync(join(PUBLIC, req))) {
      failures.push(`missing required brand asset: frontend/public/${req}`);
    }
  }

  for (const [name, [expW, expH]] of Object.entries(EXPECTED_PNGS)) {
    const filePath = join(PUBLIC, name);
    if (!existsSync(filePath)) {
      failures.push(`missing PNG icon: frontend/public/${name}`);
      continue;
    }
    const [w, h] = readPngSize(filePath);
    if (w !== expW || h !== expH) {
      failures.push(`${name}: expected ${expW}x${expH}, got ${w}x${h}`);
    }
  }

  const ogPath = join(PUBLIC, "og-image.jpg");
  if (existsSync(ogPath)) {
    const [w, h] = readJpegSize(ogPath);
    if (w !== 1200 || h !== 630) {
      failures.push(`og-image.jpg: expected 1200x630, got ${w}x${h}`);
    }
  }

  const manifestPath = join(PUBLIC, "site.webmanifest");
  if (existsSync(manifestPath)) {
    const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
    for (const key of [
      "id",
      "name",
      "short_name",
      "lang",
      "start_url",
      "scope",
      "display",
      "background_color",
      "theme_color",
    ]) {
      if (!manifest[key]) {
        failures.push(`site.webmanifest missing '${key}'`);
      }
    }
    const icons = Array.isArray(manifest.icons) ? manifest.icons : [];
    const hasAny = icons.some((icon) => icon.purpose === "any");
    const hasMaskable = icons.some((icon) => icon.purpose === "maskable");
    if (!hasAny) {
      failures.push("site.webmanifest missing an icon with purpose 'any'");
    }
    if (!hasMaskable) {
      failures.push("site.webmanifest missing an icon with purpose 'maskable'");
    }
    for (const icon of icons) {
      if (icon.src && !existsSync(join(PUBLIC, icon.src))) {
        failures.push(`site.webmanifest references missing icon: ${icon.src}`);
      }
    }
  }

  if (failures.length > 0) {
    console.error("check:branding: failed");
    for (const failure of failures) {
      console.error(`  ${failure}`);
    }
    process.exit(1);
  }

  console.log("check:branding: ok");
}

const mode = process.argv[2] ?? "--check";
if (mode === "--write") {
  writeAssets();
  checkAssets();
} else if (mode === "--check") {
  checkAssets();
} else {
  console.error(`Unknown argument: ${mode}. Use --write or --check.`);
  process.exit(1);
}
