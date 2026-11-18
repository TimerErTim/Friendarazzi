---
id: SPEC-visual-identity
title: Visual identity and design system
status: Active
owner: TimerErTim
---

# Visual identity and design system

> **Friendarazzi (Sleek Paperstyle)** — warm ivory paper aesthetic, forest green primary accent,
> warm ink, hairline pencil rules, light and dark themes.
>
> Implementation rules and token tables live in
> [`.agents/skills/design-system/SKILL.md`](../../.agents/skills/design-system/SKILL.md); literal token values
> live in `frontend/src/app/globals.css` and are enforced by `mise run check:design`.

## Design Principles

1. **Semantic tokens only.** Components consume semantic utilities (`bg-canvas`, `bg-surface`,
   `bg-elevated`, `text-ink`, `text-primary`); raw Tailwind palette classes and hex codes are
   rejected by `mise run check:design`.
2. **Colour is never the only signal.** Game phases, tile status, visibility modes, and form
   validation always pair colour with a label or icon.
3. **Mobile-first ergonomics.** Interactive controls meet the 44×44 px minimum target size and
   WCAG AA contrast in both light and dark modes.
4. **Two typefaces + mono.** `Lora` for headings, `Inter` for UI copy and controls, `Geist Mono`
   for invite codes, timestamps, and metrics.
5. **Dev gallery.** Running `mise run dev:frontend` exposes `/dev` with all primitives in light and
   dark modes without requiring a backend (excluded from static production exports).
