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
AGENTSVIEW_PID=""
TARGET_SIDECAR_PID=""
BYSTANDER_SIDECAR_PID=""

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
	for owned_pid in "$TARGET_SIDECAR_PID" "$BYSTANDER_SIDECAR_PID" "$AGENTSVIEW_PID"; do
		[ -z "$owned_pid" ] || kill -TERM "$owned_pid" 2>/dev/null || true
	done
	for owned_pid in "$TARGET_SIDECAR_PID" "$BYSTANDER_SIDECAR_PID" "$AGENTSVIEW_PID"; do
		[ -z "$owned_pid" ] || wait "$owned_pid" 2>/dev/null || true
	done
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
    if [ "${ZAPHOD_KEEP_TEST_ROOT:-}" != 1 ] && [ -n "$ROOT" ] && [ -d "$ROOT" ]; then
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
REGISTRY_DIR="$ROOT/registry"
SOCKET_DIR="$ROOT/socket"
HOME_DIR="$ROOT/home"
SESSION_NAME="zr$$"
TMUX_SERVER="zr$$"
mkdir -p "$CONFIG_DIR/layouts" "$DATA_DIR" "$REGISTRY_DIR" "$SOCKET_DIR"
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
    '                    recipient_token "bystander-token"' \
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

TARGET_PANE_ID="$(jq -er '
    [.[] | select((.tab_id | tostring) == "'"$TARGET_TAB_ID"'" and (.is_plugin | not) and .is_selectable and (.is_suppressed | not)) | .id]
    | if length == 1 then .[0] else error("expected one Target terminal") end
' "$PANES")"
BYSTANDER_PANE_ID="$(jq -er '
    [.[] | select((.tab_id | tostring) == "'"$BYSTANDER_TAB_ID"'" and (.is_plugin | not) and .is_selectable and (.is_suppressed | not)) | .id]
    | if length == 1 then .[0] else error("expected one Bystander terminal") end
' "$PANES")"

register_session() {
	local pane_id="$1" session_id="$2"
	printf '%s\n' "{\"session_id\":\"$session_id\",\"transcript_path\":null,\"cwd\":\"/same/cwd\",\"hook_event_name\":\"SessionStart\",\"model\":\"fixture\",\"permission_mode\":\"default\",\"source\":\"startup\"}" |
		ZAPHOD_REGISTRY_DIR="$REGISTRY_DIR" ZELLIJ_SESSION_NAME="$SESSION_NAME" ZELLIJ_PANE_ID="$pane_id" \
		"$REPO_ROOT/target/zaphod" register-agent-session
}
register_session "$TARGET_PANE_ID" 019f5f94-a596-7d92-9928-398653669161
register_session "$BYSTANDER_PANE_ID" 019f5f95-bbfd-7993-8620-0d698008217f

go build -o "$ROOT/registered-agentsview-fixture" "$SCRIPT_DIR/helpers/registered-agentsview-fixture.go"
"$ROOT/registered-agentsview-fixture" --ready-file "$ROOT/agentsview-url" --request-log "$ROOT/agentsview-requests.log" &
AGENTSVIEW_PID=$!
for _attempt in $(seq 1 100); do
	[ ! -s "$ROOT/agentsview-url" ] || break
	kill -0 "$AGENTSVIEW_PID" 2>/dev/null || break
	sleep 0.05
done
[ -s "$ROOT/agentsview-url" ] || fail "registered AgentsView fixture did not become ready"
AGENTSVIEW_URL="$(cat "$ROOT/agentsview-url")"

