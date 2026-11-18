# Friendarazzi — Repository Context

Friendarazzi is a multiplayer social photo-hunting Bingo web application: a Rust backend
(edition **2024**, **async** throughout) and a Next.js App Router frontend that is exported
statically. The backend follows **Domain-Driven Design** and **hexagonal architecture**.
Crates are prefixed `friendarazzi-`.

## Where the truth lives

Read the one that owns the question; do not restate it here.

| Question | Document |
|----------|----------|
| How work flows and how to contribute | [`.github/CONTRIBUTING.md`](.github/CONTRIBUTING.md) |
| Product charter and decisions | [`specs/`](specs/README.md) |
| In which order to build one feature slice | [`docs/vertical-slice.md`](docs/vertical-slice.md) |
| How to model domain types | [`crates/friendarazzi-core/src/domain/README.md`](crates/friendarazzi-core/src/domain/README.md) — module docs |
| How the UI looks and behaves | [`.agents/skills/design-system/SKILL.md`](.agents/skills/design-system/SKILL.md) |
| Server configuration and environment variables | [`backend/README.md`](backend/README.md) |
| The code map (symbols, dependencies) | `graphify` (see [`.agents/skills/graphify/SKILL.md`](.agents/skills/graphify/SKILL.md)) |

## Workspace layout

```
crates/
├── friendarazzi-core/         # Domain, use cases, port definitions
├── friendarazzi-api/          # Public API surface: DTOs, FriendarazziAppApi, OpenAPI
├── friendarazzi-sqlite/       # SQLite/libsql repository adapter
├── friendarazzi-fakes/        # In-memory adapters, FixedClock
├── friendarazzi-filesystemstore/  # Object-store adapter
└── molting/                # Generic async migration framework
backend/server/             # Standalone HTTP server (friendarazzi-server)
frontend/src/               # Next.js UI (App Router, TanStack Query)
tasks/                      # mise task definitions
specs/                      # Product charter & ADRs
mise.toml                   # Tool, env, and task orchestration
```

## Layers

```
backend/server (Poem OpenAPI)          driving adapter
        ↓
friendarazzi-api — FriendarazziAppApi, schema DTOs, HTTP status mapping
        ↓
friendarazzi-core — App, use cases, domain, ports
        ↓
friendarazzi-sqlite — SQLite UoW, repositories, migrations
```

Dependency rule: **inward only**. Core knows nothing about HTTP or SQLite.

## Language & naming

Docs, comments, commit messages, and Friendarazzi game/board/capture/profile types are **English**
(`Game`, `Board`, `BoardTile`, `Capture`, `StarterDeck`, `UserProfile`). The inherited auth
subdomain (`konto`) keeps its established German domain nouns (`Benutzer`, `Konto`,
`PasswortWiederherstellung`, `Bestätigung`) paired with English `Session` / token nouns and English
repository methods (`find_by_id`, `find_all`, `create`, `update`, `delete`). Use UTF-8 consistently
for existing umlaut identifiers in `konto`.

## Patterns an agent can silently break

- **`UseCase`** — one struct per write operation; `async fn perform(self, ctx)` returns
  `ResultReport<Output, Context>`. Use cases enter and finish the UoW themselves. Reads are not
  use cases: they run inline as `execute_fn` closures in `FriendarazziAppApi`.
- **UoW** — `ctx.enter()` for an owned UoW, `ctx.sub(&uow)` for nested work (nested `enter` joins
  the outer TX; nested `commit`/`rollback` are no-ops). Collect work without `?` across the UoW,
  then `uow.finish(result, ctx)`. Dropping a UoW that neither committed nor rolled back panics.
- **Repositories are dumb** — they persist and load domain types; state transitions belong in the
  domain or the use case. Every repository has the same five shapes: `find_by_id`, `find_all` (or a
  scoped `find_by_*`), `create(Neues*)`, `update(&mut Versioned<T>) -> RepositoryResult<()>`,
  `delete(Versioned<T>) -> RepositoryResult<()>`. `update` writes the whole row back and bumps the
  version **in place**; `delete` takes the aggregate as it was loaded, so a stale version is a
  `VersionMismatch` in both directions. The load → domain mutation → write-back loop is the
  caller's, and a purpose-built mutator (`revoke_all`, `set_role`) is a sign the logic belongs in a
  use-case loop instead.
