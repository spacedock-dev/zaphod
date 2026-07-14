#!/bin/bash
# ABOUTME: Black-box regressions for the selected-checkout Zellij tab entry point.
# ABOUTME: Uses fake build/Zellij commands and proves standing KDL is read-only.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd -P)"
TEST_ROOT=""

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

sha256() {
    shasum -a 256 "$1" | awk '{print $1}'
}

cleanup() {
    local status=$?
    trap - EXIT INT TERM HUP
    if [ -n "$TEST_ROOT" ] && [ -d "$TEST_ROOT" ]; then
        rm -rf "$TEST_ROOT"
    fi
    exit "$status"
}

trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP

write_fake_build() {
    local fixture="$1"
    printf '%s\n' \
        '#!/bin/bash' \
        'set -euo pipefail' \
        'printf "%s\\n" "$0" > "$BUILD_LOG"' \
        'mkdir -p "$(dirname "$0")/target/wasm32-wasip1/release"' \
        'printf "fixture wasm\\n" > "$(dirname "$0")/target/wasm32-wasip1/release/zellij-sidebar.wasm"' \
        'printf "%s\\n" "#!/bin/bash" "printf '\''%s\\n'\'' \"\$@\" >> \"\$FAKE_ZAPHOD_CALLS\"" "exit 99" > "$(dirname "$0")/target/zaphod"' \
        'chmod +x "$(dirname "$0")/target/zaphod"' \
        > "$fixture/build.sh"
    chmod +x "$fixture/build.sh"
}

