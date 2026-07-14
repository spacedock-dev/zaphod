#!/bin/bash
# ABOUTME: Exercises the managed Zellij-tab entry path through a real tmux-hosted client.
# ABOUTME: Keeps config, data, socket, tmux server, and cleanup isolated from standing Zellij state.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
# shellcheck source=scripts/zellij-layout-lib.sh
source "$REPO_ROOT/scripts/zellij-layout-lib.sh"

# A smoke launched from a loaded Zellij pane must not carry that client's
# identity into its disposable server or native CLI calls.
INHERITED_ZELLIJ_PRESENT="${ZELLIJ+x}"
INHERITED_ZELLIJ="${ZELLIJ-}"
INHERITED_ZELLIJ_SESSION_NAME_PRESENT="${ZELLIJ_SESSION_NAME+x}"
INHERITED_ZELLIJ_SESSION_NAME="${ZELLIJ_SESSION_NAME-}"
INHERITED_ZELLIJ_PANE_ID_PRESENT="${ZELLIJ_PANE_ID+x}"
INHERITED_ZELLIJ_PANE_ID="${ZELLIJ_PANE_ID-}"
unset ZELLIJ ZELLIJ_SESSION_NAME ZELLIJ_PANE_ID

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

file_state() {
    local path="$1"
    if [ -e "$path" ]; then
        printf 'present:%s\n' "$(shasum -a 256 "$path" | awk '{print $1}')"
    else
        printf 'missing\n'
    fi
}

STANDING_ROOT="${ZELLIJ_CONFIG_DIR:-$HOME/.config/zellij}"
STANDING_CONFIG="${ZELLIJ_CONFIG_FILE:-$STANDING_ROOT/config.kdl}"
STANDING_LAYOUT="$STANDING_ROOT/layouts/zaphod.kdl"
STANDING_CONFIG_BEFORE="$(file_state "$STANDING_CONFIG")"
STANDING_LAYOUT_BEFORE="$(file_state "$STANDING_LAYOUT")"

ROOT=""
CONFIG_DIR=""
CONFIG_FILE=""
DATA_DIR=""
SOCKET_DIR=""
HOME_DIR=""
PERMISSION_CACHE=""
PERMISSION_FIXTURE="${ZAPHOD_PERMISSION_FIXTURE:-pregranted}"
SUBSCRIBER_MODE="${ZAPHOD_SUBSCRIBER_MODE:-automatic}"
CALLER_ENV="${ZAPHOD_CALLER_ENV:-unspecified}"
NATIVE_COMMAND_TIMEOUT="${ZAPHOD_SMOKE_NATIVE_COMMAND_TIMEOUT_SECS:-10}"
INJECT_NATIVE_HANG="${ZAPHOD_SMOKE_INJECT_NATIVE_HANG:-}"
NATIVE_HANG_INJECTED=0
INJECT_STARTUP_EXIT="${ZAPHOD_SMOKE_INJECT_STARTUP_EXIT:-}"
EVIDENCE_DIR="${ZAPHOD_SMOKE_EVIDENCE_DIR:-}"
INJECT_FAILURE_PHASE="${ZAPHOD_SMOKE_INJECT_FAILURE_PHASE:-}"
INJECT_FAILURE_PAYLOAD_BYTES="${ZAPHOD_SMOKE_INJECT_FAILURE_PAYLOAD_BYTES:-0}"
INJECT_CLEANUP_PROBE_HANG="${ZAPHOD_SMOKE_INJECT_CLEANUP_PROBE_HANG:-none}"
RESPONSIVENESS_CHECK="${ZAPHOD_SMOKE_RESPONSIVENESS_CHECK:-0}"
INJECT_RESPONSIVE_TIMEOUT="${ZAPHOD_SMOKE_INJECT_RESPONSIVE_TIMEOUT:-}"
RESPONSIVE_TIMEOUT_INJECTED=0
CURRENT_PHASE="boot"
INJECTED_FAILURE=""
ENTRY_START_TIMEOUT=30
ISOLATED_CONFIG_BEFORE=""
ISOLATED_LAYOUT=""
ISOLATED_LAYOUT_BEFORE=""
SESSION_NAME=""
TMUX_SERVER=""
TMUX_SESSION="zaphod-smoke"
TMUX_PANE="$TMUX_SESSION:0.0"
AGENTSVIEW_PID=""
AGENTSVIEW_URL=""
SIDECAR_PID=""
SIDECAR_LOG=""
SIDECAR_START_FIFO=""
ENTRY_PID=""
LAYOUT_VALIDATOR=""

bounded_exec() {
    local seconds="$1"
    shift
    perl -e '
        my $seconds = shift;
        my $pid = fork();
        exit 127 unless defined $pid;
        if ($pid == 0) { exec @ARGV or exit 127 }
        $SIG{ALRM} = sub {
            kill "TERM", $pid;
            select undef, undef, undef, 0.1;
            kill "KILL", $pid;
            waitpid $pid, 0;
            exit 124;
        };
        alarm $seconds;
        waitpid $pid, 0;
        alarm 0;
        my $status = $?;
        exit(($status & 127) ? 128 + ($status & 127) : ($status >> 8));
    ' \
        "$seconds" "$@"
}

zellij_control_with_timeout() {
    local seconds="$1"
    shift
    bounded_exec "$seconds" env -u ZELLIJ -u ZELLIJ_SESSION_NAME -u ZELLIJ_PANE_ID \
        ZELLIJ_SOCKET_DIR="$SOCKET_DIR" \
        zellij --config-dir "$CONFIG_DIR" --config "$CONFIG_FILE" --data-dir "$DATA_DIR" "$@"
}

zellij_control() {
    strict_native_command zellij-control zellij_control_with_timeout "$@"
}

zellij_session_with_timeout() {
    local seconds="$1"
    shift
    bounded_exec "$seconds" env -u ZELLIJ -u ZELLIJ_SESSION_NAME -u ZELLIJ_PANE_ID \
        ZELLIJ_SOCKET_DIR="$SOCKET_DIR" \
        zellij --session "$SESSION_NAME" \
        --config-dir "$CONFIG_DIR" --config "$CONFIG_FILE" --data-dir "$DATA_DIR" "$@"
}

zellij_session() {
    strict_native_command zellij-session zellij_session_with_timeout "$@"
}

tmux_with_timeout() {
    local seconds="$1"
    shift
    bounded_exec "$seconds" tmux -L "$TMUX_SERVER" "$@"
}

tmux_command() {
    strict_native_command tmux tmux_with_timeout "$@"
}

native_hang_matches() {
    local arg
    [ -n "$INJECT_NATIVE_HANG" ] && [ "$NATIVE_HANG_INJECTED" -eq 0 ] || return 1
    for arg in "$@"; do
        [ "$arg" != "$INJECT_NATIVE_HANG" ] || return 0
    done
    return 1
}

strict_native_command() {
    local owner="$1"
    local runner="$2"
    shift 2
    local first="${1:-none}"
    local second="${2:-none}"
    local status
    if native_hang_matches "$@"; then
        NATIVE_HANG_INJECTED=1
        if bounded_exec "$NATIVE_COMMAND_TIMEOUT" sleep 60; then
            return 0
        else
            status=$?
        fi
    elif "$runner" "$NATIVE_COMMAND_TIMEOUT" "$@"; then
        return 0
    else
        status=$?
    fi
    if [ "$status" -eq 124 ]; then
        phase native-command-timeout
        fail "native-command-timeout: owner=$owner command=$first $second timeout_secs=$NATIVE_COMMAND_TIMEOUT"
    fi
    return "$status"
}

phase() {
    local name="$1"
    local marker="zaphod-phase: phase=$name pid=$$ caller=$CALLER_ENV mode=$SUBSCRIBER_MODE"
    CURRENT_PHASE="$name"
    printf '%s\n' "$marker" >&2
    if [ -n "$EVIDENCE_DIR" ]; then
        mkdir -p "$EVIDENCE_DIR"
        printf '%s\n' "$marker" >> "$EVIDENCE_DIR/phase.log"
    fi
    if [ -n "$INJECT_FAILURE_PHASE" ] && [ "$name" = "$INJECT_FAILURE_PHASE" ]; then
        INJECTED_FAILURE="$name"
        if [ "$INJECT_FAILURE_PAYLOAD_BYTES" -gt 0 ]; then
            head -c "$INJECT_FAILURE_PAYLOAD_BYTES" /dev/zero | tr '\0' x >&2
            printf '\n' >&2
        fi
        fail "injected lifecycle failure at phase $name"
    fi
}

pid_is_alive() {
    [ -n "$1" ] && kill -0 "$1" 2>/dev/null
}

terminate_owned_pid() {
    local pid="$1"
    local label="$2"
    local attempt
    [ -n "$pid" ] || return 0
    kill -TERM "$pid" 2>/dev/null || true
    for attempt in $(seq 1 40); do
        pid_is_alive "$pid" || break
        sleep 0.05
    done
    if pid_is_alive "$pid"; then
        kill -KILL "$pid" 2>/dev/null || true
    fi
    wait "$pid" 2>/dev/null || true
    if pid_is_alive "$pid"; then
        echo "$label survived cleanup: $pid" >&2
        return 1
    fi
    return 0
}

copy_bounded_native_evidence() {
    local path name
    [ -n "$EVIDENCE_DIR" ] && [ -d "$ROOT" ] || return 0
    mkdir -p "$EVIDENCE_DIR/native"
    while IFS= read -r path; do
        name="$(basename "$path")"
        case "$name" in
            *.json|*.kdl|*.err|*.stderr|*.stdout|*.out|*.log|*.screen|entry.out)
                head -c 4096 "$path" > "$EVIDENCE_DIR/native/$name" 2>/dev/null || true
                ;;
        esac
    done < <(find "$ROOT" -maxdepth 1 -type f -print | sort | head -64)
}

