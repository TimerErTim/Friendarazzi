---
id: ADR-0006
title: Single-step signup, async email verification, and 1-game unverified trial
status: Accepted
owner: TimerErTim
---

# ADR-0006 — Single-step signup, async verification, and 1-game trial

## Context

Previously, `RegistrierungStarten` → email link → `RegistrierungAbschließen` required every guest following an invite link to leave the browser, open their mail app, click a confirmation link, and only then choose a password before joining a lobby. Additionally, `GET /api/games/by-code/:invite_code` required authentication, leaving invited guests on an opaque login/register wall without seeing what game they were invited to.

## Decision

1. **No accountless play, single-step registration**: Keep mandatory user accounts, but collapse registration into one step (`POST /api/registrierung`: email, password, optional display name). Registration creates both the `Benutzer` and password `Konto`, immediately issues an active `Session`, and sends an async confirmation mail. Duplicate email addresses explicitly fail with `409 Conflict` (`"Diese E-Mail-Adresse ist bereits vergeben"`).
2. **Async email confirmation with 1-game limit**: Newly registered password users start with `email_verified_at = NULL`. Unverified users may create or join **at most one game** (`CreateGame` and `JoinGame` reject a second game with `403 Forbidden`), receive in-app notifications normally, and have game-event transactional emails suppressed until verified.
3. **Verification & resend endpoints**: Public `POST /api/konto/email/bestaetigen` consumes the mailed token and sets `email_verified_at`. Authenticated `POST /api/konto/email/bestaetigung-erneut` issues a throttled fresh confirmation link.
4. **Public invite preview**: `GET /api/games/by-code/:invite_code` is readable anonymously and returns `GameInvitePreview` (title, invite code, grid size, visibility mode, status, player count, deadlines) without player PII.
