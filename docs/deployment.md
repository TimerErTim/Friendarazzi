# Docker Compose deployment

This runbook covers the Compose-managed single-replica API, Caddy HTTPS with measured maintenance mode, the docker-mailserver relay, pre-release SQLite snapshots, OpenObserve observability, and the automated release pipeline. Docker Engine and the Docker Compose plugin run on the VPS independently of `mise`; the VPS needs neither `mise` nor a compiler. CI builds and tests the release binary once, publishes the immutable `sha-<commit>` Docker image to GHCR, and both GitHub Pages and the VPS deploy from that same commit.

## Operator-owned prerequisites

- A VPS with Docker Engine, Docker Compose plugin v2.24 or newer, Git, `curl`, `flock` (`util-linux`), and outbound access to GitHub and `ghcr.io`. The deployment account must be able to read/update the repository checkout and run Docker Compose.
- The `friendarazzi-server` package on GHCR must be set to public once after the first `deploy/image` run so the VPS pulls anonymously without stored registry credentials.
- Public DNS for `API_DOMAIN` pointing to the VPS; inbound TCP 80/443 (and optional UDP 443 for HTTP/3) for Caddy; inbound/outbound TCP 25, 465, and 587 for `docker-mailserver`.
- GitHub `production` environment configured with variables `VPS_HOST` and `VPS_USER` (plus optional `API_DOMAIN` to cross-check the VPS `.env`), and secrets `VPS_SSH_PRIVATE_KEY` and `VPS_SSH_KNOWN_HOSTS` (verified out of band, never populated via `ssh-keyscan` in CI).
- GitHub repository variable `FRIENDARAZZI_API_URL` configured for the static Pages build.

## Canonical VPS layout (`deploy/layout.env`)

The directory layout is defined once in [`deploy/layout.env`](../deploy/layout.env) and consumed by `deploy:vps`, `mail-bootstrap.sh`, and `deploy/compose.sh`:

```text
/opt/friendarazzi/                 # FRIENDARAZZI_HOST_ROOT (VPS_HOST_ROOT_DEFAULT)
├── .env                           # Operator-owned config (read-only to deploy scripts)
├── repo/                          # Full (non-shallow) Git checkout at the deployed SHA
├── data/
│   ├── sqlite/                    # friendarazzi.db (writable by container UID 10001)
│   ├── media/                     # Object store uploads (writable by container UID 10001)
│   ├── mail/                      # docker-mailserver mail-data, mail-state, mail-config
│   └── openobserve/               # OpenObserve persistent storage (/data)
├── secrets/
│   ├── mail.env                   # Operator-owned SMTP credentials
│   └── openobserve.env            # Operator-owned OpenObserve root credentials
├── state/                         # Deploy state outside the mutable checkout
│   ├── deploy.env                 # Deploy-written FRIENDARAZZI_IMAGE, HOST_ROOT, REVISION
│   ├── deploy.lock                # flock exclusive lock serializing deployments
│   ├── deployed-image             # Last successfully deployed image tag
│   ├── deployed-digest            # Registry digest of last deployed image
│   ├── deployed-revision          # Commit SHA of last deployed revision
│   ├── maintenance.log            # Measured maintenance windows (start, end, duration, sha, snapshot)
│   └── caddy/
│       └── maintenance.enabled    # Presence triggers Caddy 503 + Retry-After
└── backups/
    └── sqlite/                    # Timestamped, SHA-tagged pre-release SQLite snapshots
```

Because `state/`, `data/`, `secrets/`, and `backups/` live outside `repo/`, re-cloning or running `git clean` inside `repo/` never destroys deployment state, secrets, snapshots, or application data.

## Initial VPS installation and boot persistence

1. Clone a full (non-shallow) checkout into `/opt/friendarazzi/repo` and create the host directories:

   ```sh
   mkdir -p /opt/friendarazzi/data/{sqlite,media,mail/mail-data,mail/mail-state,mail/mail-config,openobserve} \
            /opt/friendarazzi/{secrets,state/caddy,backups/sqlite}
   chown -R 10001:10001 /opt/friendarazzi/data/sqlite /opt/friendarazzi/data/media
   chmod 700 /opt/friendarazzi/secrets
   ```

2. Copy [`.env.example`](../.env.example) to `/opt/friendarazzi/.env` and set `API_DOMAIN`, `FRONTEND_ORIGIN`, and `MAIL_ABSENDER`. Deploy scripts never rewrite `/opt/friendarazzi/.env`.
3. Create `/opt/friendarazzi/secrets/mail.env` and `/opt/friendarazzi/secrets/openobserve.env` (see sections below).
4. **No signing key file is required.** The server generates an Ed25519 key per process and stores each session's public half on its session row, so existing sessions keep verifying across restarts.
5. Deploy a published image using `mise run deploy:vps ghcr.io/<owner>/friendarazzi-server:sha-<commit>` (or trigger the workflow). For manual Compose commands on the VPS after a deployment, use the wrapper so both `/opt/friendarazzi/.env` and `/opt/friendarazzi/state/deploy.env` are loaded:

   ```sh
   bash /opt/friendarazzi/repo/deploy/compose.sh ps
   bash /opt/friendarazzi/repo/deploy/compose.sh logs -f backend caddy
   ```

