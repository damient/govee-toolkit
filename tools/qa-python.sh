#!/usr/bin/env bash
# Runs the `python` job of .github/workflows/ci.yml on packages/python, in the
# same order. The workflow is the authority; this mirror is kept in step by
# hand. `tools/qa.sh` runs it as one of its checks.
#
# Nothing here reaches the network beyond what cargo already needs.

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

# The interpreter the tests and stubtest run on. `python3` on the PATH carries
# neither pytest nor mypy on most machines, so an environment under `target/`
# wins when it has both, and `GOVEE_QA_PYTHON` names another one. Nothing here
# creates that environment — `tools/README.md` gives the command.
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

# Where the wheel is unpacked for the checks that import it.
unpacked=target/qa-wheel

# Both output directories are emptied first: a stale wheel from an earlier run
# would be read instead of this one.
python_wheel() {
  rm -rf target/wheels "$unpacked"
  maturin build --out target/wheels || return 1
  mkdir -p "$unpacked"
  local wheel
  # shellcheck disable=SC2012 # maturin writes the name; find cannot sort by time.
  wheel=$(ls target/wheels/*.whl 2>/dev/null | head -1)
  [ -n "$wheel" ] || return 1
  python3 -m zipfile -e "$wheel" "$unpacked"
}

# in_wheel <module> [args...] — run a module of the tools interpreter against
# the unpacked wheel. It starts from the repository root: the working directory
# comes first on the import path, and `packages/python/govee_toolkit/` can hold
# what an earlier `maturin develop` left there.
in_wheel() {
  [ -d "$unpacked" ] || return 1
  (cd "$root" && PYTHONPATH="$py/$unpacked" "$tools_py" -m "$@")
}

# The configuration is named, so the run keeps testpaths and the asyncio mode.
python_tests() {
  in_wheel pytest -c "$py/pyproject.toml" --rootdir "$py"
}

# stubtest compares every name in the extension module against the stubs. It
# catches a signature that drifted; mypy reads the stubs alone and believes
# them. It runs against the unpacked wheel, not the tree.
python_stubs() {
  in_wheel mypy.stubtest govee_toolkit --concise
}

# ruff and mypy read the sources, so they run whether or not maturin is here.
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

check_fmt_nightly "python binding fmt" "$py"

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