start_sidecar() {
	local tab_id="$1" label="$2" recipient_token="$3" fifo log startup_message startup_status
	fifo="$ROOT/$label.start"
	log="$ROOT/$label.log"
	mkfifo "$fifo"
	ZELLIJ_SOCKET_DIR="$SOCKET_DIR" "$REPO_ROOT/target/zaphod" subscribe \
		--server "$AGENTSVIEW_URL" \
		--zellij-bin "$(command -v zellij)" \
		--zellij-config-dir "$CONFIG_DIR" \
		--zellij-config "$CONFIG_FILE" \
		--zellij-data-dir "$DATA_DIR" \
		--zellij-session "$SESSION_NAME" \
		--tab-id "$tab_id" \
		--rail-url "$WASM_URL" \
		--checkout-cwd "$SHARED_CWD" \
		--recipient-token "$recipient_token" \
		--registry-dir "$REGISTRY_DIR" \
		--startup-fd 3 \
		3>"$fifo" >"$log" 2>&1 &
	STARTED_SIDECAR_PID=$!
	set +e
	IFS= read -r -t 10 startup_message < "$fifo"
	startup_status=$?
	set -e
	rm -f "$fifo"
	[ "$startup_status" -eq 0 ] && [ "$startup_message" = ready ] || {
		echo "debug root: $ROOT" >&2
		cat "$log" >&2 || true
		fail "$label sidecar did not become ready"
	}
}
start_sidecar "$TARGET_TAB_ID" target-sidecar target-token
TARGET_SIDECAR_PID="$STARTED_SIDECAR_PID"
zellij_session action go-to-tab-by-id "$BYSTANDER_TAB_ID"
wait_for_active_tab "$BYSTANDER_TAB_ID"
start_sidecar "$BYSTANDER_TAB_ID" bystander-sidecar bystander-token
BYSTANDER_SIDECAR_PID="$STARTED_SIDECAR_PID"

zellij_session action go-to-tab-by-id "$TARGET_TAB_ID"
wait_for_active_tab "$TARGET_TAB_ID"
wait_for_screen_marker 'KJ_TAB_A_ROW' "$TARGET_SCREEN"
grep -F 'KJ_TAB_B_ROW' "$TARGET_SCREEN" >/dev/null && fail "Target rendered Bystander's exact session"
grep -F 'KJ_CHILD_ROW' "$TARGET_SCREEN" >/dev/null && fail "Target rendered the unregistered child"

zellij_session action go-to-tab-by-id "$BYSTANDER_TAB_ID"
wait_for_active_tab "$BYSTANDER_TAB_ID"
wait_for_screen_marker 'KJ_TAB_B_ROW' "$BYSTANDER_SCREEN"
grep -F 'KJ_TAB_A_ROW' "$BYSTANDER_SCREEN" >/dev/null && fail "Bystander rendered Target's exact session"
grep -F 'KJ_CHILD_ROW' "$BYSTANDER_SCREEN" >/dev/null && fail "Bystander rendered the unregistered child"

printf '[]' | zellij_session pipe --name zaphod-agent-v1-target-token-snapshot \
	--args "recipient-tab-id=$TARGET_TAB_ID,recipient-token=target-token" > "$ROOT/clear-ack"
[ "$(cat "$ROOT/clear-ack")" = accepted ] || fail "Target did not acknowledge the empty snapshot"
zellij_session action go-to-tab-by-id "$TARGET_TAB_ID"
wait_for_active_tab "$TARGET_TAB_ID"
for _attempt in $(seq 1 100); do
	tmux_command capture-pane -p -t "$TMUX_PANE" > "$ROOT/target-cleared.screen"
	grep -F 'KJ_TAB_A_ROW' "$ROOT/target-cleared.screen" >/dev/null || break
	sleep 0.05
done
grep -F 'KJ_TAB_A_ROW' "$ROOT/target-cleared.screen" >/dev/null && fail "empty snapshot left a stale Target row"
kill -TERM "$TARGET_SIDECAR_PID"
wait "$TARGET_SIDECAR_PID" 2>/dev/null || true
TARGET_SIDECAR_PID=""
start_sidecar "$TARGET_TAB_ID" target-restart target-token
TARGET_SIDECAR_PID="$STARTED_SIDECAR_PID"
wait_for_screen_marker 'KJ_TAB_A_ROW' "$ROOT/target-rehydrated.screen"

grep -Fx '/api/v1/sessions' "$ROOT/agentsview-requests.log" >/dev/null && fail "sidecar used the global session list"
grep -F '019f5f95-cc22-77d2-9c3a-271b1edaabd8' "$ROOT/agentsview-requests.log" >/dev/null && fail "sidecar fetched the unregistered child"
[ "$(grep -Fc '/api/v1/sessions/codex:019f5f94-a596-7d92-9928-398653669161' "$ROOT/agentsview-requests.log")" -eq 2 ] || fail "Target exact ID was not fetched once per sidecar generation"
[ "$(grep -Fc '/api/v1/sessions/codex:019f5f95-bbfd-7993-8620-0d698008217f' "$ROOT/agentsview-requests.log")" -eq 1 ] || fail "Bystander exact ID was not fetched once"

echo "PASS: two same-CWD tabs projected 1/1/0 and sidecar restart rehydrated one exact row"
