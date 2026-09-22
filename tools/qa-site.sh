#!/usr/bin/env bash
# Runs every check .github/workflows/pages.yml runs on the site, in the same
# order, and prints a pass/fail summary.
#
# Kept in step with pages.yml by hand: the workflow is the authority, this is
# the local mirror of it. The reporter and the skip rule are in lib/qa.sh.
#
# `tools/qa.sh` runs this script as one of its checks.

set -uo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
site="$root/site"
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TERM_COLOR=always

# shellcheck source=SCRIPTDIR/lib/qa.sh
. "$root/tools/lib/qa.sh"
qa_init govee-qa-site "${1:-}"

# check <name> <command...> — in site/.
check() {
  local name=$1
  shift
  check_in "$site" "$name" "$@"
}

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
  check "lint types" npm run --silent lint:types
  check "lint javascript" npm run --silent lint:js
  check "lint css" npm run --silent lint:css
  check "lint html" npm run --silent lint:html
else
  for name in "site build" "lint types" "lint javascript" "lint css" "lint html"; do
    skip "$name" "cd site && npm install"
  done
fi

check "file length" "$root/tools/check-file-length.sh" site

qa_summary
