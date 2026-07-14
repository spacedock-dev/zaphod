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
PLUGIN_DEBUG="${ZAPHOD_TEST_PLUGIN_DEBUG:-}"
REFRESH_BARRIER_MILLIS="${ZAPHOD_TEST_REFRESH_BARRIER_MILLIS:-}"
REFRESH_BARRIER_PANES="${ZAPHOD_TEST_REFRESH_BARRIER_PANES:-}"
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

# The requested/default session has been resolved. Loaded-pane client identity
# must not steer version, setup, inventory, creation, or watcher route injection.
unset ZELLIJ ZELLIJ_SESSION_NAME ZELLIJ_PANE_ID

[ -n "$SESSION_NAME" ] ||
    fail "a Zellij session is required; pass --session NAME or set ZELLIJ_SESSION_NAME"
case "$PLUGIN_DEBUG" in
    ""|1) ;;
    *) fail "ZAPHOD_TEST_PLUGIN_DEBUG must be empty or 1" ;;
esac
if { [ -n "$REFRESH_BARRIER_MILLIS" ] || [ -n "$REFRESH_BARRIER_PANES" ]; } &&
    { [ "$PLUGIN_DEBUG" != 1 ] ||
      ! [[ "$REFRESH_BARRIER_MILLIS" =~ ^[1-9][0-9]*$ ]] ||
      ! [[ "$REFRESH_BARRIER_PANES" =~ ^[1-9][0-9]*$ ]]; }; then
    fail "test refresh barrier requires debug plus positive millisecond and pane values"
fi

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
if [ -n "${ZAPHOD_WATCH_DIR:-}" ]; then
	WATCH_DIR="$ZAPHOD_WATCH_DIR"
elif [ -n "${XDG_RUNTIME_DIR:-}" ]; then
	WATCH_DIR="$XDG_RUNTIME_DIR/zaphod/watch-tab-v1"
else
	WATCH_DIR="/tmp/zaphod-watch-tab-v1-$(id -u)"
