#!/bin/bash
# ABOUTME: Verifies the direct entry's two checkout-local build artifacts.
# ABOUTME: The native subscriber is zaphod, never a separately installed grout command.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd -P)"

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

"$REPO_ROOT/build.sh" >/dev/null

WASM="$REPO_ROOT/target/wasm32-wasip1/release/zellij-sidebar.wasm"
SIDECAR="$REPO_ROOT/target/zaphod"
[ -f "$WASM" ] || fail "build did not produce the canonical WASM artifact"
[ -x "$SIDECAR" ] || fail "build did not produce the checkout-local zaphod sidecar"

set +e
USAGE="$($SIDECAR 2>&1)"
STATUS=$?
set -e
[ "$STATUS" -eq 2 ] || fail "zaphod without its private subscribe command exited $STATUS, want 2"
printf '%s\n' "$USAGE" | grep -F 'zaphod subscribe' >/dev/null ||
    fail "native sidecar did not expose only the zaphod subscribe invocation"

echo "PASS: build emits canonical WASM plus private zaphod sidecar"
