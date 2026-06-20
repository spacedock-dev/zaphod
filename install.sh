#!/bin/bash
# ABOUTME: Installs the zaphod agent-tab layout into the user's zellij layouts dir.
# ABOUTME: Substitutes the absolute built-wasm path into the docked/undocked swap layout.
set -euo pipefail
cd "$(dirname "$0")"

WASM_PATH="$(pwd)/target/wasm32-wasip1/release/zellij-sidebar.wasm"
if [ ! -f "$WASM_PATH" ]; then
    echo "wasm not found at $WASM_PATH — run ./build.sh first" >&2
    exit 1
fi

DEST_DIR="${ZELLIJ_CONFIG_DIR:-$HOME/.config/zellij}/layouts"
mkdir -p "$DEST_DIR"
sed "s|__ZAPHOD_WASM__|file:$WASM_PATH|g" layouts/zaphod.kdl > "$DEST_DIR/zaphod.kdl"
echo "installed $DEST_DIR/zaphod.kdl"