write_fake_zellij() {
    local fixture="$1"
    mkdir -p "$fixture/bin"
    printf '%s\n' \
        '#!/bin/bash' \
        'set -euo pipefail' \
        'config_dir=""' \
        'config_file=""' \
        'data_dir=""' \
        'session=""' \
        'while [ "$#" -gt 0 ]; do' \
        '    case "$1" in' \
        '        --config-dir) config_dir="${2:?}"; shift 2 ;;' \
        '        --config) config_file="${2:?}"; shift 2 ;;' \
        '        --data-dir) data_dir="${2:?}"; shift 2 ;;' \
        '        --session) session="${2:?}"; shift 2 ;;' \
        '        --version)' \
        '            printf "version\\t%s\\t%s\\t%s\\tclient=%s\\tsession=%s\\tpane=%s\\n" "$config_dir" "$config_file" "$data_dir" "${ZELLIJ+x}" "${ZELLIJ_SESSION_NAME+x}" "${ZELLIJ_PANE_ID+x}" >> "$FAKE_ZELLIJ_CALLS"' \
        '            printf "zellij 0.44.3\\n"' \
        '            exit 0' \
        '            ;;' \
        '        setup)' \
        '            [ "${2:-}" = --check ] || exit 64' \
        '            printf "setup\\t%s\\t%s\\t%s\\tclient=%s\\tsession=%s\\tpane=%s\\n" "$config_dir" "$config_file" "$data_dir" "${ZELLIJ+x}" "${ZELLIJ_SESSION_NAME+x}" "${ZELLIJ_PANE_ID+x}" >> "$FAKE_ZELLIJ_CALLS"' \
        '            exit "${FAKE_ZELLIJ_SETUP_STATUS:-0}"' \
        '            ;;' \
        '        action)' \
        '            [ -n "$session" ] || { printf "missing explicit session\\n" >&2; exit 64; }' \
        '            case "${2:-}" in' \
        '                list-panes) cat "$FAKE_ZELLIJ_PANES"; exit 0 ;;' \
        '                list-tabs)' \
        '                    list_tabs_count_file="${FAKE_ZELLIJ_LIST_TABS_COUNT:-$FAKE_ZELLIJ_TABS.count}"' \
        '                    count=0' \
        '                    [ ! -f "$list_tabs_count_file" ] || count="$(cat "$list_tabs_count_file")"' \
        '                    count=$((count + 1))' \
        '                    printf "%s\n" "$count" > "$list_tabs_count_file"' \
        '                    if [ "${FAKE_ZELLIJ_EMPTY_LIST_TABS_ONCE:-}" = 1 ] && [ "$count" -eq 1 ]; then exit 0; fi' \
        '                    if [ "${FAKE_ZELLIJ_EMPTY_ARRAY_LIST_TABS_ONCE:-}" = 1 ] && [ "$count" -eq 1 ]; then printf "[]\n"; exit 0; fi' \
        '                    cat "$FAKE_ZELLIJ_TABS"' \
        '                    exit 0' \
        '                    ;;' \
        '                focus-pane-id)' \
        '                    [ "${3:-}" = plugin_50 ] && [ "$#" -eq 3 ] || exit 64' \
        '                    printf "focus-pane-id\\t%s\\t%s\\n" "$session" "$3" >> "$FAKE_ZELLIJ_CALLS"' \
        '                    exit 0' \
        '                    ;;' \
        '                new-tab) ;;' \
        '                *) printf "unexpected action: %s\\n" "${2:-}" >&2; exit 64 ;;' \
        '            esac' \
        '            shift 2' \
        '            [ "${1:-}" = --name ] && [ -n "${2:-}" ] || exit 64' \
        '            name="$2"' \
        '            shift 2' \
        '            [ "${1:-}" = --cwd ] && [ -n "${2:-}" ] || exit 64' \
        '            printf "%s\\n" "$2" > "$FAKE_ZELLIJ_CWD"' \
        '            shift 2' \
        '            [ "${1:-}" = --layout-string ] && [ -n "${2:-}" ] || exit 64' \
        '            printf "action-new-tab\\t%s\\t%s\\t%s\\t%s\\n" "$config_dir" "$config_file" "$data_dir" "$session" >> "$FAKE_ZELLIJ_CALLS"' \
        '            printf "%s\\n" "$session" > "$FAKE_ZELLIJ_SESSION"' \
        '            printf "%s\\n" "$name" > "$FAKE_ZELLIJ_NAME"' \
        '            printf "%s" "$2" > "$FAKE_ZELLIJ_LAYOUT"' \
        '            shift 2' \
        '            [ "${1:-}" = -- ] && [ "$#" -ge 4 ] || exit 64' \
        '            [ -z "${FAKE_ZELLIJ_COMMAND:-}" ] || printf "%s\\n" "$@" > "$FAKE_ZELLIJ_COMMAND"' \
        '            count=0' \
        '            [ ! -f "$FAKE_ZELLIJ_NEW_TAB_COUNT" ] || count="$(cat "$FAKE_ZELLIJ_NEW_TAB_COUNT")"' \
        '            printf "%s\\n" "$((count + 1))" > "$FAKE_ZELLIJ_NEW_TAB_COUNT"' \
        '            if [ -n "${FAKE_ZELLIJ_READY_FILE:-}" ]; then' \
        '                : > "$FAKE_ZELLIJ_READY_FILE"' \
        '                while [ ! -e "${FAKE_ZELLIJ_RELEASE_FILE:?}" ]; do sleep 0.05; done' \
        '            fi' \
        '            status="${FAKE_ZELLIJ_NEW_TAB_STATUS:-0}"' \
        '            [ "$status" -eq 0 ] || exit "$status"' \
        '            if [ "${FAKE_ZELLIJ_TABS_AFTER+x}" = x ]; then' \
        '                printf "%s" "$FAKE_ZELLIJ_TABS_AFTER" > "$FAKE_ZELLIJ_TABS"' \
        '            else' \
        '                printf "[{\"tab_id\":4},{\"tab_id\":%s}]\\n" "${FAKE_ZELLIJ_TAB_ID:-73}" > "$FAKE_ZELLIJ_TABS"' \
        '            fi' \
        '            [ "${FAKE_ZELLIJ_NEW_TAB_STDERR+x}" != x ] || printf "%s" "$FAKE_ZELLIJ_NEW_TAB_STDERR" >&2' \
        '            if [ "${FAKE_ZELLIJ_NEW_TAB_STDOUT+x}" = x ]; then' \
        '                printf "%s" "$FAKE_ZELLIJ_NEW_TAB_STDOUT"' \
        '            else' \
        '                printf "%s\\n" "${FAKE_ZELLIJ_TAB_ID:-73}"' \
        '            fi' \
        '            exit 0' \
        '            ;;' \
        '        *) printf "unexpected fake zellij invocation: %s\\n" "$*" >&2; exit 64 ;;' \
        '    esac' \
        'done' \
        'exit 64' \
        > "$fixture/bin/zellij"
    chmod +x "$fixture/bin/zellij"
}

write_valid_unrelated_config() {
    local config_file="$1"
    printf '%s\n' \
        '// unrelated quoted brace must remain valid and byte-identical' \
        'keybinds clear-defaults=true {' \
        '    normal {' \
        '        bind "Alt Shift x" { WriteChars "{"; }' \
        '        bind "Alt Shift z" {' \
        '            NewTab { layout "/fixed/operator/layout.kdl"; }' \
        '        }' \
        '        bind "Alt /" { NewPane; }' \
        '    }' \
        '}' > "$config_file"
}

