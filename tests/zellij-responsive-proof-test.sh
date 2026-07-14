#!/bin/bash
# ABOUTME: Unit-tests the native pane identity invariants used by the responsive-action smoke.
# ABOUTME: Rejects tab reassignment and sidebar replacement even when pane counts stay constant.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
# shellcheck source=tests/helpers/zellij-responsive-proof-lib.sh
source "$SCRIPT_DIR/helpers/zellij-responsive-proof-lib.sh"

ROOT="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-responsive-proof.XXXXXX")"
trap 'rm -rf "$ROOT"' EXIT

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

cat > "$ROOT/before.json" <<'JSON'
[
  {"id": 7, "is_plugin": false, "tab_id": 2, "plugin_url": null},
  {"id": 9, "is_plugin": true, "tab_id": 2, "plugin_url": "file:/candidate.wasm"}
]
JSON

cat > "$ROOT/added.json" <<'JSON'
[
  {"id": 7, "is_plugin": false, "tab_id": 2, "plugin_url": null},
  {"id": 9, "is_plugin": true, "tab_id": 2, "plugin_url": "file:/candidate.wasm"},
  {"id": 10, "is_plugin": false, "tab_id": 2, "plugin_url": null}
]
JSON
zaphod_existing_pane_tuples_preserved "$ROOT/before.json" "$ROOT/added.json" ||
    fail "one added terminal did not preserve the existing pane tuples"

jq 'map(if .id == 7 then .tab_id = 3 else . end)' \
    "$ROOT/before.json" > "$ROOT/moved.json"
if zaphod_existing_pane_tuples_preserved "$ROOT/before.json" "$ROOT/moved.json"; then
    fail "terminal tab reassignment incorrectly preserved its native tuple"
fi

jq 'map(if .id == 9 then .id = 11 else . end)' \
    "$ROOT/before.json" > "$ROOT/replaced-sidebar.json"
if zaphod_existing_pane_tuples_preserved "$ROOT/before.json" "$ROOT/replaced-sidebar.json"; then
    fail "sidebar replacement incorrectly preserved its native tuple"
fi

zaphod_pane_tuple_inventories_equal "$ROOT/before.json" "$ROOT/before.json" ||
    fail "an unchanged pane tuple inventory was rejected"
if zaphod_pane_tuple_inventories_equal "$ROOT/before.json" "$ROOT/added.json"; then
    fail "a changed pane tuple inventory was accepted as unchanged"
fi

echo "PASS: responsive proof preserves full native pane tuples and sidebar identity"
