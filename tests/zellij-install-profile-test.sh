#!/bin/bash
# ABOUTME: Process regressions for canonical Zaphod installation and disposable profiles.
# ABOUTME: Uses temporary Git worktrees and Zellij roots; never mutates standing user state.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd -P)"
# shellcheck source=scripts/zellij-layout-lib.sh
source "$REPO_ROOT/scripts/zellij-layout-lib.sh"
TEST_ROOT=""

cleanup_test_root() {
    if [ -n "$TEST_ROOT" ] && [ -d "$TEST_ROOT" ]; then
        rm -rf "$TEST_ROOT"
    fi
}

remove_test_root() {
    cleanup_test_root
    TEST_ROOT=""
}

wait_for_profile_value() {
    local transcript="$1"
    local key="$2"
    local launcher_pid="$3"
    local attempt
    WAIT_VALUE=""
    for attempt in $(seq 1 100); do
        WAIT_VALUE="$(tr -d '\r' < "$transcript" | awk -F= -v wanted="$key" '$1 == wanted { sub(/^[^=]*=/, ""); print; exit }')"
        if [ -n "$WAIT_VALUE" ]; then
            return 0
        fi
        if ! kill -0 "$launcher_pid" 2>/dev/null; then
            sed -n '1,160p' "$transcript" >&2
            fail "profile exited before printing $key"
        fi
        sleep 0.1
    done
    sed -n '1,160p' "$transcript" >&2
    fail "timed out waiting for profile value $key"
}

start_profile_process() {
    local run_name="$1"
    local profile_script="$2"
    local profile_cwd="$3"
    local global_root="$4"
    PROFILE_TRANSCRIPT="$TEST_ROOT/$run_name.transcript"
    PROFILE_PID_FILE="$TEST_ROOT/$run_name.pid"
    : > "$PROFILE_TRANSCRIPT"
    PROFILE_SCRIPT="$profile_script" \
        PROFILE_CWD="$profile_cwd" \
        PROFILE_PID_FILE="$PROFILE_PID_FILE" \
        ZELLIJ_CONFIG_DIR="$global_root" \
        script -q /dev/null /bin/bash -c \
            'echo $$ > "$PROFILE_PID_FILE"; exec "$PROFILE_SCRIPT" --cwd "$PROFILE_CWD"' \
            > "$PROFILE_TRANSCRIPT" 2>&1 &
    PROFILE_LAUNCHER_PID=$!
    wait_for_profile_value "$PROFILE_TRANSCRIPT" PROFILE_ROOT "$PROFILE_LAUNCHER_PID"
    PROFILE_ROOT="$WAIT_VALUE"
    wait_for_profile_value "$PROFILE_TRANSCRIPT" SESSION_NAME "$PROFILE_LAUNCHER_PID"
    PROFILE_SESSION="$WAIT_VALUE"
}

run_profile_signal_foreground() {
    local run_name="$1"
    local signal_name="$2"
    local profile_script="$3"
    local profile_cwd="$4"
    local global_root="$5"
    local metadata driver_pid launcher_status
    PROFILE_TRANSCRIPT="$TEST_ROOT/$run_name.transcript"
    PROFILE_PID_FILE="$TEST_ROOT/$run_name.pid"
    metadata="$TEST_ROOT/$run_name.metadata"
    : > "$PROFILE_TRANSCRIPT"

    (
        local attempt profile_pid profile_root session_name
        profile_pid=""
        profile_root=""
        session_name=""
        for attempt in $(seq 1 100); do
            [ ! -s "$PROFILE_PID_FILE" ] || profile_pid="$(cat "$PROFILE_PID_FILE")"
            profile_root="$(tr -d '\r' < "$PROFILE_TRANSCRIPT" | awk -F= '$1 == "PROFILE_ROOT" { sub(/^[^=]*=/, ""); print; exit }')"
            session_name="$(tr -d '\r' < "$PROFILE_TRANSCRIPT" | awk -F= '$1 == "SESSION_NAME" { sub(/^[^=]*=/, ""); print; exit }')"
            if [ -n "$profile_pid" ] && [ -n "$profile_root" ] && [ -n "$session_name" ] &&
                zellij list-sessions 2>/dev/null | grep -F "$session_name" >/dev/null; then
                printf '%s\n%s\n' "$profile_root" "$session_name" > "$metadata"
                /bin/kill -"$signal_name" "-$profile_pid"
                exit 0
            fi
            sleep 0.1
        done
        exit 1
    ) &
    driver_pid=$!

    set +e
    PROFILE_SCRIPT="$profile_script" \
        PROFILE_CWD="$profile_cwd" \
        PROFILE_PID_FILE="$PROFILE_PID_FILE" \
        ZELLIJ_CONFIG_DIR="$global_root" \
        script -q /dev/null /bin/bash -c \
            'echo $$ > "$PROFILE_PID_FILE"; exec "$PROFILE_SCRIPT" --cwd "$PROFILE_CWD"' \
            > "$PROFILE_TRANSCRIPT" 2>&1
    launcher_status=$?
    set -e
    wait "$driver_pid" || {
        sed -n '1,160p' "$PROFILE_TRANSCRIPT" >&2
        fail "$signal_name driver could not reach the disposable session"
    }
    [ "$launcher_status" -ne 0 ] || fail "$signal_name profile unexpectedly exited zero"
    PROFILE_ROOT="$(sed -n '1p' "$metadata")"
    PROFILE_SESSION="$(sed -n '2p' "$metadata")"
}

