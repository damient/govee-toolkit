#!/usr/bin/env bash
# Fails when a Rust source file grows past LIMIT lines.
#
# Rust has no conventional file-length limit and rustfmt does not enforce one;
# this is a repository rule, not an ecosystem one. Per-function size is covered
# separately by clippy::too_many_lines.

set -euo pipefail

LIMIT=${LIMIT:-400}
root=$(cd "$(dirname "$0")/.." && pwd)

status=0
while read -r count path; do
  if [ "$count" -gt "$LIMIT" ]; then
    echo "$path: $count lines, over the $LIMIT-line limit"
    status=1
  fi
done < <(cd "$root" && find packages/rust/src packages/rust/tests packages/rust/crates \
  -name '*.rs' -not -path '*/target/*' -exec wc -l {} + |
  awk '$2 != "total" { print $1, $2 }')

[ $status -eq 0 ] || echo "Split the file along its responsibilities; see CONTRIBUTING.md."
exit $status