6. Install [`deploy/friendarazzi.service`](../deploy/friendarazzi.service) so the stack starts automatically on host reboot:

   ```sh
   sudo cp /opt/friendarazzi/repo/deploy/friendarazzi.service /etc/systemd/system/friendarazzi.service
   sudo systemctl daemon-reload
   sudo systemctl enable --now friendarazzi.service
   ```

   All containers in [`compose.yaml`](../compose.yaml) also configure `restart: unless-stopped` and bounded `json-file` log rotation (`max-size: 10m`, `max-file: 5`). Never run `docker compose down -v` in production: it removes Caddy's named certificate and state volumes (`caddy_data`, `caddy_config`, `mail_certs`).

## Release sequence and synchronized deploy

- Pull requests never deploy.
- On a push to `main`, `ci` (`pr/base`) and `release-check` must both pass before `deploy/image` builds and pushes `ghcr.io/<owner>/friendarazzi-server:sha-<commit>` (plus the human-browsing `:main` tag).
- Both `deploy/pages` (GitHub Pages static export) and `deploy/backend` (VPS Compose deploy) wait for `deploy/image` to succeed and deploy artifacts built from that exact commit SHA. There is no second backend replica, staging environment, or frontend/backend compatibility matrix; brief UI/API mismatches during the short rollout window are expected.
- `deploy/release` publishes the GitHub release only after `ci`, `release-check`, `deploy/pages`, and `deploy/backend` all succeed.
- `workflow_dispatch` runs the same `ci -> deploy/image -> {deploy/pages, deploy/backend}` pipeline from any branch (skipping `release-check` and `deploy/release`) without moving `:main`.
- Concurrent VPS deployments are serialized with a non-blocking `flock` on `/opt/friendarazzi/state/deploy.lock`; a second concurrent run exits immediately with a clear error message.

## Measured maintenance window

During backend replacement, Caddy serves an explicit `503 Service Unavailable` with `Retry-After: 30` (and CORS headers for `FRONTEND_ORIGIN`) whenever `/opt/friendarazzi/state/caddy/maintenance.enabled` exists:

1. `deploy:vps` checks out the target SHA, updates `state/deploy.env`, pulls the new backend image, and updates/reloads supporting services **before** enabling maintenance mode.
2. It creates `state/caddy/maintenance.enabled`, records the start timestamp, and verifies Caddy returns `503` with `Retry-After`.
3. It stops the old `backend` container, creates and verifies the pre-release SQLite snapshot, starts the new `backend` container (which runs `molting` migrations on startup), and verifies `/api/health`, `/api/version`, `/api/health/mail`, and OpenObserve `/healthz` before removing `maintenance.enabled`.
4. After removing `maintenance.enabled`, it records the start time, end time, and duration in `state/maintenance.log` and in the deploy output, then verifies public HTTPS and OpenObserve signal ingestion.

### Manual maintenance toggle and recovery

If a deployment aborts during maintenance mode (for example, a failed snapshot or health check), `maintenance.enabled` stays in place so traffic receives `503` rather than broken responses.

- Enable manually: `touch /opt/friendarazzi/state/caddy/maintenance.enabled`
- Disable manually after verifying health: `rm -f /opt/friendarazzi/state/caddy/maintenance.enabled`
- Inspect measured history: `cat /opt/friendarazzi/state/maintenance.log`

## Pre-release SQLite snapshot

Before any release starts the new backend container (and before `molting` migrations run), `deploy:vps` stops `backend` and creates a consistent SQLite snapshot in `/opt/friendarazzi/backups/sqlite/friendarazzi-<YYYYMMDDTHHMMSSZ>-<sha12>.db`:

- Created using `sqlite3 ... ".backup ..."` inside the pulled `friendarazzi-server` image as UID `10001` (not a raw file copy of a live database).
- Verified both inside the backup step and by re-opening the written host file read-only with `PRAGMA integrity_check;` (`ok`) and `SELECT count(*) FROM sqlite_master;`.
- If snapshot creation or verification fails, deployment stops immediately without starting the new release or running migrations.

The SQLite snapshot is an extra pre-migration safeguard; it does not replace host/provider-level backups.

## Version endpoint (`GET /api/version`)

The backend exposes `GET /api/version` returning:

- `version`: `CARGO_PKG_VERSION`
- `buildTime`: UTC compile-time RFC 3339 timestamp
- `commit`: compile-time Git commit SHA when supplied (`FRIENDARAZZI_BUILD_COMMIT` / `GITHUB_SHA` / `git rev-parse HEAD`), or `null`

It exposes no environment variables or secrets. `deploy:vps` verifies `/api/version` reports the expected `commit` before lifting maintenance mode.

## OpenObserve observability

[`compose.yaml`](../compose.yaml) runs OpenObserve (`public.ecr.aws/zinclabs/openobserve:v1.1.2`) alongside Caddy and the backend:

- **Exposure & auth**: OpenObserve binds strictly to `127.0.0.1:5080` on the VPS; it is never exposed publicly. Access the UI over an SSH tunnel:

  ```sh
  ssh -N -L 5080:127.0.0.1:5080 <VPS_USER>@<VPS_HOST>
  # Open http://127.0.0.1:5080 in your browser
  ```

- **Credentials (`/opt/friendarazzi/secrets/openobserve.env`)**:

  ```env
  ZO_ROOT_USER_EMAIL=ops@example.invalid
  ZO_ROOT_USER_PASSWORD=replace-with-strong-password
  ```

- **Backend OTLP traces**: `backend` exports OpenTelemetry spans over OTLP HTTP to `http://openobserve:5080/api/default/v1/traces` (`stream-name: friendarazzi_traces`), authenticating via HTTP Basic auth from `secrets/openobserve.env`.
- **Caddy logs**: Caddy logs remain in Docker's rotated `json-file` container log stream (`bash /opt/friendarazzi/repo/deploy/compose.sh logs caddy`) without an extra log-shipper sidecar in this iteration.
- **Verification**: `deploy:vps` automatically verifies `friendarazzi_traces` after lifting maintenance mode. To query traces manually on the VPS:

  ```sh
  set -a; . /opt/friendarazzi/secrets/openobserve.env; set +a
  NOW_US=$(( $(date +%s) * 1000000 ))
  START_US=$(( NOW_US - 900000000 ))

  curl -fsS -X POST "http://127.0.0.1:5080/api/default/_search?type=traces" \
    -u "${ZO_ROOT_USER_EMAIL}:${ZO_ROOT_USER_PASSWORD}" \
    -H "Content-Type: application/json" \
    -d "{\"query\":{\"sql\":\"SELECT * FROM \\\"friendarazzi_traces\\\" LIMIT 5\",\"start_time\":${START_US},\"end_time\":${NOW_US},\"from\":0,\"size\":5},\"search_type\":\"ui\"}"
  ```

## Mail (`docker-mailserver`)

The mail server runs unconditionally in `compose.yaml` (MVP registration and password recovery depend on it). Caddy obtains one TLS certificate for `API_DOMAIN`, `certsync` republishes it into `mail_certs`, and `docker-mailserver` reloads Postfix/Dovecot automatically when the pair changes.

1. Populate `/opt/friendarazzi/secrets/mail.env`:

   ```env
   FRIENDARAZZI_SMTP_BENUTZERNAME=<submission-account>
   FRIENDARAZZI_SMTP_PASSWORT=<submission-password>
   ```

2. After the first deployment brings up Caddy and `mailserver`, run the one-time bootstrap on the VPS:

   ```sh
   cd /opt/friendarazzi/repo && bash tasks/scripts/mail-bootstrap.sh
   ```

3. Publish the printed `MX`, `SPF`, `DKIM`, `DMARC`, and provider `PTR` records.
4. `GET /api/health/mail` probes SMTP submission in the background without blocking API startup (`GET /api/health` stays `OK`); `deploy:vps` waits for `/api/health/mail` to report `Ok` before ending maintenance mode.

## Recovery responsibilities

- **Server-level recovery**: Hetzner VPS-provider server backups are the primary rollback/recovery mechanism for host failures or post-migration rollbacks. Confirm provider backups cover `/opt/friendarazzi/{data,secrets,state,backups}` and Docker's named volumes (`caddy_data`, `caddy_config`, `mail_certs`).
- **Pre-release SQLite snapshots**: Located under `/opt/friendarazzi/backups/sqlite/`. If a release fails after migration and you need to inspect or restore the pre-release database state alongside provider recovery, restore from the matching `friendarazzi-<timestamp>-<sha12>.db` snapshot while `backend` is stopped and maintenance mode is enabled.
- **Local development build**: On a workstation, build with `mise run build:server`, stage `.deploy/incoming/friendarazzi-server`, set `FRIENDARAZZI_HOST_ROOT=.` with local `secrets/{mail,openobserve}.env`, and run `docker compose -f compose.yaml -f compose.build.yaml up -d --build`. `deploy:vps` never passes `compose.build.yaml`.