assert_session_absent() {
    local session_name="$1"
    if zellij list-sessions 2>/dev/null | grep -F "$session_name" >/dev/null; then
        fail "disposable session still exists: $session_name"
    fi
}

wait_for_session_panes() {
    local profile_root="$1"
    local session_name="$2"
    local launcher_pid="$3"
    local output_file="$4"
    local attempt
    for attempt in $(seq 1 100); do
        ZELLIJ_SESSION_NAME="$session_name" \
            zellij --config-dir "$profile_root/config" --data-dir "$profile_root/data" \
                action list-panes --json -a -g -t > "$output_file" 2>/dev/null || true
        if grep -F '"pane_cwd"' "$output_file" >/dev/null; then
            return 0
        fi
        if ! kill -0 "$launcher_pid" 2>/dev/null; then
            fail "profile exited before its session became inspectable"
        fi
        sleep 0.1
    done
    fail "profile session did not become inspectable"
}

trap cleanup_test_root EXIT

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

sha256() {
    shasum -a 256 "$1" | awk '{print $1}'
}

copy_installer_under_test() {
    local checkout="$1"
    cp "$REPO_ROOT/install.sh" "$checkout/install.sh"
    if [ -f "$REPO_ROOT/scripts/zellij-layout-lib.sh" ]; then
        mkdir -p "$checkout/scripts"
        cp "$REPO_ROOT/scripts/zellij-layout-lib.sh" "$checkout/scripts/zellij-layout-lib.sh"
    fi
}

seed_wasm() {
    local checkout="$1"
    local wasm_source="$REPO_ROOT/target/wasm32-wasip1/release/zellij-sidebar.wasm"
    [ -f "$wasm_source" ] || fail "fixture wasm missing; run ./build.sh"
    mkdir -p "$checkout/target/wasm32-wasip1/release"
    cp "$wasm_source" "$checkout/target/wasm32-wasip1/release/zellij-sidebar.wasm"
}

write_coherent_config() {
    local config_file="$1"
    local wasm_url="$2"
    printf '%s\n' \
        'keybinds {' \
        '    shared {' \
        '        bind "Alt /" {' \
        "            MessagePlugin \"$wasm_url\" {" \
        '                name "toggle"' \
        '                floating true' \
        '                skip_cache true' \
        '                rail "1"' \
        '            }' \
        '        }' \
        '    }' \
        '}' > "$config_file"
}

write_identity_case() {
    local case_name="$1"
    local config_file="$2"
    local primary_url="$3"
    local foreign_url="file:/foreign/zaphod/target/wasm32-wasip1/release/zellij-sidebar.wasm"
    case "$case_name" in
        foreign)
            write_coherent_config "$config_file" "$foreign_url"
            ;;
        mixed)
            write_coherent_config "$config_file" "$primary_url"
            printf '%s\n' \
                'keybinds {' \
                '    shared {' \
                '        bind "Alt ." {' \
                "            MessagePlugin \"$foreign_url\" {" \
                '                name "navigate"' \
                '                rail "1"' \
                '            }' \
                '        }' \
                '    }' \
                '}' >> "$config_file"
            ;;
        missing-keybind)
            printf '%s\n' \
                'keybinds {' \
                '    shared {' \
                '        bind "Alt x" { Quit; }' \
                '    }' \
                '}' > "$config_file"
            ;;
        missing-rail)
            printf '%s\n' \
                'keybinds {' \
                '    shared {' \
                '        bind "Alt /" {' \
                "            MessagePlugin \"$primary_url\" {" \
                '                name "toggle"' \
                '            }' \
                '        }' \
                '    }' \
                '}' > "$config_file"
            ;;
        coherent)
            write_coherent_config "$config_file" "$primary_url"
            ;;
        *)
            fail "unknown identity fixture: $case_name"
            ;;
    esac
}

