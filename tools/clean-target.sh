#!/usr/bin/env bash
# Removes the build artifacts that no later build reads. Cargo has no garbage
# collector for `target`: `-Zgc` collects the registry cache in ~/.cargo and
# not this. cargo-sweep reads the access time of each artifact, so it keeps
# what the last build touched and costs no cold rebuild.
#
# cargo-sweep reads the build artifacts only. It leaves `target/*/incremental/`
# alone, and those session directories are the larger half of the cache, so
# this script removes them itself. It keeps the QA_KEEP_INCREMENTAL most recent
# sessions of each crate (default 1), because that cache is what makes the next
# build after an edit fast. Every older session belongs to a build
# configuration that nothing runs again.
#
# Usage:
#   tools/clean-target.sh --stamp      record the time, before a build
#   tools/clean-target.sh              remove what the stamped build left behind
#   tools/clean-target.sh --maxsize 5G remove the oldest until target fits
#   tools/clean-target.sh --force      full cargo clean, whatever the size
#   tools/clean-target.sh --dry-run    report what a run would remove
#   tools/clean-target.sh --keep-incremental 0   drop every incremental session
#
# Without cargo-sweep the script falls back to a full clean above
# QA_CLEAN_ABOVE_GIB gibibytes (default 5):
#   cargo install cargo-sweep

set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
rust="$root/packages/rust"
export PATH="$HOME/.cargo/bin:$PATH"

above=${QA_CLEAN_ABOVE_GIB:-5}
keep_incremental=${QA_KEEP_INCREMENTAL:-1}
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
  --keep-incremental)
    keep_incremental=${2:-}
    shift
    ;;
  --keep-incremental=*) keep_incremental=${1#*=} ;;
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

case $keep_incremental in
'' | *[!0-9]*)
  echo "$0: --keep-incremental wants a whole number, got: '$keep_incremental'" >&2
  exit 2
  ;;
esac

have_sweep() { cargo sweep --version >/dev/null 2>&1; }

# Removes the incremental sessions, and keeps the newest of each crate. A
# session directory is named `<crate>-<hash>`, where the hash covers the build
# configuration: one crate has one session per configuration, and only the
# configuration a person builds by hand is read again.
prune_incremental() {
  local dir session
  for dir in "$rust"/target/*/incremental; do
    [ -d "$dir" ] || continue
    # ls -t sorts by the last use, newest first.
    ls -t "$dir" | awk -v keep="$keep_incremental" '
      { crate = $0; sub(/-[^-]*$/, "", crate); if (++seen[crate] > keep) print }
    ' | while IFS= read -r session; do
      if [ "$dry_run" = yes ]; then
        printf '%s\n' "$dir/$session"
      else
        rm -rf "${dir:?}/${session:?}"
      fi
    done
  done
}

# cargo-sweep takes a whole number of mebibytes. Accept the K, M and G suffixes
# a person writes, and convert them.
to_mib() {
  local value=$1 number=${1%[KkMmGg]} suffix=${1#"${1%?}"}
  case $value in
  *[!0-9KkMmGg]* | '' | *[0-9]*[KkMmGg]*[0-9]*)
    echo "$0: --maxsize wants a size, such as 5G" >&2
    exit 2
    ;;
  esac
  case $suffix in
  K | k) printf '%d' "$((number / 1024))" ;;
  M | m) printf '%d' "$number" ;;
  G | g) printf '%d' "$((number * 1024))" ;;
  *) printf '%d' "$value" ;;
  esac
}

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

full_clean() {
  if [ "$dry_run" = yes ]; then
    echo "would run: cargo clean"
    exit 0
  fi
  cd "$rust" && exec cargo clean
}

if [ "$mode" = force ]; then
  full_clean
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
  full_clean
fi

sweep=(cargo sweep)
[ "$dry_run" = yes ] && sweep+=(--dry-run)

# Before the sweep, which removes the stamp file the incremental prune reads.
prune_incremental

if [ "$mode" = maxsize ]; then
  if [ -z "$maxsize" ]; then
    echo "$0: --maxsize wants a size, such as 5G" >&2
    exit 2
  fi
  sweep+=(--maxsize "$(to_mib "$maxsize")")
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
