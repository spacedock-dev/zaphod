#!/bin/bash
# ABOUTME: Proves a forced native lifecycle failure retains bounded evidence after live cleanup.
# ABOUTME: The retained bundle must identify the phase, owned processes, native state, and cleanup result.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
ROOT="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-stress-evidence-test.XXXXXX")"
EVIDENCE="$ROOT/evidence"
OUT="$ROOT/stress.out"
ERR="$ROOT/stress.err"
trap 'rm -rf "$ROOT"' EXIT

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

file_state() {
    local path="$1"
    if [ -e "$path" ]; then
        printf 'present:%s\n' "$(shasum -a 256 "$path" | awk '{print $1}')"
    else
        printf 'missing\n'
    fi
}

assert_cleanup_proven() {
    local result="$1"
    local label="$2"
    grep -F 'cleanup_status=0' "$result" >/dev/null || fail "$label cleanup reported an error"
    grep -F 'session_probe_status_after=1' "$result" >/dev/null ||
        fail "$label cleanup omitted the native absent-session probe status"
    grep -F 'session_absence_confirmed=1' "$result" >/dev/null ||
        fail "$label cleanup did not prove native session absence"
    grep -F 'tmux_probe_status_after=1' "$result" >/dev/null ||
        fail "$label cleanup omitted the absent tmux-server probe status"
    grep -F 'tmux_absence_confirmed=1' "$result" >/dev/null ||
        fail "$label cleanup did not prove tmux server absence"
    grep -F 'tmux_probe_command=list-sessions' "$result" >/dev/null ||
        fail "$label cleanup used only a named-session probe"
    grep -F 'tmux_server_unreachable_after=1' "$result" >/dev/null ||
        fail "$label cleanup did not prove the dedicated tmux server unreachable"
    grep -F 'tmux_server_pid_alive_after=0' "$result" >/dev/null ||
        fail "$label cleanup did not prove the dedicated tmux server PID exited"
    grep -F 'tmux_absence_basis=native-unreachable+pid-exited' "$result" >/dev/null ||
        fail "$label cleanup conflated socket remediation with server-exit proof"
    grep -F 'tmux_socket_absent_after=1' "$result" >/dev/null ||
        fail "$label cleanup did not prove the dedicated tmux socket disappeared"
}

STANDING_ROOT="${ZELLIJ_CONFIG_DIR:-$HOME/.config/zellij}"
STANDING_CONFIG="${ZELLIJ_CONFIG_FILE:-$STANDING_ROOT/config.kdl}"
STANDING_LAYOUT="$STANDING_ROOT/layouts/zaphod.kdl"
CONFIG_BEFORE="$(file_state "$STANDING_CONFIG")"
LAYOUT_BEFORE="$(file_state "$STANDING_LAYOUT")"

set +e
ZAPHOD_LAYOUT_STRESS_EVIDENCE_DIR="$EVIDENCE" \
ZAPHOD_LAYOUT_STRESS_INJECT_FAILURE_PHASE=tmux-zellij-launched \
ZAPHOD_SMOKE_INJECT_FAILURE_PAYLOAD_BYTES=131072 \
ZAPHOD_LAYOUT_STRESS_TIMEOUT_SECS=30 \
ZAPHOD_LAYOUT_STRESS_SERIAL_ROUNDS=1 \
    "$SCRIPT_DIR/zellij-watcher-layout-stress-test.sh" > "$OUT" 2> "$ERR"
status=$?
set -e

[ "$status" -ne 0 ] || fail "injected lifecycle failure unexpectedly passed"
grep -F "retained failure evidence: $EVIDENCE" "$ERR" >/dev/null ||
    fail "stress failure did not report its retained evidence path"
[ -f "$EVIDENCE/bundle-manifest.txt" ] || fail "failure bundle omitted its manifest"
[ -f "$EVIDENCE/serial-1/case.stdout" ] || fail "failure bundle omitted case stdout"
[ -f "$EVIDENCE/serial-1/case.stderr" ] || fail "failure bundle omitted case stderr"
[ -f "$EVIDENCE/serial-1/lifecycle-phases.log" ] || fail "failure bundle omitted lifecycle phases"
[ -f "$EVIDENCE/serial-1/outside-manual/phase.log" ] || fail "failure bundle omitted smoke phases"
[ -f "$EVIDENCE/serial-1/outside-manual/process-ownership.txt" ] ||
    fail "failure bundle omitted process ownership"
[ -f "$EVIDENCE/serial-1/outside-manual/tmux-pane.txt" ] ||
    fail "failure bundle omitted bounded tmux pane evidence"
[ -d "$EVIDENCE/serial-1/outside-manual/native" ] ||
    fail "failure bundle omitted bounded native replies"
