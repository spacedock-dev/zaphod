#!/bin/bash
# ABOUTME: Exercises the managed Zellij-tab entry path through a real tmux-hosted client.
# ABOUTME: Keeps config, data, socket, tmux server, and cleanup isolated from standing Zellij state.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
# shellcheck source=scripts/zellij-layout-lib.sh
source "$REPO_ROOT/scripts/zellij-layout-lib.sh"

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
STANDING_CONFIG_BEFORE="$(file_state "$STANDING_CONFIG")"
STANDING_LAYOUT_BEFORE="$(file_state "$STANDING_LAYOUT")"

ROOT=""
CONFIG_DIR=""
CONFIG_FILE=""
DATA_DIR=""
SOCKET_DIR=""
SESSION_NAME=""
TMUX_SERVER=""
TMUX_SESSION="zaphod-smoke"
TMUX_PANE="$TMUX_SESSION:0.0"

zellij_control() {
    env ZELLIJ_SOCKET_DIR="$SOCKET_DIR" \
        zellij --config-dir "$CONFIG_DIR" --config "$CONFIG_FILE" --data-dir "$DATA_DIR" "$@"
}

zellij_session() {
    env ZELLIJ_SOCKET_DIR="$SOCKET_DIR" \
        zellij --session "$SESSION_NAME" \
        --config-dir "$CONFIG_DIR" --config "$CONFIG_FILE" --data-dir "$DATA_DIR" "$@"
}

tmux_command() {
    tmux -L "$TMUX_SERVER" "$@"
}

cleanup() {
    local status=$?
    local cleanup_status=0
    trap - EXIT INT TERM HUP
    set +e
    if [ -n "$SESSION_NAME" ]; then
        zellij_control delete-session --force "$SESSION_NAME" >/dev/null 2>&1 || true
    fi
    if [ -n "$TMUX_SERVER" ]; then
        tmux -L "$TMUX_SERVER" kill-server >/dev/null 2>&1 || true
    fi
    if [ -n "$ROOT" ] && [ -d "$ROOT" ]; then
        rm -rf "$ROOT" || cleanup_status=1
    fi
    if [ "$(file_state "$STANDING_CONFIG")" != "$STANDING_CONFIG_BEFORE" ]; then
        echo "standing Zellij config changed during tmux smoke: $STANDING_CONFIG" >&2
        cleanup_status=1
    fi
    if [ "$(file_state "$STANDING_LAYOUT")" != "$STANDING_LAYOUT_BEFORE" ]; then
        echo "standing Zellij layout changed during tmux smoke: $STANDING_LAYOUT" >&2
        cleanup_status=1
    fi
    if [ "$cleanup_status" -ne 0 ]; then
        exit "$cleanup_status"
    fi
    exit "$status"
}

trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP

for required in tmux jq shasum; do
    command -v "$required" >/dev/null 2>&1 || fail "$required is required for the tmux smoke"
done
zaphod_require_zellij_0443

# Zellij's Unix socket is capped at 103 bytes on macOS. Keep this disposable
# root under /tmp rather than the much longer per-user $TMPDIR.
ROOT="$(mktemp -d /tmp/zs.XXXXXX)" || fail "could not create a short isolated smoke root"
CONFIG_DIR="$ROOT/config"
CONFIG_FILE="$CONFIG_DIR/config.kdl"
DATA_DIR="$ROOT/data"
SOCKET_DIR="$ROOT/socket"
SESSION_NAME="zs$$"
TMUX_SERVER="zs$$"
mkdir -p "$CONFIG_DIR/layouts" "$DATA_DIR" "$SOCKET_DIR" "$ROOT/tmp"
cp "$SCRIPT_DIR/fixtures/zellij-tmux-smoke-config.kdl" "$CONFIG_FILE"

WASM_PATH="$REPO_ROOT/target/wasm32-wasip1/release/zellij-sidebar.wasm"
WASM_URL=""

start_tmux_zellij() {
    local command
    printf -v command 'env ZELLIJ_SOCKET_DIR=%q %q --config-dir %q --config %q --data-dir %q attach --create %q' \
        "$SOCKET_DIR" "$(command -v zellij)" "$CONFIG_DIR" "$CONFIG_FILE" "$DATA_DIR" "$SESSION_NAME"
    tmux_command new-session -d -x 160 -y 45 -s "$TMUX_SESSION" "$command"
}

wait_for_nonempty_panes() {
    local output="$1"
    local attempt
    for attempt in $(seq 1 160); do
        if zellij_session action list-panes --json --all --command --geometry --state --tab \
            > "$output" 2>"$ROOT/list-panes.err" && \
            jq -e 'type == "array" and length > 0' "$output" >/dev/null 2>&1; then
            return
        fi
        sleep 0.05
    done
    cat "$ROOT/list-panes.err" >&2 || true
    fail "isolated Zellij session did not become ready"
}

send_literal() {
    tmux_command send-keys -l -t "$TMUX_PANE" -- "$1"
}

capture_state() {
    local json="$1"
    local layout="$2"
    local screen="$3"
    zellij_session action list-panes --json --all --command --geometry --state --tab > "$json"
    jq -S . "$json" > "$json.sorted"
    zellij_session action dump-layout > "$layout"
    tmux_command capture-pane -p -t "$TMUX_PANE" > "$screen"
}

