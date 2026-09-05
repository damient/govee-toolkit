#!/usr/bin/env bash
# Checks a release tag against the package it names, and prints the changelog
# section for it — the body of the GitHub release.
#
#   tools/release-notes.sh rust rust-v0.3.0
#
# The tag, the version in the manifest and the changelog heading carry the same
# number. A tag pushed past a manifest nobody bumped would otherwise publish a
# version no changelog describes, so the three are compared before anything is
# built.

set -euo pipefail

usage() {
  echo "usage: ${0##*/} <rust|python|node> <tag>" >&2
  exit 2
}

[ $# -eq 2 ] || usage
pkg=$1 tag=$2
dir=$(cd "$(dirname "$0")/../packages/$pkg" 2>/dev/null && pwd) || usage

case $pkg in
rust) manifest=$dir/Cargo.toml ;;
python) manifest=$dir/pyproject.toml ;;
node) manifest=$dir/package.json ;;
*) usage ;;
esac

case $pkg in
node) pattern='.*"version" *: *"\([^"]*\)".*' ;;
*) pattern='^version *= *"\([^"]*\)".*' ;;
esac

# The Rust manifest carries the number twice, under [package] and under
# [workspace.package]; the first is the published one.
version=$(sed -n "s/$pattern/\1/p" "$manifest" | head -1)
[ -n "$version" ] || {
  echo "error: no version in $manifest" >&2
  exit 1
}

if [ "$tag" != "$pkg-v$version" ]; then
  echo "error: tag $tag against $manifest at $version; expected $pkg-v$version" >&2
  exit 1
fi

notes=$(awk -v want="## [$version]" '
  index($0, want) == 1 { section = 1; next }
  section && /^## / { exit }
  section { print }
' "$dir/CHANGELOG.md")

# A blank section means the heading is there and the entries are not.
if [ -z "$(printf '%s' "$notes" | tr -d '[:space:]')" ]; then
  echo "error: $dir/CHANGELOG.md has no entries under '## [$version]'" >&2
  exit 1
fi

# `$(...)` has dropped the trailing blank lines; this drops the leading ones.
printf '%s\n' "$notes" | sed '/./,$!d'