setup_fixture() {
    local root="$1"
    FIXTURE="$root/fixture"
    FIXTURE_CONFIG_DIR="$root/isolated-config"
    FIXTURE_CONFIG_FILE="$root/custom-config/fixture-config.kdl"
    FIXTURE_DATA_DIR="$root/isolated-data"
    FIXTURE_HOME="$root/fixture-home"
    FIXTURE_LAYOUT="$FIXTURE_CONFIG_DIR/layouts/zaphod.kdl"
    FIXTURE_TMP="$root/tmp"
    FIXTURE_OUTPUT="$root/output"
    FIXTURE_ERROR="$root/error"
    FIXTURE_BUILD_LOG="$root/build.log"
    FAKE_ZELLIJ_CALLS="$root/fake-zellij-calls"
    FAKE_ZELLIJ_SESSION="$root/fake-session"
    FAKE_ZELLIJ_NAME="$root/fake-name"
    FAKE_ZELLIJ_CWD="$root/fake-cwd"
    FAKE_ZELLIJ_COMMAND="$root/fake-command"
    FAKE_ZELLIJ_LAYOUT="$root/fake-layout.kdl"
    FAKE_ZELLIJ_NEW_TAB_COUNT="$root/fake-new-tab-count"
    FAKE_ZELLIJ_LIST_TABS_COUNT="$root/fake-list-tabs-count"
    FAKE_ZELLIJ_PANES="$root/fake-panes.json"
    FAKE_ZELLIJ_TABS="$root/fake-tabs.json"
    FAKE_ZAPHOD_CALLS="$root/fake-zaphod-calls"

    mkdir -p "$FIXTURE/scripts" "$FIXTURE/layouts" \
        "$FIXTURE_CONFIG_DIR/layouts" "$(dirname "$FIXTURE_CONFIG_FILE")" \
        "$FIXTURE_DATA_DIR" "$FIXTURE_HOME" "$FIXTURE_TMP"
    cp "$REPO_ROOT/layouts/zaphod.kdl" "$FIXTURE/layouts/zaphod.kdl"
    cp "$REPO_ROOT/scripts/zellij-layout-lib.sh" "$FIXTURE/scripts/zellij-layout-lib.sh"
    cp "$REPO_ROOT/scripts/zellij-new-tab.sh" "$FIXTURE/scripts/zellij-new-tab.sh"
    chmod +x "$FIXTURE/scripts/zellij-new-tab.sh"
    write_fake_build "$FIXTURE"
    write_fake_zellij "$FIXTURE"
    write_valid_unrelated_config "$FIXTURE_CONFIG_FILE"
    printf '%s\n' 'operator fixed-layout sentinel' > "$FIXTURE_LAYOUT"
    FIXTURE_PHYSICAL="$(cd "$FIXTURE" && pwd -P)"
    printf '[{"id":50,"tab_id":73,"is_plugin":true,"plugin_url":"file:%s/target/wasm32-wasip1/release/zellij-sidebar.wasm","is_floating":false,"is_suppressed":false}]\n' \
        "$FIXTURE_PHYSICAL" > "$FAKE_ZELLIJ_PANES"
    printf '[{"tab_id":4}]\n' > "$FAKE_ZELLIJ_TABS"
    record_initial_file_bytes
}

run_entry() {
    PATH="$FIXTURE/bin:$PATH" \
        BUILD_LOG="$FIXTURE_BUILD_LOG" \
        FAKE_ZELLIJ_CALLS="$FAKE_ZELLIJ_CALLS" \
        FAKE_ZELLIJ_SESSION="$FAKE_ZELLIJ_SESSION" \
        FAKE_ZELLIJ_NAME="$FAKE_ZELLIJ_NAME" \
        FAKE_ZELLIJ_CWD="$FAKE_ZELLIJ_CWD" \
        FAKE_ZELLIJ_COMMAND="$FAKE_ZELLIJ_COMMAND" \
        FAKE_ZELLIJ_LAYOUT="$FAKE_ZELLIJ_LAYOUT" \
        FAKE_ZELLIJ_NEW_TAB_COUNT="$FAKE_ZELLIJ_NEW_TAB_COUNT" \
        FAKE_ZELLIJ_LIST_TABS_COUNT="$FAKE_ZELLIJ_LIST_TABS_COUNT" \
        FAKE_ZELLIJ_PANES="$FAKE_ZELLIJ_PANES" \
        FAKE_ZELLIJ_TABS="$FAKE_ZELLIJ_TABS" \
        FAKE_ZAPHOD_CALLS="$FAKE_ZAPHOD_CALLS" \
        ZELLIJ_CONFIG_DIR="$FIXTURE_CONFIG_DIR" \
        ZELLIJ_CONFIG_FILE="$FIXTURE_CONFIG_FILE" \
        ZELLIJ_DATA_DIR="$FIXTURE_DATA_DIR" \
        ZAPHOD_WATCH_DIR="$FIXTURE_DATA_DIR/watch-tab-v1" \
        SHELL=/bin/sh \
        TMPDIR="$FIXTURE_TMP" \
        "$FIXTURE/scripts/zellij-new-tab.sh" "$@"
}

