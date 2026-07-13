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
HOME_DIR=""
PERMISSION_CACHE=""
ISOLATED_CONFIG_BEFORE=""
ISOLATED_LAYOUT=""
ISOLATED_LAYOUT_BEFORE=""
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
        if zellij_control --session "$SESSION_NAME" action list-panes --json --all \
            >/dev/null 2>&1; then
            echo "isolated Zellij session survived cleanup: $SESSION_NAME" >&2
            cleanup_status=1
        fi
    fi
    if [ -n "$TMUX_SERVER" ]; then
        tmux -L "$TMUX_SERVER" kill-server >/dev/null 2>&1 || true
        if tmux -L "$TMUX_SERVER" has-session -t "$TMUX_SESSION" >/dev/null 2>&1; then
            echo "dedicated tmux server survived cleanup: $TMUX_SERVER" >&2
            cleanup_status=1
        fi
    fi
    if [ -n "$ROOT" ] && [ -d "$ROOT" ]; then
        rm -rf "$ROOT" || cleanup_status=1
        if [ -e "$ROOT" ]; then
            echo "isolated smoke root survived cleanup: $ROOT" >&2
            cleanup_status=1
        fi
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
ISOLATED_LAYOUT="$CONFIG_DIR/layouts/zaphod.kdl"
sed "s|<FIXED_OPERATOR_LAYOUT>|$ISOLATED_LAYOUT|" \
    "$SCRIPT_DIR/fixtures/zellij-tmux-smoke-config.kdl" > "$CONFIG_FILE"
printf '%s\n' 'layout { pane; }' > "$ISOLATED_LAYOUT"
ISOLATED_CONFIG_BEFORE="$(file_state "$CONFIG_FILE")"
ISOLATED_LAYOUT_BEFORE="$(file_state "$ISOLATED_LAYOUT")"

WASM_PATH="$REPO_ROOT/target/wasm32-wasip1/release/zellij-sidebar.wasm"
"$REPO_ROOT/build.sh" >/dev/null
WASM_URL="$(zaphod_canonical_file_url "$WASM_PATH")" ||
    fail "could not derive the candidate WASM URL"

# This is a deliberately pre-authorized, disposable permission fixture. The
# server starts with HOME under ROOT, so the real prompt remains available for
# AC-I1 while this headless smoke never writes the operator's cache or fakes
# consent with injected keys.
HOME_DIR="$ROOT/home"
PERMISSION_CACHE="$HOME_DIR/Library/Caches/org.Zellij-Contributors.Zellij/permissions.kdl"
mkdir -p "$(dirname "$PERMISSION_CACHE")"
{
    # Zellij's permission cache is keyed by the local plugin path, not by the
    # file: URL that appears in the layout and pane inventory.
    printf '"%s" {\n' "$WASM_PATH"
    printf '%s\n' \
        '    ReadApplicationState' \
        '    ChangeApplicationState' \
        '    ReadPaneContents' \
        '    Reconfigure' \
        '    RunCommands'
    printf '%s\n' '}'
} > "$PERMISSION_CACHE"

start_tmux_zellij() {
    local command
    printf -v command 'env HOME=%q ZELLIJ_SOCKET_DIR=%q %q --config-dir %q --config %q --data-dir %q attach --create %q' \
        "$HOME_DIR" "$SOCKET_DIR" "$(command -v zellij)" "$CONFIG_DIR" "$CONFIG_FILE" "$DATA_DIR" "$SESSION_NAME"
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

capture_tabs() {
    local tabs="$1"
    zellij_session action list-tabs --json --all --state --layout > "$tabs"
    jq -S . "$tabs" > "$tabs.sorted"
}

without_geometry() {
    local source="$1"
    local normalized="$2"
    jq -S 'map(del(
        .pane_x,
        .pane_content_x,
        .pane_y,
        .pane_content_y,
        .pane_rows,
        .pane_content_rows,
        .pane_columns,
        .pane_content_columns
    ))' "$source" > "$normalized"
}

candidate_width() {
    local panes="$1"
    jq -er --arg wasm_url "$WASM_URL" \
        '[.[] | select(.is_plugin and .plugin_url == $wasm_url) | .pane_columns]
         | if length == 1 then .[0] else error("expected one candidate rail") end' \
        "$panes"
}

