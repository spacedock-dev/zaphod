#!/bin/bash
# ABOUTME: One disposable Zellij 0.44.3/tmux proof for bb receiver isolation.
# ABOUTME: Targets by native profile/session/tab/plugin-pane identity; CWD is only the adversary.
set -euo pipefail

SPIKE_DIR="$(cd "$(dirname "$0")" && pwd -P)"
REPO_ROOT=""
OUT_DIR=""

usage() {
    echo "usage: $0 --repo /absolute/path/to/zaphod --out /new/output-directory" >&2
    exit 2
}
while [ "$#" -gt 0 ]; do
    case "$1" in
        --repo) [ "$#" -ge 2 ] || usage; REPO_ROOT="$2"; shift 2 ;;
        --out) [ "$#" -ge 2 ] || usage; OUT_DIR="$2"; shift 2 ;;
        *) usage ;;
    esac
done
[ -d "$REPO_ROOT" ] && [ -n "$OUT_DIR" ] && [ ! -e "$OUT_DIR" ] || usage
for cmd in jq tmux zellij; do command -v "$cmd" >/dev/null || { echo "missing $cmd" >&2; exit 2; }; done
[ "$(zellij --version)" = "zellij 0.44.3" ] || { echo "requires zellij 0.44.3" >&2; exit 2; }

