#!/bin/bash
# ABOUTME: Activates this checkout's Zaphod layout and creates one fresh managed tab.
# ABOUTME: Repoints only existing Zaphod keybind routes and uses isolated roots when requested.
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
CONFIG_DIR="$(dirname "$CONFIG_FILE")"
LAYOUT_DIR="$ZELLIJ_ROOT/layouts"
TARGET_LAYOUT="$LAYOUT_DIR/zaphod.kdl"
ZELLIJ_BIN="${ZELLIJ_BIN:-zellij}"

ZELLIJ_ARGS=(--config-dir "$ZELLIJ_ROOT" --config "$CONFIG_FILE")
ZELLIJ_ARGS+=(--data-dir "$DATA_DIR")

zellij_cmd() {
    "$ZELLIJ_BIN" "${ZELLIJ_ARGS[@]}" "$@"
}

zellij_check_config() {
    local config_file="$1"
    local args=(--config-dir "$ZELLIJ_ROOT" --config "$config_file" --data-dir "$DATA_DIR")
    "$ZELLIJ_BIN" "${args[@]}" setup --check >/dev/null
}

TEMP_ROOT=""
CONFIG_TEMP=""
LAYOUT_TEMP=""
CONFIG_BACKUP=""
LAYOUT_BACKUP=""
HAD_LAYOUT=0
ROLLBACK_NEEDED=0

cleanup() {
    local original_status=$?
    local cleanup_status=0
    trap - EXIT INT TERM HUP
    set +e
    if [ "$ROLLBACK_NEEDED" -eq 1 ]; then
        if [ -n "$CONFIG_BACKUP" ] && [ -f "$CONFIG_BACKUP" ]; then
            mv "$CONFIG_BACKUP" "$CONFIG_FILE" || cleanup_status=1
            CONFIG_BACKUP=""
        fi
        if [ "$HAD_LAYOUT" -eq 1 ] && [ -n "$LAYOUT_BACKUP" ] && [ -f "$LAYOUT_BACKUP" ]; then
            mv "$LAYOUT_BACKUP" "$TARGET_LAYOUT" || cleanup_status=1
            LAYOUT_BACKUP=""
        elif [ "$HAD_LAYOUT" -eq 0 ]; then
            rm -f "$TARGET_LAYOUT" || cleanup_status=1
        fi
    fi
    [ -z "$CONFIG_TEMP" ] || rm -f "$CONFIG_TEMP" || cleanup_status=1
    [ -z "$LAYOUT_TEMP" ] || rm -f "$LAYOUT_TEMP" || cleanup_status=1
    [ -z "$CONFIG_BACKUP" ] || rm -f "$CONFIG_BACKUP" || cleanup_status=1
    [ -z "$LAYOUT_BACKUP" ] || rm -f "$LAYOUT_BACKUP" || cleanup_status=1
    [ -z "$TEMP_ROOT" ] || rm -rf "$TEMP_ROOT" || cleanup_status=1
    if [ "$cleanup_status" -ne 0 ]; then
        echo "failed to clean up or roll back Zaphod activation" >&2
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
[ -f "$SCRIPT_DIR/zellij-config-activate.awk" ] || fail "Zaphod config transformer not found"

zellij_cmd setup --check >/dev/null
"$REPO_ROOT/build.sh"

WASM_PATH="$REPO_ROOT/target/wasm32-wasip1/release/zellij-sidebar.wasm"
[ -f "$WASM_PATH" ] || fail "wasm not found after build: $WASM_PATH"
SIDECAR_PATH="$REPO_ROOT/target/zaphod"
[ -x "$SIDECAR_PATH" ] || fail "zaphod sidecar not found after build: $SIDECAR_PATH"
WASM_URL="$(zaphod_canonical_file_url "$WASM_PATH")" ||
    fail "could not derive a canonical URL for $WASM_PATH"
LAYOUT_PATH_KDL="$(zaphod_kdl_escape "$TARGET_LAYOUT")" ||
    fail "could not derive a KDL-safe path for $TARGET_LAYOUT"

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
    mkdir -p "$DATA_DIR" ||
        fail "sidecar-start-failed: could not create private sidecar log directory"
    SIDECAR_LOG="$(mktemp "$DATA_DIR/zaphod-sidecar.XXXXXX")" ||
        fail "sidecar-start-failed: could not create private sidecar log"
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
        </dev/null >>"$SIDECAR_LOG" 2>&1 &
    local start_status=$?
    set -e
    [ "$start_status" -eq 0 ] ||
        fail "sidecar-start-failed: could not launch private zaphod sidecar"
}

TEMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-new-tab.XXXXXX")" ||
    fail "could not create a temporary Zaphod layout directory"
RENDERED_LAYOUT="$TEMP_ROOT/zaphod.kdl"
CHECK_CONFIG="$TEMP_ROOT/config.kdl"
CANDIDATE_CONFIG="$TEMP_ROOT/candidate-config.kdl"
zaphod_render_layout "$REPO_ROOT/layouts/zaphod.kdl" "$WASM_URL" "$RENDERED_LAYOUT"
zaphod_validate_layout_identity "$RENDERED_LAYOUT" "$WASM_URL"
awk -v wasm_url="$WASM_URL" -v layout_path="$LAYOUT_PATH_KDL" \
    -f "$SCRIPT_DIR/zellij-config-activate.awk" \
    "$CONFIG_FILE" > "$CANDIDATE_CONFIG"
zaphod_validate_message_plugin_identity "$CANDIDATE_CONFIG" "$WASM_URL"
# Zellij resolves a NewTab layout while parsing the config. Validate against
# the rendered candidate first; the final absolute target is checked again
# after its atomic layout install below.
RENDERED_LAYOUT_PATH_KDL="$(zaphod_kdl_escape "$RENDERED_LAYOUT")" ||
    fail "could not derive a KDL-safe path for $RENDERED_LAYOUT"
awk -v wasm_url="$WASM_URL" -v layout_path="$RENDERED_LAYOUT_PATH_KDL" \
    -f "$SCRIPT_DIR/zellij-config-activate.awk" \
    "$CONFIG_FILE" > "$CHECK_CONFIG"
zellij_check_config "$CHECK_CONFIG"

mkdir -p "$LAYOUT_DIR"
CONFIG_TEMP_BASE="$(mktemp "$CONFIG_DIR/.zaphod-config.XXXXXX")" ||
    fail "could not create an atomic config temporary file"
CONFIG_TEMP="$CONFIG_TEMP_BASE.kdl"
mv "$CONFIG_TEMP_BASE" "$CONFIG_TEMP"
cp -p "$CANDIDATE_CONFIG" "$CONFIG_TEMP"
LAYOUT_TEMP_BASE="$(mktemp "$LAYOUT_DIR/.zaphod-layout.XXXXXX")" ||
    fail "could not create an atomic layout temporary file"
LAYOUT_TEMP="$LAYOUT_TEMP_BASE.kdl"
mv "$LAYOUT_TEMP_BASE" "$LAYOUT_TEMP"
cp -p "$RENDERED_LAYOUT" "$LAYOUT_TEMP"

CONFIG_BACKUP="$(mktemp "$CONFIG_DIR/.zaphod-config-backup.XXXXXX")" ||
    fail "could not create a config rollback file"
cp -p "$CONFIG_FILE" "$CONFIG_BACKUP"
if [ -e "$TARGET_LAYOUT" ]; then
    LAYOUT_BACKUP="$(mktemp "$LAYOUT_DIR/.zaphod-layout-backup.XXXXXX")" ||
        fail "could not create a layout rollback file"
    cp -p "$TARGET_LAYOUT" "$LAYOUT_BACKUP"
    HAD_LAYOUT=1
fi

ROLLBACK_NEEDED=1
mv "$CONFIG_TEMP" "$CONFIG_FILE"
CONFIG_TEMP=""
mv "$LAYOUT_TEMP" "$TARGET_LAYOUT"
LAYOUT_TEMP=""
zaphod_validate_message_plugin_identity "$CONFIG_FILE" "$WASM_URL"
zaphod_validate_layout_identity "$TARGET_LAYOUT" "$WASM_URL"
zellij_cmd setup --check >/dev/null

TAB_ID="$(ZELLIJ_SESSION_NAME="$SESSION_NAME" zellij_cmd --session "$SESSION_NAME" action new-tab \
    --name "$TAB_NAME" --layout-string "$(cat "$RENDERED_LAYOUT")")"

ROLLBACK_NEEDED=0
rm -f "$CONFIG_BACKUP" "$LAYOUT_BACKUP"
CONFIG_BACKUP=""
LAYOUT_BACKUP=""
if ! [[ "$TAB_ID" =~ ^(0|[1-9][0-9]*)$ ]] || ! wait_for_sidecar_target; then
    echo "sidecar-target-unready" >&2
    exit 1
fi
start_private_sidecar
printf 'TAB_ID=%s\n' "$TAB_ID"
printf 'WASM_URL=%s\n' "$WASM_URL"
printf 'SIDECAR_LOG=%s\n' "$SIDECAR_LOG"
