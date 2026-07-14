#!/bin/bash
# ABOUTME: Repeats the exact lifecycle smoke serially and concurrently under owned deadlines.
# ABOUTME: Each child owns disposable tmux/Zellij state and must clean it before PASS.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
ROOT="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-layout-stress.XXXXXX")"
TIMEOUT_SECS="${ZAPHOD_LAYOUT_STRESS_TIMEOUT_SECS:-180}"
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

CARGO_TARGET_DIR="$REPO_ROOT/target" "$REPO_ROOT/build.sh" >/dev/null
cargo build --quiet --manifest-path "$REPO_ROOT/Cargo.toml" \
    --target-dir "$REPO_ROOT/target" \
    --features host-kdl-validator --bin zaphod-kdl-validate

run_owned_case() {
    local name="$1"
    local expected="$2"
    shift 2
    local child watchdog status
    perl -MPOSIX -e 'defined POSIX::setsid() or die "setsid failed: $!"; exec @ARGV or die "exec failed: $!"' \
        "$@" \
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
    grep -F "$expected" "$ROOT/$name.out" >/dev/null || return 1
    printf 'PASS: %s native stress case\n' "$name"
}

for round in $(seq 1 "$SERIAL_ROUNDS"); do
    run_owned_case "serial-$round" \
        'PASS: outside foreground diagnosis and inside automatic subscriber handoff both completed' \
        env ZAPHOD_SMOKE_PREBUILT_ARTIFACTS=1 \
        "$SCRIPT_DIR/zellij-subscription-lifecycle-smoke-test.sh" ||
        fail "serial lifecycle stress round $round failed"
done

run_owned_case concurrent-foreground \
    'PASS: outside caller with foreground target/zaphod subscribe' \
    env -u ZELLIJ -u ZELLIJ_SESSION_NAME -u ZELLIJ_PANE_ID \
    ZAPHOD_SMOKE_PREBUILT_ARTIFACTS=1 \
    ZAPHOD_CALLER_ENV=outside ZAPHOD_SUBSCRIBER_MODE=foreground \
    "$SCRIPT_DIR/zellij-tmux-smoke-test.sh" &
pid_a=$!
run_owned_case concurrent-automatic \
    'PASS: inside caller with automatic target/zaphod subscribe' \
    env ZELLIJ=0 ZELLIJ_SESSION_NAME=ambient-work ZELLIJ_PANE_ID=98765 \
    ZAPHOD_SMOKE_PREBUILT_ARTIFACTS=1 \
    ZAPHOD_CALLER_ENV=inside ZAPHOD_SUBSCRIBER_MODE=automatic \
    "$SCRIPT_DIR/zellij-tmux-smoke-test.sh" &
pid_b=$!
set +e
wait "$pid_a"
status_a=$?
wait "$pid_b"
status_b=$?
set -e
[ "$status_a" -eq 0 ] || fail "concurrent foreground stress failed"
[ "$status_b" -eq 0 ] || fail "concurrent automatic stress failed"

echo "PASS: repeated serial and concurrent layout lifecycle stress completed"
