## Sections

`Games` manages lobbies, invite codes, drafting, balanced board assembly, and post-game wraps. `Captures` handles multipart photo-proof uploads and visibility-gated media retrieval. `Profiles` covers user profiles, Gold Medals, rank titles, and Caught/Captured photo galleries. `Decks` provides Quick-Start starter decks for drafting. `Konto` covers registration, sign-in, sessions, password recovery, and account settings.

## Authentication

A protected route takes an access token as `Authorization: Bearer <token>`. Sign-in,
registration step 2, and session renewal hand that token back in the response body
and place the refresh token in the `__Host-logandaim_refresh` cookie. Every session
response is `Cache-Control: no-store`; the refresh token never appears in a body.

## Errors

Every failed request answers the same envelope: `message` says what failed,
`attachments` carries the free-text detail attached to it, and `sources` nests the
causes beneath it. A `500` answers a constant body and never reveals its cause.