[ -f "$EVIDENCE/serial-1/outside-manual/cleanup-result.txt" ] ||
    fail "failure bundle omitted smoke cleanup result"
[ -f "$EVIDENCE/stress-cleanup.txt" ] || fail "failure bundle omitted stress cleanup result"
[ "$(wc -c < "$EVIDENCE/serial-1/case.stderr" | tr -d '[:space:]')" -le 65536 ] ||
    fail "failure bundle retained unbounded case stderr"
grep -E '^case_stderr_len=1[3-9][0-9]{4,}$' "$EVIDENCE/bundle-manifest.txt" >/dev/null ||
    fail "failure bundle omitted original oversized stderr length"
grep -F 'case_stderr_truncated=1' "$EVIDENCE/bundle-manifest.txt" >/dev/null ||
    fail "failure bundle omitted stderr truncation provenance"

grep -F 'phase=tmux-zellij-launched' "$EVIDENCE/serial-1/outside-manual/phase.log" >/dev/null ||
    fail "smoke phase evidence did not reach the injected failure"
grep -F 'injected_failure=tmux-zellij-launched' "$EVIDENCE/serial-1/outside-manual/process-ownership.txt" >/dev/null ||
    fail "process evidence omitted the injected failure identity"
grep -F 'session_alive_after=0' "$EVIDENCE/serial-1/outside-manual/cleanup-result.txt" >/dev/null ||
    fail "cleanup evidence did not prove the Zellij session stopped"
grep -F 'tmux_alive_after=0' "$EVIDENCE/serial-1/outside-manual/cleanup-result.txt" >/dev/null ||
    fail "cleanup evidence did not prove the tmux server stopped"
grep -F 'root_exists_after=0' "$EVIDENCE/serial-1/outside-manual/cleanup-result.txt" >/dev/null ||
    fail "cleanup evidence did not prove the smoke root was removed"
assert_cleanup_proven "$EVIDENCE/serial-1/outside-manual/cleanup-result.txt" "forced failure"
grep -F 'stress_root_exists_after=0' "$EVIDENCE/stress-cleanup.txt" >/dev/null ||
    fail "cleanup evidence did not prove the stress root was removed"
[ "$(file_state "$STANDING_CONFIG")" = "$CONFIG_BEFORE" ] || fail "forced failure changed standing config"
[ "$(file_state "$STANDING_LAYOUT")" = "$LAYOUT_BEFORE" ] || fail "forced failure changed standing layout"

echo "PASS: forced lifecycle failure retains bounded evidence after complete cleanup"

HANG_EVIDENCE="$ROOT/hang-evidence"
HANG_OUT="$ROOT/hang.out"
HANG_ERR="$ROOT/hang.err"
set +e
ZAPHOD_LAYOUT_STRESS_EVIDENCE_DIR="$HANG_EVIDENCE" \
ZAPHOD_SMOKE_INJECT_NATIVE_HANG=list-panes \
ZAPHOD_SMOKE_NATIVE_COMMAND_TIMEOUT_SECS=2 \
ZAPHOD_LAYOUT_STRESS_TIMEOUT_SECS=15 \
ZAPHOD_LAYOUT_STRESS_SERIAL_ROUNDS=1 \
    "$SCRIPT_DIR/zellij-watcher-layout-stress-test.sh" > "$HANG_OUT" 2> "$HANG_ERR"
hang_status=$?
set -e

[ "$hang_status" -ne 0 ] || fail "injected native command hang unexpectedly passed"
grep -F 'phase=native-command-timeout' \
    "$HANG_EVIDENCE/serial-1/outside-manual/phase.log" >/dev/null ||
    fail "native command hang reached the outer watchdog without an inner timeout marker"
grep -F 'native-command-timeout: owner=zellij-session command=action list-panes' \
    "$HANG_EVIDENCE/serial-1/outside-manual/native/list-panes.err" >/dev/null ||
    fail "native command hang bundle omitted the bounded command identity"
grep -F 'phase=session-ready-wait' \
    "$HANG_EVIDENCE/serial-1/outside-manual/phase.log" >/dev/null ||
    fail "native command hang bundle omitted its last entered phase"
grep -F 'session_alive_after=0' \
    "$HANG_EVIDENCE/serial-1/outside-manual/cleanup-result.txt" >/dev/null ||
    fail "native command hang cleanup left the Zellij session alive"
grep -F 'tmux_alive_after=0' \
    "$HANG_EVIDENCE/serial-1/outside-manual/cleanup-result.txt" >/dev/null ||
    fail "native command hang cleanup left the tmux server alive"
assert_cleanup_proven "$HANG_EVIDENCE/serial-1/outside-manual/cleanup-result.txt" "native hang"
[ "$(file_state "$STANDING_CONFIG")" = "$CONFIG_BEFORE" ] || fail "native hang changed standing config"
[ "$(file_state "$STANDING_LAYOUT")" = "$LAYOUT_BEFORE" ] || fail "native hang changed standing layout"

