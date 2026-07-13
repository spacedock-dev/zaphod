#!/bin/bash
# ABOUTME: Proves stable-tab agent-event admission in two real Zellij rails.
# ABOUTME: Uses tmux only as the isolated terminal harness; no custom PTY or standing-config mutation.
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
SESSION_NAME=""
TMUX_SERVER=""
TMUX_SESSION="zaphod-two-rail"
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
    local original_status=$?
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
    fi
    if [ "$(file_state "$STANDING_CONFIG")" != "$STANDING_CONFIG_BEFORE" ]; then
        echo "standing Zellij config changed during two-rail smoke: $STANDING_CONFIG" >&2
        cleanup_status=1
    fi
    if [ "$(file_state "$STANDING_LAYOUT")" != "$STANDING_LAYOUT_BEFORE" ]; then
        echo "standing Zellij layout changed during two-rail smoke: $STANDING_LAYOUT" >&2
        cleanup_status=1
    fi
    exit "$((cleanup_status != 0 ? cleanup_status : original_status))"
}

trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP

for required in tmux jq shasum; do
    command -v "$required" >/dev/null 2>&1 || fail "$required is required for the tmux smoke"
done
zaphod_require_zellij_0443
"$REPO_ROOT/build.sh" >/dev/null

WASM_PATH="$REPO_ROOT/target/wasm32-wasip1/release/zellij-sidebar.wasm"
WASM_URL="$(zaphod_canonical_file_url "$WASM_PATH")" ||
    fail "could not derive candidate WASM URL"
SHARED_CWD="$(zaphod_physical_dir "$REPO_ROOT")" ||
    fail "could not derive shared terminal CWD"
ESCAPED_CWD="$(zaphod_kdl_escape "$SHARED_CWD")" ||
    fail "could not escape shared terminal CWD"

ROOT="$(mktemp -d /tmp/zr.XXXXXX)" || fail "could not create short isolated root"
CONFIG_DIR="$ROOT/config"
CONFIG_FILE="$CONFIG_DIR/config.kdl"
DATA_DIR="$ROOT/data"
SOCKET_DIR="$ROOT/socket"
HOME_DIR="$ROOT/home"
SESSION_NAME="zr$$"
TMUX_SERVER="zr$$"
mkdir -p "$CONFIG_DIR/layouts" "$DATA_DIR" "$SOCKET_DIR"
printf '%s\n' \
    'keybinds clear-defaults=true {' \
    '}' \
    'default_mode "locked"' > "$CONFIG_FILE"

# A disposable, pre-authorized cache keeps the harness noninteractive and
# never writes the operator's permission cache.
PERMISSION_CACHE="$HOME_DIR/Library/Caches/org.Zellij-Contributors.Zellij/permissions.kdl"
mkdir -p "$(dirname "$PERMISSION_CACHE")"
{
    printf '"%s" {\n' "$WASM_PATH"
    printf '%s\n' \
        '    ReadApplicationState' \
        '    ChangeApplicationState' \
        '    ReadPaneContents' \
        '    ReadCliPipes' \
        '    Reconfigure' \
        '    RunCommands'
    printf '%s\n' '}'
} > "$PERMISSION_CACHE"

printf '%s\n' \
    'layout {' \
    '    tab name="Target" {' \
    '        pane split_direction="vertical" {' \
    '            pane size=28 borderless=true {' \
    "                plugin location=\"$WASM_URL\" {" \
    '                    rail "1"' \
    '                    recipient_token "target-token"' \
    '                }' \
    '            }' \
    "            pane cwd=\"$ESCAPED_CWD\"" \
    '        }' \
    '    }' \
    '    tab name="Bystander" {' \
    '        pane split_direction="vertical" {' \
    '            pane size=28 borderless=true {' \
    "                plugin location=\"$WASM_URL\" {" \
    '                    rail "1"' \
    '                    recipient_token "target-token"' \
    '                }' \
    '            }' \
    "            pane cwd=\"$ESCAPED_CWD\"" \
    '        }' \
    '    }' \
    '}' > "$CONFIG_DIR/layouts/two-rails.kdl"

zellij_control setup --check >/dev/null

start_tmux_zellij() {
    local command
    printf -v command 'env HOME=%q ZELLIJ_SOCKET_DIR=%q %q --config-dir %q --config %q --data-dir %q --session %q --new-session-with-layout two-rails' \
        "$HOME_DIR" "$SOCKET_DIR" "$(command -v zellij)" "$CONFIG_DIR" "$CONFIG_FILE" "$DATA_DIR" "$SESSION_NAME"
    tmux_command new-session -d -x 180 -y 45 -s "$TMUX_SESSION" "$command"
}

PANES="$ROOT/panes.json"
TABS="$ROOT/tabs.json"
TARGET_SCREEN="$ROOT/target.screen"
BYSTANDER_SCREEN="$ROOT/bystander.screen"

