#!/bin/bash
# ABOUTME: Proves the repo-local Codex hook is a transparent tab-watcher bridge.
# ABOUTME: Outside Zellij it is a no-op even before the selected binary exists.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd -P)"
ROOT="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-codex-hook.XXXXXX")"
trap 'rm -rf "$ROOT"' EXIT

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

jq -e '
    .hooks.SessionStart == [{
      "matcher": "startup|resume",
      "hooks": [{"type": "command", "command": "scripts/zaphod-codex-session-hook.sh"}]
    }]
' "$REPO_ROOT/.codex/hooks.json" >/dev/null || fail "project hook is not the exact startup/resume command"

PAYLOAD='{"session_id":"019f5f94-a596-7d92-9928-398653669161","transcript_path":null,"cwd":"/same/cwd","hook_event_name":"SessionStart","model":"gpt-5.6","permission_mode":"default","source":"startup"}'
printf '%s\n' "$PAYLOAD" |
    ZAPHOD_BIN="$ROOT/not-built" "$REPO_ROOT/scripts/zaphod-codex-session-hook.sh" ||
    fail "outside-Zellij hook required a built receiver"

cat > "$ROOT/zaphod" <<'EOF'
#!/bin/bash
set -euo pipefail
printf '%s\n' "$@" > "$CAPTURE_ARGS"
printf '%s\n%s\n' "$ZELLIJ_SESSION_NAME" "$ZELLIJ_PANE_ID" > "$CAPTURE_IDENTITY"
cat > "$CAPTURE_STDIN"
EOF
chmod +x "$ROOT/zaphod"

printf '%s\n' "$PAYLOAD" |
    CAPTURE_ARGS="$ROOT/args" CAPTURE_IDENTITY="$ROOT/identity" CAPTURE_STDIN="$ROOT/stdin" \
    ZAPHOD_BIN="$ROOT/zaphod" ZELLIJ_SESSION_NAME=managed ZELLIJ_PANE_ID=7 \
    "$REPO_ROOT/scripts/zaphod-codex-session-hook.sh"

[ "$(cat "$ROOT/args")" = register-agent-session ] || fail "hook did not invoke the watcher registrar"
[ "$(sed -n '1p' "$ROOT/identity")" = managed ] || fail "hook lost the Zellij session"
[ "$(sed -n '2p' "$ROOT/identity")" = 7 ] || fail "hook lost the terminal pane"
[ "$(cat "$ROOT/stdin")" = "$PAYLOAD" ] || fail "hook changed the complete SessionStart record"

set +e
printf '%s\n' "$PAYLOAD" |
    ZAPHOD_BIN="$ROOT/not-built" ZELLIJ_SESSION_NAME=managed \
    "$REPO_ROOT/scripts/zaphod-codex-session-hook.sh" >/dev/null 2>&1
status=$?
set -e
[ "$status" -ne 0 ] || fail "partial Zellij identity became an outside-Zellij no-op"

echo "PASS: Codex hook is an exact watcher bridge and an outside-Zellij no-op"
