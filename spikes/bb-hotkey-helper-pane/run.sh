#!/bin/bash
# ABOUTME: Reproduces Zellij 0.44.3's visible helper pane from one NewTab + Run keybind.
# ABOUTME: Uses a disposable tmux-hosted Zellij server and leaves the requested raw evidence in --out.
set -euo pipefail

SPIKE_DIR="$(cd "$(dirname "$0")" && pwd -P)"
REPO_ROOT=""
OUT_DIR=""

usage() {
    echo "usage: $0 --repo /absolute/path/to/zaphod --out /empty/output-directory" >&2
    exit 2
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --repo)
            [ "$#" -ge 2 ] || usage
            REPO_ROOT="$2"
            shift 2
            ;;
        --out)
            [ "$#" -ge 2 ] || usage
            OUT_DIR="$2"
            shift 2
            ;;
        *)
            usage
            ;;
    esac
done

[ -n "$REPO_ROOT" ] && [ -d "$REPO_ROOT" ] || usage
[ -n "$OUT_DIR" ] || usage
[ ! -e "$OUT_DIR" ] || {
    echo "output directory already exists: $OUT_DIR" >&2
    exit 2
}

for required in jq tmux zellij; do
    command -v "$required" >/dev/null 2>&1 || {
        echo "missing required command: $required" >&2
        exit 2
    }
done
[ "$(zellij --version)" = "zellij 0.44.3" ] || {
    echo "zellij 0.44.3 is required; found $(zellij --version)" >&2
    exit 2
}

