#!/bin/bash
# ABOUTME: Proves the manual watcher journey from outside and inside Zellij.
# ABOUTME: Both passes use disposable tmux/Zellij state and post-readiness SSE.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
EVIDENCE_DIR="${ZAPHOD_LIFECYCLE_EVIDENCE_DIR:-}"
INJECT_FAILURE_PHASE="${ZAPHOD_LIFECYCLE_INJECT_FAILURE_PHASE:-}"

lifecycle_phase() {
    local name="$1" marker="lifecycle-phase: phase=$1 pid=$$"
    printf '%s\n' "$marker" >&2
    if [ -n "$EVIDENCE_DIR" ]; then
        mkdir -p "$EVIDENCE_DIR"
        printf '%s\n' "$marker" >> "$EVIDENCE_DIR/lifecycle-phases.log"
    fi
}

run_outside() {
    local smoke_evidence=""
    [ -z "$EVIDENCE_DIR" ] || smoke_evidence="$EVIDENCE_DIR/outside-manual"
    env -u ZELLIJ -u ZELLIJ_SESSION_NAME -u ZELLIJ_PANE_ID \
        ZAPHOD_SMOKE_EVIDENCE_DIR="$smoke_evidence" \
        ZAPHOD_SMOKE_INJECT_FAILURE_PHASE="$INJECT_FAILURE_PHASE" \
        ZAPHOD_CALLER_ENV=outside "$SCRIPT_DIR/zellij-tmux-smoke-test.sh"
}

run_inside() {
    local smoke_evidence=""
    [ -z "$EVIDENCE_DIR" ] || smoke_evidence="$EVIDENCE_DIR/inside-manual"
    env ZELLIJ=0 ZELLIJ_SESSION_NAME=ambient-work ZELLIJ_PANE_ID=98765 \
        ZAPHOD_SMOKE_EVIDENCE_DIR="$smoke_evidence" \
        ZAPHOD_SMOKE_INJECT_FAILURE_PHASE="$INJECT_FAILURE_PHASE" \
        ZAPHOD_CALLER_ENV=inside "$SCRIPT_DIR/zellij-tmux-smoke-test.sh"
}

lifecycle_phase outside-manual-start
run_outside
lifecycle_phase outside-manual-complete
lifecycle_phase inside-manual-start
run_inside
lifecycle_phase inside-manual-complete

printf '%s\n' 'PASS: outside and inside callers completed the manual watcher journey'
