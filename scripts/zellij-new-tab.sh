#!/bin/bash
# ABOUTME: Creates one fresh managed Zellij tab from the selected checkout.
# ABOUTME: Leaves the operator's standing config and layout files unchanged.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
# shellcheck source=scripts/zellij-layout-lib.sh
source "$SCRIPT_DIR/zellij-layout-lib.sh"

usage() {
    cat >&2 <<EOF
usage: $0 [--session NAME] [--name NAME] [--agentsview-url URL]

Create one fresh Zaphod tab in NAME (or \$ZELLIJ_SESSION_NAME).
EOF
    exit 2
}

fail() {
    echo "$*" >&2
    exit 1
}

SESSION_NAME="${ZELLIJ_SESSION_NAME:-}"
TAB_NAME="Zaphod"
AGENTSVIEW_URL="${ZAPHOD_AGENTSVIEW_URL:-http://127.0.0.1:8080}"
while [ "$#" -gt 0 ]; do
    case "$1" in
        --session)
            [ "$#" -ge 2 ] || usage
            SESSION_NAME="$2"
            shift 2
            ;;
        --name)
            [ "$#" -ge 2 ] || usage
            TAB_NAME="$2"
            shift 2
            ;;
        --agentsview-url)
            [ "$#" -ge 2 ] || usage
            AGENTSVIEW_URL="$2"
            shift 2
            ;;
        --help|-h)
            usage
            ;;
        *)
            usage
            ;;
    esac
done

[ -n "$SESSION_NAME" ] ||
    fail "a Zellij session is required; pass --session NAME or set ZELLIJ_SESSION_NAME"

ZELLIJ_ROOT="${ZELLIJ_CONFIG_DIR:-$HOME/.config/zellij}"
CONFIG_FILE="${ZELLIJ_CONFIG_FILE:-$ZELLIJ_ROOT/config.kdl}"
default_zellij_data_dir() {
    case "$(uname -s)" in
        Darwin)
            printf '%s\n' "$HOME/Library/Application Support/org.Zellij-Contributors.Zellij"
            ;;
        *)
            printf '%s\n' "${XDG_DATA_HOME:-$HOME/.local/share}/zellij"
            ;;
    esac
}

DATA_DIR="${ZELLIJ_DATA_DIR:-$(default_zellij_data_dir)}"
ZELLIJ_BIN="${ZELLIJ_BIN:-zellij}"

ZELLIJ_ARGS=(--config-dir "$ZELLIJ_ROOT" --config "$CONFIG_FILE")
ZELLIJ_ARGS+=(--data-dir "$DATA_DIR")

zellij_cmd() {
    "$ZELLIJ_BIN" "${ZELLIJ_ARGS[@]}" "$@"
}

TEMP_ROOT=""
SIDECAR_START_FIFO=""

cleanup() {
    local original_status=$?
    local cleanup_status=0
    trap - EXIT INT TERM HUP
    set +e
    [ -z "$SIDECAR_START_FIFO" ] || rm -f "$SIDECAR_START_FIFO" || cleanup_status=1
    [ -z "$TEMP_ROOT" ] || rm -rf "$TEMP_ROOT" || cleanup_status=1
    if [ "$cleanup_status" -ne 0 ]; then
        echo "failed to clean up the temporary Zaphod layout" >&2
        exit 1
    fi
    exit "$original_status"
}

trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP

if ! command -v "$ZELLIJ_BIN" >/dev/null 2>&1; then
    fail "zellij 0.44.3 is required"
fi
command -v jq >/dev/null 2>&1 ||
    fail "jq is required to verify the fresh managed tab"
VERSION="$(zellij_cmd --version)" || exit $?
if [ "$VERSION" != "zellij 0.44.3" ]; then
    fail "zellij 0.44.3 is required; found $VERSION"
fi
[ -f "$CONFIG_FILE" ] || fail "Zaphod config not found: $CONFIG_FILE"
[ -f "$REPO_ROOT/layouts/zaphod.kdl" ] || fail "Zaphod layout template not found: $REPO_ROOT/layouts/zaphod.kdl"

zellij_cmd setup --check >/dev/null
"$REPO_ROOT/build.sh"

