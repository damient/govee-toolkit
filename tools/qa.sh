#!/usr/bin/env bash
# Runs every check .github/workflows/ci.yml runs, in the same order.
#
# Kept in step with ci.yml by hand: the workflow is the authority, this is the
# local mirror of it. A check whose tool is missing is reported as skipped
# rather than passed, because a skip that reads as a pass is how a red CI gets
# discovered on the pull request instead of here.

set -uo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
rust="$root/packages/rust"
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TERM_COLOR=always
export RUSTFLAGS=${RUSTFLAGS:--D warnings}

MSRV=$(sed -n 's/^rust-version *= *"\([^"]*\)".*/\1/p' "$rust/Cargo.toml" | head -1)

only=${1:-}
names=() results=()
log=$(mktemp -t govee-qa)
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

# check <name> <command...> — runs the command with output captured, so a
# passing check stays quiet and a failing one prints its log.
check() {
  local name=$1
  shift
  if [ -n "$only" ] && [[ $name != *"$only"* ]]; then return; fi
  printf '%s\n' "$name"
  if (cd "$rust" && "$@") >"$log" 2>&1; then
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

if have rustup && rustup toolchain list | grep -q '^nightly'; then
  # rustfmt.toml uses nightly-only options; stable rustfmt formats differently.
  check "rust fmt" env RUSTUP_TOOLCHAIN=nightly cargo fmt --all --check
else
  skip "rust fmt" "rustup toolchain install nightly"
fi

check "rust clippy" cargo clippy --all-targets --all-features -- -D warnings
check "rust test" cargo test --all-features
check "rust doc" env RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features

if [ -z "$MSRV" ]; then
  skip "rust msrv" "no rust-version in packages/rust/Cargo.toml"
elif have rustup && rustup toolchain list | grep -q "^$MSRV"; then
  check "rust msrv ($MSRV)" env RUSTUP_TOOLCHAIN="$MSRV" cargo check --workspace --all-features
else
  skip "rust msrv ($MSRV)" "rustup toolchain install $MSRV"
fi

if have cargo-deny; then
  check "licenses and advisories" cargo deny check
else
  skip "licenses and advisories" "cargo install cargo-deny"
fi

if have typos; then
  check "spelling" typos "$root"
else
  skip "spelling" "cargo install typos-cli, or brew install typos-cli"
fi

check "file length" "$root/tools/check-file-length.sh"

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
