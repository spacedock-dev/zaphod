#!/bin/bash
# ABOUTME: Black-box regressions for the fresh Zaphod-tab activation entry point.
# ABOUTME: Uses a copied checkout with fake build and Zellij commands; no real WASM or Zellij server.
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
        'while [ "$#" -gt 0 ]; do' \
        '    case "$1" in' \
        '        --config-dir)' \
        '            config_dir="${2:?missing --config-dir value}"' \
        '            shift 2' \
        '            ;;' \
        '        --config)' \
        '            config_file="${2:?missing --config value}"' \
        '            shift 2' \
        '            ;;' \
        '        --data-dir)' \
        '            data_dir="${2:?missing --data-dir value}"' \
        '            shift 2' \
        '            ;;' \
        '        --version)' \
        '            printf "version\\t%s\\t%s\\t%s\\t%s\\n" "$config_dir" "$config_file" "$data_dir" "${ZELLIJ_SESSION_NAME:-}" >> "$FAKE_ZELLIJ_CALLS"' \
        '            printf "zellij 0.44.3\\n"' \
        '            exit 0' \
        '            ;;' \
        '        setup)' \
        '            [ "${2:-}" = "--check" ] || { printf "unexpected setup invocation\\n" >&2; exit 64; }' \
        '            printf "setup\\t%s\\t%s\\t%s\\t%s\\n" "$config_dir" "$config_file" "$data_dir" "${ZELLIJ_SESSION_NAME:-}" >> "$FAKE_ZELLIJ_CALLS"' \
        '            exit "${FAKE_ZELLIJ_SETUP_STATUS:-0}"' \
        '            ;;' \
        '        action)' \
        '            [ "${2:-}" = "new-tab" ] || { printf "unexpected action invocation\\n" >&2; exit 64; }' \
        '            shift 2' \
        '            if [ "${1:-}" != "--name" ] || [ -z "${2:-}" ]; then' \
        '                printf "missing --name in fake zellij invocation\\n" >&2' \
        '                exit 64' \
        '            fi' \
        '            name="$2"' \
        '            shift 2' \
        '            if [ "${1:-}" != "--layout-string" ] || [ -z "${2:-}" ] || [ "$#" -ne 2 ]; then' \
        '                printf "missing inline layout in fake zellij invocation\\n" >&2' \
        '                exit 64' \
        '            fi' \
        '            printf "action-new-tab\\t%s\\t%s\\t%s\\t%s\\n" "$config_dir" "$config_file" "$data_dir" "${ZELLIJ_SESSION_NAME:-}" >> "$FAKE_ZELLIJ_CALLS"' \
        '            printf "%s\\n" "${ZELLIJ_SESSION_NAME:-}" > "$FAKE_ZELLIJ_SESSION"' \
        '            printf "%s\\n" "$name" > "$FAKE_ZELLIJ_NAME"' \
        '            printf "%s" "$2" > "$FAKE_ZELLIJ_LAYOUT"' \
        '            count=0' \
        '            if [ -f "$FAKE_ZELLIJ_NEW_TAB_COUNT" ]; then' \
        '                count="$(cat "$FAKE_ZELLIJ_NEW_TAB_COUNT")"' \
        '            fi' \
        '            printf "%s\\n" "$((count + 1))" > "$FAKE_ZELLIJ_NEW_TAB_COUNT"' \
        '            if [ -n "${FAKE_ZELLIJ_READY_FILE:-}" ]; then' \
        '                : > "$FAKE_ZELLIJ_READY_FILE"' \
        '                while [ ! -e "${FAKE_ZELLIJ_RELEASE_FILE:?}" ]; do sleep 0.05; done' \
        '            fi' \
        '            status="${FAKE_ZELLIJ_NEW_TAB_STATUS:-0}"' \
        '            if [ "$status" -ne 0 ]; then' \
        '                exit "$status"' \
        '            fi' \
        '            printf "%s\\n" "${FAKE_ZELLIJ_TAB_ID:-73}"' \
        '            exit 0' \
        '            ;;' \
        '        *)' \
        '            printf "unexpected fake zellij invocation: %s\\n" "$*" >&2' \
        '            exit 64' \
        '            ;;' \
        '    esac' \
        'done' \
        'printf "missing zellij command\\n" >&2' \
        'exit 64' \
        > "$fixture/bin/zellij"
    chmod +x "$fixture/bin/zellij"
}