capture_panes() {
    zellij_session action list-panes --json --all --command --geometry --state --tab > "$PANES"
}

capture_tabs() {
    zellij_session action list-tabs --json --all --state > "$TABS"
}

wait_for_two_resident_rails() {
    local attempt
    for attempt in $(seq 1 160); do
        capture_panes 2>"$ROOT/list-panes.err" || true
        capture_tabs 2>"$ROOT/list-tabs.err" || true
        if jq -e --arg wasm_url "$WASM_URL" '
            [.[] | select(.is_plugin and .plugin_url == $wasm_url and .is_floating == false and .is_suppressed == false)]
            | length == 2
        ' "$PANES" >/dev/null 2>&1 && \
            jq -e '
                any(.[]; .name == "Target") and any(.[]; .name == "Bystander")
            ' "$TABS" >/dev/null 2>&1; then
            return 0
        fi
        sleep 0.05
    done
    cat "$ROOT/list-panes.err" >&2 || true
    cat "$ROOT/list-tabs.err" >&2 || true
    fail "two resident rails did not settle"
}

wait_for_active_tab() {
    local stable_tab_id="$1" attempt
    for attempt in $(seq 1 100); do
        capture_tabs
        if jq -e --arg id "$stable_tab_id" '
            any(.[]; .active and ((.tab_id | tostring) == $id))
        ' "$TABS" >/dev/null; then
            return 0
        fi
        sleep 0.05
    done
    fail "stable tab $stable_tab_id did not become active"
}

wait_for_screen_marker() {
    local marker="$1" screen="$2" attempt
    for attempt in $(seq 1 100); do
        tmux_command capture-pane -p -t "$TMUX_PANE" > "$screen"
        grep -F "$marker" "$screen" >/dev/null && return 0
        sleep 0.05
    done
    cat "$screen" >&2 || true
    find "$ROOT" -type f -name '*zellij*.log' -exec sh -c 'echo "--- $1" >&2; tail -n 120 "$1" >&2' _ {} \; || true
    fail "active rail did not render $marker"
}

start_tmux_zellij
wait_for_two_resident_rails

TARGET_TAB_ID="$(jq -er --arg wasm_url "$WASM_URL" '
    [.[] | select(.is_plugin and .plugin_url == $wasm_url and .tab_name == "Target") | .tab_id]
    | unique
    | if length == 1 then .[0] else error("expected one Target stable tab ID") end
' "$PANES")"
BYSTANDER_TAB_ID="$(jq -er --arg wasm_url "$WASM_URL" '
    [.[] | select(.is_plugin and .plugin_url == $wasm_url and .tab_name == "Bystander") | .tab_id]
    | unique
    | if length == 1 then .[0] else error("expected one Bystander stable tab ID") end
' "$PANES")"
[ "$TARGET_TAB_ID" != "$BYSTANDER_TAB_ID" ] ||
    fail "two rails did not receive distinct stable server tab IDs"

# Deliver while the same-CWD bystander is active. Both rails deliberately
# share the private token, so the broadcast reaches both plugin instances.
# Active-tab state and later screens therefore prove the stable-tab guard.
zellij_session action go-to-tab-by-id "$BYSTANDER_TAB_ID"
wait_for_active_tab "$BYSTANDER_TAB_ID"
PAYLOAD="{\"kind\":\"session\",\"id\":\"two-rail-target\",\"cwd\":\"$SHARED_CWD\",\"agent\":\"codex\",\"state\":\"blocked\",\"summary\":\"BB_RECIPIENT_MARKER\"}"
PIPE_ACK_FILE="$ROOT/pipe-ack"
zellij_session pipe --name zaphod-agent-v1-target-token-event \
    --args "recipient-tab-id=$TARGET_TAB_ID,recipient-token=target-token" \
    -- "$PAYLOAD" > "$PIPE_ACK_FILE"
[ "$(cat "$PIPE_ACK_FILE")" = accepted ] || fail "target rail did not acknowledge the accepted row"
wait_for_active_tab "$BYSTANDER_TAB_ID"

zellij_session action go-to-tab-by-id "$TARGET_TAB_ID"
wait_for_active_tab "$TARGET_TAB_ID"
wait_for_screen_marker 'BB_RECIPIENT_MARKER' "$TARGET_SCREEN"

zellij_session action go-to-tab-by-id "$BYSTANDER_TAB_ID"
wait_for_active_tab "$BYSTANDER_TAB_ID"
tmux_command capture-pane -p -t "$TMUX_PANE" > "$BYSTANDER_SCREEN"
if grep -F 'BB_RECIPIENT_MARKER' "$BYSTANDER_SCREEN" >/dev/null; then
    cat "$BYSTANDER_SCREEN" >&2 || true
    fail "same-CWD bystander rendered the target-tab broadcast"
fi

echo "PASS: stable-tab recipient delivers only to the target rail"