dismiss_startup_tip() {
    local probe="$ROOT/after-dismiss.json"
    local attempt
    send_literal "$(printf '\033')"
    for attempt in $(seq 1 80); do
        zellij_session action list-panes --json --all --command --geometry --state --tab > "$probe" 2>/dev/null || true
        if jq -e 'all(.[]; .plugin_url != "about")' "$probe" >/dev/null 2>&1; then
            return
        fi
        sleep 0.05
    done
    fail "Zellij startup tip did not close; foreign-tab key assertion would be inconclusive"
}

wait_for_candidate() {
    local output="$1"
    local attempt
    for attempt in $(seq 1 160); do
        zellij_session action list-panes --json --all --command --geometry --state --tab \
            > "$output" 2>"$ROOT/candidate-panes.err" || true
        if jq -e --arg wasm_url "$WASM_URL" \
            'any(.[]; .is_plugin and .plugin_url == $wasm_url)' "$output" >/dev/null 2>&1; then
            return
        fi
        sleep 0.05
    done
    cat "$ROOT/candidate-panes.err" >&2 || true
    fail "literal Alt Shift z did not create a candidate Zaphod tab"
}

# First run the real entry script against a short-lived attached Zellij
# session. This persists the checked config and selected absolute layout path.
start_tmux_zellij
wait_for_nonempty_panes "$ROOT/bootstrap-panes.json"
env ZELLIJ_CONFIG_DIR="$CONFIG_DIR" ZELLIJ_CONFIG_FILE="$CONFIG_FILE" \
    ZELLIJ_DATA_DIR="$DATA_DIR" ZELLIJ_SOCKET_DIR="$SOCKET_DIR" TMPDIR="$ROOT/tmp" \
    "$REPO_ROOT/scripts/zellij-new-tab.sh" --session "$SESSION_NAME" --name 'Zaphod smoke bootstrap' \
    > "$ROOT/entry.out"
WASM_URL="$(zaphod_canonical_file_url "$WASM_PATH")" ||
    fail "entry script did not build the candidate WASM"
grep -F 'TAB_ID=' "$ROOT/entry.out" >/dev/null || fail "entry script did not create its bootstrap tab"
grep -Fx "WASM_URL=$WASM_URL" "$ROOT/entry.out" >/dev/null ||
    fail "entry script did not report the candidate WASM URL"

# Native keybinds are read at server startup. Restart the disposable server so
# the current tmux client exercises the persistent Alt Shift z and Alt / routes.
# Zellij 0.44.3 can return "Session not found" after a successful forced
# kill; tmux teardown below is the authoritative client cleanup.
zellij_control delete-session --force "$SESSION_NAME" >/dev/null 2>&1 || true
tmux_command kill-server >/dev/null 2>&1 || true
start_tmux_zellij
wait_for_nonempty_panes "$ROOT/foreign-ready.json"
dismiss_startup_tip

# In the sidebar-less foreign tab, literal Alt / must be a persistent NoOp.
capture_state "$ROOT/foreign-before.json" "$ROOT/foreign-before.kdl" "$ROOT/foreign-before.screen"
jq -e --arg wasm_url "$WASM_URL" \
    'all(.[]; .plugin_url != $wasm_url)' "$ROOT/foreign-before.json" >/dev/null ||
    fail "fresh server unexpectedly started on a candidate Zaphod tab"
send_literal "$(printf '\033/')"
sleep 0.10
capture_state "$ROOT/foreign-after.json" "$ROOT/foreign-after.kdl" "$ROOT/foreign-after.screen"
cmp -s "$ROOT/foreign-before.json.sorted" "$ROOT/foreign-after.json.sorted" || {
    diff -u "$ROOT/foreign-before.json.sorted" "$ROOT/foreign-after.json.sorted" >&2 || true
    fail "foreign-tab Alt / changed native pane, focus, tab, or process state"
}
cmp -s "$ROOT/foreign-before.kdl" "$ROOT/foreign-after.kdl" || {
    diff -u "$ROOT/foreign-before.kdl" "$ROOT/foreign-after.kdl" >&2 || true
    fail "foreign-tab Alt / changed the native layout"
}

# The fresh native entry key must create a tab whose visible and native state
# both identify this checkout's candidate WASM.
send_literal "$(printf '\033Z')"
wait_for_candidate "$ROOT/candidate.json"
capture_state "$ROOT/candidate.json" "$ROOT/candidate.kdl" "$ROOT/candidate.screen"
jq -e --arg wasm_url "$WASM_URL" \
    'any(.[]; .is_plugin and .plugin_url == $wasm_url and .tab_name == "zaphod")' \
    "$ROOT/candidate.json" >/dev/null || fail "candidate pane did not appear in the native Zellij state"
grep -F "plugin location=\"$WASM_URL\"" "$ROOT/candidate.kdl" >/dev/null ||
    fail "candidate URL did not appear in the native Zellij layout dump"
grep -F 'asks permission to:' "$ROOT/candidate.screen" >/dev/null ||
    fail "candidate Zaphod pane was not visibly rendered in the tmux client"

[ "$(file_state "$STANDING_CONFIG")" = "$STANDING_CONFIG_BEFORE" ] ||
    fail "standing Zellij config changed during tmux smoke: $STANDING_CONFIG"
[ "$(file_state "$STANDING_LAYOUT")" = "$STANDING_LAYOUT_BEFORE" ] ||
    fail "standing Zellij layout changed during tmux smoke: $STANDING_LAYOUT"

printf '%s\n' 'PASS: tmux-hosted Zellij smoke created the candidate tab and left foreign Alt / inert'