- **Errors** — `Report<E>` at every boundary; use `.change_context()` directly. Value-object
  validation is the one exception: plain `Result` with a dedicated `thiserror` enum.
- **Concurrency** — `Versioned<T>` bundles an entity with its optimistic-lock version.
- **All mutations go through `App::execute` / `App::execute_fn`.** No adapter calls a repository
  directly. `App` only builds an `ExecutionContext`; it does not wrap the use-case result.

## Comments

A comment earns its place by saying something the code cannot. That is a narrow bar and it is the
reviewer's, but two cases are always wrong:

1. **Narrating the obvious.** `// set the field`, `// return the result`, `// loop over the ids`, a
   doc comment restating the signature, a paragraph where a sentence carries the same fact. Delete
   it; the identifier already says it.
2. **More comment than code.** Within any function, item, or module doc, comment lines must not
   outnumber code lines. A long "why" belongs in the module doc or the spec, not stapled above the
   function that happens to need it.

Write the reason, not the mechanics: why a shape is this way, what a wrong alternative would break,
which acceptance criterion or rule pins it. One line beats three, and a module doc that explains the
file beats a comment on every block. If removing a comment loses nothing, it was never a comment.

## Tracing

Instrumentation lives at boundaries, in `application/instrumented.rs` and
`application/instrumented_repos.rs`. `App::execute` and `ExecutionContext::enter` carry spans;
adapters emit `debug!`/`trace!` events only, never duplicate spans.

| Level | Use for |
|-------|---------|
| `info` | Use-case business milestones |
| `debug` | Transaction commit/rollback, port I/O, repository writes |
| `trace` | Repository reads, `SystemClock` |

1. No `#[instrument]` on a use-case `perform` — `App::execute` already names it, and a second span
   hides the first.
2. No spans on the public `UnitOfWork`; wrap at `InstrumentedUnitOfWork` and the ports.
3. New repository trait: add an `Instrumented*Repository` with `impl_instrumented_repo!`, add a
   `RepoStorage` field, and return it from `InstrumentedUnitOfWork`. The `uow` field stays last —
   [`docs/vertical-slice.md`](docs/vertical-slice.md) explains the pointer wiring.
4. New non-repository port: add an `Instrumented*` wrapper and wire it in `ExecutionContext`.

`backend/server/src/tracing_setup.rs` reads `FRIENDARAZZI_LOG_LEVEL` (default `info`).

## API layer

`FriendarazziAppApi` is a thin translation: request → use case → schema DTO. Writes go through
`App::execute`; reads run inline as one-off `execute_fn` closures and keep their refusal enum in
`api/errors.rs`. No business rules, no `poem` or OpenAPI types leaking into core, and none in the
facade either: the library compiles with `--no-default-features`. Schema DTOs are separate from
domain types; request DTOs are `…Request` and convert with `From`/`TryFrom`.

`FriendarazziApiSpec` in `crates/friendarazzi-api/src/http/mod.rs` is the HTTP edge: `#[OpenApi]` handlers
and native `poem_openapi` extractors. Its supporting HTTP code is split into `http/auth.rs`,
`http/jwt.rs`, `http/cookie.rs`, `http/extractors.rs`, and `http/responses/`. The typed facade is
`api/mod.rs`; request DTOs are grouped by domain under `api/requests/`. HTTP status mapping lives in
`errors/http_status.rs`: an attached `StatusCode` wins, then the current context, then the first
nested mapped context, else 500. An orchestration error variant returns `None` so the domain error
beneath it can speak. Statuses are attached at the API boundary, never in the domain.

After any schema or route change, run `mise run sync:openapi`; `mise run check` fails on drift.

## Testing

