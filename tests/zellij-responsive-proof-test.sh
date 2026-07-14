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
  {"id": 7, "is_plugin": false, "tab_id": 2, "plugin_url": null, "exited": false, "is_floating": false, "is_suppressed": false, "is_selectable": true, "title": "zaphod-long-running-non-shell", "terminal_command": ["tail", "-f", "/dev/null"]},
  {"id": 9, "is_plugin": true, "tab_id": 2, "plugin_url": "file:/candidate.wasm", "exited": false, "is_floating": false, "is_suppressed": false, "is_selectable": false}
]
JSON

cat > "$ROOT/added.json" <<'JSON'
[
  {"id": 7, "is_plugin": false, "tab_id": 2, "plugin_url": null, "exited": false, "is_floating": false, "is_suppressed": false, "is_selectable": true, "title": "zaphod-long-running-non-shell", "terminal_command": ["tail", "-f", "/dev/null"]},
  {"id": 9, "is_plugin": true, "tab_id": 2, "plugin_url": "file:/candidate.wasm", "exited": false, "is_floating": false, "is_suppressed": false, "is_selectable": false},
  {"id": 10, "is_plugin": false, "tab_id": 2, "plugin_url": null, "exited": false, "is_floating": false, "is_suppressed": false, "is_selectable": true}
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

for field in exited is_floating is_suppressed is_selectable; do
    jq --arg field "$field" \
        'map(if .id == 9 then .[$field] = (if $field == "is_selectable" then true else true end) else . end)' \
        "$ROOT/before.json" > "$ROOT/sidebar-$field.json"
    if zaphod_existing_pane_tuples_preserved \
        "$ROOT/before.json" "$ROOT/sidebar-$field.json"; then
        fail "sidebar $field change incorrectly preserved its native state"
    fi
done

zaphod_pane_tuple_inventories_equal "$ROOT/before.json" "$ROOT/before.json" ||
    fail "an unchanged pane tuple inventory was rejected"
if zaphod_pane_tuple_inventories_equal "$ROOT/before.json" "$ROOT/added.json"; then
    fail "a changed pane tuple inventory was accepted as unchanged"
fi

cat > "$ROOT/refresh.json" <<'JSON'
{"event":"complete","plugin_id":9,"refresh_id":4,"pane_ids":[7]}
JSON
zaphod_fixture_refresh_record_valid "$ROOT/before.json" "$ROOT/refresh.json" \
    7 2 file:/candidate.wasm 9 ||
    fail "the exact terminal/sidebar/refresh record was rejected"

printf '%s\n' '{"event":"complete","plugin_id":7,"refresh_id":4,"pane_ids":[]}' > "$ROOT/wrong-field-refresh.json"
if zaphod_fixture_refresh_record_valid "$ROOT/before.json" \
    "$ROOT/wrong-field-refresh.json" 7 2 file:/candidate.wasm 9; then
    fail "fixture ID outside pane_ids incorrectly proved refresh membership"
fi
printf '%s\n' '{"event":"complete","plugin_id":9,"refresh_id":4,"pane_ids":[70]}' > "$ROOT/wrong-pane-refresh.json"
if zaphod_fixture_refresh_record_valid "$ROOT/before.json" \
    "$ROOT/wrong-pane-refresh.json" 7 2 file:/candidate.wasm 9; then
    fail "adjacent wrong pane incorrectly proved refresh membership"
fi

cat > "$ROOT/refresh.log" <<'LOG'
2026-07-14 zaphod-trace[9]: zaphod-refresh {"event":"start","plugin_id":9,"refresh_id":4,"pane_ids":[7]}
2026-07-14 zaphod-trace[8]: zaphod-refresh {"event":"complete","plugin_id":8,"refresh_id":4,"pane_ids":[7]}
2026-07-14 zaphod-trace[9]: zaphod-refresh {"event":"complete","plugin_id":9,"refresh_id":3,"pane_ids":[7]}
2026-07-14 zaphod-trace[9]: fixture=7 zaphod-refresh {"event":"complete","plugin_id":9,"refresh_id":4,"pane_ids":[]}
2026-07-14 zaphod-trace[9]: zaphod-refresh {"event":"complete","plugin_id":9,"refresh_id":4,"pane_ids":[7]}
LOG
zaphod_refresh_log_records "$ROOT/refresh.log" 9 complete > "$ROOT/completes.jsonl"
[ "$(wc -l < "$ROOT/completes.jsonl" | tr -d '[:space:]')" = 2 ] ||
    fail "structured refresh parser accepted a wrong-plugin or non-record decoy"
[ "$(tail -1 "$ROOT/completes.jsonl" | jq -r .refresh_id)" = 4 ] ||
    fail "structured refresh parser lost the exact latest completion"

printf '%s\n' \
    'zaphod-trace[9]: zaphod-refresh {"event":"start","plugin_id":9,"refresh_id":5,"pane_ids":[7]}' \
    > "$ROOT/in-flight.log"
zaphod_refresh_id_is_in_flight "$ROOT/in-flight.log" 9 5 ||
    fail "a started refresh without a matching completion was not in flight"
printf '%s\n' \
    'zaphod-trace[9]: zaphod-refresh {"event":"complete","plugin_id":9,"refresh_id":5,"pane_ids":[7]}' \
    >> "$ROOT/in-flight.log"
if zaphod_refresh_id_is_in_flight "$ROOT/in-flight.log" 9 5; then
    fail "a completed refresh remained in flight"
fi
printf '%s\n' \
    'zaphod-trace[9]: zaphod-refresh {"event":"start","plugin_id":9,"refresh_id":6,"pane_ids":[7]}' \
    'zaphod-trace[9]: zaphod-refresh {"event":"abort","plugin_id":9,"refresh_id":6,"pane_ids":[]}' \
    >> "$ROOT/in-flight.log"
if zaphod_refresh_id_is_in_flight "$ROOT/in-flight.log" 9 6; then
    fail "an aborted refresh remained in flight"
fi

[ "$(zaphod_action_deadline_ms 42000 1)" = 43000 ] ||
    fail "the action deadline was not derived from the pre-send monotonic sample"
zaphod_action_deadline_is_live 43000 42999 ||
    fail "a pre-deadline action was rejected"
if zaphod_action_deadline_is_live 43000 43000; then
    fail "an action at the absolute deadline was accepted"
fi

echo "PASS: responsive proof binds full pane state, exact refresh authority, and pre-send deadlines"
