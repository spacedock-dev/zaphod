#!/bin/bash
# ABOUTME: Proves the subscriber lifecycle first as a foreground diagnostic, then via direct-entry handoff.
# ABOUTME: Both passes use disposable tmux/Zellij state and require a distinct post-readiness SSE row.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
EVIDENCE_DIR="${ZAPHOD_LIFECYCLE_EVIDENCE_DIR:-}"
INJECT_FAILURE_PHASE="${ZAPHOD_LIFECYCLE_INJECT_FAILURE_PHASE:-}"

lifecycle_phase() {
    local name="$1"
    local marker="lifecycle-phase: phase=$name pid=$$"
    printf '%s\n' "$marker" >&2
    if [ -n "$EVIDENCE_DIR" ]; then
        mkdir -p "$EVIDENCE_DIR"
        printf '%s\n' "$marker" >> "$EVIDENCE_DIR/lifecycle-phases.log"
    fi
}

run_outside() {
    local smoke_evidence=""
    [ -z "$EVIDENCE_DIR" ] || smoke_evidence="$EVIDENCE_DIR/outside-$1"
    env -u ZELLIJ -u ZELLIJ_SESSION_NAME -u ZELLIJ_PANE_ID \
        ZAPHOD_SMOKE_EVIDENCE_DIR="$smoke_evidence" \
        ZAPHOD_SMOKE_INJECT_FAILURE_PHASE="$INJECT_FAILURE_PHASE" \
        ZAPHOD_CALLER_ENV=outside ZAPHOD_SUBSCRIBER_MODE="$1" \
        "$SCRIPT_DIR/zellij-tmux-smoke-test.sh"
}

run_inside() {
    local smoke_evidence=""
    [ -z "$EVIDENCE_DIR" ] || smoke_evidence="$EVIDENCE_DIR/inside-$1"
    env ZELLIJ=0 ZELLIJ_SESSION_NAME=ambient-work ZELLIJ_PANE_ID=98765 \
        ZAPHOD_SMOKE_EVIDENCE_DIR="$smoke_evidence" \
        ZAPHOD_SMOKE_INJECT_FAILURE_PHASE="$INJECT_FAILURE_PHASE" \
        ZAPHOD_CALLER_ENV=inside ZAPHOD_SUBSCRIBER_MODE="$1" \
        "$SCRIPT_DIR/zellij-tmux-smoke-test.sh"
}

lifecycle_phase outside-foreground-start
run_outside foreground
lifecycle_phase outside-foreground-complete
lifecycle_phase inside-automatic-start
run_inside automatic
lifecycle_phase inside-automatic-complete

printf '%s\n' 'PASS: outside foreground diagnosis and inside automatic subscriber handoff both completed'
