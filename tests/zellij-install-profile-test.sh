#!/bin/bash
# ABOUTME: Process regressions for canonical Zaphod installation and disposable profiles.
# ABOUTME: Uses temporary Git worktrees and Zellij roots; never mutates standing user state.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd -P)"
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

case "${1:-all}" in
    linked-install)
        test_linked_install
        ;;
    install-identity)
        test_install_identity
        ;;
    all)
        test_linked_install
        test_install_identity
        ;;
    *)
        fail "unknown test: $1"
        ;;
esac
