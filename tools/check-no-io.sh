#!/usr/bin/env bash
# Fails when the codec reaches for the network.
#
# `src/codec/` turns devices/*.yaml plus arguments into bytes and does nothing
# else: no socket, no async runtime, no filesystem. That used to be enforced by
# a crate boundary. It is one crate now, so it is enforced here instead — see
# docs/architecture.md.
#
# The list is narrow on purpose. It catches the imports that would make the
# codec-only build (`cargo check --no-default-features`) stop being a codec-only
# build, which is the property this protects.

set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
codec="$root/packages/rust/src/codec"

# std::net       — addresses and sockets
# std::fs        — the catalog is handed to the codec, never read by it
# tokio, socket2 — the transport's dependencies
# std::thread    — the codec is synchronous
banned='std::net|std::fs|std::thread|\btokio\b|\bsocket2\b|async fn|\.await'

status=0
while IFS=: read -r file line text; do
  echo "${file#"$root/"}:$line: $text"
  status=1
done < <(grep -rnE "$banned" "$codec" --include='*.rs' || true)

[ $status -eq 0 ] || cat <<'MSG'

The codec does no I/O. Move this to src/lan/ (or to the transport the mode
needs), or hand the codec the bytes it should work on. See CONTRIBUTING.md.
MSG
exit $status
