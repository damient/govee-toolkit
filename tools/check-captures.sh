#!/usr/bin/env bash
# Fails when a tracked file under tests/fixtures/ or devices/ carries something
# that should have been redacted: a real MAC, a routable IPv4 address, a
# credential, or a Wi-Fi network name. The placeholders it accepts are the ones
# listed in tests/fixtures/README.md.
#
# Git keeps a leaked capture after the fix, so the check runs before the commit
# rather than on the pull request. The patterns are deliberately narrow: a false
# positive that blocks a legitimate capture costs more than a miss.

set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

status=0

report() {
  echo "$1:$2: $3"
  status=1
}

# NUL-separated: a path with a space in it must be checked, not skipped.
while IFS= read -r -d '' path; do
  [ -f "$path" ] || continue

  # A MAC-shaped run of exactly six hex pairs. A longer run is a hex dump of a
  # frame, not an address, and grep -o takes the longest run, so those drop out
  # on the colon count.
  while read -r out; do
    line=${out%%:*}
    match=${out#*:}
    colons=${match//[^:]/}
    [ ${#colons} -eq 5 ] || continue
    case $(printf '%s' "$match" | tr 'a-f' 'A-F') in
    AA:BB:CC:DD:EE:FF | 11:22:33:44:55:66 | 99:88:77:66:55:44 | 00:00:00:00:00:00 | FF:FF:FF:FF:FF:FF) continue ;;
    esac
    report "$path" "$line" "MAC address $match, not a documented placeholder"
  done < <(grep -aonE '([0-9A-Fa-f]{2}:)+[0-9A-Fa-f]{2}' "$path" || true)

  # An IPv4 address outside the RFC 5737 documentation ranges, loopback, the
  # unspecified and broadcast addresses, and the discovery multicast group the
  # protocol itself uses. The octet patterns reject anything over 255 and
  # anything with a leading zero, which is what keeps four-part firmware
  # versions out; a line naming a version field is skipped outright.
  while read -r out; do
    line=${out%%:*}
    match=${out#*:}
    dots=${match//[^.]/}
    [ ${#dots} -eq 3 ] || continue
    ok=1
    for octet in ${match//./ }; do
      case $octet in
      0 | [1-9] | [1-9][0-9] | 1[0-9][0-9] | 2[0-4][0-9] | 25[0-5]) ;;
      *) ok=0 ;;
      esac
    done
    [ $ok -eq 1 ] || continue
    case $match in
    127.* | 192.0.2.* | 198.51.100.* | 203.0.113.* | 0.0.0.0 | 255.255.255.255 | 239.255.255.250) continue ;;
    esac
    case $(sed -n "${line}p" "$path") in
    *ersion*) continue ;;
    esac
    report "$path" "$line" "IPv4 address $match, use the 192.0.2.0/24 documentation range"
  done < <(grep -aonE '([0-9]{1,3}\.)+[0-9]{1,3}' "$path" || true)

  # A bearer token, a key-shaped assignment carrying a value, or a UUID — the
  # shape a Govee API key has. A value already reading REDACTED is the redacted
  # form and passes.
  while read -r out; do
    match=${out#*:}
    case $match in *REDACTED*) continue ;; esac
    report "$path" "${out%%:*}" "looks like a credential, replace the value with REDACTED"
  done < <(grep -aonEi 'bearer[[:space:]]+[A-Za-z0-9._-]{16,}|(api[_-]?key|auth[_-]?token|access[_-]?token|authorization)"?[[:space:]]*[:=][[:space:]]*"?[A-Za-z0-9._-]{8,}|[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' "$path" || true)

  # An ssid or bssid key carrying a value. An empty value never matches: the
  # value has to start with an alphanumeric. The key has to be a whole word, so
  # that a word merely ending in ssid does not trip it.
  while read -r out; do
    match=${out#*:}
    case $match in *EXAMPLE-SSID* | *AA:BB:CC:DD:EE:FF* | *REDACTED*) continue ;; esac
    report "$path" "${out%%:*}" "ssid or bssid with a value, use EXAMPLE-SSID or AA:BB:CC:DD:EE:FF"
  done < <(grep -aonEi '(^|[^A-Za-z0-9])b?ssid"?[[:space:]]*[:=][[:space:]]*"?[A-Za-z0-9][^",}]*' "$path" || true)
done < <(git ls-files -z tests/fixtures devices)

[ $status -eq 0 ] || echo "Redact the capture before committing; see tests/fixtures/README.md."
exit $status
