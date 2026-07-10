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

DEST_DIR="${ZELLIJ_CONFIG_DIR:-$HOME/.config/zellij}/layouts"
mkdir -p "$DEST_DIR"
zaphod_render_layout "$SCRIPT_DIR/layouts/zaphod.kdl" "$WASM_URL" "$DEST_DIR/zaphod.kdl"
echo "installed $DEST_DIR/zaphod.kdl"