write_routable_config() {
    local config_file="$1"
    printf '%s\n' \
        'keybinds clear-defaults=true {' \
        '    locked {' \
        '        bind "Alt /" {' \
        '            MessagePlugin "file:/stale/locked/zellij-sidebar.wasm" {' \
        '                name "toggle"' \
        '                floating true' \
        '                rail "0"' \
        '            }' \
        '        }' \
        '        bind "Alt ." {' \
        '            MessagePlugin "file:/stale/locked/zellij-sidebar.wasm" {' \
        '                name "navigate"' \
        '                floating true' \
        '                rail "1"' \
        '            }' \
        '        }' \
        '        bind "Alt Shift z" { NewPane; }' \
        '        bind "Alt Shift x" { WriteChars "unrelated locked binding"; }' \
        '    }' \
        '    shared_except "locked" {' \
        '        bind "Alt /" {' \
        '            MessagePlugin "file:/stale/shared/zellij-sidebar.wasm" {' \
        '                name "toggle"' \
        '                floating true' \
        '                rail "1"' \
        '            }' \
        '        }' \
        '        bind "Alt ." {' \
        '            MessagePlugin "file:/stale/shared/zellij-sidebar.wasm" {' \
        '                name "navigate"' \
        '                floating true' \
        '                rail "1"' \
        '            }' \
        '        }' \
        '        bind "Alt q" {' \
        '            MessagePlugin "file:/unrelated/tool.wasm" {' \
        '                name "unrelated"' \
        '            }' \
        '        }' \
        '    }' \
        '    normal {' \
        '        bind "Alt Shift x" { WriteChars "unrelated normal binding"; }' \
        '    }' \
        '}' > "$config_file"
}

setup_fixture() {
    local root="$1"
    FIXTURE="$root/fixture"
    FIXTURE_PHYSICAL=""
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
    FAKE_ZELLIJ_LAYOUT="$root/fake-layout.kdl"
    FAKE_ZELLIJ_NEW_TAB_COUNT="$root/fake-new-tab-count"

    mkdir -p "$FIXTURE/scripts" "$FIXTURE/layouts" \
        "$FIXTURE_CONFIG_DIR/layouts" "$(dirname "$FIXTURE_CONFIG_FILE")" \
        "$FIXTURE_DATA_DIR" "$FIXTURE_HOME" "$FIXTURE_TMP"
    cp "$REPO_ROOT/layouts/zaphod.kdl" "$FIXTURE/layouts/zaphod.kdl"
    cp "$REPO_ROOT/scripts/zellij-layout-lib.sh" "$FIXTURE/scripts/zellij-layout-lib.sh"
    cp "$REPO_ROOT/scripts/zellij-config-activate.awk" "$FIXTURE/scripts/zellij-config-activate.awk"
    [ -f "$REPO_ROOT/scripts/zellij-new-tab.sh" ] ||
        fail "entry script missing: $REPO_ROOT/scripts/zellij-new-tab.sh"
    cp "$REPO_ROOT/scripts/zellij-new-tab.sh" "$FIXTURE/scripts/zellij-new-tab.sh"
    chmod +x "$FIXTURE/scripts/zellij-new-tab.sh"
    write_fake_build "$FIXTURE"
    write_fake_zellij "$FIXTURE"
    write_routable_config "$FIXTURE_CONFIG_FILE"
    FIXTURE_PHYSICAL="$(cd "$FIXTURE" && pwd -P)"
}

