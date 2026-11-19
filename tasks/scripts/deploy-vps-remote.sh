#!/usr/bin/env bash
set -Eeuo pipefail

# Remote half of `deploy:vps`, streamed after `deploy/layout.env` over SSH stdin.
#
# Operator `.env` and `secrets/*.env` remain strictly read-only to this script.
# Deploy-managed runtime variables and state live under `$host_root/state/`,
# outside the mutable git checkout.
#
# Arguments arrive as `bash -s --` words:
#   1 image reference   2 revision   3 host root   4 optional api domain check

image="${1:?usage: deploy-vps-remote.sh <image> <revision> <host-root> [api-domain]}"
revision="${2:?usage: deploy-vps-remote.sh <image> <revision> <host-root> [api-domain]}"
host_root="${3:?usage: deploy-vps-remote.sh <image> <revision> <host-root> [api-domain]}"
expected_api_domain="${4:-}"

[[ "$image" =~ ^ghcr\.io/[a-z0-9._-]+/friendarazzi-server:sha-[0-9a-f]{40}$ ]] ||
  { echo "deploy:vps: invalid image reference '$image'" >&2; exit 2; }
[[ "$revision" =~ ^[0-9a-f]{40}$ ]] || { echo "deploy:vps: invalid revision '$revision'" >&2; exit 2; }
[[ "${image##*:sha-}" == "$revision" ]] ||
  { echo "deploy:vps: image tag and revision disagree" >&2; exit 2; }
if [[ -n "$expected_api_domain" ]]; then
  [[ "$expected_api_domain" =~ ^[A-Za-z0-9.-]+$ ]] ||
    { echo "deploy:vps: invalid API domain '$expected_api_domain'" >&2; exit 2; }
fi