record_precleanup_evidence() {
    local original_status="$1"
    local failed_phase="$CURRENT_PHASE"
    local tmux_status=125 session_status=125
    [ -n "$EVIDENCE_DIR" ] || return 0
    mkdir -p "$EVIDENCE_DIR/native"
    phase cleanup-start
    {
        printf 'original_status=%s\n' "$original_status"
        printf 'last_phase=%s\n' "$failed_phase"
        printf 'injected_failure=%s\n' "$INJECTED_FAILURE"
        printf 'smoke_pid=%s\nparent_pid=%s\n' "$$" "$PPID"
        printf 'root=%s\nsession=%s\ntmux_server=%s\ntmux_pane=%s\n' \
            "$ROOT" "$SESSION_NAME" "$TMUX_SERVER" "$TMUX_PANE"
        printf 'entry_pid=%s entry_alive_before=%s\n' "$ENTRY_PID" "$(pid_is_alive "$ENTRY_PID" && echo 1 || echo 0)"
        printf 'sidecar_pid=%s sidecar_alive_before=%s\n' "$SIDECAR_PID" "$(pid_is_alive "$SIDECAR_PID" && echo 1 || echo 0)"
        printf 'agentsview_pid=%s agentsview_alive_before=%s\n' "$AGENTSVIEW_PID" "$(pid_is_alive "$AGENTSVIEW_PID" && echo 1 || echo 0)"
    } > "$EVIDENCE_DIR/process-ownership.txt"
    if [ -n "$TMUX_SERVER" ]; then
        tmux_with_timeout 1 capture-pane -p -t "$TMUX_PANE" \
            > "$EVIDENCE_DIR/tmux-pane.txt" 2> "$EVIDENCE_DIR/tmux-pane.err"
        tmux_status=$?
    else
        : > "$EVIDENCE_DIR/tmux-pane.txt"
    fi
    if [ -n "$SESSION_NAME" ] && [ -n "$CONFIG_DIR" ]; then
        zellij_control_with_timeout 1 --session "$SESSION_NAME" \
            action list-panes --json --all --command --geometry --state --tab \
            > "$EVIDENCE_DIR/native/list-panes-cleanup.json" \
            2> "$EVIDENCE_DIR/native/list-panes-cleanup.err"
        session_status=$?
    fi
    {
        printf 'tmux_capture_status=%s\n' "$tmux_status"
        printf 'zellij_probe_status=%s\n' "$session_status"
    } >> "$EVIDENCE_DIR/process-ownership.txt"
    copy_bounded_native_evidence
}

cleanup() {
    local status=$?
    local cleanup_status=0
    local session_alive_after=0 tmux_alive_after=0 root_exists_after=0
    local session_delete_status=125 session_probe_status_after=125 session_absence_confirmed=0
    local tmux_kill_status=125 tmux_probe_status_after=125 tmux_absence_confirmed=0
    local tmux_socket_status=125 tmux_socket_path="" tmux_socket_absent_after=0
    local tmux_server_unreachable_after=0
    local tmux_server_pid="" tmux_server_pid_alive_after=0 tmux_socket_removed_after=0
    local tmux_absence_basis="unproven"
    local config_after layout_after _cleanup_attempt
    trap - EXIT INT TERM HUP
    set +e
    record_precleanup_evidence "$status"
    terminate_owned_pid "$ENTRY_PID" "entry process" || cleanup_status=1
    terminate_owned_pid "$SIDECAR_PID" "private sidecar" || cleanup_status=1
    if [ -n "$TMUX_SERVER" ]; then
        tmux_with_timeout 1 display-message -p '#{pid} #{socket_path}' \
            > "$ROOT/tmux-socket-path.stdout" 2> "$ROOT/tmux-socket-path.stderr"
        tmux_socket_status=$?
        if [ "$tmux_socket_status" -eq 0 ]; then
            IFS=' ' read -r tmux_server_pid tmux_socket_path < "$ROOT/tmux-socket-path.stdout" || true
        fi
        tmux_with_timeout 2 kill-server > "$ROOT/tmux-kill.stdout" 2> "$ROOT/tmux-kill.stderr"
        tmux_kill_status=$?
        if [ "$INJECT_CLEANUP_PROBE_HANG" = tmux ] || [ "$INJECT_CLEANUP_PROBE_HANG" = both ]; then
            bounded_exec 1 sleep 60 > "$ROOT/tmux-probe.stdout" 2> "$ROOT/tmux-probe.stderr"
        else
            tmux_with_timeout 1 list-sessions \
                > "$ROOT/tmux-probe.stdout" 2> "$ROOT/tmux-probe.stderr"
        fi
        tmux_probe_status_after=$?
        for _cleanup_attempt in $(seq 1 20); do
            pid_is_alive "$tmux_server_pid" || break
            sleep 0.05
        done
        pid_is_alive "$tmux_server_pid" && tmux_server_pid_alive_after=1
        case "$tmux_probe_status_after" in
            0)
                echo "dedicated tmux server survived cleanup: $TMUX_SERVER" >&2
                tmux_alive_after=1
                cleanup_status=1
                ;;
            1)
                if grep -F "$tmux_socket_path" "$ROOT/tmux-probe.stderr" >/dev/null &&
                    { grep -F 'no server running' "$ROOT/tmux-probe.stderr" >/dev/null ||
                    { grep -F 'error connecting to ' "$ROOT/tmux-probe.stderr" >/dev/null &&
                      grep -F 'No such file or directory' "$ROOT/tmux-probe.stderr" >/dev/null; }; }; then
                    tmux_server_unreachable_after=1
                    if [ -n "$tmux_server_pid" ] && [ "$tmux_server_pid_alive_after" -eq 0 ]; then
                        tmux_absence_confirmed=1
                        tmux_absence_basis="native-unreachable+pid-exited"
                    else
                        echo "dedicated tmux server PID did not conclusively exit: ${tmux_server_pid:-missing}" >&2
                        cleanup_status=1
                    fi
                else
                    echo "dedicated tmux server probe failed without absence evidence" >&2
                    cleanup_status=1
                fi
                ;;
            *)
                echo "dedicated tmux cleanup probe was inconclusive: status $tmux_probe_status_after" >&2
                cleanup_status=1
                ;;
        esac
        if [ "$tmux_absence_confirmed" -eq 1 ] && [ -n "$tmux_socket_path" ] && [ -e "$tmux_socket_path" ]; then
            rm -f "$tmux_socket_path" || cleanup_status=1
            [ -e "$tmux_socket_path" ] || tmux_socket_removed_after=1
        fi
        if [ -n "$tmux_socket_path" ] && [ ! -e "$tmux_socket_path" ]; then
            tmux_socket_absent_after=1
        fi
    fi
    if [ -n "$SESSION_NAME" ]; then
        zellij_control_with_timeout 2 delete-session --force "$SESSION_NAME" \
            > "$ROOT/zellij-delete.stdout" 2> "$ROOT/zellij-delete.stderr"
        session_delete_status=$?
        if [ "$INJECT_CLEANUP_PROBE_HANG" = zellij ] || [ "$INJECT_CLEANUP_PROBE_HANG" = both ]; then
            bounded_exec 1 sleep 60 > "$ROOT/zellij-probe.stdout" 2> "$ROOT/zellij-probe.stderr"
        else
            zellij_control_with_timeout 1 --session "$SESSION_NAME" action list-panes --json --all \
                > "$ROOT/zellij-probe.stdout" 2> "$ROOT/zellij-probe.stderr"
        fi
        session_probe_status_after=$?
        case "$session_probe_status_after" in
            0)
                echo "isolated Zellij session survived cleanup: $SESSION_NAME" >&2
                session_alive_after=1
                cleanup_status=1
                ;;
            1)
                if grep -F 'There is no active session!' "$ROOT/zellij-probe.stderr" >/dev/null; then
                    session_absence_confirmed=1
                else
                    echo "isolated Zellij cleanup probe failed without absence evidence" >&2
                    cleanup_status=1
                fi
                ;;
            *)
                echo "isolated Zellij cleanup probe was inconclusive: status $session_probe_status_after" >&2
                cleanup_status=1
                ;;
        esac
    fi
    terminate_owned_pid "$AGENTSVIEW_PID" "AgentsView fixture" || cleanup_status=1
    copy_bounded_native_evidence
    if [ -n "$ROOT" ] && [ -d "$ROOT" ]; then
        rm -rf "$ROOT" || cleanup_status=1
        if [ -e "$ROOT" ]; then
            echo "isolated smoke root survived cleanup: $ROOT" >&2
            root_exists_after=1
            cleanup_status=1
        fi
    fi
    config_after="$(file_state "$STANDING_CONFIG")"
    layout_after="$(file_state "$STANDING_LAYOUT")"
    if [ "$config_after" != "$STANDING_CONFIG_BEFORE" ]; then
        echo "standing Zellij config changed during tmux smoke: $STANDING_CONFIG" >&2
        cleanup_status=1
    fi
    if [ "$layout_after" != "$STANDING_LAYOUT_BEFORE" ]; then
        echo "standing Zellij layout changed during tmux smoke: $STANDING_LAYOUT" >&2
        cleanup_status=1
    fi
    if [ -n "$EVIDENCE_DIR" ]; then
        {
            printf 'original_status=%s\ncleanup_status=%s\n' "$status" "$cleanup_status"
            printf 'session_alive_after=%s\ntmux_alive_after=%s\n' "$session_alive_after" "$tmux_alive_after"
            printf 'session_delete_status=%s\nsession_probe_status_after=%s\nsession_absence_confirmed=%s\n' \
                "$session_delete_status" "$session_probe_status_after" "$session_absence_confirmed"
            printf 'tmux_kill_status=%s\ntmux_probe_status_after=%s\ntmux_absence_confirmed=%s\n' \
                "$tmux_kill_status" "$tmux_probe_status_after" "$tmux_absence_confirmed"
            printf 'tmux_probe_command=list-sessions\ntmux_server_unreachable_after=%s\n' \
                "$tmux_server_unreachable_after"
            printf 'tmux_server_pid=%s\ntmux_server_pid_alive_after=%s\ntmux_absence_basis=%s\n' \
                "$tmux_server_pid" "$tmux_server_pid_alive_after" "$tmux_absence_basis"
            printf 'tmux_socket_status=%s\ntmux_socket_path=%s\ntmux_socket_removed_after=%s\ntmux_socket_absent_after=%s\n' \
                "$tmux_socket_status" "$tmux_socket_path" "$tmux_socket_removed_after" "$tmux_socket_absent_after"
            printf 'entry_alive_after=%s\n' "$(pid_is_alive "$ENTRY_PID" && echo 1 || echo 0)"
            printf 'sidecar_alive_after=%s\n' "$(pid_is_alive "$SIDECAR_PID" && echo 1 || echo 0)"
            printf 'agentsview_alive_after=%s\n' "$(pid_is_alive "$AGENTSVIEW_PID" && echo 1 || echo 0)"
            printf 'root_exists_after=%s\n' "$root_exists_after"
            printf 'standing_config_unchanged=%s\n' "$([ "$config_after" = "$STANDING_CONFIG_BEFORE" ] && echo 1 || echo 0)"
            printf 'standing_layout_unchanged=%s\n' "$([ "$layout_after" = "$STANDING_LAYOUT_BEFORE" ] && echo 1 || echo 0)"
        } > "$EVIDENCE_DIR/cleanup-result.txt"
        printf 'zaphod-phase: phase=cleanup-complete pid=%s caller=%s mode=%s\n' \
            "$$" "$CALLER_ENV" "$SUBSCRIBER_MODE" >> "$EVIDENCE_DIR/phase.log"
    fi
    if [ "$cleanup_status" -ne 0 ]; then
        exit "$cleanup_status"
    fi
    exit "$status"
}

trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP

for required in tmux jq shasum go cargo perl; do
    command -v "$required" >/dev/null 2>&1 || fail "$required is required for the tmux smoke"
done
[[ "$NATIVE_COMMAND_TIMEOUT" =~ ^[1-9][0-9]*$ ]] ||
    fail "ZAPHOD_SMOKE_NATIVE_COMMAND_TIMEOUT_SECS must be a positive integer"
if [ -n "$INJECT_STARTUP_EXIT" ]; then
    [[ "$INJECT_STARTUP_EXIT" =~ ^([1-9]|[1-9][0-9]|1[0-9][0-9]|2[0-4][0-9]|25[0-5])$ ]] ||
        fail "ZAPHOD_SMOKE_INJECT_STARTUP_EXIT must be an exit status from 1 through 255"
fi
[[ "$INJECT_FAILURE_PAYLOAD_BYTES" =~ ^(0|[1-9][0-9]*)$ ]] &&
    [ "$INJECT_FAILURE_PAYLOAD_BYTES" -le 1048576 ] ||
    fail "ZAPHOD_SMOKE_INJECT_FAILURE_PAYLOAD_BYTES must be from 0 through 1048576"
case "$INJECT_CLEANUP_PROBE_HANG" in
    none|tmux|zellij|both) ;;
    *) fail "ZAPHOD_SMOKE_INJECT_CLEANUP_PROBE_HANG must be none, tmux, zellij, or both" ;;
esac
case "$RESPONSIVENESS_CHECK" in
    0|1) ;;
    *) fail "ZAPHOD_SMOKE_RESPONSIVENESS_CHECK must be 0 or 1" ;;
esac
case "$INJECT_RESPONSIVE_TIMEOUT" in
    ""|pane-1|pane-2|pane-3|new-tab|tab-1|tab-2|sidebar-closed|quiet-after) ;;
    *) fail "ZAPHOD_SMOKE_INJECT_RESPONSIVE_TIMEOUT names an unknown observation" ;;
esac
[ -z "$INJECT_RESPONSIVE_TIMEOUT" ] || [ "$RESPONSIVENESS_CHECK" = 1 ] ||
    fail "responsive timeout injection requires ZAPHOD_SMOKE_RESPONSIVENESS_CHECK=1"
zaphod_require_zellij_0443
WEDGE_THRESHOLD_SECS="$(sed -nE \
    's/^const WEDGE_THRESHOLD: Duration = Duration::from_secs\(([0-9]+)\);/\1/p' \
    "$REPO_ROOT/src/main.rs")"
[ "$WEDGE_THRESHOLD_SECS" = 1 ] ||
    fail "could not derive the one-second action deadline from WEDGE_THRESHOLD"
for inherited_client_var in ZELLIJ ZELLIJ_SESSION_NAME ZELLIJ_PANE_ID; do
    if printenv "$inherited_client_var" >/dev/null 2>&1; then
        fail "inherited Zellij client identity reached isolated smoke: $inherited_client_var"
    fi
done
case "$PERMISSION_FIXTURE" in
    pregranted|upgrade) ;;
    *) fail "ZAPHOD_PERMISSION_FIXTURE must be pregranted or upgrade" ;;
esac
case "$SUBSCRIBER_MODE" in
    automatic|foreground) ;;
    *) fail "ZAPHOD_SUBSCRIBER_MODE must be automatic or foreground" ;;
esac
[ "$SUBSCRIBER_MODE" != foreground ] || [ "$PERMISSION_FIXTURE" = pregranted ] ||
    fail "foreground subscriber smoke requires the disposable pregranted fixture"
if [ "$CALLER_ENV" = inside ]; then
    [ "$INHERITED_ZELLIJ_PRESENT" = x ] &&
        [ "$INHERITED_ZELLIJ_SESSION_NAME_PRESENT" = x ] &&
        [ "$INHERITED_ZELLIJ_PANE_ID_PRESENT" = x ] ||
        fail "inside caller fixture did not supply a complete loaded-client identity"
fi
[ "$PERMISSION_FIXTURE" = pregranted ] || ENTRY_START_TIMEOUT=10

# Zellij's Unix socket is capped at 103 bytes on macOS. Keep this disposable
# root under /tmp rather than the much longer per-user $TMPDIR.
ROOT="$(mktemp -d /tmp/zs.XXXXXX)" || fail "could not create a short isolated smoke root"
CONFIG_DIR="$ROOT/config"
CONFIG_FILE="$CONFIG_DIR/config.kdl"
DATA_DIR="$ROOT/data"
SOCKET_DIR="$ROOT/socket"
SESSION_NAME="zs$$"
TMUX_SERVER="zs$$"
mkdir -p "$CONFIG_DIR/layouts" "$DATA_DIR" "$SOCKET_DIR" "$ROOT/tmp"
phase root-created
ISOLATED_LAYOUT="$CONFIG_DIR/layouts/zaphod.kdl"
sed "s|<FIXED_OPERATOR_LAYOUT>|$ISOLATED_LAYOUT|" \
    "$SCRIPT_DIR/fixtures/zellij-tmux-smoke-config.kdl" > "$CONFIG_FILE"
printf '%s\n' 'layout { pane; }' > "$ISOLATED_LAYOUT"
ISOLATED_CONFIG_BEFORE="$(file_state "$CONFIG_FILE")"
ISOLATED_LAYOUT_BEFORE="$(file_state "$ISOLATED_LAYOUT")"

phase agentsview-building
go build -o "$ROOT/agentsview-fixture" "$SCRIPT_DIR/helpers/agentsview-fixture.go"
"$ROOT/agentsview-fixture" --ready-file "$ROOT/agentsview-url" --cwd "$REPO_ROOT" \
    --trigger-file "$ROOT/agentsview-second-session" >"$ROOT/agentsview.out" 2>"$ROOT/agentsview.err" &
AGENTSVIEW_PID=$!
for _attempt in $(seq 1 100); do
    [ ! -s "$ROOT/agentsview-url" ] || break
    kill -0 "$AGENTSVIEW_PID" 2>/dev/null || break
    sleep 0.05
done
[ -s "$ROOT/agentsview-url" ] || {
    cat "$ROOT/agentsview.err" >&2 || true
    fail "isolated AgentsView fixture did not become ready"
}
AGENTSVIEW_URL="$(cat "$ROOT/agentsview-url")"
phase agentsview-ready

WASM_PATH="$REPO_ROOT/target/wasm32-wasip1/release/zellij-sidebar.wasm"
if [ "${ZAPHOD_SMOKE_PREBUILT_ARTIFACTS:-}" != 1 ]; then
    CARGO_TARGET_DIR="$REPO_ROOT/target" "$REPO_ROOT/build.sh" >/dev/null
    "$(command -v cargo)" build --quiet --manifest-path "$REPO_ROOT/Cargo.toml" \
        --target-dir "$REPO_ROOT/target" \
        --features host-kdl-validator --bin zaphod-kdl-validate
fi
LAYOUT_VALIDATOR="$REPO_ROOT/target/debug/zaphod-kdl-validate"
[ -f "$WASM_PATH" ] || fail "prebuilt candidate WASM is missing"
[ -x "$REPO_ROOT/target/zaphod" ] || fail "prebuilt zaphod sidecar is missing"
[ -x "$LAYOUT_VALIDATOR" ] || fail "host KDL validator was not built"
WASM_URL="$(zaphod_canonical_file_url "$WASM_PATH")" ||
    fail "could not derive the candidate WASM URL"
phase artifacts-ready

