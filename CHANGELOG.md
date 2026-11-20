# Changelog

All notable user-visible changes to Friendarazzi are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- Quick-Start deck autofill and slot reroll now guarantee unique rendered prompts per player draft and leave remaining slots empty with guidance if a deck runs out of unused suggestions.
- Normalize uploaded capture photos and avatars by baking EXIF orientation into pixel data and stripping all EXIF metadata before storage.
- Offer separate camera capture and gallery picker controls in the tile capture dialog on mobile browsers while keeping drag-and-drop file upload on desktop.
### Added
- Neubrutalism visual identity & brand kit: complete frontend redesign with Electric Yellow, Hot Pink, and Cyan blocks, heavy ink contours, hard offset shadows, `Archivo Black` / `Space Grotesk` / `JetBrains Mono` typography, a new `/` landing page with interactive Bingo preview, recreated multi-colour `Wordmark` & `BrandMark`, single-source favicon/PWA icons (including maskable icon), and 1200×630 Open Graph / Twitter social preview.
- Unified Polaroid Bingo board (`BingoBoard`) shared identically between live matches and post-game revealed boards: classic grid lines, contained full-frame photos over blurred edge backdrops, lightbox inspection on captured tiles, and viewfinder-style capture dialog.
- Simplified game invitations via Web Share API, clipboard/selectable-link fallback, and scannable offline QR codes; passive auto-save for lobby settings and prompt drafting; instant-join & auth redirect back to `/join`; quick switching across parallel active games; header profile avatar and crisp unblurred timeline previews.
- Friendarazzi social photo-hunting Bingo MVP: lobbies, invite links, equal-distribution prompt drafting with Quick-Start decks, balanced board assembly, Undercover/LiveFeed/BlurredTeaser visibility modes, multipart photo proof capture, Gold Medal prestige & rank titles, automated post-game superlatives, 1:1 linear-scaled timeline, and game-grouped Caught/Captured profile galleries.
- Account screens for registration, sign-in, and password recovery.

[Unreleased]: https://github.com/TimerErTim/Friendarazzi/commits/main