host_root="${host_root/#\~/$HOME}"
[[ "$host_root" = /* ]] || { echo "deploy:vps: host root must be absolute: '$host_root'" >&2; exit 2; }

repo_dir="$host_root/${VPS_REPO_DIRNAME:-repo}"
data_dir="$host_root/${VPS_DATA_DIRNAME:-data}"
secrets_dir="$host_root/${VPS_SECRETS_DIRNAME:-secrets}"
state_dir="$host_root/${VPS_STATE_DIRNAME:-state}"
backups_dir="$host_root/${VPS_BACKUPS_DIRNAME:-backups}"

env_file="$host_root/.env"
mail_env="$secrets_dir/mail.env"
openobserve_env="$secrets_dir/openobserve.env"
deploy_env="$state_dir/deploy.env"
lock_file="$state_dir/deploy.lock"
caddy_state_dir="$state_dir/caddy"
maint_flag="$caddy_state_dir/maintenance.enabled"
maint_log="$state_dir/maintenance.log"
sqlite_snapshots_dir="$backups_dir/sqlite"

deployed_image_file="$state_dir/deployed-image"
deployed_digest_file="$state_dir/deployed-digest"
deployed_revision_file="$state_dir/deployed-revision"

read_env_value() {
  local key="$1" file="$2"
  sed -n "s/^${key}=//p" "$file" | tail -n 1 | tr -d '\r'
}

require_nonempty_key() {
  local key="$1" file="$2" val
  val="$(read_env_value "$key" "$file")"
  [[ -n "$val" ]] || { echo "deploy:vps: missing $key in $file" >&2; exit 2; }
}

now_ms() {
  local raw
  raw="$(date +%s%3N 2>/dev/null || true)"
  if [[ "$raw" =~ ^[0-9]+$ ]]; then
    printf '%s' "$raw"
  else
    printf '%s000' "$(date +%s)"
  fi
}

[[ -d "$repo_dir" ]] || { echo "deploy:vps: missing checkout at $repo_dir" >&2; exit 2; }
[[ -f "$env_file" ]] || { echo "deploy:vps: missing operator config at $env_file" >&2; exit 2; }
[[ -f "$mail_env" ]] || { echo "deploy:vps: missing SMTP credentials at $mail_env" >&2; exit 2; }
[[ -f "$openobserve_env" ]] || { echo "deploy:vps: missing OpenObserve credentials at $openobserve_env" >&2; exit 2; }
[[ -d "$data_dir/sqlite" ]] || { echo "deploy:vps: missing $data_dir/sqlite" >&2; exit 2; }
[[ -d "$data_dir/media" ]] || { echo "deploy:vps: missing $data_dir/media" >&2; exit 2; }
[[ -d "$data_dir/openobserve" ]] || { echo "deploy:vps: missing $data_dir/openobserve" >&2; exit 2; }

require_nonempty_key API_DOMAIN "$env_file"
require_nonempty_key FRONTEND_ORIGIN "$env_file"
require_nonempty_key MAIL_ABSENDER "$env_file"
require_nonempty_key FRIENDARAZZI_SMTP_BENUTZERNAME "$mail_env"
require_nonempty_key FRIENDARAZZI_SMTP_PASSWORT "$mail_env"
require_nonempty_key ZO_ROOT_USER_EMAIL "$openobserve_env"
require_nonempty_key ZO_ROOT_USER_PASSWORD "$openobserve_env"

api_domain="$(read_env_value API_DOMAIN "$env_file")"
[[ "$api_domain" =~ ^[A-Za-z0-9.-]+$ ]] || { echo "deploy:vps: invalid API_DOMAIN in $env_file" >&2; exit 2; }
if [[ -n "$expected_api_domain" && "$expected_api_domain" != "$api_domain" ]]; then
  echo "deploy:vps: API_DOMAIN mismatch: flag/CI has '$expected_api_domain' but $env_file has '$api_domain'" >&2
  exit 2
fi

zo_user="$(read_env_value ZO_ROOT_USER_EMAIL "$openobserve_env")"
zo_password="$(read_env_value ZO_ROOT_USER_PASSWORD "$openobserve_env")"

curl --version >/dev/null
command -v flock >/dev/null || { echo "deploy:vps: flock is required on the VPS" >&2; exit 2; }

mkdir -p "$state_dir" "$caddy_state_dir" "$sqlite_snapshots_dir"

# Serialize concurrent deployments with a fail-fast exclusive lock outside the checkout.
exec 9>"$lock_file"
if ! flock -n 9; then
  echo "deploy:vps: another deployment is already in progress (lock held on $lock_file)." >&2
  exit 1
fi

cd "$repo_dir"
repo_top="$(git rev-parse --show-toplevel)"
cd "$repo_top"

git fetch --no-tags origin '+refs/heads/*:refs/remotes/origin/*' ||
  { echo "deploy:vps: cannot fetch from origin." >&2; exit 2; }
git cat-file -e "${revision}^{commit}" 2>/dev/null || {
  echo "deploy:vps: deployment revision ${revision} is not a commit known to this checkout." >&2
  exit 2
}

git checkout --detach "$revision"

write_deploy_env() {
  local tmp="$deploy_env.tmp"
  {
    printf 'FRIENDARAZZI_IMAGE=%s\n' "$image"
    printf 'FRIENDARAZZI_HOST_ROOT=%s\n' "$host_root"
    printf 'FRIENDARAZZI_DEPLOYED_REVISION=%s\n' "$revision"
    printf 'FRIENDARAZZI_DEPLOYED_AT=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  } > "$tmp"
  mv "$tmp" "$deploy_env"
}

write_deploy_env

compose() {
  docker compose \
    --project-directory "$repo_top" \
    -f "$repo_top/compose.yaml" \
    --env-file "$env_file" \
    --env-file "$deploy_env" \
    "$@"
}

# Pull the release image before opening the maintenance window so image download time
# does not extend user-visible downtime.
compose pull backend

# Ensure supporting services (Caddy, certsync, mailserver, OpenObserve) match
# the checked-out compose definition and Caddyfile before toggling maintenance mode.
compose up -d --no-deps caddy certsync mailserver openobserve
compose exec -T caddy caddy reload --config /etc/caddy/Caddyfile >/dev/null

# ---------------------------------------------------------------------------
# Measured maintenance window: enable Caddy 503 + Retry-After before stopping
# backend or touching the database.
# ---------------------------------------------------------------------------
maint_start_iso="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
maint_start_ms="$(now_ms)"
printf '%s %s\n' "$maint_start_iso" "$revision" > "$maint_flag"
echo "deploy:vps: maintenance mode enabled at ${maint_start_iso} (revision ${revision})."

verify_maintenance_503() {
  local headers _
  for _ in $(seq 1 10); do
    headers="$(curl -sS -D - -o /dev/null --connect-timeout 3 \
      --resolve "${api_domain}:443:127.0.0.1" "https://${api_domain}/api/health" 2>/dev/null || true)"
    if printf '%s' "$headers" | grep -Eq '^HTTP/[0-9.]+ 503' &&
       printf '%s' "$headers" | grep -Eqi '^retry-after:[[:space:]]*[0-9]+'; then
      return 0
    fi
    sleep 1
  done
  echo "deploy:vps: Caddy did not return 503 with Retry-After after enabling maintenance mode." >&2
  return 1
}

verify_maintenance_503

# Stop the old backend cleanly so no writes occur between the SQLite snapshot and the new
# backend's startup migrations.
compose stop backend

# ---------------------------------------------------------------------------
# Pre-release SQLite snapshot (before migrations or schema-changing startup)
# ---------------------------------------------------------------------------
db_host_path="$data_dir/sqlite/friendarazzi.db"
snapshot_path="none (initial bootstrap)"

if [[ -f "$db_host_path" ]]; then
  snapshot_ts="$(date -u +%Y%m%dT%H%M%SZ)"
  snapshot_path="$sqlite_snapshots_dir/friendarazzi-${snapshot_ts}-${revision:0:12}.db"
  snapshot_tmp="${snapshot_path}.tmp"

  # Run sqlite3 inside the pulled release image as UID 10001 so WAL/file permissions match
  # the database directory and the VPS host needs no separate sqlite3 installation.
  if ! docker run --rm \
    --user 10001:10001 \
    --entrypoint sh \
    -v "$data_dir/sqlite:/data/sqlite" \
    "$image" \
    -c 'set -eu; sqlite3 /data/sqlite/friendarazzi.db ".timeout 5000" ".backup /tmp/snapshot.db"; integrity="$(sqlite3 -readonly /tmp/snapshot.db "PRAGMA integrity_check;")"; [ "$integrity" = "ok" ] || { echo "integrity_check failed: $integrity" >&2; exit 1; }; cat /tmp/snapshot.db' \
    > "$snapshot_tmp"; then
    rm -f "$snapshot_tmp"
    echo "deploy:vps: SQLite snapshot creation failed; aborting before starting release ${revision}. Maintenance mode remains active." >&2
    exit 1
  fi

  # Independently verify the written snapshot file on host disk is non-empty, readable, and
  # passes integrity_check + schema query before starting the release.
  if ! docker run --rm \
    --user 10001:10001 \
    --entrypoint sh \
    -v "$snapshot_tmp:/snapshot.db:ro" \
    "$image" \
    -c 'set -eu; test -s /snapshot.db; integrity="$(sqlite3 -readonly /snapshot.db "PRAGMA integrity_check;")"; [ "$integrity" = "ok" ] || { echo "snapshot verification failed: $integrity" >&2; exit 1; }; count="$(sqlite3 -readonly /snapshot.db "SELECT count(*) FROM sqlite_master;")"; [ "$count" -ge 0 ]' ; then
    rm -f "$snapshot_tmp"
    echo "deploy:vps: SQLite snapshot verification failed; aborting before starting release ${revision}. Maintenance mode remains active." >&2
    exit 1
  fi

  mv "$snapshot_tmp" "$snapshot_path"
  echo "deploy:vps: verified SQLite snapshot: ${snapshot_path} (release ${revision})."
else
  echo "deploy:vps: no existing SQLite database at ${db_host_path}; skipping pre-release snapshot."
fi

# ---------------------------------------------------------------------------
# Deploy/restart the single backend and verify health BEFORE ending maintenance
# ---------------------------------------------------------------------------
compose up -d --remove-orphans backend

backend_internal_health_check() {
  local version_body mail_body _
  for _ in $(seq 1 30); do
    if compose exec -T backend curl -fsS http://127.0.0.1:3000/api/health 2>/dev/null | grep -qx '"OK"'; then
      version_body="$(compose exec -T backend curl -fsS http://127.0.0.1:3000/api/version 2>/dev/null || true)"
      if printf '%s' "$version_body" | grep -Eq '"version"[[:space:]]*:[[:space:]]*"[^"]+"' &&
         { ! printf '%s' "$version_body" | grep -Eq '"commit"[[:space:]]*:[[:space:]]*"' ||
           printf '%s' "$version_body" | grep -Eq "\"commit\"[[:space:]]*:[[:space:]]*\"${revision}\""; }; then
        return 0
      fi
    fi
    sleep 3
  done
  echo "deploy:vps: backend internal health or version check failed; maintenance mode remains active." >&2
  return 1
}

backend_internal_mail_check() {
  local body="" _
  for _ in $(seq 1 30); do
    if body="$(compose exec -T backend curl -fsS http://127.0.0.1:3000/api/health/mail 2>/dev/null)"; then
      if printf '%s' "$body" | grep -q '"status":"Ok"'; then
        echo "deploy:vps: SMTP relay verified (Ok)."
        return 0
      fi
    fi
    sleep 5
  done
  echo "deploy:vps: SMTP relay health not confirmed: ${body:-no response}; maintenance mode remains active." >&2
  return 1
}

openobserve_health_check() {
  local _
  for _ in $(seq 1 15); do
    if curl -fsS http://127.0.0.1:5080/healthz >/dev/null 2>&1; then
      return 0
    fi
    sleep 2
  done
  echo "deploy:vps: OpenObserve /healthz check failed; maintenance mode remains active." >&2
  return 1
}

backend_internal_health_check
backend_internal_mail_check
openobserve_health_check

# Remove maintenance mode only after the backend, mail relay, and observability store are healthy.
rm -f "$maint_flag"
maint_end_iso="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
maint_end_ms="$(now_ms)"
maint_duration_ms=$((maint_end_ms - maint_start_ms))
maint_duration_s="$(awk "BEGIN { printf \"%.2f\", ${maint_duration_ms}/1000 }")"
printf '%s\t%s\t%sms\t%s\t%s\n' \
  "$maint_start_iso" "$maint_end_iso" "$maint_duration_ms" "$revision" "$snapshot_path" >> "$maint_log"
echo "deploy:vps: maintenance mode removed at ${maint_end_iso} (duration: ${maint_duration_s}s)."

# ---------------------------------------------------------------------------
# Post-maintenance external verification & OpenObserve signal verification
# ---------------------------------------------------------------------------
verify_public_https() {
  local health_body version_body _
  for _ in $(seq 1 10); do
    health_body="$(curl -fsS --connect-timeout 3 \
      --resolve "${api_domain}:443:127.0.0.1" "https://${api_domain}/api/health" 2>/dev/null || true)"
    version_body="$(curl -fsS --connect-timeout 3 \
      --resolve "${api_domain}:443:127.0.0.1" "https://${api_domain}/api/version" 2>/dev/null || true)"
    if [[ "$health_body" == '"OK"' ]] &&
       printf '%s' "$version_body" | grep -Eq '"version"[[:space:]]*:[[:space:]]*"[^"]+"'; then
      echo "deploy:vps: public HTTPS verified (${version_body})."
      return 0
    fi
    sleep 2
  done
  echo "deploy:vps: public HTTPS verification failed after removing maintenance mode." >&2
  return 1
}

verify_openobserve_traces() {
  local end_us start_us resp _
  for _ in $(seq 1 15); do
    end_us=$(( $(date +%s) * 1000000 + 60000000 ))
    start_us=$(( end_us - 600000000 ))

    resp="$(curl -sS -X POST "http://127.0.0.1:5080/api/default/_search?type=traces" \
      -u "${zo_user}:${zo_password}" \
      -H "Content-Type: application/json" \
      -d "{\"query\":{\"sql\":\"SELECT * FROM \\\"friendarazzi_traces\\\" LIMIT 5\",\"start_time\":${start_us},\"end_time\":${end_us},\"from\":0,\"size\":5},\"search_type\":\"ui\"}" \
      2>/dev/null || true)"
    if printf '%s' "$resp" | grep -Eq '"hits"[[:space:]]*:[[:space:]]*\[[[:space:]]*\{'; then
      echo "deploy:vps: OpenObserve verified (backend OTLP traces searchable)."
      return 0
    fi
    sleep 2
  done

  echo "deploy:vps: OpenObserve trace verification failed (friendarazzi_traces had no hits)." >&2
  return 1
}

verify_public_https
verify_openobserve_traces

digest="$(docker image inspect --format '{{index .RepoDigests 0}}' "$image" 2>/dev/null || true)"
[[ -n "$digest" ]] || { echo "deploy:vps: cannot read digest of $image" >&2; exit 1; }

printf '%s\n' "$image" > "$deployed_image_file.tmp"
printf '%s\n' "$digest" > "$deployed_digest_file.tmp"
printf '%s\n' "$revision" > "$deployed_revision_file.tmp"
mv "$deployed_image_file.tmp" "$deployed_image_file"
mv "$deployed_digest_file.tmp" "$deployed_digest_file"
mv "$deployed_revision_file.tmp" "$deployed_revision_file"

echo "deploy:vps: deployed revision=${revision} image=${image} digest=${digest} snapshot=${snapshot_path} maintenance=${maint_duration_s}s"

# Remove unused older images after a successful deployment while keeping recent images
# for a week for inspection.
docker image prune --all --force --filter until=168h >/dev/null
