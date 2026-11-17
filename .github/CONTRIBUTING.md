# Contributing to Friendarazzi

Thanks for wanting to improve **Friendarazzi**! Here is everything you need to get started locally and submit a clean pull request.

## Prerequisites

The toolchain (Rust, Node.js, `cargo-nextest`, `mailpit`, `pitchfork`) is managed via [`mise`](https://mise.jdx.dev/):

```bash
mise install
```

## Local development

Start the backend (with local Mailpit SMTP catcher) and the Next.js frontend in separate terminals:

```bash
mise run dev:server     # API on http://localhost:3000, Mailpit UI on http://127.0.0.1:8034
mise run dev:frontend   # Frontend on http://localhost:3001
```

## Checks & Tests

Before opening a pull request, run the full check and test suites:

```bash
mise run check   # Formatting, Rust clippy/docs, Biome, TypeScript, OpenAPI sync, design tokens, version match
mise run test    # Unit + SQLite conformance + HTTP E2E tests (starts Mailpit automatically)
mise run build   # Backend binary + Next.js static export
mise run verify  # Starts built backend & frontend and runs smoke tests
```

## Architecture & Pull Requests

- Target `dev` for feature and bugfix pull requests (`main` receives releases from `dev`).
- Keep domain logic and invariants inside `crates/friendarazzi-core` with matching tests in `crates/friendarazzi-core/tests/cases/` (which execute against both in-memory fakes and SQLite) and HTTP assertions in `crates/friendarazzi-api/tests/e2e/`.
- If an API route or schema DTO changes, sync the TypeScript client types via `mise run sync:openapi`.
- For user-facing changes, add a brief note under `## [Unreleased]` in [`CHANGELOG.md`](../CHANGELOG.md).
