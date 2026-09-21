#!/usr/bin/env bash
# Fails when a module that does no I/O reaches for the network.
#
# `packages/rust/src/codec/` turns devices/*.yaml plus arguments into bytes and
# does nothing else: no socket, no async runtime, no filesystem. The Rust side
# is one crate, so this script is what enforces that — see
# docs/architecture.md.
#
# `packages/rust/src/profile/` turns a device file into a DMX channel table
# under the same rule, so the bridge, the catalog task and the site read one
# table — see docs/dmx.md.
#
# The list is narrow on purpose. It catches the imports that would make the
# codec-only build (`cargo check --no-default-features`) stop being a codec-only
# build, which is the property this protects.

set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
pure=(
  packages/rust/src/codec
  packages/rust/src/profile
)

# std::net       — addresses and sockets
# std::fs        — the catalog is handed to the module, never read by it
# tokio, socket2 — the transport's dependencies
# std::thread    — these modules are synchronous
banned='std::net|std::fs|std::thread|\btokio\b|\bsocket2\b|async fn|\.await'

status=0
for module in "${pure[@]}"; do
  while IFS=: read -r file line text; do
    echo "${file#"$root/"}:$line: $text"
    status=1
  done < <(grep -rnE "$banned" "$root/$module" --include='*.rs' || true)
done

[ $status -eq 0 ] || cat <<'MSG'

These modules do no I/O. Move this to src/lan/ (or to the transport the mode
needs), or hand the module the bytes it should work on. See CONTRIBUTING.md.
MSG
exit $status
