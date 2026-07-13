#!/bin/bash
# ABOUTME: Proves the subscriber lifecycle first as a foreground diagnostic, then via direct-entry handoff.
# ABOUTME: Both passes use disposable tmux/Zellij state and require a distinct post-readiness SSE row.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"

ZAPHOD_SUBSCRIBER_MODE=foreground "$SCRIPT_DIR/zellij-tmux-smoke-test.sh"
ZAPHOD_SUBSCRIBER_MODE=automatic "$SCRIPT_DIR/zellij-tmux-smoke-test.sh"

printf '%s\n' 'PASS: foreground diagnostic and automatic subscriber handoff both completed'
