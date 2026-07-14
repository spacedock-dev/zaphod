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

STANDING_ROOT="${ZELLIJ_CONFIG_DIR:-$HOME/.config/zellij}"
STANDING_CONFIG="${ZELLIJ_CONFIG_FILE:-$STANDING_ROOT/config.kdl}"
STANDING_LAYOUT="$STANDING_ROOT/layouts/zaphod.kdl"
CONFIG_BEFORE="$(file_state "$STANDING_CONFIG")"
LAYOUT_BEFORE="$(file_state "$STANDING_LAYOUT")"

set +e
ZAPHOD_LAYOUT_STRESS_EVIDENCE_DIR="$EVIDENCE" \
ZAPHOD_LAYOUT_STRESS_INJECT_FAILURE_PHASE=tmux-zellij-launched \
ZAPHOD_LAYOUT_STRESS_TIMEOUT_SECS=30 \
ZAPHOD_LAYOUT_STRESS_SERIAL_ROUNDS=1 \
    "$SCRIPT_DIR/zellij-subscription-layout-stress-test.sh" > "$OUT" 2> "$ERR"
status=$?
set -e

[ "$status" -ne 0 ] || fail "injected lifecycle failure unexpectedly passed"
grep -F "retained failure evidence: $EVIDENCE" "$ERR" >/dev/null ||
    fail "stress failure did not report its retained evidence path"
[ -f "$EVIDENCE/bundle-manifest.txt" ] || fail "failure bundle omitted its manifest"
[ -f "$EVIDENCE/serial-1/case.stdout" ] || fail "failure bundle omitted case stdout"
[ -f "$EVIDENCE/serial-1/case.stderr" ] || fail "failure bundle omitted case stderr"
[ -f "$EVIDENCE/serial-1/lifecycle-phases.log" ] || fail "failure bundle omitted lifecycle phases"
[ -f "$EVIDENCE/serial-1/outside-foreground/phase.log" ] || fail "failure bundle omitted smoke phases"
[ -f "$EVIDENCE/serial-1/outside-foreground/process-ownership.txt" ] ||
    fail "failure bundle omitted process ownership"
[ -f "$EVIDENCE/serial-1/outside-foreground/tmux-pane.txt" ] ||
    fail "failure bundle omitted bounded tmux pane evidence"
[ -d "$EVIDENCE/serial-1/outside-foreground/native" ] ||
    fail "failure bundle omitted bounded native replies"
[ -f "$EVIDENCE/serial-1/outside-foreground/cleanup-result.txt" ] ||
    fail "failure bundle omitted smoke cleanup result"
[ -f "$EVIDENCE/stress-cleanup.txt" ] || fail "failure bundle omitted stress cleanup result"

grep -F 'phase=tmux-zellij-launched' "$EVIDENCE/serial-1/outside-foreground/phase.log" >/dev/null ||
    fail "smoke phase evidence did not reach the injected failure"
grep -F 'injected_failure=tmux-zellij-launched' "$EVIDENCE/serial-1/outside-foreground/process-ownership.txt" >/dev/null ||
    fail "process evidence omitted the injected failure identity"
grep -F 'session_alive_after=0' "$EVIDENCE/serial-1/outside-foreground/cleanup-result.txt" >/dev/null ||
    fail "cleanup evidence did not prove the Zellij session stopped"
grep -F 'tmux_alive_after=0' "$EVIDENCE/serial-1/outside-foreground/cleanup-result.txt" >/dev/null ||
    fail "cleanup evidence did not prove the tmux server stopped"
grep -F 'root_exists_after=0' "$EVIDENCE/serial-1/outside-foreground/cleanup-result.txt" >/dev/null ||
    fail "cleanup evidence did not prove the smoke root was removed"
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
    "$SCRIPT_DIR/zellij-subscription-layout-stress-test.sh" > "$HANG_OUT" 2> "$HANG_ERR"
hang_status=$?
set -e

[ "$hang_status" -ne 0 ] || fail "injected native command hang unexpectedly passed"
grep -F 'phase=native-command-timeout' \
    "$HANG_EVIDENCE/serial-1/outside-foreground/phase.log" >/dev/null ||
    fail "native command hang reached the outer watchdog without an inner timeout marker"
grep -F 'native-command-timeout: owner=zellij-session command=action list-panes' \
    "$HANG_EVIDENCE/serial-1/outside-foreground/native/list-panes.err" >/dev/null ||
    fail "native command hang bundle omitted the bounded command identity"
grep -F 'phase=session-ready-wait' \
    "$HANG_EVIDENCE/serial-1/outside-foreground/phase.log" >/dev/null ||
    fail "native command hang bundle omitted its last entered phase"
grep -F 'session_alive_after=0' \
    "$HANG_EVIDENCE/serial-1/outside-foreground/cleanup-result.txt" >/dev/null ||
    fail "native command hang cleanup left the Zellij session alive"
grep -F 'tmux_alive_after=0' \
    "$HANG_EVIDENCE/serial-1/outside-foreground/cleanup-result.txt" >/dev/null ||
    fail "native command hang cleanup left the tmux server alive"
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
    "$SCRIPT_DIR/zellij-subscription-layout-stress-test.sh" > "$STARTUP_OUT" 2> "$STARTUP_ERR"
startup_status=$?
set -e

[ "$startup_status" -ne 0 ] || fail "injected tmux-hosted startup exit unexpectedly passed"
grep -F 'tmux-host-exited-before-session-ready: pane_dead_status=42' \
    "$STARTUP_EVIDENCE/serial-1/case.stderr" >/dev/null ||
    fail "vanished startup did not report the retained tmux pane exit"
grep -F 'phase=session-ready-wait' \
    "$STARTUP_EVIDENCE/serial-1/outside-foreground/phase.log" >/dev/null ||
    fail "vanished startup bundle omitted the readiness phase"
grep -F 'injected startup exit 42' \
    "$STARTUP_EVIDENCE/serial-1/outside-foreground/tmux-pane.txt" >/dev/null ||
    fail "vanished startup bundle omitted the dead pane output"
grep -F 'session_alive_after=0' \
    "$STARTUP_EVIDENCE/serial-1/outside-foreground/cleanup-result.txt" >/dev/null ||
    fail "vanished startup cleanup left a Zellij session alive"
grep -F 'tmux_alive_after=0' \
    "$STARTUP_EVIDENCE/serial-1/outside-foreground/cleanup-result.txt" >/dev/null ||
    fail "vanished startup cleanup left the tmux server alive"
[ "$(file_state "$STANDING_CONFIG")" = "$CONFIG_BEFORE" ] || fail "startup exit changed standing config"
[ "$(file_state "$STANDING_LAYOUT")" = "$LAYOUT_BEFORE" ] || fail "startup exit changed standing layout"

echo "PASS: vanished startup reports retained pane exit evidence and cleans up"