| Layer | Where | What |
|-------|-------|------|
| Domain unit | `#[cfg(test)]` in the domain source file | Isolated value-object and aggregate rules |
| Business conformance | `crates/friendarazzi-core/tests/cases/` | Full use-case flows against `friendarazzi-fakes` |
| Adapter conformance | `crates/friendarazzi-sqlite/tests/` | The same cases against real SQLite |
| E2E / API | `crates/friendarazzi-api/tests/e2e/` | `FriendarazziApiTestClient` over Poem `TestClient` |

Cases live once in `friendarazzi-core/tests/cases/` and are shared into the SQLite harness with
`#[path]`; each harness supplies its own `base_app_builder()`. Tests that need a fixed date use
`friendarazzi_fakes::FixedClock`. Never copy a case to run it twice.

`cases/` proves UoW parity across adapters. A test that validates one adapter, or one use case
against one adapter, is a normal integration test beside it (`crates/friendarazzi-core/tests/*.rs`),
not a shared case.

`mise run test` wraps two nextest profiles (`.config/nextest.toml`): `test:unit` runs everything that
needs no external service, `test:e2e` runs the `e2e` binary with the Mailpit daemon.

## Frontend

`FriendarazziApi` (`frontend/src/api/friendarazzi-api.ts`) is a transport-agnostic interface;
`HttpFriendarazziApi` implements it and `createFriendarazziApi()` in `frontend/src/api/index.ts` is the
only place that picks one. Components never see HTTP and never call the API directly: all server
state goes through the TanStack Query hooks in `frontend/src/api/hooks/`, with keys in
`query-keys.ts`. Never mirror server state in `useState`; use `queryClient.setQueryData` for
optimistic updates.

Configuration is static and baked in at build time (`NEXT_PUBLIC_FRIENDARAZZI_*`); there is no runtime
config file and no async bootstrap. Keep the access token in frontend memory only; never persist it
to `localStorage` or a cookie. The refresh token is held in the API's `HttpOnly` cookie. See
[`docs/vertical-slice.md`](docs/vertical-slice.md) for the seam and the state rules.

**UI work starts by reading [`.agents/skills/design-system/SKILL.md`](.agents/skills/design-system/SKILL.md).** It owns
tokens, components, contrast, touch targets, and the `/dev` gallery; this file does not. Values live
only in `frontend/src/app/globals.css`, used through semantic utilities. A new token, pairing,
variant, or convention updates that skill and `specs/product/visual-identity.md` in the same commit.

## Tooling

`mise` owns the toolchain and every task; see `mise.toml` and `tasks/`. `mise run ci` is the whole
gate (`check`, `build`, `test`, `verify`) and is what CI runs.

The Rust toolchain is pinned in `rust-toolchain.toml`; do not bump the nightly without testing,
because newer nightlies break poem-openapi's lifetime capture across `.await`.

- Invoke GitHub CLI via `mise x -- gh ...` (see [`.agents/skills/gh-via-mise/SKILL.md`](.agents/skills/gh-via-mise/SKILL.md)); never switch away from the agent account.
- When `graphify-out/graph.json` exists, query `graphify` before reading or searching files to explore the codebase, and update it after modifying code (see [`.agents/skills/graphify/SKILL.md`](.agents/skills/graphify/SKILL.md)).

## Version Control

- **Merge, don't rebase, to integrate upstream.** When a branch conflicts with or falls behind its
  base, create a merge commit (`jj new <branch> <base>@github`, resolve, commit). Conflict
  resolutions stay visible in history; never rewrite pushed commits to hide them.
- Rebase only for unpushed local work or explicit maintainer instruction. Never force-push a shared
  or reviewed branch.
- Commit identity and signing key come from the environment-conditional jj config (e.g. `CURSOR_AGENT`).
  Never override `user.name`, `user.email`, or `signing.key` inline, and never disable signing.

## Adding a feature

Walk [`docs/vertical-slice.md`](docs/vertical-slice.md) in order — domain → port → use case →
UoW wiring → fakes → migration → SQLite repository → API → tests → frontend — and copy its shape,
not its concrete names. Domain changes stay in core.

The changelog is end-user facing: log changes a player or host experiences, never internal refactoring.
