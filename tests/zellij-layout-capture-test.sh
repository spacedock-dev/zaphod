#!/bin/bash
# ABOUTME: Adversarial tests for atomic, identity-bound native dump-layout capture.
# ABOUTME: Complete wrong-action/stale replies retry only with authoritative candidate panes.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd -P)"
# shellcheck source=scripts/zellij-layout-lib.sh
source "$REPO_ROOT/scripts/zellij-layout-lib.sh"
ROOT="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-layout-capture.XXXXXX")"
trap 'rm -rf "$ROOT"' EXIT

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

command -v cargo >/dev/null 2>&1 || fail "cargo is required for the layout capture test"
cargo build --quiet --manifest-path "$REPO_ROOT/Cargo.toml" \
    --target-dir "$REPO_ROOT/target" \
    --features host-kdl-validator --bin zaphod-kdl-validate
VALIDATOR="$REPO_ROOT/target/debug/zaphod-kdl-validate"
EXPECTED='file:/candidate/zellij-sidebar.wasm'
PANES_PRESENT="$ROOT/panes-present.json"
PANES_ABSENT="$ROOT/panes-absent.json"
printf '%s\n' '[{"id":50,"tab_id":73,"is_plugin":true,"plugin_url":"file:/candidate/zellij-sidebar.wasm","is_floating":false,"is_suppressed":false}]' > "$PANES_PRESENT"
printf '%s\n' '[{"id":7,"tab_id":73,"is_plugin":false,"is_selectable":true,"is_suppressed":false}]' > "$PANES_ABSENT"

GOOD="$ROOT/good.kdl"
MISSING="$ROOT/missing.kdl"
WRONG="$ROOT/wrong.kdl"
MISSING_RAIL="$ROOT/missing-rail.kdl"
WRONG_RAIL="$ROOT/wrong-rail.kdl"
MALFORMED="$ROOT/malformed.kdl"
printf '%s\n' 'layout {' ' pane {' '  plugin location="file:/candidate/zellij-sidebar.wasm" {' '   rail "1"' '  }' ' }' '}' > "$GOOD"
printf '%s\n' 'layout {' ' pane' '}' > "$MISSING"
printf '%s\n' 'layout {' ' plugin location="file:/wrong/zellij-sidebar.wasm"' '}' > "$WRONG"
printf '%s\n' 'layout {' ' plugin location="file:/candidate/zellij-sidebar.wasm"' '}' > "$MISSING_RAIL"
printf '%s\n' 'layout {' ' plugin location="file:/candidate/zellij-sidebar.wasm" {' '  rail "0"' ' }' '}' > "$WRONG_RAIL"
printf '%s' 'layout { pane' > "$MALFORMED"

fake_dump() {
    local count=0 reply status=0
    [ ! -f "$ROOT/count" ] || count="$(cat "$ROOT/count")"
    count=$((count + 1))
    printf '%s\n' "$count" > "$ROOT/count"
    reply="$ROOT/reply-$count"
    [ ! -f "$reply.stderr" ] || cat "$reply.stderr" >&2
    [ ! -f "$reply.status" ] || status="$(cat "$reply.status")"
    [ ! -f "$reply.stdout" ] || cat "$reply.stdout"
    return "$status"
}

reset_case() {
    rm -f "$ROOT"/count "$ROOT"/reply-* "$ROOT"/accepted.kdl "$ROOT"/error
}

assert_attempts() {
    [ "$(cat "$ROOT/count")" = "$1" ] || fail "$2 used $(cat "$ROOT/count") attempts, want $1"
}

reset_case
cp "$MISSING" "$ROOT/reply-1.stdout"
cp "$GOOD" "$ROOT/reply-2.stdout"
zaphod_capture_validated_layout "$VALIDATOR" "$EXPECTED" present "$PANES_PRESENT" \
    "$ROOT/accepted.kdl" fake_dump 2> "$ROOT/error" || fail "stale valid layout did not recover"
cmp "$GOOD" "$ROOT/accepted.kdl" || fail "stale recovery did not atomically publish the valid reply"
assert_attempts 2 stale-recovery
grep -F 'transient-native-layout-reply:' "$ROOT/error" >/dev/null || fail "stale recovery omitted bounded provenance"