test_linked_install() {
    local root primary linked destination layout before after status primary_url
    root="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-linked-install.XXXXXX")"
    TEST_ROOT="$root"

    primary="$root/primary"
    linked="$root/linked"
    destination="$root/zellij"
    git clone -q "$REPO_ROOT" "$primary"
    git -C "$primary" worktree add -q -b linked-test "$linked"

    copy_installer_under_test "$primary"
    copy_installer_under_test "$linked"
    seed_wasm "$primary"
    seed_wasm "$linked"

    mkdir -p "$destination/layouts"
    layout="$destination/layouts/zaphod.kdl"
    printf 'sentinel-layout\n' > "$layout"
    primary_url="file:$(cd "$primary" && pwd -P)/target/wasm32-wasip1/release/zellij-sidebar.wasm"
    write_coherent_config "$destination/config.kdl" "$primary_url"
    before="$(sha256 "$layout")"

    set +e
    ZELLIJ_CONFIG_DIR="$destination" "$linked/install.sh" >"$root/linked.out" 2>"$root/linked.err"
    status=$?
    set -e

    [ "$status" -ne 0 ] || fail "expected linked install to fail before writing, got exit 0; layout changed from sentinel"
    after="$(sha256 "$layout")"
    [ "$after" = "$before" ] || fail "linked install changed destination layout bytes"

    ZELLIJ_CONFIG_DIR="$destination" "$primary/install.sh" >"$root/primary.out" 2>"$root/primary.err" || {
        sed -n '1,120p' "$root/primary.err" >&2
        fail "primary checkout control should install successfully"
    }
    [ "$(sha256 "$layout")" != "$before" ] || fail "primary checkout control left sentinel layout unchanged"
    echo "PASS: linked install refused before writes; primary control installed"
    remove_test_root
}

test_install_identity() {
    local root primary destination layout primary_url case_name before after status
    root="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-install-identity.XXXXXX")"
    TEST_ROOT="$root"
    primary="$root/primary"
    destination="$root/zellij"
    git clone -q "$REPO_ROOT" "$primary"
    copy_installer_under_test "$primary"
    seed_wasm "$primary"

    mkdir -p "$destination/layouts"
    layout="$destination/layouts/zaphod.kdl"
    primary_url="file:$(cd "$primary" && pwd -P)/target/wasm32-wasip1/release/zellij-sidebar.wasm"

    for case_name in foreign mixed missing-keybind missing-rail; do
        printf 'sentinel-layout-%s\n' "$case_name" > "$layout"
        before="$(sha256 "$layout")"
        write_identity_case "$case_name" "$destination/config.kdl" "$primary_url"
        set +e
        ZELLIJ_CONFIG_DIR="$destination" "$primary/install.sh" >"$root/$case_name.out" 2>"$root/$case_name.err"
        status=$?
        set -e
        [ "$status" -ne 0 ] || fail "identity case $case_name expected refusal, got exit 0"
        after="$(sha256 "$layout")"
        [ "$after" = "$before" ] || fail "identity case $case_name changed sentinel layout"
    done

    printf 'sentinel-layout-coherent\n' > "$layout"
    before="$(sha256 "$layout")"
    write_identity_case coherent "$destination/config.kdl" "$primary_url"
    ZELLIJ_CONFIG_DIR="$destination" "$primary/install.sh" >"$root/coherent.out" 2>"$root/coherent.err" || {
        sed -n '1,120p' "$root/coherent.err" >&2
        fail "coherent identity should install successfully"
    }
    [ "$(sha256 "$layout")" != "$before" ] || fail "coherent identity left sentinel layout unchanged"
    zellij --config-dir "$destination" setup --check >/dev/null
    echo "PASS: split URL/config identity refused; coherent identity installed"
    remove_test_root
}