# This is a deliberately pre-authorized, disposable permission fixture. The
# server starts with HOME under ROOT, so the real prompt remains available for
# AC-I1 while this headless smoke never writes the operator's cache or fakes
# consent with injected keys.
HOME_DIR="$ROOT/home"
PERMISSION_CACHE="$HOME_DIR/Library/Caches/org.Zellij-Contributors.Zellij/permissions.kdl"
mkdir -p "$(dirname "$PERMISSION_CACHE")"
{
    # Zellij's permission cache is keyed by the local plugin path, not by the
    # file: URL that appears in the layout and pane inventory.
    printf '"%s" {\n' "$WASM_PATH"
    printf '%s\n' \
        '    ReadApplicationState' \
        '    ChangeApplicationState' \
        '    ReadPaneContents'
    if [ "$PERMISSION_FIXTURE" = pregranted ]; then
        printf '%s\n' '    ReadCliPipes'
    fi
    printf '%s\n' \
        '    Reconfigure' \
        '    RunCommands'
    printf '%s\n' '}'
} > "$PERMISSION_CACHE"
phase profile-ready

start_tmux_zellij() {
    local command release="$ROOT/tmux-release"
    if [ -n "$INJECT_STARTUP_EXIT" ]; then
        printf -v command 'while [ ! -e %q ]; do sleep 0.01; done; printf %q; exit %q' \
            "$release" "injected startup exit $INJECT_STARTUP_EXIT\n" "$INJECT_STARTUP_EXIT"
    else
        printf -v command 'while [ ! -e %q ]; do sleep 0.01; done; exec env -u ZELLIJ -u ZELLIJ_SESSION_NAME -u ZELLIJ_PANE_ID HOME=%q ZELLIJ_SOCKET_DIR=%q %q --config-dir %q --config %q --data-dir %q attach --create %q' \
            "$release" "$HOME_DIR" "$SOCKET_DIR" "$(command -v zellij)" "$CONFIG_DIR" "$CONFIG_FILE" "$DATA_DIR" "$SESSION_NAME"
    fi
    tmux_command new-session -d -x 160 -y 45 -s "$TMUX_SESSION" "$command"
    tmux_command set-option -w -t "$TMUX_SESSION:0" remain-on-exit on
    touch "$release"
}

wait_for_nonempty_panes() {
    local output="$1"
    local attempt tmux_status pane_dead pane_dead_status
    for attempt in $(seq 1 160); do
        if zellij_session action list-panes --json --all --command --geometry --state --tab \
            > "$output" 2>"$ROOT/list-panes.err" && \
            jq -e 'type == "array" and length > 0' "$output" >/dev/null 2>&1; then
            return
        fi
        tmux_status=0
        tmux_command list-panes -t "$TMUX_PANE" -F '#{pane_dead} #{pane_dead_status}' \
            > "$ROOT/tmux-ready-status.txt" 2> "$ROOT/tmux-ready-status.err" || tmux_status=$?
        if [ "$tmux_status" -ne 0 ]; then
            fail "tmux-host-exited-before-session-ready: server_status=$tmux_status"
        fi
        read -r pane_dead pane_dead_status < "$ROOT/tmux-ready-status.txt" || true
        if [ "$pane_dead" = 1 ]; then
            tmux_command capture-pane -p -t "$TMUX_PANE" > "$ROOT/tmux-ready-dead.screen" 2>/dev/null || true
            fail "tmux-host-exited-before-session-ready: pane_dead_status=${pane_dead_status:-unknown}"
        fi
        sleep 0.05
    done
    cat "$ROOT/list-panes.err" >&2 || true
    fail "isolated Zellij session did not become ready"
}

send_literal() {
    tmux_command send-keys -l -t "$TMUX_PANE" -- "$1"
}

monotonic_ms() {
    perl -MTime::HiRes=clock_gettime,CLOCK_MONOTONIC -e \
        'printf "%d\n", clock_gettime(CLOCK_MONOTONIC) * 1000'
}

capture_action_inventory() {
    local label="$1"
    local panes="$2"
    local tabs="$3"
    local pane_status=0 tab_status=0
    if [ "$label" = "$INJECT_RESPONSIVE_TIMEOUT" ] &&
        [ "$RESPONSIVE_TIMEOUT_INJECTED" -eq 0 ]; then
        RESPONSIVE_TIMEOUT_INJECTED=1
        phase responsive-action-timeout
        bounded_exec "$WEDGE_THRESHOLD_SECS" sleep 60 \
            > "$panes" 2> "$panes.err" || pane_status=$?
        fail "injected $label native pane observation timed out: status $pane_status"
    fi
    zellij_session_with_timeout "$WEDGE_THRESHOLD_SECS" \
        action list-panes --json --all --command --geometry --state --tab \
        > "$panes" 2> "$panes.err" || pane_status=$?
    [ "$pane_status" -eq 0 ] ||
        fail "$label native pane observation failed or timed out: status $pane_status"
    zellij_session_with_timeout "$WEDGE_THRESHOLD_SECS" \
        action list-tabs --json --all --state --layout \
        > "$tabs" 2> "$tabs.err" || tab_status=$?
    [ "$tab_status" -eq 0 ] ||
        fail "$label native tab observation failed or timed out: status $tab_status"
    jq -e 'type == "array" and length > 0' "$panes" >/dev/null || return 1
    zaphod_valid_tab_inventory "$tabs" || return 1
}

capture_settled_action_inventory() {
    local label="$1"
    local panes="$2"
    local tabs="$3"
    local deadline now
    deadline="$(( $(monotonic_ms) + WEDGE_THRESHOLD_SECS * 1000 ))"
    while :; do
        if capture_action_inventory "$label" "$panes" "$tabs"; then
            return
        fi
        now="$(monotonic_ms)"
        [ "$now" -lt "$deadline" ] ||
            fail "$label returned incomplete native inventories for ${WEDGE_THRESHOLD_SECS}s"
        sleep 0.02
    done
}

wait_for_exact_action_state() {
    local label="$1"
    local expected_terminals="$2"
    local expected_tabs="$3"
    local expected_active="$4"
    local expected_candidate="$5"
    local panes="$ROOT/responsive-$label-panes.json"
    local tabs="$ROOT/responsive-$label-tabs.json"
    local deadline now terminal_count tab_count active_id candidate_count
    deadline="$(( $(monotonic_ms) + WEDGE_THRESHOLD_SECS * 1000 ))"
    while :; do
        if ! capture_action_inventory "$label" "$panes" "$tabs"; then
            now="$(monotonic_ms)"
            [ "$now" -lt "$deadline" ] ||
                fail "$label returned incomplete native inventories for ${WEDGE_THRESHOLD_SECS}s"
            sleep 0.02
            continue
        fi
        terminal_count="$(jq '[.[] | select((.is_plugin | not) and (.exited | not))] | length' "$panes")"
        tab_count="$(jq 'length' "$tabs")"
        active_id="$(jq -er '.[] | select(.active) | .tab_id' "$tabs" 2>/dev/null || true)"
        candidate_count="$(jq --arg wasm_url "$WASM_URL" \
            '[.[] | select(.is_plugin and .plugin_url == $wasm_url)] | length' "$panes")"
        now="$(monotonic_ms)"
        if [ "$terminal_count" -eq "$expected_terminals" ] &&
            [ "$tab_count" -eq "$expected_tabs" ] &&
            [ "$active_id" = "$expected_active" ] &&
            { [ "$expected_candidate" -lt 0 ] || [ "$candidate_count" -eq "$expected_candidate" ]; } &&
            [ "$now" -le "$deadline" ]; then
            return
        fi
        [ "$now" -lt "$deadline" ] || break
        sleep 0.02
    done
    fail "$label missed the ${WEDGE_THRESHOLD_SECS}s native-state deadline (terminals=$terminal_count tabs=$tab_count active=${active_id:-missing} candidate=$candidate_count)"
}

wait_for_complete_new_tab() {
    local label="$1"
    local expected_terminals="$2"
    local expected_tabs="$3"
    local before_tabs="$4"
    local panes="$ROOT/responsive-$label-panes.json"
    local tabs="$ROOT/responsive-$label-tabs.json"
    local deadline now terminal_count tab_count active_id new_count complete_count
    deadline="$(( $(monotonic_ms) + WEDGE_THRESHOLD_SECS * 1000 ))"
    while :; do
        if ! capture_action_inventory "$label" "$panes" "$tabs"; then
            now="$(monotonic_ms)"
            [ "$now" -lt "$deadline" ] ||
                fail "$label returned incomplete native inventories for ${WEDGE_THRESHOLD_SECS}s"
            sleep 0.02
            continue
        fi
        terminal_count="$(jq '[.[] | select((.is_plugin | not) and (.exited | not))] | length' "$panes")"
        tab_count="$(jq 'length' "$tabs")"
        active_id="$(jq -er '.[] | select(.active) | .tab_id' "$tabs" 2>/dev/null || true)"
        new_count="$(jq --slurpfile before "$before_tabs" \
            '[.[] | select(.tab_id as $id | all($before[0][]; .tab_id != $id))] | length' "$tabs")"
        complete_count="$(jq --arg active_id "$active_id" \
            '[.[] | select((.is_plugin | not) and (.exited | not) and (.tab_id | tostring) == $active_id)] | length' "$panes")"
        now="$(monotonic_ms)"
        if [ "$terminal_count" -eq "$expected_terminals" ] &&
            [ "$tab_count" -eq "$expected_tabs" ] &&
            [ "$new_count" -eq 1 ] && [ "$complete_count" -gt 0 ] &&
            [ "$now" -le "$deadline" ]; then
            RESPONSIVE_NEW_TAB_ID="$active_id"
            return
        fi
        [ "$now" -lt "$deadline" ] || break
        sleep 0.02
    done
    fail "$label missed the ${WEDGE_THRESHOLD_SECS}s complete-tab deadline (terminals=$terminal_count tabs=$tab_count new=$new_count active_terminals=$complete_count)"
}

capture_validated_layout() {
    local panes="$1"
    local layout="$2"
    local expectation="$3"
    zaphod_capture_validated_layout "$LAYOUT_VALIDATOR" "$WASM_URL" "$expectation" \
        "$panes" "$layout" zellij_session action dump-layout
}

