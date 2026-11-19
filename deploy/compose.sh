#!/usr/bin/env bash
set -Eeuo pipefail

# Operator convenience wrapper for running `docker compose` on the VPS with the
# canonical layout, operator `.env`, and deploy-managed `state/deploy.env`.
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=deploy/layout.env
source "$script_dir/layout.env"

host_root="${FRIENDARAZZI_HOST_ROOT:-$VPS_HOST_ROOT_DEFAULT}"
repo_dir="$host_root/$VPS_REPO_DIRNAME"
state_dir="$host_root/$VPS_STATE_DIRNAME"
env_file="$host_root/.env"
deploy_env="$state_dir/deploy.env"

[[ -f "$env_file" ]] || { echo "Missing operator config: $env_file" >&2; exit 2; }

args=(
  --project-directory "$repo_dir"
  -f "$repo_dir/compose.yaml"
  --env-file "$env_file"
)
if [[ -f "$deploy_env" ]]; then
  args+=(--env-file "$deploy_env")
fi

exec docker compose "${args[@]}" "$@"
