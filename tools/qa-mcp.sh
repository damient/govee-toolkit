#!/usr/bin/env bash
# Runs the checks of integrations/mcp: the types, the lint, the build and the
# tests, the control tests against crates/sim. `tools/qa.sh` runs it as one of
# its checks.
#
# Nothing here reaches the network except the simulator, which answers on
# the real lan ports. The dependencies have to be installed already, with
# the binding of this checkout linked in: `npm ci && npm run link:binding`.

set -uo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
mcp="$root/integrations/mcp"
rust="$root/packages/rust"
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TERM_COLOR=always

# shellcheck source=SCRIPTDIR/lib/qa.sh
. "$root/tools/lib/qa.sh"
qa_init govee-qa-mcp "${1:-}"

# check <name> <command...> — in integrations/mcp/.
check() {
  local name=$1
  shift
  check_in "$mcp" "$name" "$@"
}

node_checks=("mcp types" "mcp lint" "mcp build" "mcp tests")

# The package ships dist/api.json, so the build needs a current one. Writing
# it checks the join too, so `tools/qa.sh` does not check it again. The
# control tests need the simulator.
if have cargo; then
  check "mcp api join" cargo run -q --manifest-path "$rust/Cargo.toml" -p xtask -- api
  check "mcp simulator" cargo build -q --manifest-path "$rust/Cargo.toml" -p govee-toolkit-sim
  export GOVEE_SIM="${CARGO_TARGET_DIR:-$rust/target}/debug/govee-toolkit-sim"
else
  skip "mcp api join" "cargo, to write dist/api.json"
  skip "mcp simulator" "cargo, to build crates/sim for the control tests"
fi

if ! have node; then
  for name in "${node_checks[@]}"; do skip "$name" "install Node.js 24 or newer"; done
elif [ ! -d "$mcp/node_modules/govee-toolkit" ]; then
  for name in "${node_checks[@]}"; do
    skip "$name" "npm ci && npm run link:binding, in integrations/mcp"
  done
else
  # The build copies data/, which the types and the tests read.
  check "mcp build" npm run --silent build
  check "mcp types" npm run --silent lint:types
  check "mcp lint" npm run --silent lint:js
  check "mcp tests" npm test
fi

check "file length" "$root/tools/check-file-length.sh" mcp

qa_summary
