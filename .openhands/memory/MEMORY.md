# Project memory — Friendarazzi

## GitHub
- Repo `TimerErTim/Friendarazzi`, default branch `dev`. The issue tracker was empty before 2026-11-18; issues #2–#6 (notifications, deployment, branding/favicon, neubrutalism redesign, encrypted media) are the initial backlog.
- Native labels that matter: `type: feature|bug|chore|spike`, `area: backend|frontend|infra|branding|specs`, `P0`–`P3`, `needs-PO`, `ready`, `agent-ok`, `blocked`.
- `mise x -- gh ...` only works **inside the repo root**: run from `/tmp` it fails with "No version is set for shim: gh". `gh issue list --template` requires `--json`; `gh api -f ...` on `GET` sends a body and 422s — use the `?query` URL form.

## Facts worth not rediscovering
- Runbook `docs/deployment.md` is long and authoritative: `sha-<commit>` image tags, `.deploy/known-good-*`, mandatory mail verdict, docker-mailserver + Caddy shared cert.
- Deploy rollback restores image + checkout only; migrations run at startup and are forward-only — the main reliability risk (issue #3).
- Media is plaintext on disk (`FileSystemObjectStore`, `data/media`); authorization lives in `media_get` (issue #6).
- Visual identity is "Sleek Paperstyle" and is enforced by `mise run check:design`; a neubrutalism redesign (issue #5) must rewrite `specs/product/visual-identity.md` + `.agents/skills/design-system/SKILL.md` and add an ADR.
- `assets/logandaim-*.svg` is inherited branding from the previous product name, unused by the frontend.
- `specs/product/betrieb.md` and `datenschutz.md` still use archery wording (`Runden`, `Schüsse`, `Anlage`) from the inherited domain, unlike the Friendarazzi terms in `charter.md`.
