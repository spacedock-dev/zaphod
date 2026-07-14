# Chat-guided exact AgentsView binding demo

This drill proves that rows follow explicit Codex SessionStart registrations,
not checkout history or CWD. It uses two fresh managed tabs with the same CWD.
Do not press `Alt /` or `Alt Shift z` during the identity drill.

The project-local hook in `.codex/hooks.json` calls this checkout's
`target/zaphod register-agent-session`. It records the authoritative Codex
`session_id` with inherited `ZELLIJ_SESSION_NAME` and `ZELLIJ_PANE_ID`.
It never edits global Codex configuration.

## 1. Prepare AgentsView and the selected checkout

Run these commands in a control terminal:

```bash
set -euo pipefail
FQ="$(git rev-parse --show-toplevel)"
AGENTSVIEW_URL=http://127.0.0.1:8080
agentsview serve status
until curl -fsS "$AGENTSVIEW_URL/api/v1/sessions?limit=1" >/dev/null; do
  sleep 2
done

CONFIG_ROOT="${ZELLIJ_CONFIG_DIR:-$HOME/.config/zellij}"
CONFIG_FILE="${ZELLIJ_CONFIG_FILE:-$CONFIG_ROOT/config.kdl}"
LAYOUT_FILE="$CONFIG_ROOT/layouts/zaphod.kdl"
if [ -n "${ZELLIJ_DATA_DIR:-}" ]; then
  DATA_DIR="$ZELLIJ_DATA_DIR"
elif [ "$(uname -s)" = Darwin ]; then
  DATA_DIR="$HOME/Library/Application Support/org.Zellij-Contributors.Zellij"
else
  DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/zellij"
fi
shasum -a 256 "$CONFIG_FILE" "$LAYOUT_FILE" > /tmp/kj-kdl-before.sha256
```

AgentsView must report a running server, not a resync or startup phase.

## 2. Create two same-CWD managed tabs

From the selected checkout, run the direct entry twice:

```bash
./scripts/zellij-new-tab.sh --session WORK --name 'KJ exact A'   --agentsview-url "$AGENTSVIEW_URL" | tee /tmp/kj-entry-a.out
./scripts/zellij-new-tab.sh --session WORK --name 'KJ exact B'   --agentsview-url "$AGENTSVIEW_URL" | tee /tmp/kj-entry-b.out
```

Each command must report a distinct `TAB_ID`, one `WASM_URL`, one
`SIDECAR_PID`, and one `SIDECAR_LOG`. Both sidecars may start with an empty
snapshot. An empty rail is correct until a trusted SessionStart registration
exists.

## 3. Trust the checkout hook once

In tab A, start `codex` and open `/hooks`. Review and trust the
project-local `SessionStart` command
`scripts/zaphod-codex-session-hook.sh`. Exit that Codex process after trust is
recorded. Repeat the trust check in tab B if Codex asks there.

The startup that displayed the trust prompt may have skipped the hook. Do not
use it as evidence. Start a new Codex process for the measured run.

## 4. Start one measured top-level session per tab

In tab A, start Codex and send:

```text
KJ_TAB_A_REAL: inspect README.md without edits, then wait.
```

In tab B, start Codex and send:

```text
KJ_TAB_B_REAL: inspect README.md without edits, then wait.
```

Ask the tab-B Codex process to spawn one subagent that replies with a short
marker. The child creates AgentsView history but no top-level SessionStart
registration.

The rails must settle to these cardinalities:

- tab A: exactly one top-level row, `KJ_TAB_A_REAL`;
- tab B: exactly one top-level row, `KJ_TAB_B_REAL`;
- both rails combined: zero child rows and zero older same-CWD history rows.

A row in both tabs, an `unbound` row, or any child row fails the drill.

## 5. Verify registry and API identity

Derive the private registry root with the same rule as the native binary:

```bash
if [ -n "${ZAPHOD_REGISTRY_DIR:-}" ]; then
  REGISTRY_DIR="$ZAPHOD_REGISTRY_DIR"
elif [ -n "${XDG_RUNTIME_DIR:-}" ]; then
  REGISTRY_DIR="$XDG_RUNTIME_DIR/zaphod/agent-sessions-v1"
else
  REGISTRY_DIR="${TMPDIR:-/tmp}/zaphod-agent-sessions-v1-$(id -u)"
fi
find "$REGISTRY_DIR" -maxdepth 1 -name 'session-*.json' -print
```

