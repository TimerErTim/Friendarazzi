---
name: design-system
description: "Friendarazzi visual identity (Neubrutalism) — semantic design tokens, brand kit & wordmark, shadcn-style UI primitives, WCAG contrast and touch-target rules, screen composition, and the /dev gallery. Read before creating or modifying any frontend UI, CSS tokens, or components."
---

# Design system — Friendarazzi Neubrutalism

The operative rulebook. Follow it literally; it is the only document an implementer needs to build
UI. Values live in [`frontend/src/app/globals.css`](../../../frontend/src/app/globals.css) — this file
names tokens and states rules, never hex codes. Rationale and history live in
[`specs/product/visual-identity.md`](../../../specs/product/visual-identity.md) and
[`specs/decisions/0005-neubrutalist-design-system.md`](../../../specs/decisions/0005-neubrutalist-design-system.md).

**Scope: appearance only.** How a screen looks, feels, and behaves visually — never what the product
does, which journey it supports, or how data flows. Content decisions live in product specs.

## The identity

Bold, tactile, candid photo-hunt energy on off-white paper: heavy ink contours, crisp unblurred
offset shadows, flat saturated Electric Yellow, Hot Pink, and Cyan sticker blocks, Polaroid-style
photo frames, and geometric grotesque display typography.

| ✅ DO | ❌ DON'T |
|-------|----------|
| Warm off-white canvas (`--bg-canvas`) & crisp cards (`--bg-surface`). | Cold tech white or washed-out blue-grey Slate neutrals. |
| `--color-primary` (Electric Yellow), `--color-pink` (Hot Pink), `--color-cyan` (Cyan) paired with stark ink text. | White or yellow text on light paper, blurred drop shadows, or CSS gradients. |
| Heavy `2px`–`2.5px` contours (`--border-control` / `--border-strong`) with hard offset shadows (`brutal-shadow`). | Hairline-only flat borders, glassmorphism (`backdrop-blur`), or soft `blur()` elevation. |
| Tactile press depression (`brutal-press`: `translate(2px, 2px)`). | Bouncy spring animations or hover elevation without shadow crispness. |
| `Archivo Black` display headings + `Space Grotesk` body + `JetBrains Mono` codes/timestamps. | Serif (`Lora`), cursive/script, or soft editorial book styling. |
| `--semantic-*` as meaning: colour appears with a label or icon. | Decorative red/green/amber without status text or icon. |

This table wins over any rule below that disagrees with it.

## Tokens

Every token is defined for light and dark. Add a token only with an entry here.

**One name per colour.** Utility suffix = token suffix: strip `--color-` or `--bg-` and you have the
class. The one collision: "primary" belongs to the brand, so ink is **`text-ink`**.

| Token | Utilities | Use for |
|----|----|----|
| `--bg-canvas` | `bg-canvas` | Page background / desk |
| `--bg-surface` | `bg-surface` | Cards, panels, top bar, sidebar, polaroid frames |
| `--bg-elevated` | `bg-elevated` | Inputs, selects, dropdowns, dialogs |
| `--bg-surface-muted` | `bg-surface-muted` | Table heads, secondary wells, skeletons, hover fill |
| `--border-subtle` | `border-subtle`, `bg-subtle`, `divide-subtle` | Internal table row dividers and subtle separators |
| `--border-strong` | `border-strong` | Structural `2px`–`2.5px` neubrutalist contours on cards, badges, and panels |
| `--border-control` | `border-control` | **Every interactive control's boundary** (≥ 3:1, WCAG 1.4.11) |
| `--text-primary` | `text-ink`, `bg-ink` | Body, headings, default icons, text on brand colour blocks |
| `--text-secondary` | `text-secondary` | Labels, subtitles, metadata, units |
| `--text-muted` | `text-muted` | Placeholders and disabled controls only |
| `--color-primary` | `bg-primary`, `text-primary`, `border-primary`, `ring-primary` | Electric Yellow: primary action fill, active nav highlight, brand blocks |
| `--color-primary-hover` | `bg-primary-hover` | Pressed/hovered primary button |
| `--color-primary-subtle` | `bg-primary-subtle` | Soft yellow tint wash |
| `--primary-foreground` | `text-primary-foreground` | Stark ink label on `--color-primary` |
| `--color-pink` | `bg-pink`, `text-pink`, `border-pink` | Hot Pink: playful brand highlights, secondary stickers, wordmark letters |
| `--color-pink-subtle` | `bg-pink-subtle` | Soft pink tint wash |
| `--pink-foreground` | `text-pink-foreground` | Stark ink label on `--color-pink` |
| `--color-cyan` | `bg-cyan`, `text-cyan`, `border-cyan` | Electric Cyan: Bingo callouts, info stickers, wordmark letters |
| `--color-cyan-subtle` | `bg-cyan-subtle` | Soft cyan tint wash |
| `--cyan-foreground` | `text-cyan-foreground` | Stark ink label on `--color-cyan` |
| `--color-focus` | `outline-focus`, `ring-focus` | High-contrast keyboard focus ring (`--text-primary` in light, `--color-primary` in dark) |
| `--shadow-ink` | `brutal-shadow`, `brutal-shadow-sm`, `brutal-shadow-lg` | Solid unblurred offset shadow colour (`4px 4px 0 var(--shadow-ink)`) |
| `--semantic-impact` | `text-impact`, `bg-impact`, `border-impact`, `outline-impact`, `border-l-impact` | Destructive actions, errors, critical failures |
| `--semantic-warning` | `text-warning`, `bg-warning`, `border-warning` | Warnings, pending player counts, medal/trophy accents |
| `--semantic-success` | `text-success`, `bg-success`, `border-success` | Confirmations, captured Bingo tiles, locked readiness |
| `--semantic-info` | `text-info`, `bg-info`, `border-info` | System/helper status notes |