record_initial_file_bytes() {
    CONFIG_BEFORE="$(sha256 "$FIXTURE_CONFIG_FILE")"
    LAYOUT_BEFORE="$(sha256 "$FIXTURE_LAYOUT")"
}

assert_standing_kdl_unchanged() {
    [ "$(sha256 "$FIXTURE_CONFIG_FILE")" = "$CONFIG_BEFORE" ] ||
        fail "entry point changed standing config.kdl"
    [ "$(sha256 "$FIXTURE_LAYOUT")" = "$LAYOUT_BEFORE" ] ||
        fail "entry point changed standing layouts/zaphod.kdl"
    [ "$(grep -Fc 'WriteChars "{"' "$FIXTURE_CONFIG_FILE")" -eq 1 ] ||
        fail "entry point changed the quoted-brace binding"
    [ "$(grep -Fc 'layout "/fixed/operator/layout.kdl"' "$FIXTURE_CONFIG_FILE")" -eq 1 ] ||
        fail "entry point changed the fixed Alt Shift z layout"
}

assert_temporary_files_cleaned() {
    if find "$FIXTURE_TMP" -mindepth 1 -maxdepth 1 -type d \
        -name 'zaphod-new-tab.*' -print -quit | grep -q .; then
        fail "entry point left a rendered-layout temporary directory"
    fi
    if find "$FIXTURE_CONFIG_DIR" "$(dirname "$FIXTURE_CONFIG_FILE")" \
        -maxdepth 1 -type f -name '.zaphod-*' -print -quit | grep -q .; then
        fail "entry point left a config/layout backup or temporary file"
    fi
}

assert_manual_watcher_route() {
    local expected_url="$1" recipient_token
    recipient_token="$(sed -n 's/.*recipient_token "\([^"]*\)".*/\1/p' "$FAKE_ZELLIJ_LAYOUT" | head -1)"
    [ -n "$recipient_token" ] || fail "inline layout did not carry a recipient token"
    for route in \
        "ZAPHOD_WATCH_DIR=$FIXTURE_DATA_DIR/watch-tab-v1" \
        "ZAPHOD_AGENTSVIEW_URL=http://127.0.0.1:8080" \
        "ZAPHOD_RAIL_URL=$expected_url" \
        "ZAPHOD_RECIPIENT_TOKEN=$recipient_token" \
        "ZAPHOD_ZELLIJ_CONFIG_DIR=$FIXTURE_CONFIG_DIR" \
        "ZAPHOD_ZELLIJ_CONFIG_FILE=$FIXTURE_CONFIG_FILE" \
        "ZAPHOD_ZELLIJ_DATA_DIR=$FIXTURE_DATA_DIR" \
        "ZELLIJ_BIN=zellij"; do
        grep -Fx "$route" "$FAKE_ZELLIJ_COMMAND" >/dev/null || fail "managed terminal omitted $route"
    done
    [ "$(tail -n 2 "$FAKE_ZELLIJ_COMMAND" | head -n 1)" = /bin/sh ] ||
        fail "managed terminal did not preserve the selected shell"
    [ ! -e "$FAKE_ZAPHOD_CALLS" ] || fail "direct entry unexpectedly started a watcher"
}

test_selected_checkout_creates_one_inline_tab_without_writes() {
    local root expected_url
    root="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-new-tab-test.XXXXXX")"
    TEST_ROOT="$root"
    setup_fixture "$root"
    expected_url="file:$FIXTURE_PHYSICAL/target/wasm32-wasip1/release/zellij-sidebar.wasm"

    run_entry --session WORK --name 'Zaphod fixture' > "$FIXTURE_OUTPUT" 2> "$FIXTURE_ERROR" || {
        sed -n '1,200p' "$FIXTURE_ERROR" >&2
        fail "selected-checkout fresh-tab entry failed"
    }

    [ "$(cat "$FIXTURE_BUILD_LOG")" = "$FIXTURE_PHYSICAL/build.sh" ] ||
        fail "entry point did not build the selected checkout"
    grep -Fx 'TAB_ID=73' "$FIXTURE_OUTPUT" >/dev/null || fail "entry point did not report stable tab ID"
    grep -Fx "WASM_URL=$expected_url" "$FIXTURE_OUTPUT" >/dev/null || fail "entry point did not report selected URL"
    grep -Fx "WATCH_DIR=$FIXTURE_DATA_DIR/watch-tab-v1" "$FIXTURE_OUTPUT" >/dev/null || fail "entry point did not report watch root"
    grep -Fx "WATCH_COMMAND=$FIXTURE_PHYSICAL/target/zaphod watch-tab" "$FIXTURE_OUTPUT" >/dev/null || fail "entry point did not report manual watcher command"
    [ "$(cat "$FAKE_ZELLIJ_NEW_TAB_COUNT")" = 1 ] || fail "entry point did not create exactly one tab"
    [ "$(cat "$FAKE_ZELLIJ_SESSION")" = WORK ] || fail "new-tab used the wrong session"
    [ "$(cat "$FAKE_ZELLIJ_NAME")" = 'Zaphod fixture' ] || fail "new-tab used the wrong name"
    [ "$(cat "$FAKE_ZELLIJ_CWD")" = "$FIXTURE_PHYSICAL" ] || fail "new-tab used the wrong cwd"
    [ -f "$FAKE_ZELLIJ_COMMAND" ] || fail "new-tab did not pass a managed shell command"
    ! grep -F $'focus-pane-id\t' "$FAKE_ZELLIJ_CALLS" >/dev/null ||
        fail "entry point moved focus away from the selected terminal"
    grep -F "plugin location=\"$expected_url\"" "$FAKE_ZELLIJ_LAYOUT" >/dev/null ||
        fail "inline layout did not contain selected checkout URL"
    assert_manual_watcher_route "$expected_url"
    assert_standing_kdl_unchanged
    assert_temporary_files_cleaned
    [ ! -e "$FIXTURE/scripts/zellij-config-activate.awk" ] || fail "fixture unexpectedly supplied AWK transformer"

    echo "PASS: selected checkout creates one verified inline tab without standing KDL writes"
}

test_setup_failure_stops_before_new_tab() {
    local root status
    root="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-new-tab-test.XXXXXX")"
    TEST_ROOT="$root"
    setup_fixture "$root"

    set +e
    FAKE_ZELLIJ_SETUP_STATUS=23 run_entry --session WORK > "$FIXTURE_OUTPUT" 2> "$FIXTURE_ERROR"
    status=$?
    set -e
    [ "$status" -ne 0 ] || fail "setup-check failure unexpectedly succeeded"
    [ ! -e "$FAKE_ZELLIJ_NEW_TAB_COUNT" ] || fail "setup-check failure reached new-tab"
    assert_standing_kdl_unchanged
    assert_temporary_files_cleaned
    echo "PASS: setup-check failure leaves standing KDL unchanged and creates no tab"
}

test_new_tab_failure_leaves_standing_kdl_unchanged() {
    local root status
    root="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-new-tab-test.XXXXXX")"
    TEST_ROOT="$root"
    setup_fixture "$root"
    set +e
    FAKE_ZELLIJ_NEW_TAB_STATUS=17 run_entry --session WORK > "$FIXTURE_OUTPUT" 2> "$FIXTURE_ERROR"
    status=$?
    set -e
    [ "$status" -ne 0 ] || fail "new-tab failure unexpectedly succeeded"
    assert_standing_kdl_unchanged
    assert_temporary_files_cleaned
    echo "PASS: new-tab failure leaves standing KDL unchanged"
}

