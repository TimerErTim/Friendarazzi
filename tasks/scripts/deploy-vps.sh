#!/usr/bin/env bash
set -Eeuo pipefail

# Local half of `deploy:vps`. Validates the request, loads the canonical VPS layout,
# and streams `layout.env` + the remote half into the VPS over SSH stdin.
#
# Streaming from stdin ensures a checkout switch on the VPS never mutates the script
# currently executing.

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
layout_file="$repo_root/deploy/layout.env"
remote_script="$script_dir/deploy-vps-remote.sh"

fail() {
  echo "deploy:vps: $1" >&2
  exit 2
}

[[ -f "$layout_file" ]] || fail "missing $layout_file"
[[ -f "$remote_script" ]] || fail "missing $remote_script"
# shellcheck source=deploy/layout.env
source "$layout_file"

image="${usage_image:?missing <image>}"
revision="${usage_revision:-${image##*:sha-}}"
host="${usage_host:?set --host or VPS_HOST}"
user="${usage_user:?set --user or VPS_USER}"
api_domain="${usage_api_domain:-}"
host_root="${usage_root:-$VPS_HOST_ROOT_DEFAULT}"
identity="${usage_identity:-$HOME/.ssh/id_ed25519}"

# One reference format, everywhere: the commit tag. `:main` is pushed for humans
# browsing the registry and is never a valid deployment input, and a digest would
# name an image whose revision the VPS cannot check out.
[[ "$image" =~ ^ghcr\.io/[a-z0-9._-]+/friendarazzi-server:sha-[0-9a-f]{40}$ ]] ||
  fail "'$image' is not ghcr.io/<owner>/friendarazzi-server:sha-<commit>"
[[ "$revision" =~ ^[0-9a-f]{40}$ ]] || fail "'$revision' is not a full commit sha"

# The tag and the revision must describe the same build: checking out one revision
# and running an image from another would deploy a composition that does not belong
# to the binary.
[[ "${image##*:sha-}" == "$revision" ]] ||
  fail "image tag '${image##*:sha-}' does not match revision '$revision'"

[[ "$host" =~ ^[A-Za-z0-9.-]+$ ]] || fail "'$host' is not a hostname or IPv4 address"
[[ "$user" =~ ^[a-z_][a-z0-9_-]*$ ]] || fail "'$user' is not a Unix account name"
if [[ -n "$api_domain" ]]; then
  [[ "$api_domain" =~ ^[A-Za-z0-9.-]+$ ]] || fail "'$api_domain' is not a DNS hostname"
fi
[[ -f "$identity" ]] || fail "no SSH key at '$identity'"
[[ "$host_root" =~ ^[A-Za-z0-9_./~-]+$ ]] || fail "unsafe --root '$host_root'"

{
  cat "$layout_file"
  printf '\n'
  cat "$remote_script"
} | ssh -i "$identity" -o IdentitiesOnly=yes -o BatchMode=yes -o ConnectTimeout=15 "$user@$host" \
  "bash -s -- '$image' '$revision' '$host_root' '$api_domain'"