| Mechanics | Use for |
|----|----|
| `--font-display` (`Archivo Black`) | Display headings (`h1`, `h2`, `CardTitle`, `DialogTitle`), hero banners, `Wordmark` |
| `--font-sans` (`Space Grotesk`) | Interface copy: menus, tables, forms, buttons, descriptions |
| `--font-mono` (`JetBrains Mono`) | Machine values: invite codes, 1:1 timeline timestamps, tabular counters, plate indices |
| `--radius` | `0.375rem` (6px). Controls `rounded-sm` (4px), containers `rounded-md` (6px) |

Dark mode is class-based (`@custom-variant dark`), defaulting to `prefers-color-scheme`; the gallery
sets `.light` or `.dark` on a wrapper to show both side by side.

## Contrast

Every text and control pair meets WCAG 2.2 AA (`4.5:1` text, `3:1` controls) in both themes,
measured live in `/dev`.

- Saturated brand fills (`--color-primary`, `--color-pink`, `--color-cyan`) **never** carry white
  or coloured text; they always pair with `--primary-foreground` / `--pink-foreground` /
  `--cyan-foreground` (stark ink).
- `--color-primary` (Electric Yellow) is **never** used as text on `--bg-canvas` or `--bg-surface`;
  use `text-ink` for headings and labels, reserving `--color-primary` for fills, stickers, and
  dark-mode focus rings.
- `--text-muted` is restricted to placeholders and disabled controls.
- Focus outlines always use `outline-focus` (`focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus`).

## Brand Kit & Wordmark

Single-source brand assets live in `assets/` and are synced/verified into `frontend/public/` via
`node tasks/scripts/sync-brand-assets.mjs`:

- **Mark (`assets/favicon.svg` → `frontend/public/favicon.svg`)**: The 3×3 neubrutalist Bingo grid
  with geometric patterns and central camera lens on Electric Yellow.
  - PNG/ICO exports: `favicon.ico`, `favicon-16x16.png`, `favicon-32x32.png`,
    `apple-touch-icon.png` (180×180 opaque), `android-chrome-192x192.png`,
    `android-chrome-512x512.png`, `android-chrome-maskable-512x512.png` (512×512 with 80% safe-zone
    padding on opaque yellow).
  - Monochrome mask icon: `frontend/public/safari-pinned-tab.svg`.
  - Social preview (1200×630): `frontend/public/og-image.jpg` derived from `assets/banner_landscape.png`.
  - README banner: `assets/banner_panorama.webp`.
- **In-app `Wordmark` (`frontend/src/components/brand/wordmark.tsx`)**:
  - Renders `"Friendarazzi"` in `font-display` (`Archivo Black`) with the banner's signature
    alternating letter colour pattern (`bg-primary`, `bg-cyan`, `bg-pink` fills), dark ink stroke
    contour, and hard neubrutalist offset text-shadow — built solely from semantic CSS variables.
  - Clear space: at least `0.5em` around the mark/wordmark; minimum `18px` font size for full
    multi-colour wordmark, or pair with the `BrandMark` icon (`>= 24×24px`) in compact spaces.
  - Never recolour individual letters outside the three brand tokens (`--color-primary`,
    `--color-cyan`, `--color-pink`).

## Neubrutalist Vocabulary

1. **The Sticker Card (`Card`, `brutal-shadow`).** Crisp `border-[2.5px] border-strong`,
   `bg-surface`, and `4px 4px 0 var(--shadow-ink)`.
2. **Tactile Press (`brutal-press`).** Buttons and interactive tiles carry `2px`–`2.5px`
   `border-control` with `brutal-shadow-sm` and depress (`translate(2px, 2px)`) on `:active`.
3. **Graph Paper & Checkerboard (`brutal-grid`, `brutal-checker`, `brutal-hatch`).** Subtle
   structural grid backgrounds for the hero, 1:1 timeline, and section dividers — using only
   `--border-subtle` / `--border-strong`.
