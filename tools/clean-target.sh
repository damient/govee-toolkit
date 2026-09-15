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
# cargo-sweep also keeps every artifact of an installed toolchain, so it never
# removes these three, and the script removes them itself:
#   - the copies of one artifact. Cargo names a copy `<name>-<hash>`, where the
#     hash covers the build configuration. A build with another feature set
#     writes a new copy and cargo removes none. The script keeps the
#     QA_KEEP_COPIES most recent copies of each artifact (default 2).
#   - the examples that lost their source file. Cargo removes no artifact of a
#     deleted example.
#   - the `.rcgu.o` object files that a link step left behind.
#
# Usage:
#   tools/clean-target.sh --stamp      record the time, before a build
#   tools/clean-target.sh              remove what the stamped build left behind
#   tools/clean-target.sh --maxsize 5G remove the oldest until target fits
#   tools/clean-target.sh --force      full cargo clean, whatever the size
#   tools/clean-target.sh --dry-run    report what a run would remove
#   tools/clean-target.sh --keep-incremental 0   drop every incremental session
#   tools/clean-target.sh --keep-copies 3        keep 3 copies of each artifact
#   tools/clean-target.sh --root packages/python sweep another cargo workspace
#
# Without cargo-sweep the script falls back to a full clean above
# QA_CLEAN_ABOVE_GIB gibibytes (default 5):
#   cargo install cargo-sweep

set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
# The workspace to sweep. One run sweeps one target directory.
workspace="$root/packages/rust"
export PATH="$HOME/.cargo/bin:$PATH"

above=${QA_CLEAN_ABOVE_GIB:-5}
keep_incremental=${QA_KEEP_INCREMENTAL:-1}
keep_copies=${QA_KEEP_COPIES:-2}
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
  --keep-copies)
    keep_copies=${2:-}
    shift
    ;;
  --keep-copies=*) keep_copies=${1#*=} ;;
  --root)
    workspace=$2
    shift
    ;;
  --root=*) workspace=${1#*=} ;;
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

case $keep_copies in
'' | *[!0-9]*)
  echo "$0: --keep-copies wants a whole number, got: '$keep_copies'" >&2
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
  for dir in "$workspace"/target/*/incremental; do
    [ -d "$dir" ] || continue
    # ls -t sorts by the last use, newest first.
    # shellcheck disable=SC2012 # cargo writes these names; find cannot sort by time.
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

# The example names a workspace member declares. Cargo writes an underscore
# where a name carries a dash.
example_names() {
  local file name
  for file in "$workspace"/examples/*.rs "$workspace"/examples/*/main.rs \
    "$workspace"/crates/*/examples/*.rs "$workspace"/crates/*/examples/*/main.rs; do
    [ -e "$file" ] || continue
    name=${file##*/}
    if [ "$name" = main.rs ]; then
      name=${file%/main.rs}
      name=${name##*/}
    else
      name=${name%.rs}
    fi
    printf '%s ' "${name//-/_}"
  done
  # A manifest can name an example whose source sits somewhere else.
  awk -F'"' '
    /^\[\[example\]\]/ { in_example = 1; next }
    /^\[/ { in_example = 0 }
    in_example && /^name *=/ { gsub(/-/, "_", $2); printf "%s ", $2 }
  ' "$workspace"/Cargo.toml "$workspace"/crates/*/Cargo.toml 2>/dev/null
}

# Prints the names to remove in the directory $1, and keeps the $2 most recent
# copies of each artifact. $3 lists the example names; a stem that the list does
# not carry has lost its source and goes, whatever its age. Pass `-` for $3 to
# skip that test, which is what the dependency directory wants.
list_stale_copies() {
  # shellcheck disable=SC2012 # cargo writes these names; find cannot sort by time.
  ls -t "$1" 2>/dev/null | awk -v keep="$2" -v sources="$3" '
    BEGIN {
      check = (sources != "-")
      count = split(sources, list, " ")
      for (i = 1; i <= count; i++) have[list[i]] = 1
    }
    {
      name = $0
      dot = index(name, ".")
      head = dot ? substr(name, 1, dot - 1) : name
      stem = head
      hash = ""
      for (i = length(head); i > 0; i--)
        if (substr(head, i, 1) == "-") break
      if (i > 0) {
        tail = substr(head, i + 1)
        if (length(tail) == 16 && tail ~ /^[0-9a-f]+$/) {
          stem = substr(head, 1, i - 1)
          hash = tail
        }
      }
      if (check) {
        source = stem
        sub(/^lib/, "", source)
        if (!(source in have)) { print name; next }
      }
      # The copy that carries no hash is the one the last build published.
      if (hash == "") next
      key = stem "\t" hash
      if (!(key in rank)) rank[key] = ++copies[stem]
      if (rank[key] > keep) print name
    }'
}

# Removes the stale copies from the directory $1 of every profile.
prune_copies() {
  local subdir=$1 sources=$2 dir name
  for dir in "$workspace"/target/*/"$subdir"; do
    [ -d "$dir" ] || continue
    list_stale_copies "$dir" "$keep_copies" "$sources" | while IFS= read -r name; do
      if [ "$dry_run" = yes ]; then
        printf '%s\n' "$dir/$name"
      else
        rm -rf "${dir:?}/${name:?}"
      fi
    done
  done
}

# rustc writes one `.rcgu.o` per codegen unit next to the binary, and leaves
# them there when the link does not finish.
prune_stray_objects() {
  local dir file name base
  for dir in "$workspace"/target/*/examples "$workspace"/target/*/deps; do
    [ -d "$dir" ] || continue
    for file in "$dir"/*.rcgu.o; do
      [ -e "$file" ] || continue
      name=${file##*/}
      base=${name%%.*}
      [ -e "$dir/$base" ] && continue
      if [ "$dry_run" = yes ]; then
        printf '%s\n' "$file"
      else
        rm -f "$file"
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
    cargo sweep --stamp "$workspace" >/dev/null
  fi
  exit 0
fi

if [ ! -d "$workspace/target" ]; then
  echo "target is already gone"
  exit 0
fi

size() { printf '%d' "$(($(du -sk "$workspace/target" | cut -f1) / 1024))"; }
before=$(size)
printf 'target: %d MiB, %s files\n' "$before" \
  "$(find "$workspace/target" -type f | wc -l | tr -d ' ')"

report() {
  local after
  [ "$dry_run" = yes ] && return 0
  after=$(size)
  printf 'target: %d MiB, %d MiB freed\n' "$after" "$((before - after))"
}

full_clean() {
  if [ "$dry_run" = yes ]; then
    echo "would run: cargo clean"
    exit 0
  fi
  cd "$workspace" && exec cargo clean
}

if [ "$mode" = force ]; then
  full_clean
fi

# cargo-sweep keeps these as artifacts of an installed toolchain, so no sweep
# option reaches them and all three run in every mode.
prune_copies examples "$(example_names)"
prune_copies deps -
prune_stray_objects

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
  if [ "$(($(size) / 1024))" -le "$above" ]; then
    printf 'at or below the %s GiB threshold, keeping the cache\n' "$above"
    report
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
elif [ -f "$workspace/sweep.timestamp" ]; then
  # Everything the stamped build did not touch.
  sweep+=(--file)
else
  # No stamp: keep what the installed toolchains built, drop the rest.
  echo "no sweep.timestamp, keeping the artifacts of the installed toolchains"
  sweep+=(--installed)
fi

"${sweep[@]}" "$workspace"

report
