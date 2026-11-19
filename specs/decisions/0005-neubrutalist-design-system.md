---
id: ADR-0005
title: Neubrutalist visual identity and brand kit superseding Sleek Paperstyle
status: Proposed
date: 2026-11-19
deciders: TimerErTim
---

# ADR-0005: Neubrutalist visual identity and brand kit

## Context

`ADR-0003` established a semantic token layer in `frontend/src/app/globals.css`, a dev-only
component gallery at `/dev`, and the `mise run check:design` gate, initially styled in the
*Sleek Paperstyle* aesthetic (warm ivory paper, hairline rules, `Lora` serif headings, one hunter
green accent, no shadows).

Friendarazzi is a fast, loud, social photo-hunting Bingo game for friend groups. Its visual assets
(`assets/banner_landscape.*`, `assets/banner_panorama.*`, `assets/favicon.*`) establish a
high-energy **Neubrutalism** identity: electric yellow, hot pink, and cyan colour blocks over warm
off-white paper, heavy ink contours (`2.5px`–`3px`), crisp unblurred hard offset shadows, polaroid
frames, and bold geometric grotesque typography.

## Decision

1. **Supersede the Sleek Paperstyle aesthetic; keep the architecture.** `ADR-0003`'s semantic
   token architecture (`globals.css` → `@theme inline` → component utilities), `/dev` gallery
   under `next dev`, and `check:design` mechanical gate remain intact. All Sleek Paperstyle
   visual rules, paper-furniture abstractions (`paper-rule.tsx`, `rule-ticks`, `dog-ear`,
   `ruler-edge`, `millimetre-grid`), and serif typography (`Lora`) are retired and replaced by
   the Neubrutalism design system.
2. **Neubrutalist palette (Light & Dark).**
   - Light (*Papier / Sticker*): Off-white canvas (`#FAF8F2`), pure white cards (`#FFFFFF`), stark
     ink (`#0D0D0D`), Primary Electric Yellow (`#FFE600`), Accent Hot Pink (`#FF3B9A`), Secondary
     Cyan (`#00E5FF`), and accessible semantic status colours.
   - Dark (*Nacht / Blitzlicht*): Warm charcoal canvas (`#1C1B18`), elevated surfaces (`#262420`,
     `#2F2C27`), off-white ink & borders (`#FAF8F2`), deep black hard offset shadows (`#050505`),
     and Electric Yellow focus indicators (`#FFE600`).
3. **Typography.** Retire `Lora` and `Inter`. Pair `Archivo Black` (`--font-display`) for display
   headings and the multi-colour wordmark with `Space Grotesk` (`--font-sans`) for UI copy and
   controls, and `JetBrains Mono` (`--font-mono`) for invite codes, timestamps on the 1:1 game
   timeline, and tabular metrics.
4. **Contours, shadows, and tactile motion.** Heavy `2.5px` (`border-[2.5px]`) and `2px`
   (`border-2`) contours with `--border-control` / `--border-strong`, paired with unblurred hard
   offset shadows (`4px 4px 0 var(--shadow-ink)`) and active press depression
   (`translate(2px, 2px)`).
5. **Accessibility without compromise.** Every text/background pair and control boundary in both
   themes clears WCAG 2.2 AA (`>= 4.5:1` for text, `>= 3:1` for control contours), verified live
   in `/dev`. Saturated brand blocks (`#FFE600`, `#FF3B9A`, `#00E5FF`) always carry stark ink
   (`#0D0D0D`) text rather than white or coloured text. Focus outlines use `--color-focus`
   (`#0D0D0D` in light, `#FFE600` in dark). Interactive targets keep `>= 44×44 px`.
6. **Single-source brand kit.** `assets/favicon.svg` and `assets/favicon_io.zip` plus
   `assets/banner_landscape.png` drive `frontend/public/` icons, maskable icon, monochrome mask
   icon, `site.webmanifest`, and 1200×630 Open Graph / Twitter preview via
   `tasks/scripts/sync-brand-assets.mjs`. In-app branding renders the `Wordmark` component using
   semantic tokens and display typography.

## Consequences

- `frontend/src/components/ui/` and all app routes are redesigned around neubrutalist primitives
  while preserving all API hooks, TanStack Query caches, auth/game state machines, and variant names.
- `check:design` and `check:branding` enforce token documentation, absence of raw palette classes
  or retired utilities, and icon dimension/manifest integrity in CI.
