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
printf 'selected checkout: %s\nagent prompt marker: %s\n' "$FQ" "$MARKER"
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
`WASM_URL`, and `SIDECAR_LOG`.

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

## 4. Capture the subscription proof

Return to the ordinary terminal. Wait for the real session to reach the served
API:

```bash
until curl -fsS "$AGENTSVIEW_URL/api/v1/sessions?include_one_shot=true&include_children=true&limit=1000" > /tmp/fq-agentsview-sessions.json && jq -e --arg cwd "$FQ" --arg marker "$MARKER" '[.sessions[] | select(.cwd == $cwd and ((.first_message // "") | startswith($marker)))] | length == 1' /tmp/fq-agentsview-sessions.json > /dev/null; do
  sleep 5
done
```

Capture the exact managed target:

```bash
TAB_ID="$(sed -n 's/^TAB_ID=//p' /tmp/fq-entry.out)"
WASM_URL="$(sed -n 's/^WASM_URL=//p' /tmp/fq-entry.out)"
SIDECAR_LOG="$(sed -n 's/^SIDECAR_LOG=//p' /tmp/fq-entry.out)"
test -n "$TAB_ID"
test -n "$WASM_URL"
test -n "$SIDECAR_LOG"
"$ZELLIJ_BIN" "${ZELLIJ_PROFILE_ARGS[@]}" --session WORK action list-panes --json --all --command --geometry --state --tab > /tmp/fq-panes.json
"$ZELLIJ_BIN" "${ZELLIJ_PROFILE_ARGS[@]}" --session WORK action list-tabs --json --all --state --layout > /tmp/fq-tabs.json
```

Verify one stable-ID resident, one active target tab, no startup refusal, and
unchanged standing KDL:

```bash
jq -e --arg id "$TAB_ID" --arg url "$WASM_URL" '[.[] | select((.tab_id | tostring) == $id and .is_plugin and .plugin_url == $url and (.is_floating | not) and (.is_suppressed | not))] | length == 1' /tmp/fq-panes.json > /dev/null
jq -e --arg id "$TAB_ID" '[.[] | select((.tab_id | tostring) == $id and .active)] | length == 1' /tmp/fq-tabs.json > /dev/null
jq -e --arg cwd "$FQ" --arg marker "$MARKER" '[.sessions[] | select(.cwd == $cwd and ((.first_message // "") | startswith($marker)))] | length == 1' /tmp/fq-agentsview-sessions.json > /dev/null
test -f "$SIDECAR_LOG"
! grep -qF 'connection refused' "$SIDECAR_LOG"
shasum -a 256 "$CONFIG_FILE" "$LAYOUT_FILE" > /tmp/fq-kdl-after.sha256
cmp /tmp/fq-kdl-before.sha256 /tmp/fq-kdl-after.sha256
```

Every command must exit zero. The captain must also report the visible
`AGENTS` header, `codex` row, and exact per-run marker in chat. The visible
rail and served API must identify the same real agent session. Reject the demo
if any condition fails.