WASM_PATH="$REPO_ROOT/target/wasm32-wasip1/release/zellij-sidebar.wasm"
[ -f "$WASM_PATH" ] || fail "wasm not found after build: $WASM_PATH"
SIDECAR_PATH="$REPO_ROOT/target/zaphod"
[ -x "$SIDECAR_PATH" ] || fail "zaphod sidecar not found after build: $SIDECAR_PATH"
WASM_URL="$(zaphod_canonical_file_url "$WASM_PATH")" ||
    fail "could not derive a canonical URL for $WASM_PATH"

sidecar_target_ready() {
    local panes candidate_count
    panes="$(ZELLIJ_SESSION_NAME="$SESSION_NAME" zellij_cmd --session "$SESSION_NAME" \
        action list-panes --json --all --command --geometry --state --tab 2>/dev/null)" ||
        return 1
    candidate_count="$(printf '%s' "$panes" | jq -er \
        --arg tab_id "$TAB_ID" \
        --arg wasm_url "$WASM_URL" \
        '[.[] | select(
            ((.tab_id | tostring) == $tab_id)
            and .is_plugin == true
            and .plugin_url == $wasm_url
            and .is_floating == false
            and .is_suppressed == false
        )] | length' 2>/dev/null)" || return 1
    [ "$candidate_count" = "1" ]
}

wait_for_sidecar_target() {
    local attempt
    for attempt in $(seq 1 80); do
        sidecar_target_ready && return 0
        sleep 0.05
    done
    return 1
}

start_private_sidecar() {
    local start_status startup_message startup_status
    mkdir -p "$DATA_DIR" ||
        fail "sidecar-start-failed: could not create private sidecar directory"
    SIDECAR_LOG="$(mktemp "$DATA_DIR/zaphod-sidecar.XXXXXX")" ||
        fail "sidecar-start-failed: could not create private sidecar log"
    SIDECAR_START_FIFO="$(mktemp "$DATA_DIR/zaphod-sidecar-start.XXXXXX")" ||
        fail "sidecar-start-failed: could not create private sidecar startup path"
    rm -f "$SIDECAR_START_FIFO"
    mkfifo "$SIDECAR_START_FIFO" ||
        fail "sidecar-start-failed: could not create private sidecar startup path"
    set +e
    nohup "$SIDECAR_PATH" subscribe \
        --server "$AGENTSVIEW_URL" \
        --zellij-bin "$ZELLIJ_BIN" \
        --zellij-config-dir "$ZELLIJ_ROOT" \
        --zellij-config "$CONFIG_FILE" \
        --zellij-data-dir "$DATA_DIR" \
        --zellij-session "$SESSION_NAME" \
        --tab-id "$TAB_ID" \
        --rail-url "$WASM_URL" \
        --startup-fd 3 \
        3>"$SIDECAR_START_FIFO" </dev/null >>"$SIDECAR_LOG" 2>&1 &
    start_status=$?
    set -e
    if [ "$start_status" -ne 0 ]; then
        fail "sidecar-start-failed: could not launch private zaphod sidecar"
    fi
    set +e
    IFS= read -r -t 2 startup_message < "$SIDECAR_START_FIFO"
    startup_status=$?
    set -e
    rm -f "$SIDECAR_START_FIFO"
    SIDECAR_START_FIFO=""
    if [ "$startup_status" -ne 0 ] || [ "$startup_message" != "ready" ]; then
        fail "sidecar-start-failed: private zaphod sidecar did not exec"
    fi
}

TEMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-new-tab.XXXXXX")" ||
    fail "could not create a temporary Zaphod layout directory"
RENDERED_LAYOUT="$TEMP_ROOT/zaphod.kdl"
zaphod_render_layout "$REPO_ROOT/layouts/zaphod.kdl" "$WASM_URL" "$RENDERED_LAYOUT"
zaphod_validate_layout_identity "$RENDERED_LAYOUT" "$WASM_URL"

TAB_ID="$(ZELLIJ_SESSION_NAME="$SESSION_NAME" zellij_cmd --session "$SESSION_NAME" action new-tab \
    --name "$TAB_NAME" --layout-string "$(cat "$RENDERED_LAYOUT")")"
if ! [[ "$TAB_ID" =~ ^(0|[1-9][0-9]*)$ ]] || ! wait_for_sidecar_target; then
    echo "sidecar-target-unready" >&2
    exit 1
fi
start_private_sidecar
printf 'TAB_ID=%s\n' "$TAB_ID"
printf 'WASM_URL=%s\n' "$WASM_URL"
printf 'SIDECAR_LOG=%s\n' "$SIDECAR_LOG"
