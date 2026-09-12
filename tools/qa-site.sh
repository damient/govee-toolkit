#!/usr/bin/env bash
# Runs every check .github/workflows/pages.yml runs on the site, in the same
# order, and prints a pass/fail summary.
#
# Kept in step with pages.yml by hand: the workflow is the authority, this is
# the local mirror of it. A check whose tool is missing is reported as skipped
# rather than passed, because a skip that reads as a pass is how a red CI gets
# discovered on the pull request instead of here.
#
# `tools/qa.sh` runs this script as one of its checks, so a full local run
# covers the site as well.

set -uo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
site="$root/site"
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TERM_COLOR=always

only=${1:-}
names=() results=()
log=$(mktemp -t govee-qa-site)
trap 'rm -f "$log"' EXIT

# record <name> <state>, where state is pass, fail or skip.
record() {
  names+=("$1")
  results+=("$2")
  case $2 in
  pass) printf '  ok\n' ;;
  skip) printf '  skipped: %s\n' "${3:-}" ;;
  fail)
    printf '  FAILED\n'
    sed 's/^/  | /' "$log"
    ;;
  esac
}

# check <name> <command...> — runs the command in site/, with output captured,
# so a passing check stays quiet and a failing one prints its log.
check() {
  local name=$1
  shift
  if [ -n "$only" ] && [[ $name != *"$only"* ]]; then return; fi
  printf '%s\n' "$name"
  if (cd "$site" && "$@") >"$log" 2>&1; then
    record "$name" pass
  else
    record "$name" fail
  fi
}

skip() {
  if [ -n "$only" ] && [[ $1 != *"$only"* ]]; then return; fi
  printf '%s\n' "$1"
  record "$1" skip "$2"
}

have() { command -v "$1" >/dev/null 2>&1; }

# The devices page reads this file, so the site build needs it before it runs.
# An existing one is left alone when cargo is missing: it is the same artifact
# the workflow uploads.
if have cargo; then
  check "device catalog" cargo run -q --manifest-path "$root/packages/rust/Cargo.toml" -p xtask
elif [ -f "$root/dist/catalog.json" ]; then
  skip "device catalog" "cargo, to regenerate dist/catalog.json"
else
  skip "device catalog" "cargo, to write dist/catalog.json"
fi

if have npm && [ -d "$site/node_modules" ]; then
  check "site build" npm run --silent build
  # The HTML linter reads dist/, so it runs after the build and not before.
  check "lint javascript" npm run --silent lint:js
  check "lint css" npm run --silent lint:css
  check "lint html" npm run --silent lint:html
else
  for name in "site build" "lint javascript" "lint css" "lint html"; do
    skip "$name" "cd site && npm install"
  done
fi

check "file length" "$root/tools/check-file-length.sh" site

echo
printf '%s\n' "-- summary"
failed=0 skipped=0
for i in "${!names[@]}"; do
  case ${results[$i]} in
  pass) printf 'pass  %s\n' "${names[$i]}" ;;
  fail)
    printf 'FAIL  %s\n' "${names[$i]}"
    failed=$((failed + 1))
    ;;
  skip)
    printf 'skip  %s\n' "${names[$i]}"
    skipped=$((skipped + 1))
    ;;
  esac
done
printf '%d failed, %d skipped, %d total\n' "$failed" "$skipped" "${#names[@]}"

[ "$failed" -eq 0 ] || exit 1
[ "$skipped" -eq 0 ] || exit 2