test_term_during_new_tab_leaves_standing_kdl_unchanged() {
    local root ready release runner_pid status attempt config_during layout_during
    root="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-new-tab-test.XXXXXX")"
    TEST_ROOT="$root"
    setup_fixture "$root"
    ready="$root/fake-zellij-ready"
    release="$root/fake-zellij-release"

    env "PATH=$FIXTURE/bin:$PATH" "BUILD_LOG=$FIXTURE_BUILD_LOG" \
        "FAKE_ZELLIJ_CALLS=$FAKE_ZELLIJ_CALLS" "FAKE_ZELLIJ_SESSION=$FAKE_ZELLIJ_SESSION" \
        "FAKE_ZELLIJ_NAME=$FAKE_ZELLIJ_NAME" "FAKE_ZELLIJ_CWD=$FAKE_ZELLIJ_CWD" \
        "FAKE_ZELLIJ_LAYOUT=$FAKE_ZELLIJ_LAYOUT" \
        "FAKE_ZELLIJ_NEW_TAB_COUNT=$FAKE_ZELLIJ_NEW_TAB_COUNT" "FAKE_ZELLIJ_PANES=$FAKE_ZELLIJ_PANES" \
        "FAKE_ZELLIJ_TABS=$FAKE_ZELLIJ_TABS" \
        "FAKE_ZAPHOD_CALLS=$FAKE_ZAPHOD_CALLS" "FAKE_ZELLIJ_READY_FILE=$ready" \
        "FAKE_ZELLIJ_RELEASE_FILE=$release" "ZELLIJ_CONFIG_DIR=$FIXTURE_CONFIG_DIR" \
        "ZELLIJ_CONFIG_FILE=$FIXTURE_CONFIG_FILE" "ZELLIJ_DATA_DIR=$FIXTURE_DATA_DIR" \
        "TMPDIR=$FIXTURE_TMP" "$FIXTURE/scripts/zellij-new-tab.sh" --session WORK \
        > "$FIXTURE_OUTPUT" 2> "$FIXTURE_ERROR" &
    runner_pid=$!
    for attempt in $(seq 1 100); do [ ! -e "$ready" ] || break; sleep 0.05; done
    [ -e "$ready" ] || { cat "$FIXTURE_ERROR" >&2; fail "TERM fixture never reached new-tab"; }
    config_during="$(sha256 "$FIXTURE_CONFIG_FILE" 2>/dev/null || printf '%s\n' unreadable)"
    layout_during="$(sha256 "$FIXTURE_LAYOUT" 2>/dev/null || printf '%s\n' unreadable)"
    kill -TERM "$runner_pid"
    : > "$release"
    set +e
    wait "$runner_pid"
    status=$?
    set -e
    [ "$status" -eq 143 ] || fail "TERM fixture exited $status instead of 143"
    [ "$config_during" = "$CONFIG_BEFORE" ] || fail "entry point changed standing config.kdl in flight"
    [ "$layout_during" = "$LAYOUT_BEFORE" ] || fail "entry point changed standing layouts/zaphod.kdl in flight"
    assert_standing_kdl_unchanged
    assert_temporary_files_cleaned
    echo "PASS: TERM during new-tab leaves standing KDL unchanged"
}

