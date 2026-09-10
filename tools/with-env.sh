#!/usr/bin/env bash
# Runs a command with the repository's `.env` in its environment.
#
#   tools/with-env.sh cargo run --example cloud_tour --features cloud
#
# `GOVEE_ENV_FILE` names another file to read instead. See CONTRIBUTING.md,
# "Local credentials".

set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
env_file="${GOVEE_ENV_FILE:-$root/.env}"

if [ "$#" -eq 0 ]; then
  echo "usage: $(basename "$0") <command> [args...]" >&2
  exit 64
fi

if [ ! -f "$env_file" ]; then
  echo "$(basename "$0"): no $env_file — copy .env.example to .env" >&2
  exit 1
fi

# Collected rather than exported, so this script's own environment stays clean.
declare -a vars=()
while IFS= read -r line; do
  case "$line" in
    ''|'#'*) continue ;;
  esac
  name=${line%%=*}
  value=${line#*=}
  # An empty assignment is a placeholder in .env.example, not a value.
  [ -n "$value" ] || continue
  # The caller's environment wins.
  [ -z "${!name-}" ] || continue
  vars+=("$name=$value")
done < "$env_file"

# `${vars[@]}` on an empty array is an unbound variable under `set -u` in bash
# 3.2, which is the bash macOS ships. The file can leave the array empty: every
# value blank, or every one overridden by the caller.
if [ "${#vars[@]}" -eq 0 ]; then
  exec "$@"
fi

exec env "${vars[@]}" "$@"
