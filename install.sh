#!/bin/bash
# ABOUTME: Installs the zaphod agent-tab layout into the user's zellij layouts dir.
# ABOUTME: Substitutes the absolute built-wasm path into the docked/undocked swap layout.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
# shellcheck source=scripts/zellij-layout-lib.sh
source "$SCRIPT_DIR/scripts/zellij-layout-lib.sh"

PRIMARY_ROOT="$(zaphod_primary_checkout_root "$SCRIPT_DIR")"
if [ "$SCRIPT_DIR" != "$PRIMARY_ROOT" ]; then
    echo "refusing global install from linked worktree: $SCRIPT_DIR" >&2
    echo "primary checkout: $PRIMARY_ROOT" >&2
    exit 1
fi

WASM_PATH="$SCRIPT_DIR/target/wasm32-wasip1/release/zellij-sidebar.wasm"
if [ ! -f "$WASM_PATH" ]; then
    echo "wasm not found at $WASM_PATH — run ./build.sh first" >&2
    exit 1
fi
WASM_URL="$(zaphod_canonical_file_url "$WASM_PATH")"

ZELLIJ_ROOT="${ZELLIJ_CONFIG_DIR:-$HOME/.config/zellij}"
CONFIG_FILE="${ZELLIJ_CONFIG_FILE:-$ZELLIJ_ROOT/config.kdl}"
zaphod_require_zellij_0443
zaphod_validate_message_plugin_identity "$CONFIG_FILE" "$WASM_URL"
zellij --config-dir "$ZELLIJ_ROOT" --config "$CONFIG_FILE" setup --check >/dev/null

DEST_DIR="$ZELLIJ_ROOT/layouts"
mkdir -p "$DEST_DIR"
TARGET_LAYOUT="$DEST_DIR/zaphod.kdl"
TEMP_LAYOUT_BASE="$(mktemp "$DEST_DIR/.zaphod.XXXXXX")"
TEMP_LAYOUT="$TEMP_LAYOUT_BASE.kdl"
mv "$TEMP_LAYOUT_BASE" "$TEMP_LAYOUT"
BACKUP_LAYOUT=""
HAD_PREVIOUS=0

cleanup_install_temps() {
    [ -z "$TEMP_LAYOUT" ] || rm -f "$TEMP_LAYOUT"
    [ -z "$BACKUP_LAYOUT" ] || rm -f "$BACKUP_LAYOUT"
}
trap cleanup_install_temps EXIT

zaphod_render_layout "$SCRIPT_DIR/layouts/zaphod.kdl" "$WASM_URL" "$TEMP_LAYOUT"
zaphod_validate_layout_identity "$TEMP_LAYOUT" "$WASM_URL"
zaphod_validate_layout_live "$TEMP_LAYOUT" "$ZELLIJ_ROOT" "$CONFIG_FILE" "$WASM_URL"

if [ -e "$TARGET_LAYOUT" ]; then
    BACKUP_LAYOUT="$(mktemp "$DEST_DIR/.zaphod.backup.XXXXXX")"
    cp -p "$TARGET_LAYOUT" "$BACKUP_LAYOUT"
    HAD_PREVIOUS=1
fi
mv "$TEMP_LAYOUT" "$TARGET_LAYOUT"
TEMP_LAYOUT=""

if ! zaphod_validate_message_plugin_identity "$CONFIG_FILE" "$WASM_URL" ||
    ! zaphod_validate_layout_identity "$TARGET_LAYOUT" "$WASM_URL" ||
    ! zaphod_validate_layout_live "$TARGET_LAYOUT" "$ZELLIJ_ROOT" "$CONFIG_FILE" "$WASM_URL"; then
    echo "postflight identity validation failed; restoring previous layout" >&2
    if [ "$HAD_PREVIOUS" -eq 1 ]; then
        mv "$BACKUP_LAYOUT" "$TARGET_LAYOUT"
        BACKUP_LAYOUT=""
    else
        rm -f "$TARGET_LAYOUT"
    fi
    exit 1
fi

if [ -n "$BACKUP_LAYOUT" ]; then
    rm -f "$BACKUP_LAYOUT"
    BACKUP_LAYOUT=""
fi
trap - EXIT
echo "installed $TARGET_LAYOUT"
