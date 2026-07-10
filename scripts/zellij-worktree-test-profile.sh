#!/bin/bash
# ABOUTME: Builds and exercises this checkout in an isolated disposable Zellij profile.
# ABOUTME: Never installs, copies, rewrites, or restores standing global Zellij files.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
# shellcheck source=scripts/zellij-layout-lib.sh
source "$SCRIPT_DIR/zellij-layout-lib.sh"

usage() {
    echo "usage: $0 --cwd PATH" >&2
    exit 2
}

if [ "$#" -ne 2 ] || [ "$1" != "--cwd" ]; then
    usage
fi
if [ ! -d "$2" ]; then
    echo "cwd does not exist: $2" >&2
    exit 1
fi
EXPLICIT_CWD="$(zaphod_physical_dir "$2")"

file_state() {
    local path="$1"
    if [ -e "$path" ]; then
        printf 'present:%s\n' "$(shasum -a 256 "$path" | awk '{print $1}')"
    else
        printf 'missing\n'
    fi
}

ZELLIJ_GLOBAL_ROOT="${ZELLIJ_CONFIG_DIR:-$HOME/.config/zellij}"
GLOBAL_CONFIG="$ZELLIJ_GLOBAL_ROOT/config.kdl"
GLOBAL_LAYOUT="$ZELLIJ_GLOBAL_ROOT/layouts/zaphod.kdl"
GLOBAL_CONFIG_BEFORE="$(file_state "$GLOBAL_CONFIG")"
GLOBAL_LAYOUT_BEFORE="$(file_state "$GLOBAL_LAYOUT")"
PROFILE_ROOT=""
SESSION_NAME=""
CLIENT_PID=""

cleanup_profile() {
    local original_status=$?
    local cleanup_status=0
    trap - EXIT INT TERM HUP
    if [ -n "$SESSION_NAME" ]; then
        zellij delete-session --force "$SESSION_NAME" >/dev/null 2>&1 || true
    fi
    if [ -n "$CLIENT_PID" ]; then
        kill "$CLIENT_PID" >/dev/null 2>&1 || true
    fi
    if [ -n "$PROFILE_ROOT" ] && [ -d "$PROFILE_ROOT" ]; then
        rm -rf "$PROFILE_ROOT"
    fi
    if [ "$(file_state "$GLOBAL_CONFIG")" != "$GLOBAL_CONFIG_BEFORE" ]; then
        echo "global Zellij config changed during isolated profile: $GLOBAL_CONFIG" >&2
        cleanup_status=1
    fi
    if [ "$(file_state "$GLOBAL_LAYOUT")" != "$GLOBAL_LAYOUT_BEFORE" ]; then
        echo "global Zaphod layout changed during isolated profile: $GLOBAL_LAYOUT" >&2
        cleanup_status=1
    fi
    if [ "$cleanup_status" -ne 0 ]; then
        exit "$cleanup_status"
    fi
    exit "$original_status"
}

trap cleanup_profile EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP

zaphod_require_zellij_0443
"$REPO_ROOT/build.sh"

WASM_PATH="$REPO_ROOT/target/wasm32-wasip1/release/zellij-sidebar.wasm"
WASM_URL="$(zaphod_canonical_file_url "$WASM_PATH")"
CANDIDATE_COMMIT="$(git -C "$REPO_ROOT" rev-parse HEAD)"
PROFILE_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-worktree-profile.XXXXXX")"
SESSION_NAME="zwp-$$-${RANDOM:-0}"
mkdir -p "$PROFILE_ROOT/config/layouts" "$PROFILE_ROOT/data"

printf '%s\n' \
    'keybinds clear-defaults=true {' \
    '    shared {' \
    '        bind "Alt /" {' \
    "            MessagePlugin \"$WASM_URL\" {" \
    '                name "toggle"' \
    '                floating true' \
    '                skip_cache true' \
    '                rail "1"' \
    '            }' \
    '        }' \
    '    }' \
    '}' \
    'default_mode "locked"' > "$PROFILE_ROOT/config/config.kdl"

zaphod_render_layout \
    "$REPO_ROOT/layouts/zaphod.kdl" \
    "$WASM_URL" \
    "$PROFILE_ROOT/config/layouts/zaphod.kdl"

ESCAPED_CWD="$(zaphod_kdl_escape "$EXPLICIT_CWD")"
printf '%s\n' \
    'layout {' \
    '    tab name="explicit-cwd" {' \
    '        pane size=1 borderless=true {' \
    '            plugin location="zellij:tab-bar"' \
    '        }' \
    "        pane cwd=\"$ESCAPED_CWD\"" \
    '        pane size=1 borderless=true {' \
    '            plugin location="zellij:status-bar"' \
    '        }' \
    '    }' \
    '}' > "$PROFILE_ROOT/config/layouts/explicit-cwd.kdl"

zellij --config-dir "$PROFILE_ROOT/config" --data-dir "$PROFILE_ROOT/data" setup --check >/dev/null
zaphod_validate_message_plugin_identity "$PROFILE_ROOT/config/config.kdl" "$WASM_URL"
zaphod_validate_layout_identity "$PROFILE_ROOT/config/layouts/zaphod.kdl" "$WASM_URL"

printf 'PROFILE_ROOT=%s\n' "$PROFILE_ROOT"
printf 'SESSION_NAME=%s\n' "$SESSION_NAME"
printf 'CANDIDATE_COMMIT=%s\n' "$CANDIDATE_COMMIT"
printf 'CANDIDATE_URL=%s\n' "$WASM_URL"
printf 'INSPECT_PANES=ZELLIJ_SESSION_NAME=%q zellij --config-dir %q --data-dir %q action list-panes --json -a -g -t\n' \
    "$SESSION_NAME" "$PROFILE_ROOT/config" "$PROFILE_ROOT/data"
printf 'INSPECT_LAYOUT=ZELLIJ_SESSION_NAME=%q zellij --config-dir %q --data-dir %q action dump-layout\n' \
    "$SESSION_NAME" "$PROFILE_ROOT/config" "$PROFILE_ROOT/data"
printf 'CLEANUP=exit the attached session or interrupt this command; the session and %s will be removed\n' "$PROFILE_ROOT"

zellij --config-dir "$PROFILE_ROOT/config" --data-dir "$PROFILE_ROOT/data" \
    --session "$SESSION_NAME" --new-session-with-layout explicit-cwd &
CLIENT_PID=$!

SESSION_SEEN=0
for _attempt in $(seq 1 100); do
    if zellij list-sessions 2>/dev/null | grep -F "$SESSION_NAME" >/dev/null; then
        SESSION_SEEN=1
        break
    fi
    if ! kill -0 "$CLIENT_PID" 2>/dev/null; then
        break
    fi
    sleep 0.1
done
if [ "$SESSION_SEEN" -ne 1 ]; then
    echo "disposable Zellij session failed to start: $SESSION_NAME" >&2
    exit 1
fi

while kill -0 "$CLIENT_PID" 2>/dev/null; do
    if ! zellij list-sessions 2>/dev/null | grep -F "$SESSION_NAME" >/dev/null; then
        break
    fi
    sleep 0.1
done

kill "$CLIENT_PID" >/dev/null 2>&1 || true
set +e
wait "$CLIENT_PID"
set -e