capture_proven_panes() {
    local output="$1"
    local expectation="$2"
    local stderr_file="$ROOT/list-panes-capture.err"
    local attempt status provenance
    for attempt in $(seq 1 20); do
        status=0
        zellij_session action list-panes --json --all --command --geometry --state --tab \
            > "$output" 2> "$stderr_file" || status=$?
        if [ "$status" -ne 0 ]; then
            provenance="$(zaphod_bounded_reply_provenance "list-panes attempt=$attempt/20" \
                "$status" "$output" "$stderr_file")"
            echo "native-panes-unready: $provenance" >&2
            rm -f "$stderr_file"
            return 1
        fi
        if [ ! -s "$output" ]; then
            if [ "$attempt" -lt 20 ]; then
                sleep 0.05
                continue
            fi
            provenance="$(zaphod_bounded_reply_provenance "list-panes attempt=$attempt/20" \
                "$status" "$output" "$stderr_file")"
            echo "native-panes-unready: persistent empty reply; $provenance" >&2
            rm -f "$stderr_file"
            return 1
        fi
        if ! jq -e 'type == "array"' "$output" >/dev/null 2>&1; then
            provenance="$(zaphod_bounded_reply_provenance "list-panes attempt=$attempt/20" \
                "$status" "$output" "$stderr_file")"
            echo "native-panes-unready: malformed inventory; $provenance" >&2
            rm -f "$stderr_file"
            return 1
        fi
        if jq -e 'length == 0' "$output" >/dev/null 2>&1; then
            if [ "$attempt" -lt 20 ]; then
                sleep 0.05
                continue
            fi
            provenance="$(zaphod_bounded_reply_provenance "list-panes attempt=$attempt/20" \
                "$status" "$output" "$stderr_file")"
            echo "native-panes-unready: persistent empty inventory; $provenance" >&2
            rm -f "$stderr_file"
            return 1
        fi
        if zaphod_panes_prove_layout_expectation "$WASM_URL" "$expectation" "$output"; then
            rm -f "$stderr_file"
            return 0
        fi
        if [ "$attempt" -lt 20 ]; then
            sleep 0.05
            continue
        fi
        provenance="$(zaphod_bounded_reply_provenance "list-panes attempt=$attempt/20" \
            "$status" "$output" "$stderr_file")"
        echo "native-panes-unready: persistent $expectation identity mismatch; $provenance" >&2
        rm -f "$stderr_file"
        return 1
    done
    return 1
}

capture_state() {
    local json="$1"
    local layout="$2"
    local screen="$3"
    local expectation="$4"
    capture_proven_panes "$json" "$expectation" ||
        fail "native pane capture failed for $json"
    jq -S . "$json" > "$json.sorted"
    capture_validated_layout "$json" "$layout" "$expectation" ||
        fail "native layout capture failed for $layout"
    tmux_command capture-pane -p -t "$TMUX_PANE" > "$screen"
}

capture_tabs() {
    local tabs="$1"
    local stderr_file="$ROOT/list-tabs-capture.err"
    local attempt status provenance
    for attempt in $(seq 1 20); do
        status=0
        zellij_session action list-tabs --json --all --state --layout \
            > "$tabs" 2> "$stderr_file" || status=$?
        if [ "$status" -ne 0 ]; then
            provenance="$(zaphod_bounded_reply_provenance "list-tabs attempt=$attempt/20" \
                "$status" "$tabs" "$stderr_file")"
            fail "native-tabs-unready: $provenance"
        fi
        if [ -s "$tabs" ] && zaphod_valid_tab_inventory "$tabs"; then
            if jq -e 'length > 0' "$tabs" >/dev/null 2>&1; then
                jq -S . "$tabs" > "$tabs.sorted"
                rm -f "$stderr_file"
                return 0
            fi
        elif [ -s "$tabs" ]; then
            provenance="$(zaphod_bounded_reply_provenance "list-tabs attempt=$attempt/20" \
                "$status" "$tabs" "$stderr_file")"
            fail "native-tabs-unready: malformed inventory; $provenance"
        fi
        if [ "$attempt" -lt 20 ]; then
            sleep 0.05
            continue
        fi
        provenance="$(zaphod_bounded_reply_provenance "list-tabs attempt=$attempt/20" \
            "$status" "$tabs" "$stderr_file")"
        fail "native-tabs-unready: persistent empty inventory; $provenance"
    done
    return 1
}

without_geometry() {
    local source="$1"
    local normalized="$2"
    jq -S 'map(del(
        .pane_x,
        .pane_content_x,
        .pane_y,
        .pane_content_y,
        .pane_rows,
        .pane_content_rows,
        .pane_columns,
        .pane_content_columns
    ))' "$source" > "$normalized"
}

candidate_width() {
    local panes="$1"
    jq -er --arg wasm_url "$WASM_URL" \
        '[.[] | select(.is_plugin and .plugin_url == $wasm_url) | .pane_columns]
         | if length == 1 then .[0] else error("expected one candidate rail") end' \
        "$panes"
}

candidate_geometry() {
    local panes="$1"
    local geometry="$2"
    jq -S --arg wasm_url "$WASM_URL" \
        '[.[] | select(.is_plugin and .plugin_url == $wasm_url)
          | {id, pane_x, pane_y, pane_columns, pane_rows, tab_id, tab_position, tab_name}]' \
        "$panes" > "$geometry"
}

wait_for_candidate_width() {
    local output="$1"
    local expected_width="$2"
    local attempt actual
    for attempt in $(seq 1 80); do
        zellij_session action list-panes --json --all --command --geometry --state --tab \
            > "$output" 2>"$ROOT/toggle-panes.err" || true
        actual="$(candidate_width "$output" 2>/dev/null || true)"
        if [ "$actual" = "$expected_width" ]; then
            return
        fi
        sleep 0.05
    done
    cat "$ROOT/toggle-panes.err" >&2 || true
    fail "literal Alt / did not move the candidate rail to width $expected_width (last width: ${actual:-missing})"
}

wait_for_foreign_active_tab() {
    local tabs="$1"
    local foreign_tab_id="$2"
    local attempt
    for attempt in $(seq 1 80); do
        capture_tabs "$tabs"
        if jq -e --arg foreign_tab_id "$foreign_tab_id" --arg managed_tab_id "$TAB_ID" \
            'any(.[]; .active and (.tab_id | tostring) == $foreign_tab_id and (.tab_id | tostring) != $managed_tab_id)' \
            "$tabs" >/dev/null 2>&1; then
            return
        fi
        sleep 0.05
    done
    fail "native previous-tab action did not return the tmux client to the foreign tab"
}

dismiss_startup_tip() {
    local probe="$ROOT/after-dismiss.json"
    local attempt
    send_literal "$(printf '\033')"
    for attempt in $(seq 1 80); do
        zellij_session action list-panes --json --all --command --geometry --state --tab > "$probe" 2>/dev/null || true
        if jq -e 'all(.[]; .plugin_url != "about")' "$probe" >/dev/null 2>&1; then
            return
        fi
        sleep 0.05
    done
    fail "Zellij startup tip did not close; foreign-tab key assertion would be inconclusive"
}

# A pre-granted plugin still receives PermissionRequestResult asynchronously.
# Do not baseline the literal Alt / until its own visible tiled resident has
# handled that result: otherwise is_selectable can settle during the key test
# and look like a toggle side effect.
wait_for_settled_candidate_resident() {
    local panes="$1"
    local layout="$2"
    local screen="$3"
    local tabs="$4"
    local attempt
    for attempt in $(seq 1 160); do
        zellij_session action list-panes --json --all --command --geometry --state --tab \
            > "$panes" 2>"$ROOT/settled-candidate-panes.err" || true
        zellij_session action list-tabs --json --all --state --layout \
            > "$tabs" 2>"$ROOT/settled-candidate-tabs.err" || true
        tmux_command capture-pane -p -t "$TMUX_PANE" > "$screen" || true
        if jq -e --arg wasm_url "$WASM_URL" --arg tab_id "$TAB_ID" \
            'any(.[]; .is_plugin and .plugin_url == $wasm_url and (.tab_id | tostring) == $tab_id and .is_floating == false and .is_suppressed == false and .pane_columns == 28 and .is_selectable == false)' \
            "$panes" >/dev/null 2>&1 && \
            jq -e --arg tab_id "$TAB_ID" 'any(.[]; .active and (.tab_id | tostring) == $tab_id)' "$tabs" >/dev/null 2>&1 && \
            ! grep -F 'asks permission to:' "$screen" >/dev/null && \
            grep -F 'PANES' "$screen" >/dev/null; then
            jq -S . "$panes" > "$panes.sorted"
            jq -S . "$tabs" > "$tabs.sorted"
            capture_validated_layout "$panes" "$layout" present ||
                fail "settled candidate layout did not validate atomically"
            return
        fi
        sleep 0.05
    done
    cat "$ROOT/settled-candidate-panes.err" >&2 || true
    cat "$ROOT/settled-candidate-tabs.err" >&2 || true
    jq -S . "$panes" >&2 || true
    jq -S . "$tabs" >&2 || true
    sed -n '1,80p' "$screen" >&2 || true
    fail "candidate did not settle as the active tiled 28-column, non-selectable post-grant resident"
}

# Run the selected-checkout entry against a real attached client. The fixture
# already has a fixed global Alt Shift z shortcut and fail-closed Alt / policy;
# the direct command must create its own inline tab without changing either.
phase tmux-zellij-starting
start_tmux_zellij
phase tmux-zellij-launched
phase session-ready-wait
wait_for_nonempty_panes "$ROOT/foreign-ready.json"
phase session-ready
dismiss_startup_tip
zellij_control setup --check >/dev/null
capture_tabs "$ROOT/foreign-tabs-before.json"
zaphod_valid_tab_inventory "$ROOT/foreign-tabs-before.json" ||
    fail "isolated foreign tab inventory was not a complete stable-ID record"