test_unready_resident_reports_missing_rail() {
    local root status
    root="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-new-tab-test.XXXXXX")"
    TEST_ROOT="$root"
    setup_fixture "$root"
    printf '[]\n' > "$FAKE_ZELLIJ_PANES"
    set +e
    run_entry --session WORK > "$FIXTURE_OUTPUT" 2> "$FIXTURE_ERROR"
    status=$?
    set -e
    [ "$status" -ne 0 ] || fail "unready resident unexpectedly succeeded"
    grep -F 'rail-target-unready:' "$FIXTURE_ERROR" >/dev/null || fail "missing unready error"
    [ "$(cat "$FAKE_ZELLIJ_NEW_TAB_COUNT")" = 1 ] || fail "unready path changed tab count"
    assert_standing_kdl_unchanged
    echo "PASS: unready resident preserves tab and reports missing rail"
}

test_empty_new_tab_stdout_uses_inventory_stable_id() {
    local root expected_url
    root="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-new-tab-test.XXXXXX")"
    TEST_ROOT="$root"
    setup_fixture "$root"
    expected_url="file:$FIXTURE_PHYSICAL/target/wasm32-wasip1/release/zellij-sidebar.wasm"

    FAKE_ZELLIJ_NEW_TAB_STDOUT="" run_entry --session WORK --name 'Zaphod empty stdout' \
        > "$FIXTURE_OUTPUT" 2> "$FIXTURE_ERROR" || {
        sed -n '1,200p' "$FIXTURE_ERROR" >&2
        fail "empty new-tab stdout prevented stable-ID inventory discovery"
    }

    grep -Fx 'TAB_ID=73' "$FIXTURE_OUTPUT" >/dev/null ||
        fail "inventory discovery did not report stable tab ID 73"
    assert_manual_watcher_route "$expected_url"
    assert_standing_kdl_unchanged
    assert_temporary_files_cleaned
    echo "PASS: empty new-tab stdout uses stable inventory identity"
}

