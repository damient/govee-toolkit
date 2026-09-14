#!/usr/bin/env bash
# Runs every check the `python` job of .github/workflows/ci.yml runs on
# packages/python, in the same order, and prints a pass/fail summary.
#
# Kept in step with ci.yml by hand: the workflow is the authority, this is the
# local mirror of it. The reporter and the skip rule are in lib/qa.sh.
#
# `tools/qa.sh` runs this script as one of its checks.
#
# Nothing here reaches the network beyond what cargo already needs: the tests
# read the wheel maturin wrote, and pytest comes from the interpreter that runs
# the script.

set -uo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
py="$root/packages/python"
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TERM_COLOR=always
export RUSTFLAGS=${RUSTFLAGS:--D warnings}
export CARGO_INCREMENTAL=${CARGO_INCREMENTAL:-0}

# shellcheck source=lib/qa.sh
. "$root/tools/lib/qa.sh"
qa_init govee-qa-python "${1:-}"

# check <name> <command...> — in packages/python/.
check() {
  local name=$1
  shift
  check_in "$py" "$name" "$@"
}

# A stale wheel from an earlier run would be installed instead of this one, so
# the output directory is emptied first.
python_wheel() {
  rm -rf target/wheels
  maturin build --out target/wheels
}

# The tests import the extension module, so the wheel maturin wrote is
# unpacked and put on the import path. `zipfile` is in the standard library, so
# this needs neither pip nor the network, and it leaves the interpreter that
# runs the script alone.
python_tests() {
  local unpacked=target/qa-wheel
  rm -rf "$unpacked"
  mkdir -p "$unpacked"
  local wheel
  wheel=$(ls target/wheels/*.whl 2>/dev/null | head -1)
  [ -n "$wheel" ] || return 1
  python3 -m zipfile -e "$wheel" "$unpacked" || return 1
  # pytest runs from the repository root, not from packages/python: the working
  # directory comes first on the import path, and `govee_toolkit/` there holds
  # the Python half of the package without the extension module beside it. The
  # configuration is named, so the run keeps testpaths and the asyncio mode.
  (cd "$root" && PYTHONPATH="$py/$unpacked" python3 -m pytest \
    -c "$py/pyproject.toml" --rootdir "$py")
}

if have rustup && rustup toolchain list | grep -q '^nightly'; then
  # rustfmt.toml uses nightly-only options; stable rustfmt formats differently.
  check "python binding fmt" env RUSTUP_TOOLCHAIN=nightly cargo fmt --all --check
else
  skip "python binding fmt" "rustup toolchain install nightly"
fi

check "python binding clippy" cargo clippy --all-targets --all-features -- -D warnings

if ! have python3; then
  for name in "python wheel" "python tests"; do
    skip "$name" "install Python 3.11 or newer"
  done
elif ! have maturin; then
  for name in "python wheel" "python tests"; do
    skip "$name" "pip install maturin"
  done
else
  check "python wheel" python_wheel
  if python3 -c 'import pytest, pytest_asyncio' >/dev/null 2>&1; then
    check "python tests" python_tests
  else
    skip "python tests" "pip install pytest pytest-asyncio"
  fi
fi

qa_summary
