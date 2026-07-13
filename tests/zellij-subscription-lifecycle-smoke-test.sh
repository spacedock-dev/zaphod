#!/bin/bash
# ABOUTME: Proves the subscriber lifecycle first as a foreground diagnostic, then via direct-entry handoff.
# ABOUTME: Both passes use disposable tmux/Zellij state and require a distinct post-readiness SSE row.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"

run_outside() {
    env -u ZELLIJ -u ZELLIJ_SESSION_NAME -u ZELLIJ_PANE_ID \
        ZAPHOD_CALLER_ENV=outside ZAPHOD_SUBSCRIBER_MODE="$1" \
        "$SCRIPT_DIR/zellij-tmux-smoke-test.sh"
}

run_inside() {
    env ZELLIJ=0 ZELLIJ_SESSION_NAME=ambient-work ZELLIJ_PANE_ID=98765 \
        ZAPHOD_CALLER_ENV=inside ZAPHOD_SUBSCRIBER_MODE="$1" \
        "$SCRIPT_DIR/zellij-tmux-smoke-test.sh"
}

run_outside foreground
run_outside automatic
run_inside foreground
run_inside automatic

printf '%s\n' 'PASS: outside/inside callers completed foreground diagnosis and automatic subscriber handoff'