run_entry() {
    PATH="$FIXTURE/bin:$PATH" \
        BUILD_LOG="$FIXTURE_BUILD_LOG" \
        FAKE_ZELLIJ_CALLS="$FAKE_ZELLIJ_CALLS" \
        FAKE_ZELLIJ_SESSION="$FAKE_ZELLIJ_SESSION" \
        FAKE_ZELLIJ_NAME="$FAKE_ZELLIJ_NAME" \
        FAKE_ZELLIJ_LAYOUT="$FAKE_ZELLIJ_LAYOUT" \
        FAKE_ZELLIJ_NEW_TAB_COUNT="$FAKE_ZELLIJ_NEW_TAB_COUNT" \
        ZELLIJ_CONFIG_DIR="$FIXTURE_CONFIG_DIR" \
        ZELLIJ_CONFIG_FILE="$FIXTURE_CONFIG_FILE" \
        ZELLIJ_DATA_DIR="$FIXTURE_DATA_DIR" \
        TMPDIR="$FIXTURE_TMP" \
        "$FIXTURE/scripts/zellij-new-tab.sh" "$@"
}

run_entry_without_data_root() {
    (
        unset ZELLIJ_DATA_DIR
        PATH="$FIXTURE/bin:$PATH" \
            HOME="$FIXTURE_HOME" \
            BUILD_LOG="$FIXTURE_BUILD_LOG" \
            FAKE_ZELLIJ_CALLS="$FAKE_ZELLIJ_CALLS" \
            FAKE_ZELLIJ_SESSION="$FAKE_ZELLIJ_SESSION" \
            FAKE_ZELLIJ_NAME="$FAKE_ZELLIJ_NAME" \
            FAKE_ZELLIJ_LAYOUT="$FAKE_ZELLIJ_LAYOUT" \
            FAKE_ZELLIJ_NEW_TAB_COUNT="$FAKE_ZELLIJ_NEW_TAB_COUNT" \
            ZELLIJ_CONFIG_DIR="$FIXTURE_CONFIG_DIR" \
            ZELLIJ_CONFIG_FILE="$FIXTURE_CONFIG_FILE" \
            TMPDIR="$FIXTURE_TMP" \
            "$FIXTURE/scripts/zellij-new-tab.sh" "$@"
    )
}

record_initial_file_bytes() {
    CONFIG_BEFORE="$(sha256 "$FIXTURE_CONFIG_FILE")"
    LAYOUT_BEFORE="$(sha256 "$FIXTURE_LAYOUT")"
    CONFIG_BEFORE_FILE="$TEST_ROOT/config-before.kdl"
    LAYOUT_BEFORE_FILE="$TEST_ROOT/layout-before.kdl"
    cp "$FIXTURE_CONFIG_FILE" "$CONFIG_BEFORE_FILE"
    cp "$FIXTURE_LAYOUT" "$LAYOUT_BEFORE_FILE"
}

assert_temporary_files_cleaned() {
    if find "$FIXTURE_TMP" -mindepth 1 -maxdepth 1 -type d \
        -name 'zaphod-new-tab.*' -print -quit | grep -q .; then
        fail "entry point left a rendered-layout temporary directory"
    fi
    if find "$FIXTURE_CONFIG_DIR" "$(dirname "$FIXTURE_CONFIG_FILE")" \
        -maxdepth 1 -type f -name '.zaphod-*' -print -quit | grep -q .; then
        fail "entry point left an atomic-write temporary file"
    fi
}

assert_zellij_roots_are_isolated() {
    [ -s "$FAKE_ZELLIJ_CALLS" ] || fail "entry point did not invoke fake Zellij"
    awk -F '\t' \
        -v expected_config_dir="$FIXTURE_CONFIG_DIR" \
        -v expected_config_file="$FIXTURE_CONFIG_FILE" \
        -v expected_data_dir="$FIXTURE_DATA_DIR" '
        $2 != expected_config_dir || $3 != expected_config_file || $4 != expected_data_dir {
            if ($2 != expected_config_dir || $4 != expected_data_dir) {
                exit 1
            }
            if ($1 != "setup" || index($3, "/zaphod-new-tab.") == 0 || $3 !~ /config[.]kdl$/) {
                exit 1
            }
        }
    ' "$FAKE_ZELLIJ_CALLS" || fail "Zellij invocation escaped the isolated config/data roots"
}

