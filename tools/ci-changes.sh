#!/usr/bin/env bash
# Reads the changed paths of a pull request on stdin, one per line, and prints
# one `<zone>=true|false` line per CI zone, for $GITHUB_OUTPUT.
#
# A path that matches no rule sets every zone: an unknown path runs the full
# pipeline. Add a rule before a path is left out of a zone it can break.

set -euo pipefail

core=false python=false node=false mcp=false docs=false

while IFS= read -r path; do
  case $path in
  # A changelog ships in a release note and is checked by no build.
  CHANGELOG.md | */CHANGELOG.md) ;;
  # Every package embeds devices/ and builds on the core.
  packages/rust/* | devices/* | tests/*) core=true ;;
  # `xtask api` joins the Python and Node signatures for the MCP server.
  packages/python/*) python=true mcp=true ;;
  # The Python job lints these with ruff.
  site/tools/* | tools/*.py) python=true ;;
  packages/node/*) node=true mcp=true ;;
  integrations/mcp/* | tools/qa-mcp.sh | tools/lib/*) mcp=true ;;
  # The MCP server ships docs/ and serves it through `read_doc`.
  docs/*) docs=true mcp=true ;;
  # A change to the mapping itself takes every zone.
  tools/ci-changes.sh) core=true python=true node=true mcp=true docs=true ;;
  # The jobs that always run cover these, or pages.yml builds them.
  site/* | apps/* | social/* | tools/*.sh | tools/README.md) ;;
  *.md | LICENSE | typos.toml | .github/dependabot.yml) ;;
  .github/workflows/pages.yml | .github/workflows/*-release.yml) ;;
  *) core=true python=true node=true mcp=true docs=true ;;
  esac
done

printf '%s\n' "core=$core" "python=$python" "node=$node" "mcp=$mcp" "docs=$docs"
