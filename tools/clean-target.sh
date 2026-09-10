#!/usr/bin/env bash
# Removes the build artifacts that no later build reads.
#
# cargo keeps the artifacts of every earlier build: an older version of a
# dependency, an earlier set of features, an earlier hash of the same example.
# Nothing collects them, so `target` grows without a bound. One week of probe
# runs left 388054 files and 50 GiB, and most of it was hash-suffixed example
# binaries under target/debug/examples/. Cargo has no garbage collector for
# `target`: `-Zgc` collects the registry cache in ~/.cargo and not this.
#
# cargo-sweep does have one. It reads the access time of each artifact, so it
# removes the stale ones and keeps the artifacts that the last build touched.
# That needs no threshold and costs no cold rebuild.
#
# Usage:
#   tools/clean-target.sh --stamp      record the time, before a build
#   tools/clean-target.sh              remove what the stamped build left behind
#   tools/clean-target.sh --maxsize 5G remove the oldest until target fits
#   tools/clean-target.sh --force      full cargo clean, whatever the size
#   tools/clean-target.sh --dry-run    report what a run would remove
#
# tools/qa.sh stamps before the first check and sweeps after the last one, so a
# passing run leaves the artifacts of that run and nothing older.
#
# Without cargo-sweep the script falls back to a full clean above
# QA_CLEAN_ABOVE_GIB gibibytes (default 5), because the alternative is to let
# the directory grow. Install the tool for the cheaper behaviour:
#   cargo install cargo-sweep

set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
rust="$root/packages/rust"
export PATH="$HOME/.cargo/bin:$PATH"

above=${QA_CLEAN_ABOVE_GIB:-5}
mode=sweep
maxsize=
dry_run=no

while [ $# -gt 0 ]; do
  case $1 in
  --stamp | -s) mode=stamp ;;
  --force | -f) mode=force ;;
  --dry-run | -n) dry_run=yes ;;
  --maxsize | -m)
    mode=maxsize
    maxsize=${2:-}
    shift
    ;;
  --maxsize=*)
    mode=maxsize
    maxsize=${1#*=}
    ;;
  --above)
    above=${2:-}
    shift
    ;;
  --above=*) above=${1#*=} ;;
  -h | --help)
    awk 'NR == 1 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "$0"
    exit 0
    ;;
  *)
    echo "$0: unknown argument: $1" >&2
    exit 2
    ;;
  esac
  shift
done

have_sweep() { cargo sweep --version >/dev/null 2>&1; }

# The stamp is written before a build, so it runs whether or not target exists.
if [ "$mode" = stamp ]; then
  if have_sweep; then
    cargo sweep --stamp "$rust" >/dev/null
  fi
  exit 0
fi

if [ ! -d "$rust/target" ]; then
  echo "target is already gone"
  exit 0
fi

size() { printf '%d' "$(($(du -sk "$rust/target" | cut -f1) / 1024))"; }
before=$(size)
printf 'target: %d MiB, %s files\n' "$before" \
  "$(find "$rust/target" -type f | wc -l | tr -d ' ')"

if [ "$mode" = force ]; then
  if [ "$dry_run" = yes ]; then
    echo "would run: cargo clean"
    exit 0
  fi
  cd "$rust" && exec cargo clean
fi

if ! have_sweep; then
  echo "cargo-sweep is missing, falling back to a full clean above the threshold"
  echo "  install it for a sweep that keeps the last build: cargo install cargo-sweep"
  case $above in
  '' | *[!0-9]*)
    echo "$0: --above wants a whole number of GiB, got: '$above'" >&2
    exit 2
    ;;
  esac
  if [ "$above" = 0 ]; then
    echo "threshold disabled (QA_CLEAN_ABOVE_GIB=0), keeping it"
    exit 0
  fi
  if [ "$((before / 1024))" -le "$above" ]; then
    printf 'at or below the %s GiB threshold, keeping the cache\n' "$above"
    exit 0
  fi
  if [ "$dry_run" = yes ]; then
    echo "would run: cargo clean"
    exit 0
  fi
  cd "$rust" && exec cargo clean
fi

sweep=(cargo sweep)
[ "$dry_run" = yes ] && sweep+=(--dry-run)

if [ "$mode" = maxsize ]; then
  if [ -z "$maxsize" ]; then
    echo "$0: --maxsize wants a size, such as 5G" >&2
    exit 2
  fi
  sweep+=(--maxsize "$maxsize")
elif [ -f "$rust/sweep.timestamp" ]; then
  # Everything the stamped build did not touch.
  sweep+=(--file)
else
  # No stamp: keep what the installed toolchains built, drop the rest.
  echo "no sweep.timestamp, keeping the artifacts of the installed toolchains"
  sweep+=(--installed)
fi

"${sweep[@]}" "$rust"

if [ "$dry_run" = no ]; then
  after=$(size)
  printf 'target: %d MiB, %d MiB freed\n' "$after" "$((before - after))"
fi
