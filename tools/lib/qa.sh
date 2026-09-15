# shellcheck shell=bash
#
# The pass/fail reporter that the scripts under tools/ share.
#
# Source this file, call `qa_init <temp-file-label> <only>`, declare the checks
# with `check_in`, `skip` and `have`, and end with `qa_summary`.
#
# A check whose tool is missing is reported as skipped rather than passed,
# because a skip that reads as a pass is how a red CI gets discovered on the
# pull request instead of here. `qa_summary` returns 1 when a check failed and
# 2 when one was skipped, so the caller can exit with what it returns.

names=()
results=()
only=''
log=''

# qa_init <label> <only> — <label> names the temporary log, <only> filters the
# checks by substring and is empty for a full run.
qa_init() {
  log=$(mktemp -t "$1")
  only=${2:-}
  trap 'rm -f "$log"' EXIT
}

# record <name> <state>, where state is pass, fail or skip.
record() {
  names+=("$1")
  results+=("$2")
  case $2 in
  pass) printf '  ok\n' ;;
  skip) printf '  skipped: %s\n' "${3:-}" ;;
  fail)
    printf '  FAILED\n'
    sed 's/^/  | /' "$log"
    ;;
  esac
}

# check_in <dir> <name> <command...> — runs the command in the directory, with
# output captured, so a passing check stays quiet and a failing one prints its
# log.
check_in() {
  local dir=$1 name=$2
  shift 2
  if [ -n "$only" ] && [[ $name != *"$only"* ]]; then return; fi
  printf '%s\n' "$name"
  if (cd "$dir" && "$@") >"$log" 2>&1; then
    record "$name" pass
  else
    record "$name" fail
  fi
}

# check_script <name> <path> — run another qa script as one check. It exits 2
# when it skipped a check of its own, which is a skip here as well.
check_script() {
  local name=$1 script=$2
  if [ -n "$only" ] && [[ $name != *"$only"* ]]; then return; fi
  printf '%s\n' "$name"
  "$script" >"$log" 2>&1
  case $? in
  0) record "$name" pass ;;
  2) record "$name" skip "a $name check was skipped; run $script" ;;
  *) record "$name" fail ;;
  esac
}

# skip <name> <what is missing>
skip() {
  if [ -n "$only" ] && [[ $1 != *"$only"* ]]; then return; fi
  printf '%s\n' "$1"
  record "$1" skip "$2"
}

have() { command -v "$1" >/dev/null 2>&1; }

# check_fmt_nightly <name> <dir> — cargo fmt in the directory, under nightly.
# rustfmt.toml uses nightly-only options; stable rustfmt formats differently.
# Returns 0 when nightly is here, so a caller that needs it for another check
# reads the answer from this one.
check_fmt_nightly() {
  if have rustup && rustup toolchain list | grep -q '^nightly'; then
    check_in "$2" "$1" env RUSTUP_TOOLCHAIN=nightly cargo fmt --all --check
    return 0
  fi
  skip "$1" "rustup toolchain install nightly"
  return 1
}

# Print the summary. Returns 1 if a check failed, 2 if one was skipped.
qa_summary() {
  local failed=0 skipped=0 i
  echo
  printf '%s\n' "-- summary"
  for i in "${!names[@]}"; do
    case ${results[$i]} in
    pass) printf 'pass  %s\n' "${names[$i]}" ;;
    fail)
      printf 'FAIL  %s\n' "${names[$i]}"
      failed=$((failed + 1))
      ;;
    skip)
      printf 'skip  %s\n' "${names[$i]}"
      skipped=$((skipped + 1))
      ;;
    esac
  done
  printf '%d failed, %d skipped, %d total\n' "$failed" "$skipped" "${#names[@]}"

  [ "$failed" -eq 0 ] || return 1
  [ "$skipped" -eq 0 ] || return 2
}