test_install_postflight_rollback() {
    local root primary destination layout primary_url before status real_zellij shim_dir
    root="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-install-postflight.XXXXXX")"
    TEST_ROOT="$root"
    primary="$root/primary"
    destination="$root/zellij"
    shim_dir="$root/bin"
    git clone -q "$REPO_ROOT" "$primary"
    copy_installer_under_test "$primary"
    seed_wasm "$primary"

    mkdir -p "$destination/layouts" "$shim_dir"
    layout="$destination/layouts/zaphod.kdl"
    printf 'sentinel-layout-postflight\n' > "$layout"
    before="$(sha256 "$layout")"
    primary_url="file:$(cd "$primary" && pwd -P)/target/wasm32-wasip1/release/zellij-sidebar.wasm"
    write_coherent_config "$destination/config.kdl" "$primary_url"
    write_identity_case foreign "$root/foreign-config.kdl" "$primary_url"
    real_zellij="$(command -v zellij)"

    printf '%s\n' \
        '#!/bin/bash' \
        'set -u' \
        'if [ "${*: -2}" = "setup --check" ] && [ ! -e "$MUTATE_MARKER" ]; then' \
        '    "$REAL_ZELLIJ" "$@"' \
        '    status=$?' \
        '    if [ "$status" -eq 0 ]; then' \
        '        cp "$MUTATE_SOURCE" "$MUTATE_CONFIG"' \
        '        : > "$MUTATE_MARKER"' \
        '    fi' \
        '    exit "$status"' \
        'fi' \
        'exec "$REAL_ZELLIJ" "$@"' > "$shim_dir/zellij"
    chmod +x "$shim_dir/zellij"

    set +e
    PATH="$shim_dir:$PATH" \
        REAL_ZELLIJ="$real_zellij" \
        MUTATE_SOURCE="$root/foreign-config.kdl" \
        MUTATE_CONFIG="$destination/config.kdl" \
        MUTATE_MARKER="$root/mutated" \
        ZELLIJ_CONFIG_DIR="$destination" \
        "$primary/install.sh" >"$root/postflight.out" 2>"$root/postflight.err"
    status=$?
    set -e

    [ "$status" -ne 0 ] || fail "postflight mismatch expected install failure, got exit 0"
    [ "$(sha256 "$layout")" = "$before" ] || fail "postflight mismatch did not restore prior layout bytes"
    echo "PASS: postflight mismatch restored prior layout bytes"
    remove_test_root
}

test_install_signal_rollback() {
    local root primary destination layout primary_url before status real_zellij shim_dir
    root="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-install-signal.XXXXXX")"
    TEST_ROOT="$root"
    primary="$root/primary"
    destination="$root/zellij"
    shim_dir="$root/bin"
    git clone -q "$REPO_ROOT" "$primary"
    copy_installer_under_test "$primary"
    seed_wasm "$primary"

    mkdir -p "$destination/layouts" "$shim_dir"
    layout="$destination/layouts/zaphod.kdl"
    printf 'sentinel-layout-signal\n' > "$layout"
    before="$(sha256 "$layout")"
    primary_url="file:$(cd "$primary" && pwd -P)/target/wasm32-wasip1/release/zellij-sidebar.wasm"
    write_coherent_config "$destination/config.kdl" "$primary_url"
    real_zellij="$(command -v zellij)"

    printf '%s\n' \
        '#!/bin/bash' \
        'set -u' \
        'case " $* " in' \
        '    *" attach "*)' \
        '        count=0' \
        '        [ ! -f "$ATTACH_COUNT" ] || count="$(cat "$ATTACH_COUNT")"' \
        '        count=$((count + 1))' \
        '        printf "%s\n" "$count" > "$ATTACH_COUNT"' \
        '        if [ "$count" -eq 2 ]; then' \
        '            /bin/kill -TERM "$PPID"' \
        '        fi' \
        '        ;;' \
        'esac' \
        'exec "$REAL_ZELLIJ" "$@"' > "$shim_dir/zellij"
    chmod +x "$shim_dir/zellij"

    set +e
    PATH="$shim_dir:$PATH" \
        REAL_ZELLIJ="$real_zellij" \
        ATTACH_COUNT="$root/attach-count" \
        ZELLIJ_CONFIG_DIR="$destination" \
        "$primary/install.sh" >"$root/signal.out" 2>"$root/signal.err"
    status=$?
    set -e

    [ "$status" -ne 0 ] || fail "signal during postflight expected install failure, got exit 0"
    [ "$(sha256 "$layout")" = "$before" ] || fail "signal during postflight did not restore prior layout bytes"
    echo "PASS: signal during postflight restored prior layout bytes"
    remove_test_root
}

