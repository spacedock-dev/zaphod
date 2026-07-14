#!/bin/bash
# ABOUTME: Verifies the direct entry's two checkout-local build artifacts.
# ABOUTME: The native manual watcher is zaphod, never a separately installed command.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd -P)"

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

"$REPO_ROOT/build.sh" >/dev/null

WASM="$REPO_ROOT/target/wasm32-wasip1/release/zellij-sidebar.wasm"
WATCHER="$REPO_ROOT/target/zaphod"
[ -f "$WASM" ] || fail "build did not produce the canonical WASM artifact"
[ -x "$WATCHER" ] || fail "build did not produce the checkout-local zaphod watcher"

set +e
USAGE="$($WATCHER 2>&1)"
STATUS=$?
set -e
[ "$STATUS" -eq 2 ] || fail "zaphod without a manual watcher command exited $STATUS, want 2"
printf '%s\n' "$USAGE" | grep -F 'zaphod watch-tab' >/dev/null ||
    fail "native binary did not expose the manual watcher invocation"
! printf '%s\n' "$USAGE" | grep -F 'zaphod subscribe' >/dev/null ||
    fail "native binary still exposed the automatic subscriber"

echo "PASS: build emits canonical WASM plus manual zaphod watcher"