4. **Sticker Badges & Callouts (`Badge`, `Alert`).** Solid high-contrast stamps (`border-2
   border-strong`) with tracked uppercase `Space Grotesk` or `JetBrains Mono` labels and icons.
5. **Polaroid Photo Frames.** Captured photos and gallery items sit inside thick white/surface
   frames with `border-[2.5px] border-strong`, hard offset shadows, and bold caption strips.

## Rules

1. **Semantic utilities only.** `bg-canvas`, `bg-surface`, `bg-elevated`, `bg-primary`,
   `bg-pink`, `bg-cyan`, `text-ink`, `text-secondary`, `border-strong`, `border-control`,
   `outline-focus`. Never a raw Tailwind palette class (`bg-yellow-400`, `text-pink-500`) or hex
   value outside `globals.css`. Retired paper/shadcn names (`text-foreground`, `border-border`,
   `paper-rule`, `rule-ticks`, `dog-ear`, `ruler-edge`, `millimetre-grid`, `leader`) must not
   reappear.
2. **Contrast is a gate.** Every pair in `TokenSwatches` must pass AA in both `.light` and `.dark`.
3. **Colour is never the only signal.** Status and validation always carry text or an icon.
4. **Touch targets ≥ 44×44 px** on every interactive element (`min-h-11` or `::before` hit-area
   overlay with `inset-x-0`). Replaced elements (`input`, `select`, `textarea`) are drawn at
   `h-11` (`44px`).
5. **Focus always visible.** `focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus`.
6. **One primary action per screen.** Everything else `secondary` or `ghost`. Destructive uses
   `destructive`.
7. **State facts once and group related controls.** Avoid repeating the same phase, status, or metric across adjacent badges and headers, and keep all edits for one concept (such as profile avatar, display name, and title) in one surface rather than splitting them across banners and tabs.
8. **Per-screen budget & single-purpose surfaces** (enforced mechanically by `check:design` on app routes outside `/dev` and on `components/friendarazzi`):
   - At most **1** `Badge variant="primary"`, **1** `CalloutNote`, and **1** `Progress` per screen/component file.
   - `CodeTag` holds machine values only (invite codes, counts, timestamps, step indices), never prose.
   - Secondary lists during active work (crew rosters, inactive switchers, long activity streams) live inside a compact `Popover`, `DropdownMenu`, or progressive toggle so the primary task (board, prompt field, upload preview) starts above the fold on a 360×640 phone.
   - Components serve their single immediate purpose: no empty image placeholders inside uncaptured prompt tiles, and no redundant form chrome alongside an active photo preview.
9. **German UI copy, English code.** Labels German, identifiers and comments English.
10. **Spacing** uses Tailwind's scale (`4, 8, 12, 16, 24, 32`). Cards and page bodies `p-6`.
11. **Variants come from `cva`.** Keep public variant names stable (`primary`, `secondary`, `ghost`,
    `destructive`, `default`, `success`, `warning`, `error`, `info`, `outline`).
12. **URLs respect `NEXT_PUBLIC_FRIENDARAZZI_BASE_PATH`.** All static metadata/icon/manifest paths
    and shareable `inviteUrl(code)` links respect `basePath` (see `frontend/src/lib/invite-url.ts`).
13. **Bingo boards & overlays.** `BingoBoard` renders as one unified card with hairline grid
    lines. Uncaptured tiles focus cleanly on target chip + prompt text (no image placeholder).
    Captured tiles fill the cell with a subtly tilted full-bleed instant-photo frame
    (`contain-blurred` over blurred edge backdrop) and a compact opaque `bg-surface` bottom caption
    strip. Floating toasts, popovers, and avatar circles sit on opaque `bg-surface` / `bg-elevated`
    backings so transparent images remain high-contrast. Invite QR codes render dark modules on an
    opaque `#ffffff` square with a 4-module quiet zone in both themes.

Review checklist (beyond mechanical checks in `check:design`):
- Exactly one rendered `<h1>` and one primary `Button` per screen; secondary card links use `variant="secondary"`.
- On mobile (360×640), the active board top edge or first prompt `<Input />` appears in the first viewport without scrolling past decorative cards.
- Every state or count is printed once in its region.

## The gallery at `/dev`

`frontend/src/app/dev/page.dev.tsx` exists only under `next dev` (`next.config.ts` phase check).
It renders every token, live WCAG contrast ratios, typography (`Archivo Black`, `Space Grotesk`,
`JetBrains Mono`), the `Wordmark` & `BrandMark`, and all UI primitives side by side in `.light` and
`.dark`, plus the full composed page plate and a 44 px touch-target overlay toggle.

## Keeping in sync

- Every literal token in `globals.css` must be documented in this file (`check:design` fails
  otherwise).
- Update `specs/product/visual-identity.md` and this skill in the same commit as any token or
  component rule change.
