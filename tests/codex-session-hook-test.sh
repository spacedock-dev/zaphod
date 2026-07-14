#!/bin/bash
# ABOUTME: Proves the trusted repo-local Codex hook invokes this checkout's registrar.
# ABOUTME: Uses an isolated registry and never mutates the operator's Codex configuration.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd -P)"
ROOT="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-codex-hook.XXXXXX")"
trap 'rm -rf "$ROOT"' EXIT

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

cd "$REPO_ROOT/grout"
mkdir -p "$REPO_ROOT/target"
go build -o "$REPO_ROOT/target/zaphod" .

jq -e '
    .hooks.SessionStart == [{
      "matcher": "startup|resume",
      "hooks": [{"type": "command", "command": "scripts/zaphod-codex-session-hook.sh"}]
    }]
' "$REPO_ROOT/.codex/hooks.json" >/dev/null || fail "project hook is not the exact startup/resume command"

SESSION_ID=019f5f94-a596-7d92-9928-398653669161
printf '%s\n' "{\"session_id\":\"$SESSION_ID\",\"transcript_path\":null,\"cwd\":\"/wrong/same/cwd\",\"hook_event_name\":\"SessionStart\",\"model\":\"gpt-5.6\",\"permission_mode\":\"default\",\"source\":\"startup\"}" |
    ZAPHOD_REGISTRY_DIR="$ROOT/registry" \
    ZELLIJ_SESSION_NAME=managed \
    ZELLIJ_PANE_ID=7 \
    "$REPO_ROOT/scripts/zaphod-codex-session-hook.sh"

REGISTRY="$(find "$ROOT/registry" -name 'session-*.json' -type f -maxdepth 1)"
[ -n "$REGISTRY" ] || fail "hook did not create a registry generation"
jq -e --arg id "$SESSION_ID" '
    .version == 1 and
    .zellij_session == "managed" and
    .registrations == [{
      "zellij_session": "managed",
      "pane_id": 7,
      "agent": "codex",
      "agent_session_id": $id,
      "agentsview_session_id": ("codex:" + $id),
      "pid": .registrations[0].pid,
      "updated_at": .registrations[0].updated_at
    }]
' "$REGISTRY" >/dev/null || fail "hook record did not retain exact provider and pane identity"

before="$(shasum -a 256 "$REGISTRY")"
printf '%s\n' '{"session_id":"019f5f95-bbfd-7993-8620-0d698008217f","hook_event_name":"SubagentStart","source":"startup"}' |
    ZAPHOD_REGISTRY_DIR="$ROOT/registry" \
    ZELLIJ_SESSION_NAME=managed \
    ZELLIJ_PANE_ID=7 \
    "$REPO_ROOT/target/zaphod" register-agent-session >/dev/null 2>&1 &&
    fail "SubagentStart unexpectedly registered"
after="$(shasum -a 256 "$REGISTRY")"
[ "$before" = "$after" ] || fail "rejected child input changed the registry"

echo "PASS: repo-local Codex hook registered one exact top-level session and rejected child input"
