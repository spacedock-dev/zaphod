#!/bin/bash
# ABOUTME: Provides pure jq predicates for responsive-action native pane identity proofs.
# ABOUTME: Compares pane identity and all relevant visible native state atomically.

zaphod_existing_pane_tuples_preserved() {
    local before="$1"
    local after="$2"
    jq -e --slurpfile before "$before" '
        def tuple: {
            id, is_plugin, tab_id, plugin_url, exited,
            is_floating, is_suppressed, is_selectable
        };
        ($before[0] | map(tuple)) as $old
        | (map(tuple)) as $new
        | all($old[]; . as $tuple | $new | index($tuple) != null)
    ' "$after" >/dev/null
}

zaphod_pane_tuple_inventories_equal() {
    local before="$1"
    local after="$2"
    jq -e --slurpfile before "$before" '
        def tuples: map({
            id, is_plugin, tab_id, plugin_url, exited,
            is_floating, is_suppressed, is_selectable
        }) | sort_by(.is_plugin, .id);
        tuples == ($before[0] | tuples)
    ' "$after" >/dev/null
}

zaphod_fixture_refresh_record_valid() {
    local panes="$1"
    local refresh="$2"
    local fixture_id="$3"
    local tab_id="$4"
    local wasm_url="$5"
    local sidebar_id="$6"
    jq -e --slurpfile refresh "$refresh" \
        --arg fixture_id "$fixture_id" \
        --arg tab_id "$tab_id" \
        --arg wasm_url "$wasm_url" \
        --arg sidebar_id "$sidebar_id" '
        ($refresh[0]) as $record
        | [.[] | select(
            (.id | tostring) == $fixture_id
            and .is_plugin == false
            and (.tab_id | tostring) == $tab_id
            and .exited == false
            and .is_floating == false
            and .is_suppressed == false
            and .is_selectable == true
            and .title == "zaphod-long-running-non-shell"
            and (.terminal_command | tostring | contains("tail"))
        )] as $fixture
        | [.[] | select(
            (.id | tostring) == $sidebar_id
            and .is_plugin == true
            and (.tab_id | tostring) == $tab_id
            and .plugin_url == $wasm_url
            and .exited == false
            and .is_floating == false
            and .is_suppressed == false
            and .is_selectable == false
        )] as $sidebar
        | ($record | type == "object")
          and (($record | keys | sort) == ["event", "pane_ids", "plugin_id", "refresh_id"])
          and ($record.event == "complete")
          and (($record.plugin_id | tostring) == $sidebar_id)
          and (($record.refresh_id | type) == "number")
          and ($record.refresh_id >= 1)
          and ($record.pane_ids | type == "array")
          and (all($record.pane_ids[]; type == "number"))
          and (($record.pane_ids | unique | length) == ($record.pane_ids | length))
          and (([$record.pane_ids[] | select((tostring) == $fixture_id)] | length) == 1)
          and (($fixture | length) == 1)
          and (($sidebar | length) == 1)
    ' "$panes" >/dev/null
}

zaphod_refresh_log_records() {
    local log="$1"
    local plugin_id="$2"
    local event="$3"
    awk -v plugin_id="$plugin_id" '
        BEGIN { needle = "zaphod-trace[" plugin_id "]: zaphod-refresh " }
        index($0, needle) {
            print substr($0, index($0, needle) + length(needle))
        }
    ' "$log" | jq -Rrc --arg event "$event" --arg plugin_id "$plugin_id" '
        fromjson?
        | select(
            type == "object"
            and (keys | sort) == ["event", "pane_ids", "plugin_id", "refresh_id"]
            and .event == $event
            and (.plugin_id | tostring) == $plugin_id
            and (.refresh_id | type) == "number"
            and .refresh_id >= 1
            and (.pane_ids | type) == "array"
            and all(.pane_ids[]; type == "number")
            and ((.pane_ids | unique | length) == (.pane_ids | length))
        )
    '
}

zaphod_refresh_id_is_in_flight() {
    local log="$1"
    local plugin_id="$2"
    local refresh_id="$3"
    local starts completes aborts
    starts="$(zaphod_refresh_log_records "$log" "$plugin_id" start |
        jq -sc --arg refresh_id "$refresh_id" \
            '[.[] | select((.refresh_id | tostring) == $refresh_id)] | length')"
    completes="$(zaphod_refresh_log_records "$log" "$plugin_id" complete |
        jq -sc --arg refresh_id "$refresh_id" \
            '[.[] | select((.refresh_id | tostring) == $refresh_id)] | length')"
    aborts="$(zaphod_refresh_log_records "$log" "$plugin_id" abort |
        jq -sc --arg refresh_id "$refresh_id" \
            '[.[] | select((.refresh_id | tostring) == $refresh_id)] | length')"
    [ "$starts" -eq 1 ] && [ "$completes" -eq 0 ] && [ "$aborts" -eq 0 ]
}

zaphod_action_deadline_ms() {
    local monotonic_start_ms="$1"
    local threshold_secs="$2"
    printf '%s\n' "$((monotonic_start_ms + threshold_secs * 1000))"
}

zaphod_action_deadline_is_live() {
    local deadline_ms="$1"
    local monotonic_now_ms="$2"
    [ "$monotonic_now_ms" -lt "$deadline_ms" ]
}
