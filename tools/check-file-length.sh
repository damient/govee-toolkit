#!/usr/bin/env bash
# Fails when a source file grows past the limit of its language.
#
# Neither Rust nor the web has a conventional file-length limit, and no
# formatter enforces one; this is a repository rule. Per-function size is
# covered separately by clippy::too_many_lines.
#
# The site is held tighter than Rust: a page of CSS or of JavaScript that one
# person reads in one sitting is the unit the site is built from.

set -euo pipefail

RUST_LIMIT=${RUST_LIMIT:-400}
SITE_LIMIT=${SITE_LIMIT:-300}
root=$(cd "$(dirname "$0")/.." && pwd)

# One group, or every group when no argument is given.
group=${1:-all}
case $group in
all | rust | site) ;;
*)
  echo "usage: $(basename "$0") [rust|site]" >&2
  exit 2
  ;;
esac

status=0

# over <limit> <find arguments...>
over() {
  local limit=$1
  shift
  while read -r count path; do
    if [ "$count" -gt "$limit" ]; then
      echo "$path: $count lines, over the $limit-line limit"
      status=1
    fi
  done < <(cd "$root" && find "$@" -exec wc -l {} + |
    awk '$2 != "total" { print $1, $2 }')
}

if [ "$group" != site ]; then
  over "$RUST_LIMIT" packages/rust/src packages/rust/tests packages/rust/crates \
    -name '*.rs' -not -path '*/target/*'
fi

if [ "$group" != rust ]; then
  over "$SITE_LIMIT" site/build.mjs site/lib site/src \
    \( -name '*.mjs' -o -name '*.js' -o -name '*.css' -o -name '*.html' \) \
    -not -path '*/node_modules/*'
fi

[ $status -eq 0 ] || echo "Split the file along its responsibilities; see CONTRIBUTING.md."
exit $status
