#!/bin/bash
# ABOUTME: Provides pure jq predicates for responsive-action native pane identity proofs.
# ABOUTME: Compares pane ID, plugin kind, stable tab ID, and plugin URL as one tuple.

zaphod_existing_pane_tuples_preserved() {
    local before="$1"
    local after="$2"
    jq -e --slurpfile before "$before" '
        def tuple: {id, is_plugin, tab_id, plugin_url};
        ($before[0] | map(tuple)) as $old
        | (map(tuple)) as $new
        | all($old[]; . as $tuple | $new | index($tuple) != null)
    ' "$after" >/dev/null
}

zaphod_pane_tuple_inventories_equal() {
    local before="$1"
    local after="$2"
    jq -e --slurpfile before "$before" '
        def tuples: map({id, is_plugin, tab_id, plugin_url}) | sort_by(.is_plugin, .id);
        tuples == ($before[0] | tuples)
    ' "$after" >/dev/null
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