TAB_COUNT_BEFORE="$(jq -er 'length' "$ROOT/foreign-tabs-before.json")"
FOREIGN_TAB_ID="$(jq -er '.[] | select(.active) | .tab_id' "$ROOT/foreign-tabs-before.json")"
capture_state "$ROOT/foreign-ready.json" "$ROOT/foreign-ready.kdl" "$ROOT/foreign-ready.screen" absent
phase foreign-baseline-captured
jq -e --arg wasm_url "$WASM_URL" \
    'all(.[]; .plugin_url != $wasm_url)' "$ROOT/foreign-ready.json" >/dev/null ||
    fail "isolated profile unexpectedly started on the selected checkout rail"
entry_command() {
    local client_env=(env -u ZELLIJ -u ZELLIJ_SESSION_NAME -u ZELLIJ_PANE_ID)
    if [ "$CALLER_ENV" = inside ]; then
        client_env=(env \
            "ZELLIJ=$INHERITED_ZELLIJ" \
            "ZELLIJ_SESSION_NAME=$INHERITED_ZELLIJ_SESSION_NAME" \
            "ZELLIJ_PANE_ID=$INHERITED_ZELLIJ_PANE_ID")
    fi
        "${client_env[@]}" \
        ZELLIJ_CONFIG_DIR="$CONFIG_DIR" ZELLIJ_CONFIG_FILE="$CONFIG_FILE" \
        ZELLIJ_DATA_DIR="$DATA_DIR" ZELLIJ_SOCKET_DIR="$SOCKET_DIR" TMPDIR="$ROOT/tmp" \
        CARGO_TARGET_DIR="$REPO_ROOT/target" \
        ZAPHOD_TEST_PREBUILT_ARTIFACTS="${ZAPHOD_SMOKE_PREBUILT_ARTIFACTS:-}" \
        ZAPHOD_SIDECAR_START_TIMEOUT="$ENTRY_START_TIMEOUT" \
        "$REPO_ROOT/scripts/zellij-new-tab.sh" --session "$SESSION_NAME" --name 'Zaphod selected checkout' \
        --agentsview-url "$AGENTSVIEW_URL"
}

foreground_entry() {
    local rendered_layout="$ROOT/foreground-zaphod.kdl"
    local recipient_token="zaphod-smoke-$$-$(date +%s)"
    local target_pane_id=""
    local startup_message=""
    local startup_status=0
    local new_tab_status=0
    local tabs_after_status=1
    local target_status=1
    local attempt

    zaphod_render_layout "$REPO_ROOT/layouts/zaphod.kdl" "$WASM_URL" \
        "$rendered_layout" "$recipient_token"
    zaphod_validate_layout_identity "$rendered_layout" "$WASM_URL"
    set +e
    zellij_session action new-tab --name 'Zaphod foreground subscriber' \
        --cwd "$REPO_ROOT" --layout-string "$(cat "$rendered_layout")" \
        > "$ROOT/foreground-new-tab.stdout" 2> "$ROOT/foreground-new-tab.stderr"
    new_tab_status=$?
    set -e
    if [ "$new_tab_status" -ne 0 ]; then
        printf 'FAIL: foreground %s\n' \
            "$(zaphod_bounded_reply_provenance new-tab "$new_tab_status" \
                "$ROOT/foreground-new-tab.stdout" "$ROOT/foreground-new-tab.stderr")" >&2
        return 1
    fi
    TAB_ID=""
    : > "$ROOT/foreground-tabs-after.json"
    : > "$ROOT/foreground-tabs-after.err"
    for attempt in $(seq 1 80); do
        set +e
        zellij_session action list-tabs --json --all --state --layout \
            > "$ROOT/foreground-tabs-after.json" 2> "$ROOT/foreground-tabs-after.err"
        tabs_after_status=$?
        set -e
        if [ "$tabs_after_status" -eq 0 ] && zaphod_valid_tab_inventory "$ROOT/foreground-tabs-after.json"; then
            TAB_ID="$(zaphod_new_tab_id_from_inventories "$ROOT/foreign-tabs-before.json" \
                "$ROOT/foreground-tabs-after.json" 2>/dev/null || true)"
            [ -z "$TAB_ID" ] || break
        fi
        sleep 0.05
    done
    if ! [[ "$TAB_ID" =~ ^(0|[1-9][0-9]*)$ ]]; then
        printf 'FAIL: foreground stable-ID discovery: %s; %s\n' \
            "$(zaphod_bounded_reply_provenance new-tab "$new_tab_status" \
                "$ROOT/foreground-new-tab.stdout" "$ROOT/foreground-new-tab.stderr")" \
            "$(zaphod_bounded_reply_provenance list-tabs-after "$tabs_after_status" \
                "$ROOT/foreground-tabs-after.json" "$ROOT/foreground-tabs-after.err")" >&2
        return 1
    fi
    for attempt in $(seq 1 80); do
        set +e
        zellij_session action list-panes --json --all --command --geometry --state --tab \
            > "$ROOT/foreground-target.json" 2>"$ROOT/foreground-target.err"
        target_status=$?
        set -e
        target_pane_id="$(jq -er --arg tab_id "$TAB_ID" --arg wasm_url "$WASM_URL" \
            '[.[] | select((.tab_id | tostring) == $tab_id and .is_plugin and .plugin_url == $wasm_url and (.is_floating | not) and (.is_suppressed | not))] | if length == 1 then .[0].id | tostring else empty end' \
            "$ROOT/foreground-target.json" 2>/dev/null || true)"
        [ -n "$target_pane_id" ] && break
        sleep 0.05
    done
    [ -n "$target_pane_id" ] || {
        zaphod_bounded_reply_provenance list-panes "$target_status" \
            "$ROOT/foreground-target.json" "$ROOT/foreground-target.err" >&2
        printf '\n' >&2
        fail "foreground entry never exposed its exact target rail"
    }
    case "$target_pane_id" in
        plugin_*) ;;
        0|[1-9]|[1-9][0-9]*) target_pane_id="plugin_$target_pane_id" ;;
        *) fail "foreground target returned an invalid pane ID: $target_pane_id" ;;
    esac
    zellij_session action focus-pane-id "$target_pane_id"

    SIDECAR_LOG="$ROOT/foreground-subscriber.log"
    SIDECAR_START_FIFO="$ROOT/foreground-subscriber-start"
    mkfifo "$SIDECAR_START_FIFO"
    env -u ZELLIJ -u ZELLIJ_SESSION_NAME -u ZELLIJ_PANE_ID \
        ZELLIJ_SOCKET_DIR="$SOCKET_DIR" "$REPO_ROOT/target/zaphod" subscribe \
        --server "$AGENTSVIEW_URL" \
        --zellij-bin "$(command -v zellij)" \
        --zellij-config-dir "$CONFIG_DIR" \
        --zellij-config "$CONFIG_FILE" \
        --zellij-data-dir "$DATA_DIR" \
        --zellij-session "$SESSION_NAME" \
        --tab-id "$TAB_ID" \
        --rail-url "$WASM_URL" \
        --checkout-cwd "$REPO_ROOT" \
        --recipient-token "$recipient_token" \
        --startup-fd 3 \
        3>"$SIDECAR_START_FIFO" > >(tee -a "$SIDECAR_LOG") 2>&1 &
    SIDECAR_PID=$!
    set +e
    IFS= read -r -t "$ENTRY_START_TIMEOUT" startup_message < "$SIDECAR_START_FIFO"
    startup_status=$?
    set -e
    rm -f "$SIDECAR_START_FIFO"
    SIDECAR_START_FIFO=""
    if [ "$startup_status" -ne 0 ] || [ "$startup_message" != ready ]; then
        sed -n '1,40p' "$SIDECAR_LOG" >&2 || true
        fail "foreground target/zaphod subscribe did not become ready"
    fi
    {
        printf 'TAB_ID=%s\n' "$TAB_ID"
        printf 'WASM_URL=%s\n' "$WASM_URL"
        printf 'SIDECAR_LOG=%s\n' "$SIDECAR_LOG"
        printf 'SIDECAR_PID=%s\n' "$SIDECAR_PID"
    } > "$ROOT/entry.out"
}

phase entry-start
if [ "$SUBSCRIBER_MODE" = foreground ]; then
    foreground_entry
elif [ "$PERMISSION_FIXTURE" = upgrade ]; then
    entry_command > "$ROOT/entry.out" 2> "$ROOT/entry.err" &
    ENTRY_PID=$!
    for _attempt in $(seq 1 160); do
        tmux_command capture-pane -p -t "$TMUX_PANE" > "$ROOT/permission-prompt.screen"
        zellij_session action list-panes --json --all --command --geometry --state --tab \
            > "$ROOT/permission-prompt-panes.json" 2>/dev/null || true
        if grep -F 'Allow? (y/n)' "$ROOT/permission-prompt.screen" >/dev/null &&
            jq -e --arg wasm_url "$WASM_URL" \
                'any(.[]; .is_plugin and .plugin_url == $wasm_url and .is_focused == true)' \
                "$ROOT/permission-prompt-panes.json" >/dev/null 2>&1; then
            break
        fi
        kill -0 "$ENTRY_PID" 2>/dev/null || break
        sleep 0.05
    done
    if ! grep -F 'Allow? (y/n)' "$ROOT/permission-prompt.screen" >/dev/null ||
        ! jq -e --arg wasm_url "$WASM_URL" \
            'any(.[]; .is_plugin and .plugin_url == $wasm_url and .is_focused == true)' \
            "$ROOT/permission-prompt-panes.json" >/dev/null 2>&1; then
        cat "$ROOT/entry.err" >&2 || true
        sed -n '1,80p' "$ROOT/permission-prompt.screen" >&2 || true
        zellij_session action list-panes --json --all --command --geometry --state --tab >&2 || true
        fail "old permission cache did not expose an actionable native expanded-permission prompt"
    fi
    send_literal y
    set +e
    wait "$ENTRY_PID"
    ENTRY_STATUS=$?
    set -e
    ENTRY_PID=""
    if [ "$ENTRY_STATUS" -ne 0 ]; then
        cat "$ROOT/entry.err" >&2 || true
        fail "literal permission approval did not complete direct entry"
    fi
    grep -F 'ReadCliPipes' "$PERMISSION_CACHE" >/dev/null ||
        fail "literal permission approval did not persist the expanded CLI-pipe grant"
