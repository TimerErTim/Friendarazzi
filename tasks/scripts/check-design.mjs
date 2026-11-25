#!/usr/bin/env node
// Design-system gate.
//
// Rules, all mechanical:
//   1. every literal token in `globals.css` is named in the design rule
//   2. no raw Tailwind palette class appears in a component
//   3. no retired utility name reappears, so the palette keeps one name per colour
//   4. screen budget: app routes (excluding `/dev`) and feature components keep at most
//      1 primary Badge, 1 CalloutNote, and 1 Progress bar per file, with no
//      multi-word prose inside `CodeTag`
//
// Prose nuance (why a token exists, which role it may carry) cannot be checked here;
// that half is the "design rule updated" review checklist item.

import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("../..", import.meta.url));
const CSS = join(ROOT, "frontend/src/app/globals.css");
const RULE = join(ROOT, ".agents/skills/design-system/SKILL.md");
const SRC = join(ROOT, "frontend/src");

/** Files the rules apply to. */
const SCANNED = [".tsx", ".ts", ".css"];
const SKIP_DIRS = new Set(["node_modules", ".next", "out", "api"]);

/** Raw palette utilities, e.g. `bg-stone-100`, `text-emerald-600`. */
const PALETTE =
  /\b(?:bg|text|border|from|to|via|ring|outline|divide|placeholder|fill|stroke|accent|caret)-(?:zinc|gray|slate|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)-\d{2,3}\b/;

/**
 * Retired shadcn-era and Sleek Paperstyle names replaced by the neubrutalist system.
 * `text-primary` is deliberately absent; ink is `text-ink`.
 */
const LEGACY =
  /\b(?:bg|text|border|from|to|via|ring|outline|divide|placeholder|fill|stroke|accent|caret|decoration)-(?:foreground|background|muted-foreground|destructive|surface-elevated|placeholder|border-strong|contour|border|ring)\b|\bborder-border\b|\b(?:rule-ticks|rule-ticks-quiet|dog-ear|ruler-edge|millimetre-grid|paper-overlay|paper-rule)\b/;

const failures = [];

function walk(dir) {
  const out = [];
  for (const entry of readdirSync(dir)) {
    if (SKIP_DIRS.has(entry)) {
      continue;
    }
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) {
      out.push(...walk(full));
      continue;
    }
    if (SCANNED.some((ext) => entry.endsWith(ext))) {
      out.push(full);
    }
  }
  return out;
}

// 1. Every literal token is documented.
const css = readFileSync(CSS, "utf8");
const rule = readFileSync(RULE, "utf8");

// Tokens are literal `--name: <hex|rem>` declarations. The `@theme inline` block
// only re-exports them as `var(...)`/`calc(...)` and is skipped by that shape.
const tokens = [
  ...new Set(
    [...css.matchAll(/^\s+(--[a-z0-9-]+):\s*(?:#[0-9a-fA-F]{3,8}|[\d.]+rem)/gm)].map(
      (match) => match[1],
    ),
  ),
];

for (const token of tokens) {
  if (!rule.includes(token)) {
    failures.push(`token not documented in the design rule: ${token}`);
  }
}

const BUDGET_DIRS = [
  join(SRC, "app"),
  join(SRC, "components/friendarazzi"),
];
const PROSE_CODE_TAG =
  /<CodeTag\b[^>]*>\s*[A-Za-zÄÖÜäöüß]{3,}(?:\s+[A-Za-zÄÖÜäöüß]{2,})+\s*<\/CodeTag>/g;

function countMatches(text, pattern) {
  return [...text.matchAll(pattern)].length;
}

// 2 and 3. Components use semantic utilities only, spelled one way.
for (const file of walk(SRC)) {
  if (file === CSS) {
    continue;
  }
  const text = readFileSync(file, "utf8");
  text.split("\n").forEach((line, index) => {
    const where = `${relative(ROOT, file)}:${index + 1}`;
    if (PALETTE.test(line)) {
      failures.push(`raw palette class: ${where}: ${line.trim()}`);
    } else if (LEGACY.test(line)) {
      failures.push(`retired utility name: ${where}: ${line.trim()}`);
    }
  });
}

// 4. Screen budget on user-facing routes and Friendarazzi components.
for (const dir of BUDGET_DIRS) {
  for (const file of walk(dir)) {
    if (!file.endsWith(".tsx") || file.endsWith(".test.tsx")) {
      continue;
    }
    const rel = relative(ROOT, file);
    if (rel.startsWith("frontend/src/app/dev/")) {
      continue;
    }
    const text = readFileSync(file, "utf8");

    const primaryBadges = countMatches(
      text,
      /<Badge\b[^>]*\bvariant=["']primary["']/gs,
    );
    if (primaryBadges > 1) {
      failures.push(
        `screen budget: ${rel} has ${primaryBadges} primary Badges (max 1)`,
      );
    }

    const callouts = countMatches(text, /<CalloutNote\b/g);
    if (callouts > 1) {
      failures.push(
        `screen budget: ${rel} has ${callouts} CalloutNotes (max 1)`,
      );
    }

    const progressBars = countMatches(text, /<Progress\b/g);
    if (progressBars > 1) {
      failures.push(
        `screen budget: ${rel} has ${progressBars} Progress indicators (max 1)`,
      );
    }

    for (const match of text.matchAll(PROSE_CODE_TAG)) {
      failures.push(
        `screen budget: ${rel} uses CodeTag for prose instead of a machine value: ${match[0]}`,
      );
    }
  }
}

if (failures.length > 0) {
  console.error("check:design: failed");
  for (const failure of failures) {
    console.error(`  ${failure}`);
  }
  console.error(
    "\n  use: bg-canvas, bg-surface, bg-elevated, border-subtle, border-strong,",
  );
  console.error(
    "       text-ink, text-secondary, text-muted, text-impact, bg-primary",
  );
  process.exit(1);
}

console.log(`check:design: ok (${tokens.length} tokens documented)`);
