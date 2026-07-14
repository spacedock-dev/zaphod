#!/bin/bash
# ABOUTME: Pins the shipped sidebar's best-effort metadata and retired-scrollback documentation.
# ABOUTME: Keeps the historical prototype record while naming the remaining architecture follow-up.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd -P)"

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

grep -F 'best-effort command/CWD metadata' "$REPO_ROOT/README.md" >/dev/null ||
    fail "README still promises periodic live scrollback status"
grep -F 'preserves the last known status when live viewport data is unavailable' \
    "$REPO_ROOT/README.md" >/dev/null ||
    fail "README omits stale-not-blank degradation"
grep -F 'Zellij 0.44.3 can synchronously hold that export for five seconds' \
    "$REPO_ROOT/SPEC.md" >/dev/null ||
    fail "SPEC omits the shipped scrollback safety retirement"
grep -F 'nonblocking-pane-metadata-architecture' \
    "$REPO_ROOT/docs/docking-approach.md" >/dev/null ||
    fail "docking record omits the remaining synchronous metadata follow-up"

echo "PASS: shipped docs retire periodic scrollback and preserve the historical record"