test_empty_initial_tab_inventory_retries_before_creation() {
    local root expected_url
    root="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-new-tab-test.XXXXXX")"
    TEST_ROOT="$root"
    setup_fixture "$root"
    expected_url="file:$FIXTURE_PHYSICAL/target/wasm32-wasip1/release/zellij-sidebar.wasm"

    FAKE_ZELLIJ_EMPTY_LIST_TABS_ONCE=1 run_entry --session WORK \
        > "$FIXTURE_OUTPUT" 2> "$FIXTURE_ERROR" || {
        sed -n '1,200p' "$FIXTURE_ERROR" >&2
        fail "empty successful initial tab inventory did not recover"
    }

    [ "$(cat "$FAKE_ZELLIJ_LIST_TABS_COUNT")" -ge 3 ] ||
        fail "empty initial inventory was not retried before post-create discovery"
    grep -Fx 'TAB_ID=73' "$FIXTURE_OUTPUT" >/dev/null || fail "inventory retry lost stable tab identity"
    assert_manual_watcher_route "$expected_url"
    assert_standing_kdl_unchanged
    assert_temporary_files_cleaned
    echo "PASS: empty initial tab inventory retries before creation"
}

test_empty_array_initial_tab_inventory_retries_before_creation() {
    local root expected_url
    root="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-new-tab-test.XXXXXX")"
    TEST_ROOT="$root"
    setup_fixture "$root"
    expected_url="file:$FIXTURE_PHYSICAL/target/wasm32-wasip1/release/zellij-sidebar.wasm"

    FAKE_ZELLIJ_EMPTY_ARRAY_LIST_TABS_ONCE=1 run_entry --session WORK \
        > "$FIXTURE_OUTPUT" 2> "$FIXTURE_ERROR" || {
        sed -n '1,200p' "$FIXTURE_ERROR" >&2
        fail "empty-array initial tab inventory did not recover"
    }

    [ "$(cat "$FAKE_ZELLIJ_LIST_TABS_COUNT")" -ge 3 ] ||
        fail "empty-array initial inventory was not retried before post-create discovery"
    grep -Fx 'TAB_ID=73' "$FIXTURE_OUTPUT" >/dev/null || fail "empty-array retry lost stable tab identity"
    assert_manual_watcher_route "$expected_url"
    assert_standing_kdl_unchanged
    assert_temporary_files_cleaned
    echo "PASS: empty-array initial tab inventory retries before creation"
}

test_inside_caller_identity_is_cleared_before_native_entry_calls() {
    local root expected_url
    root="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-new-tab-test.XXXXXX")"
    TEST_ROOT="$root"
    setup_fixture "$root"
    expected_url="file:$FIXTURE_PHYSICAL/target/wasm32-wasip1/release/zellij-sidebar.wasm"

    ZELLIJ=0 ZELLIJ_SESSION_NAME=ambient-work ZELLIJ_PANE_ID=98765 \
        run_entry --session WORK > "$FIXTURE_OUTPUT" 2> "$FIXTURE_ERROR" || {
        sed -n '1,200p' "$FIXTURE_ERROR" >&2
        fail "inside-caller selected entry failed"
    }

    grep -F $'version\t' "$FAKE_ZELLIJ_CALLS" | grep -F $'client=\tsession=\tpane=' >/dev/null ||
        fail "version probe inherited loaded Zellij client identity"
    grep -F $'setup\t' "$FAKE_ZELLIJ_CALLS" | grep -F $'client=\tsession=\tpane=' >/dev/null ||
        fail "setup check inherited loaded Zellij client identity"
    grep -Fx 'TAB_ID=73' "$FIXTURE_OUTPUT" >/dev/null || fail "inside caller lost stable tab identity"
    assert_manual_watcher_route "$expected_url"
    assert_standing_kdl_unchanged
    assert_temporary_files_cleaned
    echo "PASS: inside caller identity is cleared before native entry calls"
}

