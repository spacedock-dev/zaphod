#!/bin/bash
# ABOUTME: Repeats the exact lifecycle smoke serially and concurrently under owned deadlines.
# ABOUTME: Each child owns disposable tmux/Zellij state and must clean it before PASS.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
ROOT="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-layout-stress.XXXXXX")"
TIMEOUT_SECS="${ZAPHOD_LAYOUT_STRESS_TIMEOUT_SECS:-90}"
SERIAL_ROUNDS="${ZAPHOD_LAYOUT_STRESS_SERIAL_ROUNDS:-2}"
cleanup() {
    rm -rf "$ROOT"
}
trap cleanup EXIT

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

[[ "$TIMEOUT_SECS" =~ ^[1-9][0-9]*$ ]] || fail "stress timeout must be a positive integer"
[[ "$SERIAL_ROUNDS" =~ ^[1-9][0-9]*$ ]] || fail "serial rounds must be a positive integer"
command -v perl >/dev/null 2>&1 || fail "perl is required for owned stress process groups"

run_case() {
    local name="$1"
    local child watchdog status
    perl -MPOSIX -e 'defined POSIX::setsid() or die "setsid failed: $!"; exec @ARGV or die "exec failed: $!"' \
        "$SCRIPT_DIR/zellij-subscription-lifecycle-smoke-test.sh" \
        > "$ROOT/$name.out" 2> "$ROOT/$name.err" &
    child=$!
    (
        sleep "$TIMEOUT_SECS"
        /bin/kill -TERM "-$child" 2>/dev/null || exit 0
        sleep 5
        /bin/kill -KILL "-$child" 2>/dev/null || true
    ) &
    watchdog=$!
    set +e
    wait "$child"
    status=$?
    set -e
    kill "$watchdog" 2>/dev/null || true
    wait "$watchdog" 2>/dev/null || true
    if [ "$status" -ne 0 ]; then
        sed -n '1,120p' "$ROOT/$name.out" >&2 || true
        sed -n '1,160p' "$ROOT/$name.err" >&2 || true
        return "$status"
    fi
    grep -F 'PASS: outside foreground diagnosis and inside automatic subscriber handoff both completed' \
        "$ROOT/$name.out" >/dev/null || return 1
    printf 'PASS: %s lifecycle stress case\n' "$name"
}

for round in $(seq 1 "$SERIAL_ROUNDS"); do
    run_case "serial-$round" || fail "serial lifecycle stress round $round failed"
done

run_case concurrent-a &
pid_a=$!
run_case concurrent-b &
pid_b=$!
set +e
wait "$pid_a"
status_a=$?
wait "$pid_b"
status_b=$?
set -e
[ "$status_a" -eq 0 ] || fail "concurrent lifecycle stress A failed"
[ "$status_b" -eq 0 ] || fail "concurrent lifecycle stress B failed"

echo "PASS: repeated serial and concurrent layout lifecycle stress completed"