assert_activation_rolled_back() {
    if [ "$(sha256 "$FIXTURE_CONFIG_FILE")" != "$CONFIG_BEFORE" ]; then
        diff -u "$CONFIG_BEFORE_FILE" "$FIXTURE_CONFIG_FILE" >&2 || true
        fail "failed activation changed the isolated config"
    fi
    if [ "$(sha256 "$FIXTURE_LAYOUT")" != "$LAYOUT_BEFORE" ]; then
        diff -u "$LAYOUT_BEFORE_FILE" "$FIXTURE_LAYOUT" >&2 || true
        fail "failed activation changed the isolated layout"
    fi
}

zaphod_keybind_scopes() {
    awk '
        /^[[:space:]]*locked[[:space:]]*\{/ { scope = "locked" }
        /^[[:space:]]*shared_except[[:space:]]+"locked"[[:space:]]*\{/ { scope = "shared_except_locked" }
        /^[[:space:]]*normal[[:space:]]*\{/ { scope = "normal" }
        /^[[:space:]]*bind "Alt Shift z"/ { print scope }
    ' "$1"
}

zaphod_noop_toggle_scopes() {
    awk '
        /^[[:space:]]*locked[[:space:]]*\{/ { scope = "locked" }
        /^[[:space:]]*shared_except[[:space:]]+"locked"[[:space:]]*\{/ { scope = "shared_except_locked" }
        /^[[:space:]]*normal[[:space:]]*\{/ { scope = "normal" }
        /^[[:space:]]*bind "Alt \/"[[:space:]]*\{[[:space:]]*NoOp;[[:space:]]*\}/ { print scope }
    ' "$1"
}

test_new_tab_activates_isolated_roots() {
    local root expected_url
    root="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-new-tab-test.XXXXXX")"
    TEST_ROOT="$root"
    setup_fixture "$root"
    printf '%s\n' 'standing layout sentinel: file:/stale/global/zaphod.wasm' > "$FIXTURE_LAYOUT"
    record_initial_file_bytes
    expected_url="file:$FIXTURE_PHYSICAL/target/wasm32-wasip1/release/zellij-sidebar.wasm"

    run_entry --session WORK --name 'Zaphod fixture' > "$FIXTURE_OUTPUT" 2> "$FIXTURE_ERROR" || {
        sed -n '1,200p' "$FIXTURE_ERROR" >&2
        fail "fresh-tab entry point failed"
    }

    [ "$(cat "$FIXTURE_BUILD_LOG")" = "$FIXTURE_PHYSICAL/build.sh" ] ||
        fail "entry point did not build the invoking fixture checkout"
    grep -Fx 'TAB_ID=73' "$FIXTURE_OUTPUT" >/dev/null ||
        fail "entry point did not print the new tab ID"
    grep -Fx "WASM_URL=$expected_url" "$FIXTURE_OUTPUT" >/dev/null ||
        fail "entry point did not print the fixture WASM URL"
    [ "$(cat "$FAKE_ZELLIJ_SESSION")" = 'WORK' ] ||
        fail "new-tab did not receive the requested session"
    [ "$(cat "$FAKE_ZELLIJ_NAME")" = 'Zaphod fixture' ] ||
        fail "new-tab did not receive the requested tab name"
    [ "$(cat "$FAKE_ZELLIJ_NEW_TAB_COUNT")" = '1' ] ||
        fail "entry point did not create exactly one new tab"
    grep -F "plugin location=\"$expected_url\"" "$FAKE_ZELLIJ_LAYOUT" >/dev/null ||
        fail "inline layout did not carry this fixture checkout's WASM URL"
    if grep -F 'file:/stale/' "$FAKE_ZELLIJ_LAYOUT" >/dev/null; then
        fail "inline layout reused a stale/global WASM path"
    fi

    # shellcheck source=/dev/null
    source "$FIXTURE/scripts/zellij-layout-lib.sh"
    zaphod_validate_layout_identity "$FIXTURE_LAYOUT" "$expected_url" ||
        fail "installed isolated layout did not retain the expected Zaphod rail identity"
    zaphod_validate_message_plugin_identity "$FIXTURE_CONFIG_FILE" "$expected_url" ||
        fail "isolated config did not route every Zaphod MessagePlugin to this checkout"
    if grep -F 'file:/stale/' "$FIXTURE_CONFIG_FILE" >/dev/null; then
        fail "activation left a stale Zaphod MessagePlugin URL"
    fi
    [ "$(grep -Fc 'bind "Alt Shift z"' "$FIXTURE_CONFIG_FILE")" -eq 2 ] ||
        fail "activation did not install one native keybind in each Zaphod scope"
    [ "$(grep -Fc 'layout "zaphod";' "$FIXTURE_CONFIG_FILE")" -eq 2 ] ||
        fail "native keybind did not route through the stored Zaphod layout"
    [ "$(zaphod_keybind_scopes "$FIXTURE_CONFIG_FILE")" = $'locked\nshared_except_locked' ] ||
        fail "activation routed Alt Shift z outside the scopes that own Zaphod"
    [ "$(grep -Fc 'bind "Alt /" { NoOp; }' "$FIXTURE_CONFIG_FILE")" -eq 2 ] ||
        fail "activation did not leave a fail-closed Alt / binding in each Zaphod scope"
    [ "$(zaphod_noop_toggle_scopes "$FIXTURE_CONFIG_FILE")" = $'locked\nshared_except_locked' ] ||
        fail "activation routed the fail-closed Alt / binding outside Zaphod scopes"
    grep -F 'bind "Alt Shift x" { WriteChars "unrelated locked binding"; }' \
        "$FIXTURE_CONFIG_FILE" >/dev/null || fail "activation changed an unrelated locked binding"
    grep -F 'bind "Alt Shift x" { WriteChars "unrelated normal binding"; }' \
        "$FIXTURE_CONFIG_FILE" >/dev/null || fail "activation changed an unrelated normal binding"
    grep -F 'MessagePlugin "file:/unrelated/tool.wasm"' "$FIXTURE_CONFIG_FILE" >/dev/null ||
        fail "activation changed an unrelated plugin binding"
    assert_zellij_roots_are_isolated
    assert_temporary_files_cleaned

    echo "PASS: fresh tab activates only the requested isolated Zellij roots"
}

test_missing_zaphod_route_fails_before_writes() {
    local root status
    root="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-new-tab-test.XXXXXX")"
    TEST_ROOT="$root"
    setup_fixture "$root"
    printf '%s\n' 'keybinds { shared { bind "Alt q" { Quit; } } }' > "$FIXTURE_CONFIG_FILE"
    printf '%s\n' 'standing layout sentinel' > "$FIXTURE_LAYOUT"
    record_initial_file_bytes

    set +e
    run_entry --session WORK > "$FIXTURE_OUTPUT" 2> "$FIXTURE_ERROR"
    status=$?
    set -e
    [ "$status" -ne 0 ] || fail "missing-Zaphod route fixture unexpectedly succeeded"
    assert_activation_rolled_back
    [ ! -e "$FAKE_ZELLIJ_NEW_TAB_COUNT" ] ||
        fail "missing-Zaphod route reached new-tab despite invalid config"
    assert_temporary_files_cleaned

    echo "PASS: missing Zaphod route fails before isolated config/layout writes"
}

test_new_tab_failure_rolls_back_activation() {
    local root status
    root="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-new-tab-test.XXXXXX")"
    TEST_ROOT="$root"
    setup_fixture "$root"
    printf '%s\n' 'standing layout sentinel' > "$FIXTURE_LAYOUT"
    record_initial_file_bytes

    set +e
    FAKE_ZELLIJ_NEW_TAB_STATUS=17 \
        run_entry --session WORK > "$FIXTURE_OUTPUT" 2> "$FIXTURE_ERROR"
    status=$?
    set -e
    [ "$status" -ne 0 ] || fail "new-tab failure fixture unexpectedly succeeded"
    assert_activation_rolled_back
    assert_temporary_files_cleaned

    echo "PASS: new-tab failure rolls isolated activation back"
}

test_new_tab_signal_rolls_back_activation() {
    local root ready release runner_pid status attempt
    root="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-new-tab-test.XXXXXX")"
    TEST_ROOT="$root"
    setup_fixture "$root"
    printf '%s\n' 'standing layout sentinel' > "$FIXTURE_LAYOUT"
    record_initial_file_bytes
    ready="$root/fake-zellij-ready"
    release="$root/fake-zellij-release"

    env \
        "PATH=$FIXTURE/bin:$PATH" \
        "BUILD_LOG=$FIXTURE_BUILD_LOG" \
        "FAKE_ZELLIJ_CALLS=$FAKE_ZELLIJ_CALLS" \
        "FAKE_ZELLIJ_SESSION=$FAKE_ZELLIJ_SESSION" \
        "FAKE_ZELLIJ_NAME=$FAKE_ZELLIJ_NAME" \
        "FAKE_ZELLIJ_LAYOUT=$FAKE_ZELLIJ_LAYOUT" \
        "FAKE_ZELLIJ_NEW_TAB_COUNT=$FAKE_ZELLIJ_NEW_TAB_COUNT" \
        "FAKE_ZELLIJ_READY_FILE=$ready" \
        "FAKE_ZELLIJ_RELEASE_FILE=$release" \
        "ZELLIJ_CONFIG_DIR=$FIXTURE_CONFIG_DIR" \
        "ZELLIJ_CONFIG_FILE=$FIXTURE_CONFIG_FILE" \
        "ZELLIJ_DATA_DIR=$FIXTURE_DATA_DIR" \
        "TMPDIR=$FIXTURE_TMP" \
        "$FIXTURE/scripts/zellij-new-tab.sh" --session WORK \
        > "$FIXTURE_OUTPUT" 2> "$FIXTURE_ERROR" &
    runner_pid=$!
    for attempt in $(seq 1 100); do
        [ ! -e "$ready" ] || break
        sleep 0.05
    done
    [ -e "$ready" ] || {
        cat "$FIXTURE_ERROR" >&2
        fail "new-tab signal fixture never reached fake Zellij"
    }
    kill -TERM "$runner_pid"
    : > "$release"
    set +e
    wait "$runner_pid"
    status=$?
    set -e
    [ "$status" -eq 143 ] ||
        fail "new-tab signal fixture exited $status instead of 143"
    assert_activation_rolled_back
    assert_temporary_files_cleaned

    echo "PASS: TERM during fresh-tab activation rolls isolated writes back"
}

test_default_data_root_is_explicit() {
    local root expected_data_root
    [ "$(uname)" = 'Darwin' ] || return
    root="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-new-tab-test.XXXXXX")"
    TEST_ROOT="$root"
    setup_fixture "$root"
    printf '%s\n' 'standing layout sentinel' > "$FIXTURE_LAYOUT"
    expected_data_root="$FIXTURE_HOME/Library/Application Support/org.Zellij-Contributors.Zellij"

    run_entry_without_data_root --session WORK > "$FIXTURE_OUTPUT" 2> "$FIXTURE_ERROR" || {
        sed -n '1,200p' "$FIXTURE_ERROR" >&2
        fail "default-data-root entry point failed"
    }
    awk -F '\t' -v expected_data_root="$expected_data_root" '
        $4 != expected_data_root { exit 1 }
    ' "$FAKE_ZELLIJ_CALLS" || fail "default Zellij data root was not passed to every call"
    assert_temporary_files_cleaned

    echo "PASS: default macOS data root is passed to every Zellij call"
}

test_new_tab_activates_isolated_roots
test_missing_zaphod_route_fails_before_writes
test_new_tab_failure_rolls_back_activation
test_new_tab_signal_rolls_back_activation
test_default_data_root_is_explicit
