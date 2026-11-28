---
id: ADR-0007
title: OIDC external identity providers (Google, Apple, and generic OIDC)
status: Accepted
owner: TimerErTim
---

# ADR-0007 — OIDC external identity providers

## Context

Players asked for fast one-click sign-in via Google, Apple, or self-hosted OpenID Connect providers (`#41`, `#43`) without losing account isolation or exposing client secrets in the static Next.js frontend.

## Decision

1. **Separate `external_identity`, `oidc_auth_state`, and `oidc_exchange_code` tables (Migration v9)**:
   - Because `benutzer` and `konto` are already separate tables (with `konto` holding optional password credentials), passwordless OIDC users simply have zero rows in `konto` and one or more rows in `external_identity`.
2. **Hexagonal port `OidcProviderClient` + adapter crate `crates/friendarazzi-oidc`**:
   - `friendarazzi-core` defines `OidcProviderClient`, keeping HTTP, JWKS caching, and RS256/ES256 ID-token signature verification out of core so `friendarazzi-core` stays free of network and JOSE transport crates.
   - `friendarazzi-oidc` implements Discovery + JWKS caching + RS256/ES256 verification for Google (`query`), Apple (`form_post` + ES256 client-secret JWT generation), and generic configurable OIDC providers.
3. **Zero tokens in browser redirect URLs**:
   - Provider callbacks (`GET` and Apple `POST /api/auth/oidc/{provider}/callback`) spend the single-use `OidcAuthState`, verify the provider code/ID token outside the SQLite write lock, and redirect (`302 Found`) to the frontend with only a 120-second single-use opaque `code` (or a structured error query param).
   - The frontend exchanges the `code` via `POST /api/auth/oidc/exchange` for a normal `Session` + `HttpOnly` refresh cookie (`ADR-0004`).
4. **No silent account linking by email**:
   - If an unknown `(provider_id, subject)` resolves to an email already owned by an existing `Benutzer`, login redirects with `error=email_conflict` instructing the user to sign in with their existing method and link the provider explicitly under `/optionen`.
5. **Explicit public vs authenticated start endpoints**:
   - Unauthenticated login starts via `GET /api/auth/oidc/{provider}/start`.
   - Authenticated linking and deletion re-authentication start via `POST /api/konto/anmeldemethoden/{provider}/start` (`bearerAuth`), keeping every published route's security unambiguous.
   - Unlinking a provider refuses `409 Conflict` when removing the user's last remaining login method.
   - Passwordless OIDC accounts can set an initial password once (`POST /api/konto/passwort`) or confirm `Benutzer-Löschung` with a short-lived `DeleteReauth` exchange code.