candidate_geometry() {
    local panes="$1"
    local geometry="$2"
    jq -S --arg wasm_url "$WASM_URL" \
        '[.[] | select(.is_plugin and .plugin_url == $wasm_url)
          | {id, pane_x, pane_y, pane_columns, pane_rows, tab_id, tab_position, tab_name}]' \
        "$panes" > "$geometry"
}

wait_for_candidate_width() {
    local output="$1"
    local expected_width="$2"
    local attempt actual
    for attempt in $(seq 1 80); do
        zellij_session action list-panes --json --all --command --geometry --state --tab \
            > "$output" 2>"$ROOT/toggle-panes.err" || true
        actual="$(candidate_width "$output" 2>/dev/null || true)"
        if [ "$actual" = "$expected_width" ]; then
            return
        fi
        sleep 0.05
    done
    cat "$ROOT/toggle-panes.err" >&2 || true
    fail "literal Alt / did not move the candidate rail to width $expected_width (last width: ${actual:-missing})"
}

wait_for_foreign_active_tab() {
    local tabs="$1"
    local attempt
    for attempt in $(seq 1 80); do
        capture_tabs "$tabs"
        if jq -e 'any(.[]; .active and .name != "zaphod")' "$tabs" >/dev/null 2>&1; then
            return
        fi
        sleep 0.05
    done
    fail "native previous-tab action did not return the tmux client to the foreign tab"
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

# A pre-granted plugin still receives PermissionRequestResult asynchronously.
# Do not baseline the literal Alt / until its own visible tiled resident has
# handled that result: otherwise is_selectable can settle during the key test
# and look like a toggle side effect.
wait_for_settled_candidate_resident() {
    local panes="$1"
    local layout="$2"
    local screen="$3"
    local tabs="$4"
    local attempt
    for attempt in $(seq 1 160); do
        zellij_session action list-panes --json --all --command --geometry --state --tab \
            > "$panes" 2>"$ROOT/settled-candidate-panes.err" || true
        zellij_session action list-tabs --json --all --state --layout \
            > "$tabs" 2>"$ROOT/settled-candidate-tabs.err" || true
        tmux_command capture-pane -p -t "$TMUX_PANE" > "$screen" || true
        if jq -e --arg wasm_url "$WASM_URL" --arg tab_id "$TAB_ID" \
            'any(.[]; .is_plugin and .plugin_url == $wasm_url and (.tab_id | tostring) == $tab_id and .is_floating == false and .is_suppressed == false and .pane_columns == 28 and .is_selectable == false)' \
            "$panes" >/dev/null 2>&1 && \
            jq -e --arg tab_id "$TAB_ID" 'any(.[]; .active and (.tab_id | tostring) == $tab_id)' "$tabs" >/dev/null 2>&1 && \
            ! grep -F 'asks permission to:' "$screen" >/dev/null && \
            grep -F 'PANES' "$screen" >/dev/null; then
            jq -S . "$panes" > "$panes.sorted"
            jq -S . "$tabs" > "$tabs.sorted"
            zellij_session action dump-layout > "$layout"
            return
        fi
        sleep 0.05
    done
    cat "$ROOT/settled-candidate-panes.err" >&2 || true
    cat "$ROOT/settled-candidate-tabs.err" >&2 || true
    jq -S . "$panes" >&2 || true
    jq -S . "$tabs" >&2 || true
    sed -n '1,80p' "$screen" >&2 || true
    fail "candidate did not settle as the active tiled 28-column, non-selectable post-grant resident"
}

# Run the selected-checkout entry against a real attached client. The fixture
# already has a fixed global Alt Shift z shortcut and fail-closed Alt / policy;
# the direct command must create its own inline tab without changing either.
start_tmux_zellij
wait_for_nonempty_panes "$ROOT/foreign-ready.json"
dismiss_startup_tip
zellij_control setup --check >/dev/null
capture_tabs "$ROOT/foreign-tabs-before.json"
TAB_COUNT_BEFORE="$(jq -er 'length' "$ROOT/foreign-tabs-before.json")"
capture_state "$ROOT/foreign-ready.json" "$ROOT/foreign-ready.kdl" "$ROOT/foreign-ready.screen"
jq -e --arg wasm_url "$WASM_URL" \
    'all(.[]; .plugin_url != $wasm_url)' "$ROOT/foreign-ready.json" >/dev/null ||
    fail "isolated profile unexpectedly started on the selected checkout rail"
env ZELLIJ_CONFIG_DIR="$CONFIG_DIR" ZELLIJ_CONFIG_FILE="$CONFIG_FILE" \
    ZELLIJ_DATA_DIR="$DATA_DIR" ZELLIJ_SOCKET_DIR="$SOCKET_DIR" TMPDIR="$ROOT/tmp" \
    "$REPO_ROOT/scripts/zellij-new-tab.sh" --session "$SESSION_NAME" --name 'Zaphod selected checkout' \
    > "$ROOT/entry.out"
TAB_ID="$(sed -n 's/^TAB_ID=//p' "$ROOT/entry.out")"
[[ "$TAB_ID" =~ ^(0|[1-9][0-9]*)$ ]] || fail "entry script did not report a stable tab ID"
grep -Fx "WASM_URL=$WASM_URL" "$ROOT/entry.out" >/dev/null ||
    fail "entry script did not report the candidate WASM URL"
wait_for_settled_candidate_resident \
    "$ROOT/candidate-before.json" \
    "$ROOT/candidate-before.kdl" \
    "$ROOT/candidate-before.screen" \
    "$ROOT/candidate-tabs-before.json"
TAB_COUNT_AFTER="$(jq -er 'length' "$ROOT/candidate-tabs-before.json")"
[ "$TAB_COUNT_AFTER" -eq "$((TAB_COUNT_BEFORE + 1))" ] ||
    fail "direct entry changed tab count from $TAB_COUNT_BEFORE to $TAB_COUNT_AFTER (expected one fresh tab)"
jq -e --arg wasm_url "$WASM_URL" \
    --arg tab_id "$TAB_ID" \
    '([.[] | select(.is_plugin and .plugin_url == $wasm_url and (.tab_id | tostring) == $tab_id and .is_floating == false and .is_suppressed == false)] | length) == 1' \
    "$ROOT/candidate-before.json" >/dev/null || fail "candidate pane did not appear in the native Zellij state"
jq -e --arg tab_id "$TAB_ID" 'any(.[]; .active and (.tab_id | tostring) == $tab_id)' \
    "$ROOT/candidate-tabs-before.json" >/dev/null || fail "direct entry did not activate its stable-ID tab"
grep -F "plugin location=\"$WASM_URL\"" "$ROOT/candidate-before.kdl" >/dev/null ||
    fail "candidate URL did not appear in the native Zellij layout dump"
grep -F 'asks permission to:' "$ROOT/candidate-before.screen" >/dev/null &&
    fail "candidate rail unexpectedly prompted instead of using the disposable pre-grant"
grep -F 'PANES' "$ROOT/candidate-before.screen" >/dev/null ||
    fail "candidate Zaphod rail was not visibly rendered in the tmux client"
jq -e --arg wasm_url "$WASM_URL" \
    'any(.[]; .is_plugin and .plugin_url == $wasm_url and .is_selectable == false)' \
    "$ROOT/candidate-before.json" >/dev/null ||
    fail "candidate baseline was captured before its pre-granted permission result settled"
[ "$(file_state "$CONFIG_FILE")" = "$ISOLATED_CONFIG_BEFORE" ] ||
    fail "direct entry changed the isolated profile config"
[ "$(file_state "$ISOLATED_LAYOUT")" = "$ISOLATED_LAYOUT_BEFORE" ] ||
    fail "direct entry changed the isolated profile layout"
find "$ROOT/tmp" -mindepth 1 -maxdepth 1 -name 'zaphod-new-tab.*' -print -quit | \
    grep -q . && fail "direct entry left its rendered-layout temporary root"

# A pre-authorized rail requests its runtime MessagePluginId route, but the
# request's return value is not authorization. One literal key must be
# received by the active tiled resident and change only the known dock shape.
[ "$(candidate_width "$ROOT/candidate-before.json")" = "28" ] ||
    fail "candidate did not begin in the known docked 28-column shape"
without_geometry "$ROOT/candidate-before.json" "$ROOT/candidate-before.identity.json"
candidate_geometry "$ROOT/candidate-before.json" "$ROOT/candidate-before.geometry.json"
send_literal "$(printf '\033/')"
wait_for_candidate_width "$ROOT/candidate-after.json" 1
# `wait_for_candidate_width` already captured the valid post-key native pane
# inventory. Do not issue a second list-panes call in the swap transition;
# v0.44 can briefly return an empty successful response while it redraws.
jq -S . "$ROOT/candidate-after.json" > "$ROOT/candidate-after.json.sorted"
zellij_session action dump-layout > "$ROOT/candidate-after.kdl"
tmux_command capture-pane -p -t "$TMUX_PANE" > "$ROOT/candidate-after.screen"
without_geometry "$ROOT/candidate-after.json" "$ROOT/candidate-after.identity.json"
candidate_geometry "$ROOT/candidate-after.json" "$ROOT/candidate-after.geometry.json"
cmp -s "$ROOT/candidate-before.identity.json" "$ROOT/candidate-after.identity.json" || {
    diff -u "$ROOT/candidate-before.identity.json" "$ROOT/candidate-after.identity.json" >&2 || true
    fail "managed Alt / replaced a pane, process, focus, or candidate identity"
}
cmp -s "$ROOT/candidate-before.geometry.json" "$ROOT/candidate-after.geometry.json" &&
    fail "managed Alt / did not change the candidate rail geometry"
cmp -s "$ROOT/candidate-before.kdl" "$ROOT/candidate-after.kdl" &&
    fail "managed Alt / did not change the native managed layout shape"
cmp -s "$ROOT/candidate-before.screen" "$ROOT/candidate-after.screen" &&
    fail "managed Alt / did not visibly change the tmux client"

# AC-O3 must run after the usable managed route has been observed. Return the
# same tmux client to its sidebar-less tab with a native Zellij action, then
# send literal Alt / and require byte-identical foreign state.
zellij_session action go-to-previous-tab
wait_for_foreign_active_tab "$ROOT/foreign-tabs-after-route.json"
capture_state "$ROOT/foreign-before.json" "$ROOT/foreign-before.kdl" "$ROOT/foreign-before.screen"
send_literal "$(printf '\033/')"
sleep 0.10
capture_state "$ROOT/foreign-after.json" "$ROOT/foreign-after.kdl" "$ROOT/foreign-after.screen"
cmp -s "$ROOT/foreign-before.json.sorted" "$ROOT/foreign-after.json.sorted" || {
    diff -u "$ROOT/foreign-before.json.sorted" "$ROOT/foreign-after.json.sorted" >&2 || true
    fail "post-route foreign Alt / changed native pane, focus, tab, or process state"
}
cmp -s "$ROOT/foreign-before.kdl" "$ROOT/foreign-after.kdl" || {
    diff -u "$ROOT/foreign-before.kdl" "$ROOT/foreign-after.kdl" >&2 || true
    fail "post-route foreign Alt / changed the native layout"
}
cmp -s "$ROOT/foreign-before.screen" "$ROOT/foreign-after.screen" || {
    diff -u "$ROOT/foreign-before.screen" "$ROOT/foreign-after.screen" >&2 || true
    fail "post-route foreign Alt / visibly changed the tmux client"
}
jq -e --arg wasm_url "$WASM_URL" \
    '([.[] | select(.is_plugin and .plugin_url == $wasm_url)] | length) == 1' \
    "$ROOT/foreign-after.json" >/dev/null || fail "foreign Alt / created or removed a candidate rail"

[ "$(file_state "$STANDING_CONFIG")" = "$STANDING_CONFIG_BEFORE" ] ||
    fail "standing Zellij config changed during tmux smoke: $STANDING_CONFIG"
[ "$(file_state "$STANDING_LAYOUT")" = "$STANDING_LAYOUT_BEFORE" ] ||
    fail "standing Zellij layout changed during tmux smoke: $STANDING_LAYOUT"

printf '%s\n' 'PASS: tmux-hosted Zellij smoke proved fresh entry, managed Alt /, post-route foreign inertness, and disposable cleanup'
