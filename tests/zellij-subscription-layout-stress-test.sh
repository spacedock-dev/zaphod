#!/bin/bash
# ABOUTME: Repeats the exact lifecycle smoke serially and concurrently under owned deadlines.
# ABOUTME: Each child owns disposable tmux/Zellij state and must clean it before PASS.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
ROOT="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-layout-stress.XXXXXX")"
EVIDENCE_DIR="${ZAPHOD_LAYOUT_STRESS_EVIDENCE_DIR:-}"
INJECT_FAILURE_PHASE="${ZAPHOD_LAYOUT_STRESS_INJECT_FAILURE_PHASE:-}"
TIMEOUT_SECS="${ZAPHOD_LAYOUT_STRESS_TIMEOUT_SECS:-180}"
SERIAL_ROUNDS="${ZAPHOD_LAYOUT_STRESS_SERIAL_ROUNDS:-2}"
if [ -z "$EVIDENCE_DIR" ]; then
    EVIDENCE_DIR="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-layout-stress-evidence.XXXXXX")"
else
    mkdir -p "$EVIDENCE_DIR"
fi
cleanup() {
    local original_status=$?
    trap - EXIT INT TERM HUP
    rm -rf "$ROOT"
    if [ -f "$EVIDENCE_DIR/bundle-manifest.txt" ]; then
        printf 'stress_status=%s\nstress_root=%s\nstress_root_exists_after=%s\n' \
            "$original_status" "$ROOT" "$([ -e "$ROOT" ] && echo 1 || echo 0)" \
            > "$EVIDENCE_DIR/stress-cleanup.txt"
    else
        rm -rf "$EVIDENCE_DIR"
    fi
    exit "$original_status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP

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

retain_failure_bundle() {
    local name="$1"
    local status="$2"
    local expected="$3"
    local case_evidence="$EVIDENCE_DIR/$name"
    mkdir -p "$case_evidence"
    cp "$ROOT/$name.out" "$case_evidence/case.stdout" 2>/dev/null || : > "$case_evidence/case.stdout"
    cp "$ROOT/$name.err" "$case_evidence/case.stderr" 2>/dev/null || : > "$case_evidence/case.stderr"
    {
        printf 'case=%s\nstatus=%s\nexpected=%s\n' "$name" "$status" "$expected"
        printf 'stress_pid=%s\nroot=%s\ntimeout_secs=%s\n' "$$" "$ROOT" "$TIMEOUT_SECS"
        printf 'injected_failure_phase=%s\n' "$INJECT_FAILURE_PHASE"
    } >> "$EVIDENCE_DIR/bundle-manifest.txt"
    printf 'retained failure evidence: %s\n' "$EVIDENCE_DIR" >&2
}

run_owned_case() {
    local name="$1"
    local expected="$2"
    shift 2
    local child watchdog status
    local case_evidence="$EVIDENCE_DIR/$name"
    rm -rf "$case_evidence"
    mkdir -p "$case_evidence"
    perl -MPOSIX -e 'defined POSIX::setsid() or die "setsid failed: $!"; exec @ARGV or die "exec failed: $!"' \
        env \
        ZAPHOD_LIFECYCLE_EVIDENCE_DIR="$case_evidence" \
        ZAPHOD_LIFECYCLE_INJECT_FAILURE_PHASE="$INJECT_FAILURE_PHASE" \
        ZAPHOD_SMOKE_EVIDENCE_DIR="$case_evidence" \
        ZAPHOD_SMOKE_INJECT_FAILURE_PHASE="$INJECT_FAILURE_PHASE" \
        "$@" \
        > "$ROOT/$name.out" 2> "$ROOT/$name.err" &
    child=$!
    (
        sleep "$TIMEOUT_SECS"
        /bin/kill -TERM "-$child" 2>/dev/null || exit 0
        sleep 15
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
        retain_failure_bundle "$name" "$status" "$expected"
        sed -n '1,120p' "$ROOT/$name.out" >&2 || true
        sed -n '1,160p' "$ROOT/$name.err" >&2 || true
        return "$status"
    fi
    if ! grep -F "$expected" "$ROOT/$name.out" >/dev/null; then
        retain_failure_bundle "$name" 1 "$expected"
        return 1
    fi
    rm -rf "$case_evidence"
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