echo "PASS: native command hang fails at its inner deadline with retained cleanup evidence"

STARTUP_EVIDENCE="$ROOT/startup-evidence"
STARTUP_OUT="$ROOT/startup.out"
STARTUP_ERR="$ROOT/startup.err"
set +e
ZAPHOD_LAYOUT_STRESS_EVIDENCE_DIR="$STARTUP_EVIDENCE" \
ZAPHOD_SMOKE_INJECT_STARTUP_EXIT=42 \
ZAPHOD_LAYOUT_STRESS_TIMEOUT_SECS=30 \
ZAPHOD_LAYOUT_STRESS_SERIAL_ROUNDS=1 \
    "$SCRIPT_DIR/zellij-watcher-layout-stress-test.sh" > "$STARTUP_OUT" 2> "$STARTUP_ERR"
startup_status=$?
set -e

[ "$startup_status" -ne 0 ] || fail "injected tmux-hosted startup exit unexpectedly passed"
grep -F 'tmux-host-exited-before-session-ready: pane_dead_status=42' \
    "$STARTUP_EVIDENCE/serial-1/case.stderr" >/dev/null ||
    fail "vanished startup did not report the retained tmux pane exit"
grep -F 'phase=session-ready-wait' \
    "$STARTUP_EVIDENCE/serial-1/outside-manual/phase.log" >/dev/null ||
    fail "vanished startup bundle omitted the readiness phase"
grep -F 'Pane is dead (status 42' \
    "$STARTUP_EVIDENCE/serial-1/outside-manual/tmux-pane.txt" >/dev/null ||
    fail "vanished startup bundle omitted the dead pane output"
grep -F 'session_alive_after=0' \
    "$STARTUP_EVIDENCE/serial-1/outside-manual/cleanup-result.txt" >/dev/null ||
    fail "vanished startup cleanup left a Zellij session alive"
grep -F 'tmux_alive_after=0' \
    "$STARTUP_EVIDENCE/serial-1/outside-manual/cleanup-result.txt" >/dev/null ||
    fail "vanished startup cleanup left the tmux server alive"
assert_cleanup_proven "$STARTUP_EVIDENCE/serial-1/outside-manual/cleanup-result.txt" "startup exit"
[ "$(file_state "$STANDING_CONFIG")" = "$CONFIG_BEFORE" ] || fail "startup exit changed standing config"
[ "$(file_state "$STANDING_LAYOUT")" = "$LAYOUT_BEFORE" ] || fail "startup exit changed standing layout"

echo "PASS: vanished startup reports retained pane exit evidence and cleans up"

INCONCLUSIVE_EVIDENCE="$ROOT/inconclusive-evidence"
set +e
ZAPHOD_LAYOUT_STRESS_EVIDENCE_DIR="$INCONCLUSIVE_EVIDENCE" \
ZAPHOD_LAYOUT_STRESS_INJECT_FAILURE_PHASE=tmux-zellij-launched \
ZAPHOD_SMOKE_INJECT_CLEANUP_PROBE_HANG=both \
ZAPHOD_LAYOUT_STRESS_TIMEOUT_SECS=30 \
ZAPHOD_LAYOUT_STRESS_SERIAL_ROUNDS=1 \
    "$SCRIPT_DIR/zellij-watcher-layout-stress-test.sh" \
    > "$ROOT/inconclusive.out" 2> "$ROOT/inconclusive.err"
inconclusive_status=$?
set -e

[ "$inconclusive_status" -ne 0 ] || fail "inconclusive cleanup probes unexpectedly passed"
INCONCLUSIVE_RESULT="$INCONCLUSIVE_EVIDENCE/serial-1/outside-manual/cleanup-result.txt"
grep -F 'cleanup_status=1' "$INCONCLUSIVE_RESULT" >/dev/null ||
    fail "inconclusive cleanup probes did not fail cleanup proof"
grep -F 'session_probe_status_after=124' "$INCONCLUSIVE_RESULT" >/dev/null ||
    fail "injected Zellij cleanup timeout was not retained"
grep -F 'session_absence_confirmed=0' "$INCONCLUSIVE_RESULT" >/dev/null ||
    fail "Zellij cleanup timeout falsely confirmed absence"
grep -F 'tmux_probe_status_after=124' "$INCONCLUSIVE_RESULT" >/dev/null ||
    fail "injected tmux cleanup timeout was not retained"
grep -F 'tmux_absence_confirmed=0' "$INCONCLUSIVE_RESULT" >/dev/null ||
    fail "tmux cleanup timeout falsely confirmed absence"

echo "PASS: cleanup probe timeouts remain explicitly inconclusive"
