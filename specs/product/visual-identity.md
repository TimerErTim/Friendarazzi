---
id: SPEC-visual-identity
title: Visual identity and design system
status: Active
owner: TimerErTim
---

# Visual identity and design system

> **Friendarazzi (Neubrutalism)** — off-white paper canvas, stark ink contours, hard offset
> shadows, electric yellow primary blocks, hot pink and cyan accents, bold display typography, and
> full light/dark theme parity.
>
> Implementation rules and token tables live in
> [`.agents/skills/design-system/SKILL.md`](../../.agents/skills/design-system/SKILL.md); literal token values
> live in `frontend/src/app/globals.css` and are enforced by `mise run check:design`.

## Design Principles

1. **Semantic tokens only.** Components consume semantic utilities (`bg-canvas`, `bg-surface`,
   `bg-elevated`, `text-ink`, `bg-primary`, `bg-pink`, `bg-cyan`); raw Tailwind palette classes
   and hex codes are rejected by `mise run check:design`.
2. **Bold neubrutalist structure.** Interactive cards, buttons, badges, and unified Polaroid
   `BingoBoard` containers carry crisp `2px`–`2.5px` contours (`border-strong` / `border-control`),
   classical `2px` grid lines between square cells, unblurred hard offset shadows (`brutal-shadow`),
   and tactile press depression (`translate(2px, 2px)`).
3. **Colour is never the only signal.** Game phases, tile status, visibility modes, and form
   validation always pair colour with a label or icon. Saturated brand fills (`#FFE600`, `#FF3B9A`,
   `#00E5FF`) always use stark ink text (`#0D0D0D`) to guarantee WCAG 2.2 AA contrast. Transparent
   user avatars and overlays sit on opaque `bg-surface` backdrops (never raw `bg-primary` or
   translucent toasts).
4. **QR codes stay scannable in every theme.** Invite QR codes render dark ink modules (`#0D0D0D`)
   over an opaque light paper backing (`#FFFFFF`) with a 4-module quiet zone in both light and dark
   modes.
4. **Mobile-first ergonomics.** Interactive controls meet the 44×44 px minimum target size and
   WCAG AA contrast in both light and dark modes, with high-contrast keyboard focus outlines
   (`outline-focus`).
5. **Geometric display + work sans + mono.** `Archivo Black` for display titles and the
   multi-colour `Friendarazzi` wordmark, `Space Grotesk` for UI copy and controls, `JetBrains Mono`
   for invite codes, 1:1 timeline timestamps, and metrics.
6. **Single-source brand kit.** Icons, maskable icon, monochrome mask icon, `site.webmanifest`, and
   1200×630 Open Graph preview derive from `assets/favicon.svg`, `assets/favicon_io.zip`,
   `assets/favicon_raw.jpg`, and `assets/banner_landscape.png` via
   `tasks/scripts/sync-brand-assets.mjs`.
7. **Dev gallery.** Running `mise run dev:frontend` exposes `/dev` with all primitives, contrast
   measurements, typography, and brand specimens in light and dark modes without requiring a backend
   (excluded from static production exports).
