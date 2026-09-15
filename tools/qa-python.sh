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

# shellcheck source=SCRIPTDIR/lib/qa.sh
. "$root/tools/lib/qa.sh"
qa_init govee-qa-python "${1:-}"

# check <name> <command...> — in packages/python/.
check() {
  local name=$1
  shift
  check_in "$py" "$name" "$@"
}

# The interpreter that the tests and stubtest run on. `python3` on the PATH
# carries neither pytest nor mypy on most machines, so a virtual environment
# under `target/` wins when it has both. `GOVEE_QA_PYTHON` names another one.
# `tools/README.md` gives the command that makes the environment; nothing here
# creates it, because that reaches the network.
tools_python() {
  local candidate
  for candidate in "${GOVEE_QA_PYTHON:-}" "$py/target/qa-tools-venv/bin/python" \
    "$(command -v python3)"; do
    [ -x "$candidate" ] || continue
    if "$candidate" -c 'import pytest, pytest_asyncio, mypy.stubtest' \
      >/dev/null 2>&1; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  return 1
}

# A stale wheel from an earlier run would be installed instead of this one, so
# the output directory is emptied first.
python_wheel() {
  rm -rf target/wheels
  maturin build --out target/wheels
}

# The wheel maturin wrote, unpacked where the tests and stubtest both import it
# from. `zipfile` is in the standard library, so this needs neither pip nor the
# network, and it leaves the interpreter that runs the script alone.
unpacked=target/qa-wheel
unpack_wheel() {
  rm -rf "$unpacked"
  mkdir -p "$unpacked"
  local wheel
  # shellcheck disable=SC2012 # maturin writes the name; find cannot sort by time.
  wheel=$(ls target/wheels/*.whl 2>/dev/null | head -1)
  [ -n "$wheel" ] || return 1
  python3 -m zipfile -e "$wheel" "$unpacked"
}

python_tests() {
  unpack_wheel || return 1
  # pytest runs from the repository root, not from packages/python: the working
  # directory comes first on the import path, and `govee_toolkit/` there holds
  # the Python half of the package without the extension module beside it. The
  # configuration is named, so the run keeps testpaths and the asyncio mode.
  (cd "$root" && PYTHONPATH="$py/$unpacked" "$tools_py" -m pytest \
    -c "$py/pyproject.toml" --rootdir "$py")
}

# stubtest imports the extension module and compares every name in it against
# the stubs beside it. It is what catches a signature that drifted: mypy reads
# the stubs alone and believes them. It runs against the unpacked wheel, so it
# reads the stubs the wheel carries and not the ones in the tree.
python_stubs() {
  unpack_wheel || return 1
  # From the repository root, as the tests run: the working directory comes
  # first on the import path, and `packages/python/govee_toolkit/` holds the
  # extension module an earlier `maturin develop` left there. stubtest must
  # read the wheel this run built.
  (cd "$root" && PYTHONPATH="$py/$unpacked" "$tools_py" -m mypy.stubtest \
    govee_toolkit --concise)
}

# ruff and mypy read the sources, not the built module, so they run whether or
# not maturin is here. stubtest is the one that needs the wheel installed: it
# imports the extension module and compares it against the stubs.
if have ruff; then
  check "python fmt" ruff format --check govee_toolkit tests
  check "python lint" ruff check govee_toolkit tests
else
  for name in "python fmt" "python lint"; do
    skip "$name" "pip install ruff"
  done
fi

if have mypy; then
  check "python types" mypy
else
  skip "python types" "pip install mypy"
fi

if have rustup && rustup toolchain list | grep -q '^nightly'; then
  # rustfmt.toml uses nightly-only options; stable rustfmt formats differently.
  check "python binding fmt" env RUSTUP_TOOLCHAIN=nightly cargo fmt --all --check
else
  skip "python binding fmt" "rustup toolchain install nightly"
fi

check "python binding clippy" cargo clippy --all-targets --all-features -- -D warnings

# The wheel is what the tests and stubtest import, so what one tool is missing
# skips all three.
missing=
if ! have python3; then
  missing="install Python 3.11 or newer"
elif ! have maturin; then
  missing="pip install maturin"
fi

if [ -n "$missing" ]; then
  for name in "python wheel" "python tests" "python stubs"; do
    skip "$name" "$missing"
  done
else
  check "python wheel" python_wheel
  if tools_py=$(tools_python); then
    check "python tests" python_tests
    check "python stubs" python_stubs
  else
    for name in "python tests" "python stubs"; do
      skip "$name" "make target/qa-tools-venv: python3 -m venv it, then install pytest pytest-asyncio mypy in it"
    done
  fi
fi

qa_summary