ROOT=""
SESSION=""
TMUX_SERVER=""
TMUX_SESSION="bb-two-rail"
CONFIG_DIR=""
CONFIG_FILE=""
DATA_DIR=""
SOCKET_DIR=""
HOME_DIR=""
fail() { echo "FAIL: $*" >&2; exit 1; }
zellij_control() {
    env HOME="$HOME_DIR" ZELLIJ_SOCKET_DIR="$SOCKET_DIR" zellij \
        --config-dir "$CONFIG_DIR" --config "$CONFIG_FILE" --data-dir "$DATA_DIR" "$@"
}
zellij_session() {
    env HOME="$HOME_DIR" ZELLIJ_SOCKET_DIR="$SOCKET_DIR" zellij --session "$SESSION" \
        --config-dir "$CONFIG_DIR" --config "$CONFIG_FILE" --data-dir "$DATA_DIR" "$@"
}
cleanup() {
    local status=$?
    trap - EXIT INT TERM HUP
    set +e
    [ -z "$SESSION" ] || zellij_control delete-session --force "$SESSION" >/dev/null 2>&1 || true
    [ -z "$TMUX_SERVER" ] || tmux -L "$TMUX_SERVER" kill-server >/dev/null 2>&1 || true
    [ -z "$ROOT" ] || rm -rf "$ROOT"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP

ROOT="$(mktemp -d /tmp/bbri.XXXXXX)" || fail "cannot create disposable root"
SESSION="bbri-$$"
TMUX_SERVER="bbri-$$"
CONFIG_DIR="$ROOT/config"
CONFIG_FILE="$CONFIG_DIR/config.kdl"
DATA_DIR="$ROOT/data"
SOCKET_DIR="$ROOT/socket"
HOME_DIR="$ROOT/home"
SAME_CWD="$ROOT/same-cwd"
mkdir -p "$CONFIG_DIR" "$DATA_DIR" "$SOCKET_DIR" "$HOME_DIR" "$SAME_CWD"
printf '%s\n' 'keybinds clear-defaults=true {}' > "$CONFIG_FILE"

"$REPO_ROOT/build.sh" >/dev/null
WASM_PATH="$(cd "$REPO_ROOT/target/wasm32-wasip1/release" && pwd -P)/zellij-sidebar.wasm"
[ -f "$WASM_PATH" ] || fail "built WASM missing"
WASM_URL="file:$WASM_PATH"
LAYOUT="$ROOT/fixture.kdl"
sed -e "s|__WASM_URL__|$WASM_URL|g" -e "s|__SAME_CWD__|$SAME_CWD|g" \
    "$SPIKE_DIR/fixture-layout.kdl" > "$LAYOUT"

PERMISSIONS="$HOME_DIR/Library/Caches/org.Zellij-Contributors.Zellij/permissions.kdl"
mkdir -p "$(dirname "$PERMISSIONS")"
{
    printf '"%s" {\n' "$WASM_PATH"
    printf '%s\n' '    ReadApplicationState' '    ChangeApplicationState' '    ReadPaneContents' '    Reconfigure' '    RunCommands'
    printf '%s\n' '}'
} > "$PERMISSIONS"

command=""
printf -v command 'env HOME=%q ZELLIJ_SOCKET_DIR=%q %q --config-dir %q --config %q --data-dir %q --layout %q attach --create %q' \
    "$HOME_DIR" "$SOCKET_DIR" "$(command -v zellij)" "$CONFIG_DIR" "$CONFIG_FILE" "$DATA_DIR" "$LAYOUT" "$SESSION"
tmux -L "$TMUX_SERVER" new-session -d -x 160 -y 45 -s "$TMUX_SESSION" "$command"

wait_for_rails() {
    local output="$1" expected="$2" attempt
    for attempt in $(seq 1 200); do
        zellij_session action list-panes --json --all --command --geometry --state --tab > "$output" 2>"$ROOT/list.err" || true
        if jq -e --arg url "$WASM_URL" --argjson expected "$expected" '
            [.[] | select(.is_plugin and .plugin_url == $url and .is_floating == false and .is_suppressed == false and .is_selectable == false)]
            | length == $expected and ([.[].tab_id] | unique | length == $expected)
        ' "$output" >/dev/null 2>&1; then return; fi
        sleep 0.05
    done
    cat "$ROOT/list.err" >&2 || true
    fail "expected resident rails did not settle: $expected"
}
wait_for_active_position() {
    local position="$1" attempt
    for attempt in $(seq 1 120); do
        zellij_session action list-tabs --json --all --state --layout > "$ROOT/tabs-live.json" 2>"$ROOT/tabs.err" || true
        if jq -e --argjson position "$position" 'any(.[]; .active and .position == $position)' "$ROOT/tabs-live.json" >/dev/null 2>&1; then return; fi
        sleep 0.05
    done
    fail "tab position did not become active: $position"
}
capture_tab() {
    local position="$1" label="$2"
    zellij_session action list-tabs --json --all --state --layout > "$ROOT/current-tabs.json"
    local current_position
    current_position="$(jq -er '[.[] | select(.active)] | if length == 1 then .[0].position else error("active tab") end' "$ROOT/current-tabs.json")"
    if [ "$current_position" != "$position" ]; then
        # With exactly two fixture tabs, the documented next-tab action is a
        # deterministic switch and avoids treating a stable tab ID as an index.
        zellij_session action go-to-next-tab
    fi
    wait_for_active_position "$position"
    tmux -L "$TMUX_SERVER" capture-pane -p -t "$TMUX_SESSION:0.0" > "$ROOT/$label.screen"
}

wait_for_rails "$ROOT/panes-before.json" 1
zellij_session action new-tab --layout "$LAYOUT" --cwd "$SAME_CWD" --name recipient-bystander > "$ROOT/new-tab.out"
wait_for_rails "$ROOT/panes-ready.json" 2
zellij_session action list-tabs --json --all --state --layout > "$ROOT/tabs-ready.json"
zellij_session action dump-layout > "$ROOT/dump-layout.kdl"

NEW_TAB_ID="$(tr -d '[:space:]' < "$ROOT/new-tab.out")"
case "$NEW_TAB_ID" in *[!0-9]*|'') fail "new-tab did not return one numeric tab ID" ;; esac
TARGET_RAIL_ID="$(jq -er --arg url "$WASM_URL" --argjson new_tab "$NEW_TAB_ID" '[.[] | select(.is_plugin and .plugin_url == $url and .tab_id != $new_tab)] | if length == 1 then .[0].id else error("target rail") end' "$ROOT/panes-ready.json")"
BYSTANDER_RAIL_ID="$(jq -er --arg url "$WASM_URL" --argjson new_tab "$NEW_TAB_ID" '[.[] | select(.is_plugin and .plugin_url == $url and .tab_id == $new_tab)] | if length == 1 then .[0].id else error("bystander rail") end' "$ROOT/panes-ready.json")"
TARGET_TAB_ID="$(jq -er --argjson rail "$TARGET_RAIL_ID" '[.[] | select(.id == $rail)] | .[0].tab_id' "$ROOT/panes-ready.json")"
BYSTANDER_TAB_ID="$(jq -er --argjson rail "$BYSTANDER_RAIL_ID" '[.[] | select(.id == $rail)] | .[0].tab_id' "$ROOT/panes-ready.json")"
TARGET_TAB_POSITION="$(jq -er --argjson rail "$TARGET_RAIL_ID" '[.[] | select(.id == $rail)] | .[0].tab_position' "$ROOT/panes-ready.json")"
BYSTANDER_TAB_POSITION="$(jq -er --argjson rail "$BYSTANDER_RAIL_ID" '[.[] | select(.id == $rail)] | .[0].tab_position' "$ROOT/panes-ready.json")"
[ "$TARGET_RAIL_ID" != "$BYSTANDER_RAIL_ID" ] && [ "$TARGET_TAB_ID" != "$BYSTANDER_TAB_ID" ] || fail "identity did not separate two rails"

