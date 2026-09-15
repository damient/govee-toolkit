#!/usr/bin/env bash
# Runs every check .github/workflows/ci.yml runs that makes sense locally, in
# the same order. Sign-off, commit convention and changelog entry are left out:
# they walk the pull request's commit range, which only exists on the pull
# request.
#
# Kept in step with ci.yml by hand: the workflow is the authority, this is the
# local mirror of it. The reporter and the skip rule are in lib/qa.sh.
#
# A run that passes every check sweeps the artifacts older than its own, with
# tools/clean-target.sh.

set -uo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
rust="$root/packages/rust"
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TERM_COLOR=always
export RUSTFLAGS=${RUSTFLAGS:--D warnings}
# Each check builds the workspace once, so the incremental artifacts are never
# read again. They are gibibytes per run.
export CARGO_INCREMENTAL=${CARGO_INCREMENTAL:-0}

MSRV=$(sed -n 's/^rust-version *= *"\([^"]*\)".*/\1/p' "$rust/Cargo.toml" | head -1)

# shellcheck source=SCRIPTDIR/lib/qa.sh
. "$root/tools/lib/qa.sh"
qa_init govee-qa "${1:-}"

# check <name> <command...> — in the Rust crate.
check() {
  local name=$1
  shift
  check_in "$rust" "$name" "$@"
}

# The sweep at the end removes what is older than this stamp.
"$root/tools/clean-target.sh" --stamp
"$root/tools/clean-target.sh" --stamp --root "$root/packages/python"

if check_fmt_nightly "rust fmt" "$rust"; then
  nightly=yes
else
  nightly=no
fi

check "rust clippy" cargo clippy --workspace --all-targets --all-features -- -D warnings
check "rust test" cargo test --workspace --all-features
# The codec has to keep building with no transport: no socket, no async runtime.
check "codec alone" cargo check --no-default-features
check "rust doc" env RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features
# What docs.rs runs, from `[package.metadata.docs.rs]`. `doc(cfg(...))` is
# nightly-only, so a stable pass above says nothing about the published build.
if [ "$nightly" = yes ]; then
  check "docs.rs build" env RUSTUP_TOOLCHAIN=nightly \
    RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo doc --no-deps --all-features
else
  skip "docs.rs build" "rustup toolchain install nightly"
fi
check "device catalog" cargo run -q -p xtask
check "compatibility tables" cargo run -q -p xtask -- compat --check
check "duplicated command layouts" cargo run -q -p xtask -- dupes

# The MSRV toolchain links every build script with the host's linker. An old
# toolchain next to a new SDK fails there, on the first build script. That
# failure says nothing about the code. `msrv_links` compiles an empty program,
# which separates such a host from a real MSRV break.
msrv_links() {
  local dir status
  dir=$(mktemp -d) || return 1
  printf 'fn main() {}\n' >"$dir/probe.rs"
  RUSTUP_TOOLCHAIN="$MSRV" rustc -o "$dir/probe" "$dir/probe.rs" >/dev/null 2>&1
  status=$?
  rm -rf "$dir"
  return $status
}

# `rust:<version>` carries the toolchain and a linker that agrees with it. The
# checkout is read-only, and the artifacts stay in two named volumes. The run
# writes nothing into the tree, and the next run reuses what this one built.
# `--all-features` needs the D-Bus headers, as it does in ci.yml.
# check_in invokes it through "$@". Two codes cover two shellcheck versions:
# 0.10 reports the function, 0.9 reports the body.
# shellcheck disable=SC2329,SC2317
msrv_in_container() {
  docker run --rm \
    -v "$root:/io:ro" -v govee-msrv-target:/target -v govee-msrv-cargo:/cargo \
    -e CARGO_TARGET_DIR=/target -e CARGO_HOME=/cargo -e CARGO_TERM_COLOR=never \
    -w /io/packages/rust "rust:$MSRV" \
    sh -c 'apt-get update -qq && apt-get install -y -qq libdbus-1-dev >/dev/null &&
           cargo check --workspace --all-features'
}

if [ -z "$MSRV" ]; then
  skip "rust msrv" "no rust-version in packages/rust/Cargo.toml"
elif ! have rustup || ! rustup toolchain list | grep -q "^$MSRV"; then
  skip "rust msrv ($MSRV)" "rustup toolchain install $MSRV"
elif msrv_links; then
  check "rust msrv ($MSRV)" env RUSTUP_TOOLCHAIN="$MSRV" cargo check --workspace --all-features
elif have docker && docker info >/dev/null 2>&1; then
  check_in "$root" "rust msrv ($MSRV)" msrv_in_container
else
  skip "rust msrv ($MSRV)" "the $MSRV toolchain does not link on this host; start docker"
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

# The site and the Python binding each carry a script and a workflow of their
# own. One check here runs one script there, and its summary prints inside this
# one when it fails.
check_script site "$root/tools/qa-site.sh"
check_script python "$root/tools/qa-python.sh"

# Every check above runs through the scripts under tools/. -x follows
# `# shellcheck source=`, which is how lib/qa.sh is read.
if have shellcheck; then
  check_in "$root" "shell lint" shellcheck -x "$root"/tools/*.sh "$root"/tools/lib/*.sh
else
  skip "shell lint" "brew install shellcheck"
fi

# No -ci: a case branch is not indented in this repository.
if have shfmt; then
  check_in "$root" "shell fmt" shfmt -d -i 2 "$root"/tools/*.sh "$root"/tools/lib/*.sh
else
  skip "shell fmt" "brew install shfmt, or go install mvdan.cc/sh/v3/cmd/shfmt@latest"
fi

check "file length" "$root/tools/check-file-length.sh" rust
check "codec layering" "$root/tools/check-no-io.sh"
check "capture redaction" "$root/tools/check-captures.sh"

qa_summary
status=$?

# The sweep runs whether or not a check failed: a failed run builds the same
# artifacts as a run that passes, and a person who iterates on one failure
# fills the disk.
#
# A single-check run ($1 given) leaves the artifacts alone: it built a fraction
# of them, so the sweep would remove what the other checks need.
if [ -z "$only" ]; then
  echo
  "$root/tools/clean-target.sh"
  "$root/tools/clean-target.sh" --root "$root/packages/python"
fi

exit "$status"
