---
id: SPEC-charter
title: Friendarazzi product charter
status: PO-approved
owner: TimerErTim
issues: []
superseded_by: null
---

# Friendarazzi — Product Charter

## Vision

**Friendarazzi** is a multiplayer social photo-hunting Bingo web application where groups of friends
turn hangouts, trips, parties, or everyday weeks into a candid photo-hunting game.

## Core Gameplay Loop

1. **Lobby & Invitation:** Host creates a game lobby with grid size (`3×3`, `4×4`, `5×5`, optional
   center Free Space) and proof visibility mode (`Undercover`, `LiveFeed`, `BlurredTeaser`) and
   shares an invite link (`/join/[invite_code]`).
2. **Drafting Phase:** Participants submit custom prompts evenly distributed across the other players
   in the lobby, or pick and customize prompts from curated Quick-Start decks (*Night Out*, *Urlaub &
   Festival*, *WG & Zuhause*, *Büro & Uni*).
3. **Balanced Board Generation:** On locking prompts, each participant receives an individualized
   Bingo card built with balanced matching that strictly excludes self-authored and self-targeted
   tiles.
4. **Active Hunt & Photo Proof:** Players upload photo proofs for targets in the mobile browser.
   Photos are resized/compressed and stored via `ObjectStore`, with thumbnails generated for
   `BlurredTeaser` mode.
5. **Victory, Superlatives & Memories:** Completing a row, column, or diagonal ends the round, crowns
   the winner with a Gold Medal, unlocks all boards and hidden photos, computes post-game
   superlatives (*Most Wanted*, *Ghost*, *Quickest Draw*, *Sniper*), and renders a 1:1 proportional
   match timeline alongside game-grouped *Caught* / *Captured* user galleries.

## Domain Terms

| Term | Meaning |
|------|---------|
| `Benutzer` / `Konto` | Authenticated person and credentials (`konto` supporting subdomain). |
| `UserProfile` | Public identity, avatar, `gold_medal_count`, unlocked rank title (`RANK_TIERS`). |
| `Game` | Bingo match with `InviteCode`, `GridSize`, `VisibilityMode`, and lifecycle state (`Lobby` → `Drafting` → `Active` → `Completed`). |
| `PromptSubmission` | Custom or starter-deck action written by `author_user_id` about `target_user_id`. |
| `Board` / `BoardTile` | Player's individualized grid and tile assignments (`author != owner && target != owner`). |
| `Capture` | Uploaded photo proof (`image_storage_key`, `thumbnail_storage_key`, timestamp). |
| `StarterDeck` | Curated Quick-Start deck templates seeded in SQLite migrations. |