fi
case "$WATCH_DIR" in
    /*) ;;
    *) fail "ZAPHOD_WATCH_DIR must be absolute" ;;
esac
MANAGED_SHELL="${SHELL:-/bin/sh}"
[ -x "$MANAGED_SHELL" ] || fail "managed shell is not executable: $MANAGED_SHELL"
ENV_BIN="$(command -v env)"
[ -x "$ENV_BIN" ] || fail "env command is not executable: $ENV_BIN"

ZELLIJ_ARGS=(--config-dir "$ZELLIJ_ROOT" --config "$CONFIG_FILE")
ZELLIJ_ARGS+=(--data-dir "$DATA_DIR")

zellij_cmd() {
    "$ZELLIJ_BIN" "${ZELLIJ_ARGS[@]}" "$@"
}

capture_initial_tab_inventory() {
    local output="$1"
    local stderr_file="$2"
    local attempt status provenance
    for attempt in $(seq 1 20); do
        status=0
        ZELLIJ_SESSION_NAME="$SESSION_NAME" zellij_cmd --session "$SESSION_NAME" \
            action list-tabs --json --all --state --layout > "$output" 2> "$stderr_file" || status=$?
        if [ "$status" -ne 0 ]; then
            provenance="$(zaphod_bounded_reply_provenance "list-tabs-before attempt=$attempt/20" \
                "$status" "$output" "$stderr_file")"
            printf 'tab-inventory-unready: %s\n' "$provenance" >&2
            return 1
        fi
        if [ -s "$output" ] && ! zaphod_valid_tab_inventory "$output"; then
            provenance="$(zaphod_bounded_reply_provenance "list-tabs-before attempt=$attempt/20" \
                "$status" "$output" "$stderr_file")"
            printf 'tab-inventory-unready: %s\n' "$provenance" >&2
            return 1
        fi
        if [ -s "$output" ] && jq -e 'length > 0' "$output" >/dev/null 2>&1; then
            return 0
        fi
        if [ "$attempt" -lt 20 ]; then
            sleep 0.05
            continue
        fi
        provenance="$(zaphod_bounded_reply_provenance "list-tabs-before attempt=$attempt/20" \
            "$status" "$output" "$stderr_file")"
        printf 'tab-inventory-unready: persistent empty inventory; %s\n' "$provenance" >&2
        return 1
    done
    return 1
}

TEMP_ROOT=""
RECIPIENT_TOKEN=""

cleanup() {
    local original_status=$?
    local cleanup_status=0
    trap - EXIT INT TERM HUP
    set +e
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

# Native Zellij validates the profile. Persistent key policy remains global
# setup; this selected-checkout command does not parse, repair, or retarget it.
zellij_cmd setup --check >/dev/null
if [ "${ZAPHOD_TEST_PREBUILT_ARTIFACTS:-}" != 1 ]; then
    "$REPO_ROOT/build.sh"
fi

WASM_PATH="$REPO_ROOT/target/wasm32-wasip1/release/zellij-sidebar.wasm"
[ -f "$WASM_PATH" ] || fail "wasm not found after build: $WASM_PATH"
ZAPHOD_PATH="$REPO_ROOT/target/zaphod"
[ -x "$ZAPHOD_PATH" ] || fail "zaphod native binary not found after build: $ZAPHOD_PATH"
WASM_URL="$(zaphod_canonical_file_url "$WASM_PATH")" ||
    fail "could not derive a canonical URL for $WASM_PATH"

rail_target_pane_id() {
    local panes candidate_id
    panes="$(ZELLIJ_SESSION_NAME="$SESSION_NAME" zellij_cmd --session "$SESSION_NAME" \
        action list-panes --json --all --command --geometry --state --tab 2>/dev/null)" ||
        return 1
    candidate_id="$(printf '%s' "$panes" | jq -er \
        --arg tab_id "$TAB_ID" \
        --arg wasm_url "$WASM_URL" \
        '[.[] | select(
            ((.tab_id | tostring) == $tab_id)
            and .is_plugin == true
            and .plugin_url == $wasm_url
            and .is_floating == false
            and .is_suppressed == false
        )] | if length == 1 then .[0].id | tostring else error("expected one target") end' \
        2>/dev/null)" || return 1
    case "$candidate_id" in
        plugin_*) printf '%s\n' "$candidate_id" ;;
        0|[1-9]|[1-9][0-9]*) printf 'plugin_%s\n' "$candidate_id" ;;
        *) return 1 ;;
    esac
}

wait_for_rail_target() {
    local attempt
    for attempt in $(seq 1 80); do
        if rail_target_pane_id >/dev/null; then
            return 0
        fi
        sleep 0.05
    done
    return 1
}

TEMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-new-tab.XXXXXX")" ||
    fail "could not create a temporary Zaphod layout directory"
RENDERED_LAYOUT="$TEMP_ROOT/zaphod.kdl"
TABS_BEFORE="$TEMP_ROOT/tabs-before.json"
TABS_BEFORE_STDERR="$TEMP_ROOT/tabs-before.stderr"
TABS_AFTER="$TEMP_ROOT/tabs-after.json"
TABS_AFTER_STDERR="$TEMP_ROOT/tabs-after.stderr"
NEW_TAB_STDOUT="$TEMP_ROOT/new-tab.stdout"
NEW_TAB_STDERR="$TEMP_ROOT/new-tab.stderr"
RECIPIENT_TOKEN="zaphod-$$-$RANDOM-$(date +%s)"
zaphod_render_layout "$REPO_ROOT/layouts/zaphod.kdl" "$WASM_URL" \
    "$RENDERED_LAYOUT" "$RECIPIENT_TOKEN" "$PLUGIN_DEBUG" \
    "$REFRESH_BARRIER_MILLIS" "$REFRESH_BARRIER_PANES"
zaphod_validate_layout_identity "$RENDERED_LAYOUT" "$WASM_URL"

capture_initial_tab_inventory "$TABS_BEFORE" "$TABS_BEFORE_STDERR" || exit 1
set +e
ZELLIJ_SESSION_NAME="$SESSION_NAME" zellij_cmd --session "$SESSION_NAME" action new-tab \
    --name "$TAB_NAME" --cwd "$REPO_ROOT" --layout-string "$(cat "$RENDERED_LAYOUT")" \
    -- "$ENV_BIN" \
    "ZAPHOD_WATCH_DIR=$WATCH_DIR" \
    "ZAPHOD_AGENTSVIEW_URL=$AGENTSVIEW_URL" \
    "ZAPHOD_RAIL_URL=$WASM_URL" \
    "ZAPHOD_RECIPIENT_TOKEN=$RECIPIENT_TOKEN" \
    "ZAPHOD_ZELLIJ_CONFIG_DIR=$ZELLIJ_ROOT" \
    "ZAPHOD_ZELLIJ_CONFIG_FILE=$CONFIG_FILE" \
    "ZAPHOD_ZELLIJ_DATA_DIR=$DATA_DIR" \
    "ZELLIJ_BIN=$ZELLIJ_BIN" \
    "$MANAGED_SHELL" -l \
    > "$NEW_TAB_STDOUT" 2> "$NEW_TAB_STDERR"
NEW_TAB_STATUS=$?
set -e
if [ "$NEW_TAB_STATUS" -ne 0 ]; then
    printf 'new-tab-failed: %s\n' \
        "$(zaphod_bounded_reply_provenance new-tab "$NEW_TAB_STATUS" "$NEW_TAB_STDOUT" "$NEW_TAB_STDERR")" >&2
    exit "$NEW_TAB_STATUS"
fi
TAB_ID=""
TABS_AFTER_STATUS=1
: > "$TABS_AFTER"
: > "$TABS_AFTER_STDERR"
for _attempt in $(seq 1 80); do
    set +e
    ZELLIJ_SESSION_NAME="$SESSION_NAME" zellij_cmd --session "$SESSION_NAME" \
        action list-tabs --json --all --state --layout > "$TABS_AFTER" 2> "$TABS_AFTER_STDERR"
    TABS_AFTER_STATUS=$?
    set -e
    if [ "$TABS_AFTER_STATUS" -eq 0 ] && zaphod_valid_tab_inventory "$TABS_AFTER"; then
        TAB_ID="$(zaphod_new_tab_id_from_inventories "$TABS_BEFORE" "$TABS_AFTER" 2>/dev/null || true)"
        [ -z "$TAB_ID" ] || break
    fi
    sleep 0.05
done
if ! [[ "$TAB_ID" =~ ^(0|[1-9][0-9]*)$ ]]; then
    printf 'rail-target-unready: %s; %s\n' \
        "$(zaphod_bounded_reply_provenance new-tab "$NEW_TAB_STATUS" "$NEW_TAB_STDOUT" "$NEW_TAB_STDERR")" \
        "$(zaphod_bounded_reply_provenance list-tabs-after "$TABS_AFTER_STATUS" "$TABS_AFTER" "$TABS_AFTER_STDERR")" >&2
    exit 1
fi
if ! wait_for_rail_target; then
    echo "rail-target-unready: stable tab $TAB_ID did not expose exactly one candidate rail" >&2
    exit 1
fi
printf 'TAB_ID=%s\n' "$TAB_ID"
printf 'WASM_URL=%s\n' "$WASM_URL"
printf 'WATCH_DIR=%s\n' "$WATCH_DIR"
printf 'RECIPIENT_TOKEN=%s\n' "$RECIPIENT_TOKEN"
printf 'WATCH_COMMAND=%s watch-tab\n' "$ZAPHOD_PATH"