ROOT=""
SESSION=""
TMUX_SERVER=""
KEEP_ROOT="${KEEP_ROOT:-0}"
cleanup() {
    local status=$?
    trap - EXIT INT TERM HUP
    set +e
    if [ -n "$SESSION" ]; then
        env ZELLIJ_SOCKET_DIR="$ROOT/socket" zellij \
            --config-dir "$ROOT/config" --config "$ROOT/config/config.kdl" \
            --data-dir "$ROOT/data" delete-session --force "$SESSION" >/dev/null 2>&1 || true
    fi
    if [ -n "$TMUX_SERVER" ]; then
        tmux -L "$TMUX_SERVER" kill-server >/dev/null 2>&1 || true
    fi
    if [ -n "$ROOT" ] && [ -d "$ROOT" ]; then
        if [ "$KEEP_ROOT" = "1" ]; then
            echo "retained isolated spike root: $ROOT" >&2
        else
            rm -rf "$ROOT"
        fi
    fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP

ROOT="$(mktemp -d /tmp/bb-hotkey.XXXXXX)"
SESSION="bb-hotkey-$$"
TMUX_SERVER="bb-hotkey-$$"
mkdir -p "$ROOT/config" "$ROOT/data" "$ROOT/socket" "$ROOT/home"

LAYOUT="$ROOT/candidate.kdl"
cp "$SPIKE_DIR/fixture-layout.kdl" "$LAYOUT"

CONFIG="$ROOT/config/config.kdl"
printf '%s\n' \
    'keybinds clear-defaults=true {' \
    '    shared {' \
    '        bind "Alt Shift z" {' \
    "            NewTab { layout \"$LAYOUT\"; }" \
    '            Run "sh" "-c" "sleep 30" {' \
    '                floating true' \
    '                name "bb-helper"' \
    '            }' \
    '        }' \
    '    }' \
    '}' > "$CONFIG"

printf '%s\n' \
    "zellij=$(command -v zellij)" \
    "zellij_version=$(zellij --version)" \
    "tmux=$(command -v tmux)" \
    "layout=$LAYOUT" \
    "key=Alt Shift z (bytes: ESC Z)" > "$ROOT/command.txt"

start_command="env HOME=$(printf %q "$ROOT/home") ZELLIJ_SOCKET_DIR=$(printf %q "$ROOT/socket") $(printf %q "$(command -v zellij)") --config-dir $(printf %q "$ROOT/config") --config $(printf %q "$CONFIG") --data-dir $(printf %q "$ROOT/data") attach --create $(printf %q "$SESSION")"
tmux -L "$TMUX_SERVER" new-session -d -x 160 -y 45 -s spike "$start_command"

zellij_control() {
    env ZELLIJ_SOCKET_DIR="$ROOT/socket" zellij \
        --config-dir "$ROOT/config" --config "$CONFIG" --data-dir "$ROOT/data" \
        --session "$SESSION" "$@"
}

for attempt in $(seq 1 160); do
    if zellij_control action list-panes --json --all --command --geometry --state --tab \
        > "$ROOT/before.json" 2> "$ROOT/list.err" && jq -e 'length > 0' "$ROOT/before.json" >/dev/null; then
        break
    fi
    sleep 0.05
done
jq -e 'length > 0' "$ROOT/before.json" >/dev/null || {
    cat "$ROOT/list.err" >&2 || true
    echo "Zellij session did not become ready" >&2
    exit 1
}

# Zellij's release-notes float is unrelated to the binding under test. Dismiss
# it before taking the baseline so one post-key visible float is attributable
# to `Run`, not startup chrome.
tmux -L "$TMUX_SERVER" send-keys -t spike:0.0 Escape
for attempt in $(seq 1 80); do
    zellij_control action list-panes --json --all --command --geometry --state --tab \
        > "$ROOT/before.json" 2> "$ROOT/list.err" || true
    if jq -e 'all(.[]; .plugin_url != "about" or .is_suppressed == true)' \
        "$ROOT/before.json" >/dev/null 2>&1; then
        break
    fi
    sleep 0.05
done
jq -e 'all(.[]; .plugin_url != "about" or .is_suppressed == true)' "$ROOT/before.json" >/dev/null || {
    echo "Zellij startup float did not dismiss" >&2
    exit 1
}

tmux -L "$TMUX_SERVER" send-keys -l -t spike:0.0 -- "$(printf '\033Z')"
for attempt in $(seq 1 160); do
    zellij_control action list-panes --json --all --command --geometry --state --tab \
        > "$ROOT/after.json" 2> "$ROOT/after.err" || true
    if jq -e '[.[] | select(.title == "bb-helper" and .is_floating == true and .is_suppressed == false and .is_focused == true)] | length == 1' \
        "$ROOT/after.json" >/dev/null 2>&1; then
        break
    fi
    sleep 0.05
done

jq -e '[.[] | select(.title == "bb-helper" and .is_floating == true and .is_suppressed == false and .is_focused == true)] | length == 1' \
    "$ROOT/after.json" >/dev/null || {
    cat "$ROOT/after.err" >&2 || true
    echo "expected exactly one visible floating helper pane" >&2
    exit 1
}
zellij_control action list-tabs --json --all --state --layout > "$ROOT/tabs.json"
tmux -L "$TMUX_SERVER" capture-pane -p -t spike:0.0 > "$ROOT/screen.txt"

HELPER_COUNT="$(jq '[.[] | select(.title == "bb-helper" and .is_floating == true and .is_suppressed == false)] | length' "$ROOT/after.json")"
HELPER="$(jq -c '[.[] | select(.title == "bb-helper" and .is_floating == true and .is_suppressed == false)] | .[0] | {id, is_floating, is_suppressed, is_focused, tab_id, tab_name, title, pane_command, terminal_command}' "$ROOT/after.json")"
printf '%s\n' \
    'RESULT=helper-pane-incompatible' \
    "HELPER_PANE_COUNT=$HELPER_COUNT" \
    "HELPER=$HELPER" > "$ROOT/result.txt"

mkdir -p "$OUT_DIR"
cp "$ROOT/command.txt" "$OUT_DIR/command.txt"
cp "$LAYOUT" "$OUT_DIR/fixture-layout.kdl"
cp "$CONFIG" "$OUT_DIR/config.kdl"
cp "$ROOT/before.json" "$OUT_DIR/before.json"
cp "$ROOT/after.json" "$OUT_DIR/after.json"
cp "$ROOT/tabs.json" "$OUT_DIR/tabs.json"
cp "$ROOT/screen.txt" "$OUT_DIR/screen.txt"
cp "$ROOT/result.txt" "$OUT_DIR/result.txt"
printf 'PASS: %s\n' "$(tr '\n' ' ' < "$ROOT/result.txt")"