reset_case
printf '%s\n' '[{"id":50,"tab_id":73}]' > "$ROOT/reply-1.stdout"
cp "$GOOD" "$ROOT/reply-2.stdout"
zaphod_capture_validated_layout "$VALIDATOR" "$EXPECTED" present "$PANES_PRESENT" \
    "$ROOT/accepted.kdl" fake_dump 2> "$ROOT/error" || fail "complete JSON wrong-action did not recover"
assert_attempts 2 wrong-action-recovery

reset_case
for attempt in 1 2 3; do cp "$MISSING" "$ROOT/reply-$attempt.stdout"; done
set +e
zaphod_capture_validated_layout "$VALIDATOR" "$EXPECTED" present "$PANES_PRESENT" \
    "$ROOT/accepted.kdl" fake_dump 2> "$ROOT/error"
status=$?
set -e
[ "$status" -ne 0 ] || fail "persistent missing candidate unexpectedly succeeded"
assert_attempts 3 persistent-missing
[ ! -e "$ROOT/accepted.kdl" ] || fail "persistent missing candidate published an unverified layout"
grep -F 'attempt=3/3' "$ROOT/error" >/dev/null || fail "persistent failure omitted final attempt provenance"

reset_case
cp "$MALFORMED" "$ROOT/reply-1.stdout"
set +e
zaphod_capture_validated_layout "$VALIDATOR" "$EXPECTED" present "$PANES_PRESENT" \
    "$ROOT/accepted.kdl" fake_dump 2> "$ROOT/error"
status=$?
set -e
[ "$status" -ne 0 ] || fail "malformed KDL unexpectedly succeeded"
assert_attempts 1 malformed

reset_case
printf '%s\n' 17 > "$ROOT/reply-1.status"
printf '%s' 'native route failed' > "$ROOT/reply-1.stderr"
set +e
zaphod_capture_validated_layout "$VALIDATOR" "$EXPECTED" present "$PANES_PRESENT" \
    "$ROOT/accepted.kdl" fake_dump 2> "$ROOT/error"
status=$?
set -e
[ "$status" -ne 0 ] || fail "dump-layout command failure unexpectedly succeeded"
assert_attempts 1 command-failure
grep -F 'status=17' "$ROOT/error" >/dev/null || fail "command failure omitted status provenance"

reset_case
cp "$WRONG" "$ROOT/reply-1.stdout"
set +e
zaphod_capture_validated_layout "$VALIDATOR" "$EXPECTED" absent "$PANES_ABSENT" \
    "$ROOT/accepted.kdl" fake_dump 2> "$ROOT/error"
status=$?
set -e
[ "$status" -ne 0 ] || fail "absent-pane identity mismatch unexpectedly succeeded"
assert_attempts 1 absent-identity

for rail_case in missing-rail wrong-rail; do
    reset_case
    rail_source="$MISSING_RAIL"
    [ "$rail_case" != wrong-rail ] || rail_source="$WRONG_RAIL"
    for attempt in 1 2 3; do cp "$rail_source" "$ROOT/reply-$attempt.stdout"; done
    set +e
    zaphod_capture_validated_layout "$VALIDATOR" "$EXPECTED" present "$PANES_PRESENT" \
        "$ROOT/accepted.kdl" fake_dump 2> "$ROOT/error"
    status=$?
    set -e
    [ "$status" -ne 0 ] || fail "$rail_case identity unexpectedly succeeded"
    assert_attempts 3 "$rail_case"
    [ ! -e "$ROOT/accepted.kdl" ] || fail "$rail_case identity published an invalid layout"
done

dd if=/dev/zero bs=1048576 count=5 2>/dev/null | tr '\0' x > "$ROOT/oversized.kdl"
set +e
"$VALIDATOR" "$ROOT/oversized.kdl" "$EXPECTED" present > "$ROOT/oversized.out" 2> "$ROOT/oversized.err"
status=$?
set -e
[ "$status" -eq 20 ] || fail "oversized layout exited $status instead of malformed-record status 20"
[ ! -s "$ROOT/oversized.out" ] || fail "oversized layout emitted unbounded stdout"
grep -F 'exceeds 4194304' "$ROOT/oversized.err" >/dev/null || fail "oversized layout omitted its fixed ceiling"

echo "PASS: native layout capture is atomic, bounded, and identity-bound"
