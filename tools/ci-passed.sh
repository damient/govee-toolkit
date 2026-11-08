#!/usr/bin/env bash
# Waits for the CI run of a commit to complete, and fails unless it passed.
#
#   tools/ci-passed.sh <sha>
#
# Needs `gh`, GH_TOKEN and GITHUB_REPOSITORY. The job needs `actions: read`.

set -euo pipefail

[ $# -eq 1 ] || {
  echo "usage: ${0##*/} <sha>" >&2
  exit 2
}
sha=$1

TIMEOUT_S=${TIMEOUT_S:-2700}
INTERVAL_S=${INTERVAL_S:-30}
deadline=$((SECONDS + TIMEOUT_S))

# A pull request run tests the merge result, not the commit: count push runs only.
latest() {
  gh api "repos/$GITHUB_REPOSITORY/actions/workflows/ci.yml/runs?head_sha=$sha&event=push" \
    --jq '.workflow_runs | sort_by(.run_number) | last
      | if . == null then "none" else "\(.status) \(.conclusion) \(.html_url)" end'
}

while :; do
  read -r status conclusion url <<<"$(latest)"
  if [ "$status" = completed ]; then
    if [ "$conclusion" = success ]; then
      echo "CI passed on $sha: $url"
      exit 0
    fi
    echo "::error::CI on $sha ended with $conclusion: $url" >&2
    echo "Fix or rerun CI, then rerun this job." >&2
    exit 1
  fi
  if [ "$SECONDS" -ge "$deadline" ]; then
    echo "::error::CI on $sha did not complete in ${TIMEOUT_S}s (${status})" >&2
    [ "$status" = none ] && echo "Tag a commit that was pushed to main." >&2
    exit 1
  fi
  echo "CI on $sha: $status, next check in ${INTERVAL_S}s"
  sleep "$INTERVAL_S"
done
