# Changelog

All notable user-visible changes to Friendarazzi are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Target-grouped board view (`Nach Ziel`) alongside the classic Bingo grid in both active games and the post-game recap (including an aggregated per-target match overview), plus target pinning (up to 3 focus friends), a `Nur offene` filter, instant prompt/target search, an in-game `?` rules & visibility guide, and a dismissible first-capture tip.
- Persistent in-app notification inbox (`/benachrichtigungen`) with unread badge counter, mark-read / mark-all-read controls, and deep links directly to started/completed games (`/games?id=...`) or directly into the photo inspection lightbox for new captures (`/games?id=...&capture=...`).
- Styled Neubrutalist HTML + plain-text emails with `List-Unsubscribe` headers and one-click logged-out unsubscribe (`/abbestellen`) for game starts (draft phase finished), new captures (mentioning catcher, target, and prompt without embedding images, respecting Undercover/BlurredTeaser visibility rules), and game wins (naming game and winner in the subject).
- Per-game notification watch toggle (GitHub Watch style) in the game header plus account-wide notification preferences in `/optionen` for default game-watch state on join/create and per-event email opt-out.
- Reversible prompt locking with a shared 60-second pre-game countdown once all players mark ready, allowing any player to withdraw readiness and adjust prompts before boards are dealt.
- Parallel-game session overview with progress indicators, action-needed highlights, explicit multi-game selection, and switch-safe autosave (flushing pending draft and lobby edits before switching or closing).
- Neubrutalism visual identity & brand kit: complete frontend redesign with Electric Yellow, Hot Pink, and Cyan blocks, heavy ink contours, hard offset shadows, `Archivo Black` / `Space Grotesk` / `JetBrains Mono` typography, a new `/` landing page with interactive Bingo preview, recreated multi-colour `Wordmark` & `BrandMark`, single-source favicon/PWA icons (including maskable icon), and 1200×630 Open Graph / Twitter social preview.
- Unified Polaroid Bingo board (`BingoBoard`) shared identically between live matches and post-game revealed boards: classic grid lines, contained full-frame photos over blurred edge backdrops, lightbox inspection on captured tiles, and viewfinder-style capture dialog.
- Simplified game invitations via Web Share API, clipboard/selectable-link fallback, and scannable offline QR codes; passive auto-save for lobby settings and prompt drafting; instant-join & auth redirect back to `/join`; quick switching across parallel active games; header profile avatar and crisp unblurred timeline previews.
- Friendarazzi social photo-hunting Bingo MVP: lobbies, invite links, equal-distribution prompt drafting with Quick-Start decks, balanced board assembly, Undercover/LiveFeed/BlurredTeaser visibility modes, multipart photo proof capture, Gold Medal prestige & rank titles, automated post-game superlatives, 1:1 linear-scaled timeline, and game-grouped Caught/Captured profile galleries.
- Account screens for registration, sign-in, and password recovery.

### Changed
- Grid size pickers now frame `3×3` as the recommended default with session-length hints (`1–2 Std.`, `Abend`, `Wochenende`), game notification toggles are labelled consistently (`Benachrichtigungen` / `Stumm`), all remaining UI copy uses informal German (`Du`), and background polling pauses while browser tabs are hidden.
- Prompt submissions and Quick-Start starter decks are now formulated target-agnostically (removing `{name}` prefixes and updating drafting placeholders), while associating prompts with their target player cleanly through the UI across drafting cards, capture feeds, and gallery cards.
- The live capture feed now shows the newest captures first and includes the capture date alongside the time.
- Streamlined the entire frontend around single-purpose surfaces and a strict screen budget: uncaptured Bingo tiles drop empty image placeholders so prompts stay readable, captured tiles display full-bleed subtly tilted instant photos with a compact caption strip, the match header collapses into one compact row plus a thin phase track, active play puts the board first with a crew popover and collapsible activity feed, prompt drafting opens directly with an inline deck/crew bar, and the hub, lobby, join, and profile views remove redundant badges and duplicate controls.

### Fixed
- Keep vertical mobile camera captures (`capture="environment"`) upright instead of storing rotated landscape images by preserving raw native uploads for server-side EXIF/XMP/multi-APP1 orientation normalization and only transcoding non-native browser formats like HEIC/AVIF before upload.
- Quick-Start deck autofill and slot reroll now guarantee unique rendered prompts per player draft and leave remaining slots empty with guidance if a deck runs out of unused suggestions.
- Normalize uploaded capture photos and avatars by baking EXIF orientation into pixel data and stripping all EXIF metadata before storage.
- Offer separate camera capture and gallery picker controls in the tile capture dialog on mobile browsers while keeping drag-and-drop file upload on desktop.

[Unreleased]: https://github.com/TimerErTim/Friendarazzi/commits/main
