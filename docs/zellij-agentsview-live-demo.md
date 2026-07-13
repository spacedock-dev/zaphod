# Chat-guided AgentsView live demo

Use this packet to prove the direct selected-checkout journey against a real
AgentsView-backed agent session. The first officer sends one step at a time in
chat; the captain does not need clipboard support in a Subspace TUI float.

This is not a keybinding drill. Do not press `Alt /` or `Alt Shift z`. Do not
run `target/zaphod subscribe` by hand. The direct script starts the one private
sidecar after it creates and verifies the fresh tab. No step writes standing
`config.kdl` or `layouts/zaphod.kdl`.

## 1. Wait for the real AgentsView API

Run these commands in an ordinary control terminal outside the attached
`WORK` client, not inside the Subspace review float. Keep this terminal open;
the direct command targets `WORK` remotely, so the same terminal remains
available for the final checks. Open it in the selected checkout, then derive
all paths for this run:

```bash
set -euo pipefail
FQ="$(git rev-parse --show-toplevel)"
AGENTSVIEW_BIN="$(command -v agentsview)"
AGENTSVIEW_URL=http://127.0.0.1:8080
MARKER="fq-$(date +%s)"
SSE_MARKER="${MARKER}-sse"
printf 'selected checkout: %s\ninitial marker: %s\nSSE marker: %s\n' "$FQ" "$MARKER" "$SSE_MARKER"
```

Check the real server. Start it only if no server is running:

```bash
"$AGENTSVIEW_BIN" serve status || "$AGENTSVIEW_BIN" serve --background --no-browser
```

`serve status` may report a startup phase such as `full resync`. Do not run the
Zaphod entry command during that phase. Wait until the HTTP sessions endpoint
responds:

```bash
until curl -fsS "$AGENTSVIEW_URL/api/v1/sessions?include_one_shot=true&include_children=true&limit=1" > /tmp/fq-agentsview-ready.json; do
  "$AGENTSVIEW_BIN" serve status
  sleep 5
done
jq -e '.sessions | type == "array"' /tmp/fq-agentsview-ready.json
```

The `jq` command must print `true`. A connection refusal means AgentsView is
still starting; keep waiting. The Zaphod sidecar deliberately exits on its
first source failure and has no reconnect loop.

## 2. Create the direct-entry tab

Still in the ordinary terminal, record standing hashes and run the direct
script:

```bash
cd "$FQ"
CONFIG_ROOT="${ZELLIJ_CONFIG_DIR:-$HOME/.config/zellij}"
CONFIG_FILE="${ZELLIJ_CONFIG_FILE:-$CONFIG_ROOT/config.kdl}"
LAYOUT_FILE="$CONFIG_ROOT/layouts/zaphod.kdl"
ZELLIJ_BIN="${ZELLIJ_BIN:-zellij}"
if [ -n "${ZELLIJ_DATA_DIR:-}" ]; then
  DATA_DIR="$ZELLIJ_DATA_DIR"
elif [ "$(uname -s)" = Darwin ]; then
  DATA_DIR="$HOME/Library/Application Support/org.Zellij-Contributors.Zellij"
else
  DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/zellij"
fi
ZELLIJ_PROFILE_ARGS=(--config-dir "$CONFIG_ROOT" --config "$CONFIG_FILE" --data-dir "$DATA_DIR")
shasum -a 256 "$CONFIG_FILE" "$LAYOUT_FILE" > /tmp/fq-kdl-before.sha256
./scripts/zellij-new-tab.sh --session WORK --name 'Zaphod fq AgentsView drill' --agentsview-url "$AGENTSVIEW_URL" | tee /tmp/fq-entry.out
```

Do not start another sidecar. The entry output must contain `TAB_ID`,
`WASM_URL`, `SIDECAR_LOG`, and `SIDECAR_PID`.

## 3. Create one real agent session

The direct command activates a fresh managed tab. In that tab's shell pane,
run `pwd`. It must print the selected checkout path shown in step 1. If it
does not, the first officer supplies that derived path for a short `cd`
command. Then type:

```bash
codex
```

At the Codex prompt, send this short request, replacing `<MARKER>` with the
exact marker printed in step 1:

```text
<MARKER>: inspect README.md without edits, then wait for my next instruction.
```

Keep the agent session open. AgentsView must index a session whose `cwd` is
the selected checkout and whose first message starts with the unique marker.

The rail's terminal-pane status line and its subscription section are separate
signals. A shell pane may show `unknown . unknown`; that line does not prove or
disprove subscription. The required visible proof is a separate `AGENTS`
section with a `codex` session row and the exact marker in its summary. The
captain records that visual observation in chat; Zellij 0.44.x does not expose
plugin-pane rendering through `dump-screen`.

## 4. Establish the initial row

Return to the ordinary terminal. Wait for the real session to reach the served
API:

