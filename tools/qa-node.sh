#!/usr/bin/env bash
# Runs the `node` job of .github/workflows/ci.yml on packages/node, in the
# same order. The workflow is the authority; this mirror is kept in step by
# hand. `tools/qa.sh` runs it as one of its checks.
#
# Nothing here reaches the network: the dependencies have to be installed
# already, and the checks that need them are skipped where they are not.

set -uo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
node_pkg="$root/packages/node"
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TERM_COLOR=always
export RUSTFLAGS=${RUSTFLAGS:--D warnings}
export CARGO_INCREMENTAL=${CARGO_INCREMENTAL:-0}

# shellcheck source=SCRIPTDIR/lib/qa.sh
. "$root/tools/lib/qa.sh"
qa_init govee-qa-node "${1:-}"

# check <name> <command...> — in packages/node/.
check() {
  local name=$1
  shift
  check_in "$node_pkg" "$name" "$@"
}

generated_in_step() {
  git -C "$root" diff --exit-code -- \
    packages/node/binding.cjs packages/node/binding.d.cts
}

check_fmt_nightly "node binding fmt" "$node_pkg"

check "node binding clippy" cargo clippy --all-targets --all-features -- -D warnings

if ! have node; then
  for name in "node build" "node generated binding" "node tests"; do
    skip "$name" "install Node.js 20 or newer"
  done
elif [ ! -d "$node_pkg/node_modules" ]; then
  for name in "node build" "node generated binding" "node tests"; do
    skip "$name" "npm ci, in packages/node"
  done
else
  check "node build" npm run build:debug
  check "node generated binding" generated_in_step
  check "node tests" npm test
fi

qa_summary