else
    entry_command > "$ROOT/entry.out"
fi
phase entry-complete
TAB_ID="$(sed -n 's/^TAB_ID=//p' "$ROOT/entry.out")"
SIDECAR_PID="$(sed -n 's/^SIDECAR_PID=//p' "$ROOT/entry.out")"
SIDECAR_LOG="$(sed -n 's/^SIDECAR_LOG=//p' "$ROOT/entry.out")"
[[ "$TAB_ID" =~ ^(0|[1-9][0-9]*)$ ]] || fail "entry script did not report a stable tab ID"
[[ "$SIDECAR_PID" =~ ^[1-9][0-9]*$ ]] || fail "entry script did not report a sidecar PID"
[ -n "$SIDECAR_LOG" ] && [ -f "$SIDECAR_LOG" ] || fail "subscriber did not report a diagnostic log"
kill -0 "$SIDECAR_PID" 2>/dev/null || fail "private sidecar exited before smoke assertions"
grep -Fx "WASM_URL=$WASM_URL" "$ROOT/entry.out" >/dev/null ||
    fail "entry script did not report the candidate WASM URL"
wait_for_settled_candidate_resident \
    "$ROOT/candidate-before.json" \
    "$ROOT/candidate-before.kdl" \
    "$ROOT/candidate-before.screen" \
    "$ROOT/candidate-tabs-before.json"
phase candidate-settled
for _attempt in $(seq 1 100); do
    tmux_command capture-pane -p -t "$TMUX_PANE" > "$ROOT/agents-row.screen"
    if grep -F 'AGENTS' "$ROOT/agents-row.screen" >/dev/null &&
        grep -F 'SMOKE_INITIAL_ROW' "$ROOT/agents-row.screen" >/dev/null; then
        break
    fi
    sleep 0.05
done
grep -F 'AGENTS' "$ROOT/agents-row.screen" >/dev/null || fail "initial subscriber row section never rendered"
grep -F 'SMOKE_INITIAL_ROW' "$ROOT/agents-row.screen" >/dev/null || fail "initial subscriber row was lost before recipient arming"
phase initial-row-rendered
touch "$ROOT/agentsview-second-session"
for _attempt in $(seq 1 160); do
    tmux_command capture-pane -p -t "$TMUX_PANE" > "$ROOT/agents-second-row.screen"
    if grep -F 'SMOKE_INITIAL_ROW' "$ROOT/agents-second-row.screen" >/dev/null &&
        grep -F 'SMOKE_SECOND_ROW' "$ROOT/agents-second-row.screen" >/dev/null; then
        break
    fi
    kill -0 "$SIDECAR_PID" 2>/dev/null || break
    sleep 0.05
done
if ! grep -F 'SMOKE_INITIAL_ROW' "$ROOT/agents-second-row.screen" >/dev/null ||
    ! grep -F 'SMOKE_SECOND_ROW' "$ROOT/agents-second-row.screen" >/dev/null; then
    sed -n '1,40p' "$SIDECAR_LOG" >&2 || true
    sed -n '1,80p' "$ROOT/agents-second-row.screen" >&2 || true
    fail "post-readiness data_changed did not render both distinct session rows"
fi
kill -0 "$SIDECAR_PID" 2>/dev/null || fail "subscriber exited after post-readiness data_changed delivery"
phase second-row-rendered
TAB_COUNT_AFTER="$(jq -er 'length' "$ROOT/candidate-tabs-before.json")"
[ "$TAB_COUNT_AFTER" -eq "$((TAB_COUNT_BEFORE + 1))" ] ||
    fail "direct entry changed tab count from $TAB_COUNT_BEFORE to $TAB_COUNT_AFTER (expected one fresh tab)"
jq -e --arg wasm_url "$WASM_URL" \
    --arg tab_id "$TAB_ID" \
    '([.[] | select(.is_plugin and .plugin_url == $wasm_url and (.tab_id | tostring) == $tab_id and .is_floating == false and .is_suppressed == false)] | length) == 1' \
    "$ROOT/candidate-before.json" >/dev/null || fail "candidate pane did not appear in the native Zellij state"
jq -e --arg tab_id "$TAB_ID" 'any(.[]; .active and (.tab_id | tostring) == $tab_id)' \
    "$ROOT/candidate-tabs-before.json" >/dev/null || fail "direct entry did not activate its stable-ID tab"
grep -F "plugin location=\"$WASM_URL\"" "$ROOT/candidate-before.kdl" >/dev/null ||
    fail "candidate URL did not appear in the native Zellij layout dump"
grep -F 'Allow? (y/n)' "$ROOT/candidate-before.screen" >/dev/null &&
    fail "candidate rail unexpectedly prompted instead of using the disposable pre-grant"
grep -F 'PANES' "$ROOT/candidate-before.screen" >/dev/null ||
    fail "candidate Zaphod rail was not visibly rendered in the tmux client"
jq -e --arg wasm_url "$WASM_URL" \
    'any(.[]; .is_plugin and .plugin_url == $wasm_url and .is_selectable == false)' \
    "$ROOT/candidate-before.json" >/dev/null ||
    fail "candidate baseline was captured before its pre-granted permission result settled"
[ "$(file_state "$CONFIG_FILE")" = "$ISOLATED_CONFIG_BEFORE" ] ||
    fail "direct entry changed the isolated profile config"
[ "$(file_state "$ISOLATED_LAYOUT")" = "$ISOLATED_LAYOUT_BEFORE" ] ||
    fail "direct entry changed the isolated profile layout"
find "$ROOT/tmp" -mindepth 1 -maxdepth 1 -name 'zaphod-new-tab.*' -print -quit | \
    grep -q . && fail "direct entry left its rendered-layout temporary root"

# A pre-authorized rail requests its runtime MessagePluginId route, but the
# request's return value is not authorization. One literal key must be
# received by the active tiled resident and change only the known dock shape.
[ "$(candidate_width "$ROOT/candidate-before.json")" = "28" ] ||
    fail "candidate did not begin in the known docked 28-column shape"
without_geometry "$ROOT/candidate-before.json" "$ROOT/candidate-before.identity.json"
candidate_geometry "$ROOT/candidate-before.json" "$ROOT/candidate-before.geometry.json"
send_literal "$(printf '\033/')"
wait_for_candidate_width "$ROOT/candidate-after.json" 1
phase managed-toggle-complete
# `wait_for_candidate_width` already captured the valid post-key native pane
# inventory. Do not issue a second list-panes call in the swap transition;
# v0.44 can briefly return an empty successful response while it redraws.
jq -S . "$ROOT/candidate-after.json" > "$ROOT/candidate-after.json.sorted"
capture_validated_layout "$ROOT/candidate-after.json" "$ROOT/candidate-after.kdl" present ||
    fail "post-toggle candidate layout did not validate atomically"
tmux_command capture-pane -p -t "$TMUX_PANE" > "$ROOT/candidate-after.screen"
without_geometry "$ROOT/candidate-after.json" "$ROOT/candidate-after.identity.json"
candidate_geometry "$ROOT/candidate-after.json" "$ROOT/candidate-after.geometry.json"
cmp -s "$ROOT/candidate-before.identity.json" "$ROOT/candidate-after.identity.json" || {
    diff -u "$ROOT/candidate-before.identity.json" "$ROOT/candidate-after.identity.json" >&2 || true
    fail "managed Alt / replaced a pane, process, focus, or candidate identity"
}
cmp -s "$ROOT/candidate-before.geometry.json" "$ROOT/candidate-after.geometry.json" &&
    fail "managed Alt / did not change the candidate rail geometry"
cmp -s "$ROOT/candidate-before.kdl" "$ROOT/candidate-after.kdl" &&
    fail "managed Alt / did not change the native managed layout shape"
cmp -s "$ROOT/candidate-before.screen" "$ROOT/candidate-after.screen" &&
    fail "managed Alt / did not visibly change the tmux client"

# AC-O3 must run after the usable managed route has been observed. Return the
# same tmux client to its sidebar-less tab with a native Zellij action, then
# send literal Alt / and require byte-identical foreign state.
zellij_session action go-to-previous-tab
wait_for_foreign_active_tab "$ROOT/foreign-tabs-after-route.json" "$FOREIGN_TAB_ID"
phase foreign-route-check
capture_state "$ROOT/foreign-before.json" "$ROOT/foreign-before.kdl" "$ROOT/foreign-before.screen" present
send_literal "$(printf '\033/')"
sleep 0.10
for _attempt in 1 2 3; do
    capture_state "$ROOT/foreign-after.json" "$ROOT/foreign-after.kdl" "$ROOT/foreign-after.screen" present
    if cmp -s "$ROOT/foreign-before.json.sorted" "$ROOT/foreign-after.json.sorted" &&
        cmp -s "$ROOT/foreign-before.kdl" "$ROOT/foreign-after.kdl" &&
        cmp -s "$ROOT/foreign-before.screen" "$ROOT/foreign-after.screen"; then
        break
    fi
    [ "$_attempt" -eq 3 ] || {
        printf 'transient-native-foreign-state: attempt=%s/3\n' "$_attempt" >&2
        sleep 0.05
    }