test_ambiguous_tab_discovery_reports_bounded_native_provenance() {
    local root status stdout_payload stderr_payload
    root="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-new-tab-test.XXXXXX")"
    TEST_ROOT="$root"
    setup_fixture "$root"
    stdout_payload="candidate-reply-$(printf '%0300d' 0)"
    stderr_payload="route-warning-$(printf '%0300d' 0)"

    set +e
    FAKE_ZELLIJ_TABS_AFTER='[{"tab_id":4},{"tab_id":73},{"tab_id":74}]' \
        FAKE_ZELLIJ_NEW_TAB_STDOUT="$stdout_payload" \
        FAKE_ZELLIJ_NEW_TAB_STDERR="$stderr_payload" \
        run_entry --session WORK > "$FIXTURE_OUTPUT" 2> "$FIXTURE_ERROR"
    status=$?
    set -e

    [ "$status" -ne 0 ] || fail "ambiguous tab discovery unexpectedly succeeded"
    grep -F 'rail-target-unready: new-tab status=0' "$FIXTURE_ERROR" >/dev/null ||
        fail "ambiguous discovery omitted new-tab status provenance"
    grep -F "stdout_len=${#stdout_payload}" "$FIXTURE_ERROR" >/dev/null ||
        fail "ambiguous discovery omitted stdout length"
    grep -F "stderr_len=${#stderr_payload}" "$FIXTURE_ERROR" >/dev/null ||
        fail "ambiguous discovery omitted stderr length"
    grep -F 'stdout_prefix="candidate-reply-' "$FIXTURE_ERROR" >/dev/null ||
        fail "ambiguous discovery omitted escaped stdout prefix"
    grep -F 'stderr_prefix="route-warning-' "$FIXTURE_ERROR" >/dev/null ||
        fail "ambiguous discovery omitted escaped stderr prefix"
    ! grep -F "$stdout_payload" "$FIXTURE_ERROR" >/dev/null ||
        fail "ambiguous discovery leaked unbounded stdout"
    ! grep -F "$stderr_payload" "$FIXTURE_ERROR" >/dev/null ||
        fail "ambiguous discovery leaked unbounded stderr"
    assert_standing_kdl_unchanged
    assert_temporary_files_cleaned
    echo "PASS: ambiguous tab discovery reports bounded native provenance"
}

test_tokenless_layout_render_preserves_installed_identity() {
    local root output
    root="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-layout-tokenless.XXXXXX")"
    output="$root/zaphod.kdl"
    # shellcheck source=scripts/zellij-layout-lib.sh
    source "$REPO_ROOT/scripts/zellij-layout-lib.sh"
    zaphod_render_layout "$REPO_ROOT/layouts/zaphod.kdl" \
        'file:/fixed/zellij-sidebar.wasm' "$output"
    ! grep -F 'recipient_token' "$output" >/dev/null ||
        fail "tokenless installed layout changed plugin configuration identity"
    [ "$(grep -Fc 'rail "1"' "$output")" -ge 3 ] ||
        fail "tokenless installed layout lost the fixed rail identity"
    rm -rf "$root"
    echo "PASS: tokenless render preserves fixed installed plugin identity"
}

test_direct_entry_injects_manual_watcher_route_without_starting_one() {
    local script="$REPO_ROOT/scripts/zellij-new-tab.sh"
    ! grep -E 'start_private_sidecar|[[:space:]]subscribe([[:space:]]|$)' "$script" >/dev/null ||
        fail "direct entry still launches an automatic subscriber"
    ! grep -F 'ZAPHOD_REGISTRY_DIR' "$script" >/dev/null ||
        fail "direct entry still advertises persistent registry authority"
    for name in ZAPHOD_WATCH_DIR ZAPHOD_AGENTSVIEW_URL ZAPHOD_RAIL_URL \
        ZAPHOD_RECIPIENT_TOKEN ZAPHOD_ZELLIJ_CONFIG_DIR ZAPHOD_ZELLIJ_CONFIG_FILE \
        ZAPHOD_ZELLIJ_DATA_DIR ZELLIJ_BIN; do
        grep -F "$name=" "$script" >/dev/null || fail "managed terminal omits $name"
    done
    echo "PASS: direct entry injects the manual watcher route and starts no subscriber"
}

test_selected_checkout_creates_one_inline_tab_without_writes
test_setup_failure_stops_before_new_tab
test_new_tab_failure_leaves_standing_kdl_unchanged
test_term_during_new_tab_leaves_standing_kdl_unchanged
test_unready_resident_reports_missing_rail
test_empty_new_tab_stdout_uses_inventory_stable_id
test_empty_initial_tab_inventory_retries_before_creation
test_empty_array_initial_tab_inventory_retries_before_creation
test_inside_caller_identity_is_cleared_before_native_entry_calls
test_ambiguous_tab_discovery_reports_bounded_native_provenance
test_tokenless_layout_render_preserves_installed_identity
test_direct_entry_injects_manual_watcher_route_without_starting_one
