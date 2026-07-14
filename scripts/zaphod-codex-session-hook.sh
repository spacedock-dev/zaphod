#!/bin/bash
# ABOUTME: Trusted project-local bridge from Codex SessionStart to Zaphod.
# ABOUTME: Resolves the native receiver from this exact selected checkout.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
ZAPHOD_BIN="${ZAPHOD_BIN:-$SCRIPT_DIR/../target/zaphod}"

if [ ! -x "$ZAPHOD_BIN" ]; then
    echo "zaphod SessionStart registrar is not built: $ZAPHOD_BIN" >&2
    exit 1
fi

exec "$ZAPHOD_BIN" register-agent-session