PAYLOAD="{\"kind\":\"session\",\"id\":\"bb-recipient-isolation\",\"cwd\":\"$SAME_CWD\",\"agent\":\"fixture\",\"state\":\"working\",\"summary\":\"BB_RECIPIENT_MARKER\",\"ts\":\"2026-07-13T00:00:00Z\"}"
printf 'zellij pipe --name agent-event --args recipient-pane-id=%s -- %q\n' "$TARGET_RAIL_ID" "$PAYLOAD" > "$ROOT/pipe.argv"
zellij_session pipe --name agent-event --args "recipient-pane-id=$TARGET_RAIL_ID" -- "$PAYLOAD" > "$ROOT/pipe.stdout" 2>"$ROOT/pipe.stderr"
sleep 0.20
capture_tab "$TARGET_TAB_POSITION" target
capture_tab "$BYSTANDER_TAB_POSITION" bystander
find "$ROOT" -type f -name zellij.log -print > "$ROOT/log-paths.txt"
if [ -s "$ROOT/log-paths.txt" ]; then while IFS= read -r f; do cat "$f"; done < "$ROOT/log-paths.txt" > "$ROOT/zellij.log"; else : > "$ROOT/zellij.log"; fi

grep -F "$SAME_CWD" "$ROOT/dump-layout.kdl" >/dev/null || fail "native dump did not retain same-CWD terminal fixture"
TARGET_TRACE=false
BYSTANDER_TRACE=false
grep -F "zaphod-trace[$TARGET_RAIL_ID]: pipe recv name=agent-event" "$ROOT/zellij.log" >/dev/null && TARGET_TRACE=true || true
grep -F "zaphod-trace[$BYSTANDER_RAIL_ID]: pipe recv name=agent-event" "$ROOT/zellij.log" >/dev/null && BYSTANDER_TRACE=true || true
grep -F BB_RECIPIENT_MARKER "$ROOT/target.screen" >/dev/null || fail "target rail did not render addressed row"
TARGET_ROWS="$(grep -c BB_RECIPIENT_MARKER "$ROOT/target.screen" || true)"
BYSTANDER_ROWS="$(grep -c BB_RECIPIENT_MARKER "$ROOT/bystander.screen" || true)"
if [ "$BYSTANDER_ROWS" -eq 0 ]; then RESULT=PASS; else RESULT=FAIL; fi

mkdir -p "$OUT_DIR"
cp "$LAYOUT" "$ROOT/panes-before.json" "$ROOT/panes-ready.json" "$ROOT/tabs-ready.json" "$ROOT/dump-layout.kdl" "$OUT_DIR/"
cp "$ROOT/new-tab.out" "$ROOT/pipe.argv" "$ROOT/pipe.stdout" "$ROOT/pipe.stderr" "$ROOT/target.screen" "$ROOT/bystander.screen" "$ROOT/log-paths.txt" "$ROOT/zellij.log" "$OUT_DIR/"
{
    printf 'RESULT=%s\n' "$RESULT"
    printf 'ZELLIJ_VERSION=%s\n' "$(zellij --version)"
    printf 'PROFILE_ROOT=%s\n' "$ROOT"
    printf 'SESSION=%s\n' "$SESSION"
    printf 'SAME_CWD=%s\n' "$SAME_CWD"
    printf 'WASM_URL=%s\n' "$WASM_URL"
    printf 'TARGET_TAB_ID=%s\nTARGET_RAIL_ID=%s\n' "$TARGET_TAB_ID" "$TARGET_RAIL_ID"
    printf 'BYSTANDER_TAB_ID=%s\nBYSTANDER_RAIL_ID=%s\n' "$BYSTANDER_TAB_ID" "$BYSTANDER_RAIL_ID"
    printf 'TARGET_TAB_POSITION=%s\nBYSTANDER_TAB_POSITION=%s\n' "$TARGET_TAB_POSITION" "$BYSTANDER_TAB_POSITION"
    printf 'NEW_TAB_ID=%s\n' "$NEW_TAB_ID"
    printf 'TARGET_MARKER_LINES=%s\nBYSTANDER_MARKER_LINES=%s\n' "$TARGET_ROWS" "$BYSTANDER_ROWS"
    printf 'PIPE_USES_PLUGIN=false\nTARGET_IDENTITY_TRACE=%s\nBYSTANDER_IDENTITY_TRACE=%s\n' "$TARGET_TRACE" "$BYSTANDER_TRACE"
} > "$OUT_DIR/result.txt"
printf '%s\n' "$RESULT"
