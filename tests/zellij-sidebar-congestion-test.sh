#!/bin/bash
# ABOUTME: Proves literal pane/tab actions stay responsive with the managed sidebar loaded.
# ABOUTME: Requires native-state deadlines, a six-second quiet window, and conclusive failure cleanup.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
ROOT="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-sidebar-congestion-test.XXXXXX")"
OUT="$ROOT/smoke.out"
ERR="$ROOT/smoke.err"
trap 'rm -rf "$ROOT"' EXIT

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

if ! ZAPHOD_SMOKE_RESPONSIVENESS_CHECK=1 \
    "$SCRIPT_DIR/zellij-tmux-smoke-test.sh" > "$OUT" 2> "$ERR"; then
    cat "$OUT" >&2 || true
    cat "$ERR" >&2 || true
    fail "responsive-action smoke failed"
fi

grep -F 'phase=responsive-actions-complete' "$ERR" >/dev/null ||
    fail "smoke omitted native pane/tab deadline and quiet-window proof"
grep -F 'PASS: literal Alt p/Alt n/Alt 1/Alt 2 met native one-second deadlines' "$OUT" >/dev/null ||
    fail "smoke omitted the exact native responsiveness result"

FAILURE_EVIDENCE="$ROOT/failure-evidence"
set +e
ZAPHOD_SMOKE_RESPONSIVENESS_CHECK=1 \
ZAPHOD_SMOKE_INJECT_RESPONSIVE_TIMEOUT=pane-2 \
ZAPHOD_SMOKE_EVIDENCE_DIR="$FAILURE_EVIDENCE" \
    "$SCRIPT_DIR/zellij-tmux-smoke-test.sh" \
    > "$ROOT/failure.out" 2> "$ROOT/failure.err"
failure_status=$?
set -e
[ "$failure_status" -ne 0 ] || fail "injected action timeout unexpectedly passed"
grep -F 'phase=responsive-action-timeout' "$FAILURE_EVIDENCE/phase.log" >/dev/null ||
    fail "injected action timeout omitted its retained phase"
grep -F 'cleanup_status=0' "$FAILURE_EVIDENCE/cleanup-result.txt" >/dev/null ||
    fail "injected action timeout cleanup reported an error"
grep -F 'session_absence_confirmed=1' "$FAILURE_EVIDENCE/cleanup-result.txt" >/dev/null ||
    fail "injected action timeout did not prove Zellij session absence"
grep -F 'tmux_absence_confirmed=1' "$FAILURE_EVIDENCE/cleanup-result.txt" >/dev/null ||
    fail "injected action timeout did not prove tmux server absence"
grep -F 'root_exists_after=0' "$FAILURE_EVIDENCE/cleanup-result.txt" >/dev/null ||
    fail "injected action timeout left its disposable root"

echo "PASS: managed sidebar preserves literal pane/tab responsiveness and quiet post-close state"