done
cmp -s "$ROOT/foreign-before.json.sorted" "$ROOT/foreign-after.json.sorted" || {
    diff -u "$ROOT/foreign-before.json.sorted" "$ROOT/foreign-after.json.sorted" >&2 || true
    fail "post-route foreign Alt / changed native pane, focus, tab, or process state"
}
cmp -s "$ROOT/foreign-before.kdl" "$ROOT/foreign-after.kdl" || {
    diff -u "$ROOT/foreign-before.kdl" "$ROOT/foreign-after.kdl" >&2 || true
    fail "post-route foreign Alt / changed the native layout"
}
cmp -s "$ROOT/foreign-before.screen" "$ROOT/foreign-after.screen" || {
    diff -u "$ROOT/foreign-before.screen" "$ROOT/foreign-after.screen" >&2 || true
    fail "post-route foreign Alt / visibly changed the tmux client"
}
jq -e --arg wasm_url "$WASM_URL" \
    '([.[] | select(.is_plugin and .plugin_url == $wasm_url)] | length) == 1' \
    "$ROOT/foreign-after.json" >/dev/null || fail "foreign Alt / created or removed a candidate rail"

if [ "$RESPONSIVENESS_CHECK" = 1 ]; then
    phase responsive-actions-start
    capture_settled_action_inventory responsive-start \
        "$ROOT/responsive-start-panes.json" "$ROOT/responsive-start-tabs.json"
    RESPONSIVE_TERMINALS="$(jq '[.[] | select((.is_plugin | not) and (.exited | not))] | length' \
        "$ROOT/responsive-start-panes.json")"
    RESPONSIVE_TABS="$(jq 'length' "$ROOT/responsive-start-tabs.json")"
    zellij_session action go-to-tab 2
    wait_for_exact_action_state managed-return "$RESPONSIVE_TERMINALS" "$RESPONSIVE_TABS" "$TAB_ID" 1

    # The earlier routing proof leaves the sidebar in its one-column shape.
    # Restore the same resident to 28 columns so a two-second timer tick runs
    # before the measured key sequence.
    send_literal "$(printf '\033/')"
    wait_for_candidate_width "$ROOT/responsive-docked.json" 28
    phase responsive-sidebar-docked

    cp "$ROOT/responsive-managed-return-panes.json" "$ROOT/responsive-before-fixture-panes.json"
    RESPONSIVE_TERMINALS="$((RESPONSIVE_TERMINALS + 1))"
    zellij_session action new-pane --tab-id "$TAB_ID" --name zaphod-long-running-non-shell \
        -- sh -c 'while :; do sleep 60; done' > "$ROOT/responsive-fixture-pane-id.txt"
    RESPONSIVE_FIXTURE_ID="$(tr -d '[:space:]' < "$ROOT/responsive-fixture-pane-id.txt")"
    case "$RESPONSIVE_FIXTURE_ID" in
        terminal_[0-9]*) ;;
        *) fail "non-shell fixture did not return a terminal pane identity: $RESPONSIVE_FIXTURE_ID" ;;
    esac
    wait_for_exact_action_state fixture-ready "$RESPONSIVE_TERMINALS" "$RESPONSIVE_TABS" "$TAB_ID" 1
    RESPONSIVE_FIXTURE_NUM="${RESPONSIVE_FIXTURE_ID#terminal_}"
    jq -e --arg id "$RESPONSIVE_FIXTURE_NUM" \
        'any(.[]; (.id | tostring) == $id and (.is_plugin | not) and (.exited | not) and .title == "zaphod-long-running-non-shell" and (.terminal_command | tostring | contains("tail")))' \
        "$ROOT/responsive-fixture-ready-panes.json" >/dev/null ||
        fail "non-shell tail fixture was not live with its exact native command identity"

    sleep 2.1
    capture_settled_action_inventory responsive-after-timer \
        "$ROOT/responsive-after-timer-panes.json" "$ROOT/responsive-after-timer-tabs.json"
    jq '[.[] | select((.is_plugin | not) and (.exited | not)) | .id] | sort' \
        "$ROOT/responsive-after-timer-panes.json" > "$ROOT/responsive-terminal-ids-before.json"

    for RESPONSIVE_PANE_INDEX in 1 2 3; do
        RESPONSIVE_TERMINALS="$((RESPONSIVE_TERMINALS + 1))"
        send_literal "$(printf '\033p')"
        wait_for_exact_action_state "pane-$RESPONSIVE_PANE_INDEX" \
            "$RESPONSIVE_TERMINALS" "$RESPONSIVE_TABS" "$TAB_ID" 1
        jq '[.[] | select((.is_plugin | not) and (.exited | not)) | .id] | sort' \
            "$ROOT/responsive-pane-$RESPONSIVE_PANE_INDEX-panes.json" \
            > "$ROOT/responsive-terminal-ids-after.json"
        jq -e --slurpfile before "$ROOT/responsive-terminal-ids-before.json" \
            '. as $after | all($before[0][]; . as $id | $after | index($id) != null)' \
            "$ROOT/responsive-terminal-ids-after.json" >/dev/null ||
            fail "literal Alt p $RESPONSIVE_PANE_INDEX replaced an existing terminal identity"
        mv "$ROOT/responsive-terminal-ids-after.json" "$ROOT/responsive-terminal-ids-before.json"
    done

    cp "$ROOT/responsive-pane-3-tabs.json" "$ROOT/responsive-tabs-before-new.json"
    RESPONSIVE_TERMINALS="$((RESPONSIVE_TERMINALS + 1))"
    RESPONSIVE_TABS="$((RESPONSIVE_TABS + 1))"
    RESPONSIVE_NEW_TAB_ID=""
    send_literal "$(printf '\033n')"
    wait_for_complete_new_tab new-tab "$RESPONSIVE_TERMINALS" "$RESPONSIVE_TABS" \
        "$ROOT/responsive-tabs-before-new.json"
    [[ "$RESPONSIVE_NEW_TAB_ID" =~ ^(0|[1-9][0-9]*)$ ]] ||
        fail "literal Alt n did not expose one stable active tab identity"

    send_literal "$(printf '\0331')"
    wait_for_exact_action_state tab-1 "$RESPONSIVE_TERMINALS" "$RESPONSIVE_TABS" "$FOREIGN_TAB_ID" 1
    send_literal "$(printf '\0332')"
    wait_for_exact_action_state tab-2 "$RESPONSIVE_TERMINALS" "$RESPONSIVE_TABS" "$TAB_ID" 1

    RESPONSIVE_PLUGIN_ID="$(jq -er --arg wasm_url "$WASM_URL" \
        '[.[] | select(.is_plugin and .plugin_url == $wasm_url)] | if length == 1 then .[0].id else error("candidate cardinality") end' \
        "$ROOT/responsive-tab-2-panes.json")"
    zellij_session action close-pane --pane-id "plugin_$RESPONSIVE_PLUGIN_ID"
    wait_for_exact_action_state sidebar-closed "$RESPONSIVE_TERMINALS" "$RESPONSIVE_TABS" "$TAB_ID" 0
    jq -S 'map({id, is_plugin, plugin_url, tab_id, exited}) | sort_by(.is_plugin, .id)' \
        "$ROOT/responsive-sidebar-closed-panes.json" > "$ROOT/responsive-quiet-panes-before.json"
    jq -S 'map({tab_id, active}) | sort_by(.tab_id)' \
        "$ROOT/responsive-sidebar-closed-tabs.json" > "$ROOT/responsive-quiet-tabs-before.json"

    sleep 6
    capture_settled_action_inventory quiet-after \
        "$ROOT/responsive-quiet-after-panes.json" "$ROOT/responsive-quiet-after-tabs.json"
    jq -S 'map({id, is_plugin, plugin_url, tab_id, exited}) | sort_by(.is_plugin, .id)' \
        "$ROOT/responsive-quiet-after-panes.json" > "$ROOT/responsive-quiet-panes-after.json"
    jq -S 'map({tab_id, active}) | sort_by(.tab_id)' \
        "$ROOT/responsive-quiet-after-tabs.json" > "$ROOT/responsive-quiet-tabs-after.json"
    cmp -s "$ROOT/responsive-quiet-panes-before.json" "$ROOT/responsive-quiet-panes-after.json" || {
        diff -u "$ROOT/responsive-quiet-panes-before.json" "$ROOT/responsive-quiet-panes-after.json" >&2 || true
        fail "pane identities changed during the six-second no-input window"
    }
    cmp -s "$ROOT/responsive-quiet-tabs-before.json" "$ROOT/responsive-quiet-tabs-after.json" || {
        diff -u "$ROOT/responsive-quiet-tabs-before.json" "$ROOT/responsive-quiet-tabs-after.json" >&2 || true
        fail "tab identity or active tab changed during the six-second no-input window"
    }
    phase responsive-actions-complete
    printf 'PASS: literal Alt p/Alt n/Alt 1/Alt 2 met native one-second deadlines; the six-second post-close state was stable\n'
fi

[ "$(file_state "$STANDING_CONFIG")" = "$STANDING_CONFIG_BEFORE" ] ||
    fail "standing Zellij config changed during tmux smoke: $STANDING_CONFIG"
[ "$(file_state "$STANDING_LAYOUT")" = "$STANDING_LAYOUT_BEFORE" ] ||
    fail "standing Zellij layout changed during tmux smoke: $STANDING_LAYOUT"
kill -0 "$SIDECAR_PID" 2>/dev/null || fail "private sidecar exited during smoke assertions"

phase smoke-complete
printf 'PASS: %s caller with %s target/zaphod subscribe rendered SMOKE_INITIAL_ROW then SMOKE_SECOND_ROW, stayed alive, preserved routing, and cleaned up\n' \
    "$CALLER_ENV" "$SUBSCRIBER_MODE"