```bash
until curl -fsS "$AGENTSVIEW_URL/api/v1/sessions?include_one_shot=true&include_children=true&limit=1000" > /tmp/fq-agentsview-sessions.json && jq -e --arg cwd "$FQ" --arg marker "$MARKER" '[.sessions[] | select(.cwd == $cwd and ((.first_message // "") | startswith($marker)))] | length == 1' /tmp/fq-agentsview-sessions.json > /dev/null; do
  sleep 5
done
```

The captain must now report a visible `AGENTS` header and `codex` row with the
exact initial marker. This is only the baseline: the sidecar may have emitted
it during its initial HTTP refresh, so it does not yet prove the SSE stream.

## 5. Prove a post-baseline SSE refresh

Return to the first Codex session, enter `/exit`, and wait for the shell prompt.
Start `codex` again in the same pane and selected checkout. At its prompt, send
this request, replacing `<SSE_MARKER>` with the exact SSE marker from step 1:

```text
<SSE_MARKER>: inspect README.md without edits, then wait for my next instruction.
```

This second session is created only after the first row is visible. Return to
the ordinary terminal and require AgentsView to serve exactly one session for
each marker:

```bash
until curl -fsS "$AGENTSVIEW_URL/api/v1/sessions?include_one_shot=true&include_children=true&limit=1000" > /tmp/fq-agentsview-sessions.json && jq -e --arg cwd "$FQ" --arg first "$MARKER" --arg second "$SSE_MARKER" '([.sessions[] | select(.cwd == $cwd and ((.first_message // "") | startswith($first)))] | length == 1) and ([.sessions[] | select(.cwd == $cwd and ((.first_message // "") | startswith($second)))] | length == 1)' /tmp/fq-agentsview-sessions.json > /dev/null; do
  sleep 5
done
```

The captain must now report a second visible `codex` row with the exact SSE
marker. Because that session did not exist when the initial row was visible,
its appearance in the rail proves that the sidecar consumed a later
`data_changed` event and refreshed from the persistent SSE stream.

## 6. Capture the final proof

Capture the exact managed target:

```bash
TAB_ID="$(sed -n 's/^TAB_ID=//p' /tmp/fq-entry.out)"
WASM_URL="$(sed -n 's/^WASM_URL=//p' /tmp/fq-entry.out)"
SIDECAR_LOG="$(sed -n 's/^SIDECAR_LOG=//p' /tmp/fq-entry.out)"
SIDECAR_PID="$(sed -n 's/^SIDECAR_PID=//p' /tmp/fq-entry.out)"
test -n "$TAB_ID"
test -n "$WASM_URL"
test -n "$SIDECAR_LOG"
[[ "$SIDECAR_PID" =~ ^[1-9][0-9]*$ ]]
"$ZELLIJ_BIN" "${ZELLIJ_PROFILE_ARGS[@]}" --session WORK action list-panes --json --all --command --geometry --state --tab > /tmp/fq-panes.json
"$ZELLIJ_BIN" "${ZELLIJ_PROFILE_ARGS[@]}" --session WORK action list-tabs --json --all --state --layout > /tmp/fq-tabs.json
```

Verify one stable-ID resident, one active target tab, both real sessions, a
still-live sidecar with no terminal diagnostics, and unchanged standing KDL:

```bash
jq -e --arg id "$TAB_ID" --arg url "$WASM_URL" '[.[] | select((.tab_id | tostring) == $id and .is_plugin and .plugin_url == $url and (.is_floating | not) and (.is_suppressed | not))] | length == 1' /tmp/fq-panes.json > /dev/null
jq -e --arg id "$TAB_ID" '[.[] | select((.tab_id | tostring) == $id and .active)] | length == 1' /tmp/fq-tabs.json > /dev/null
jq -e --arg cwd "$FQ" --arg first "$MARKER" --arg second "$SSE_MARKER" '([.sessions[] | select(.cwd == $cwd and ((.first_message // "") | startswith($first)))] | length == 1) and ([.sessions[] | select(.cwd == $cwd and ((.first_message // "") | startswith($second)))] | length == 1)' /tmp/fq-agentsview-sessions.json > /dev/null
test -f "$SIDECAR_LOG"
test ! -s "$SIDECAR_LOG"
kill -0 "$SIDECAR_PID"
shasum -a 256 "$CONFIG_FILE" "$LAYOUT_FILE" > /tmp/fq-kdl-after.sha256
cmp /tmp/fq-kdl-before.sha256 /tmp/fq-kdl-after.sha256
```

Every command must exit zero. The captain must also report the visible
`AGENTS` header and both distinct `codex` rows with their exact per-run markers
in chat. The second row is the required SSE refresh evidence. The visible rail
and served API must identify the same two real agent sessions. Reject the demo
if any condition fails.