test_worktree_profile_lifecycle() {
    local root profile_script profile_cwd global_root global_config global_layout
    local config_before layout_before expected_url expected_cwd pane_state control_session control_dump signal_name
    root="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-worktree-profile-test.XXXXXX")"
    TEST_ROOT="$root"
    profile_script="$REPO_ROOT/scripts/zellij-worktree-test-profile.sh"
    [ -x "$profile_script" ] || fail "profile script missing: $profile_script"
    zaphod_require_zellij_0443
    command -v script >/dev/null 2>&1 || fail "script(1) is required for attached profile tests"

    profile_cwd="$root/explicit cwd"
    global_root="$root/global-zellij"
    global_config="$global_root/config.kdl"
    global_layout="$global_root/layouts/zaphod.kdl"
    mkdir -p "$profile_cwd" "$global_root/layouts"
    printf 'global-config-sentinel\n' > "$global_config"
    printf 'global-layout-sentinel\n' > "$global_layout"
    config_before="$(sha256 "$global_config")"
    layout_before="$(sha256 "$global_layout")"
    expected_url="file:$REPO_ROOT/target/wasm32-wasip1/release/zellij-sidebar.wasm"
    expected_cwd="$(cd "$profile_cwd" && pwd -P)"

    start_profile_process normal "$profile_script" "$profile_cwd" "$global_root"
    [ -f "$PROFILE_ROOT/config/config.kdl" ] || fail "profile config missing"
    [ -f "$PROFILE_ROOT/config/layouts/zaphod.kdl" ] || fail "profile Zaphod layout missing"
    [ -f "$PROFILE_ROOT/config/layouts/explicit-cwd.kdl" ] || fail "explicit-cwd fixture missing"
    [ -d "$PROFILE_ROOT/data" ] || fail "profile data root missing"
    zaphod_validate_message_plugin_identity "$PROFILE_ROOT/config/config.kdl" "$expected_url"
    zaphod_validate_layout_identity "$PROFILE_ROOT/config/layouts/zaphod.kdl" "$expected_url"

    pane_state="$root/normal-panes.json"
    wait_for_session_panes "$PROFILE_ROOT" "$PROFILE_SESSION" "$PROFILE_LAUNCHER_PID" "$pane_state"
    [ "$(grep -c '"is_plugin": false' "$pane_state")" -eq 1 ] || fail "explicit-cwd session did not have exactly one terminal"
    grep -F '"plugin_url": "file:' "$pane_state" >/dev/null && fail "explicit-cwd session unexpectedly contained a file plugin"
    grep -F "\"pane_cwd\": \"$expected_cwd\"" "$pane_state" >/dev/null || fail "explicit terminal cwd did not match caller path"

    control_session="zpc-$$-${RANDOM:-0}"
    zellij --config-dir "$PROFILE_ROOT/config" --data-dir "$PROFILE_ROOT/data" \
        --layout "$PROFILE_ROOT/config/layouts/zaphod.kdl" attach "$control_session" --create-background
    control_dump="$root/resident-control.kdl"
    ZELLIJ_SESSION_NAME="$control_session" \
        zellij --config-dir "$PROFILE_ROOT/config" --data-dir "$PROFILE_ROOT/data" \
            action dump-layout > "$control_dump"
    zaphod_validate_layout_identity "$control_dump" "$expected_url"
    zellij kill-session "$control_session"

    zellij kill-session "$PROFILE_SESSION"
    wait "$PROFILE_LAUNCHER_PID" || fail "normal profile launcher exited nonzero"
    [ ! -e "$PROFILE_ROOT" ] || fail "normal cleanup left profile root"
    assert_session_absent "$PROFILE_SESSION"
    [ "$(sha256 "$global_config")" = "$config_before" ] || fail "normal cleanup changed global config"
    [ "$(sha256 "$global_layout")" = "$layout_before" ] || fail "normal cleanup changed global layout"

    for signal_name in TERM INT; do
        run_profile_signal_foreground "signal-$signal_name" "$signal_name" "$profile_script" "$profile_cwd" "$global_root"
        [ ! -e "$PROFILE_ROOT" ] || fail "$signal_name cleanup left profile root"
        assert_session_absent "$PROFILE_SESSION"
        [ "$(sha256 "$global_config")" = "$config_before" ] || fail "$signal_name cleanup changed global config"
        [ "$(sha256 "$global_layout")" = "$layout_before" ] || fail "$signal_name cleanup changed global layout"
    done

    echo "PASS: disposable profile preserved global bytes across normal, TERM, and INT cleanup"
    remove_test_root
}

case "${1:-all}" in
    linked-install)
        test_linked_install
        ;;
    install-identity)
        test_install_identity
        ;;
    install-postflight)
        test_install_postflight_rollback
        ;;
    install-signal)
        test_install_signal_rollback
        ;;
    worktree-profile)
        test_worktree_profile_lifecycle
        ;;
    all)
        test_linked_install
        test_install_identity
        test_install_postflight_rollback
        test_install_signal_rollback
        test_worktree_profile_lifecycle
        ;;
    *)
        fail "unknown test: $1"
        ;;
esac
