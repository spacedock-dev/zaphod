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

echo "PASS: managed sidebar preserves literal pane/tab responsiveness and quiet post-close state"