Inspect the record whose `zellij_session` is `WORK`:

```bash
jq . "$REGISTRY_DIR"/session-*.json
```

It must contain one record for each measured top-level pane. For each record,
the following relation must hold exactly:

```text
agentsview_session_id == "codex:" + agent_session_id
```

Fetch each `agentsview_session_id` through
`/api/v1/sessions/{id}`. The returned `id` must equal the requested ID.
Do not substitute a list query, CWD match, newest session, title, prompt, or
timestamp.

## 6. Prove direct focus and replacement

Click tab A's session row. Zellij must focus tab A's registered terminal pane.
Click tab B's row; it must focus tab B's registered terminal pane.

Exit the top-level Codex process in tab A, then start a new top-level Codex
session in the same pane with marker `KJ_TAB_A_REPLACEMENT`. The new
SessionStart must replace the old record. Tab A must show one replacement row,
not two. A Codex `Stop` between prompts must retain the current mapping.

If the operator's Zellij profile exposes a native move-to-tab action, move one
registered terminal between the two managed tabs. During convergence, the row
may appear in zero or one rail; it must never appear in both. It must settle in
the rail that owns the pane's current native tab. Skip this manual move when
the profile has no such action; the disposable native smoke covers membership
and stale-delivery barriers without changing the operator's keymap.

## 7. Prove sidecar rehydration

Reconstruct tab A's private invocation from the entry output, then terminate
and restart only its sidecar:

```bash
TAB_A="$(sed -n 's/^TAB_ID=//p' /tmp/kj-entry-a.out)"
WASM_A="$(sed -n 's/^WASM_URL=//p' /tmp/kj-entry-a.out)"
PID_A="$(sed -n 's/^SIDECAR_PID=//p' /tmp/kj-entry-a.out)"
TOKEN_A="$(sed -n 's/^RECIPIENT_TOKEN=//p' /tmp/kj-entry-a.out)"
REGISTRY_A="$(sed -n 's/^REGISTRY_DIR=//p' /tmp/kj-entry-a.out)"
kill -TERM "$PID_A"

nohup "$FQ/target/zaphod" subscribe \
  --server "$AGENTSVIEW_URL" \
  --zellij-bin "${ZELLIJ_BIN:-zellij}" \
  --zellij-config-dir "$CONFIG_ROOT" \
  --zellij-config "$CONFIG_FILE" \
  --zellij-data-dir "$DATA_DIR" \
  --zellij-session WORK \
  --tab-id "$TAB_A" \
  --rail-url "$WASM_A" \
  --checkout-cwd "$FQ" \
  --recipient-token "$TOKEN_A" \
  --registry-dir "$REGISTRY_A" \
  >/tmp/kj-sidecar-a-restart.log 2>&1 &
RESTARTED_SIDECAR_A=$!
```

The surviving registry entry must restore exactly one tab-A row without
another prompt or hook event. The restart log must stay empty, and
`kill -0 "$RESTARTED_SIDECAR_A"` must succeed.

For the repeatable, fully isolated version, run:

```bash
./tests/zellij-two-rail-recipient-smoke-test.sh
```

That test creates two same-CWD tabs, registers two exact top-level IDs, exposes
one unregistered child, asserts `1/1/0`, clears one snapshot, restarts its
sidecar, and proves one-row rehydration. It then closes the registered native
terminal and proves the surviving rail prunes the claim and renders zero
stale rows.

## 8. Confirm cleanup boundaries

Back in the control terminal:

```bash
SIDECAR_A="$(sed -n 's/^SIDECAR_PID=//p' /tmp/kj-entry-a.out)"
SIDECAR_B="$(sed -n 's/^SIDECAR_PID=//p' /tmp/kj-entry-b.out)"
kill -0 "$SIDECAR_A"
kill -0 "$SIDECAR_B"
shasum -a 256 "$CONFIG_FILE" "$LAYOUT_FILE" > /tmp/kj-kdl-after.sha256
cmp /tmp/kj-kdl-before.sha256 /tmp/kj-kdl-after.sha256
```

The drill fails on a guessed row, duplicate row, child row, mismatched exact
ID, stale focus action, orphan helper, or standing KDL change.
