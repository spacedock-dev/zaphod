# Isolated Zellij smoke harness

Use this harness for the live Zellij gate. It proves the candidate layout and
real key path without changing the operator's standing Zellij state:

```bash
./tests/zellij-tmux-smoke-test.sh
```

## Contract

Create one short-lived root with separate `config`, `data`, and `socket` directories. Render the candidate layout and config inside that root. Start a dedicated tmux server with a unique `-L` name, then launch Zellij inside one tmux pane with these values:

```text
ZELLIJ_CONFIG_DIR=<root>/config
ZELLIJ_CONFIG_FILE=<root>/config/config.kdl
ZELLIJ_DATA_DIR=<root>/data
ZELLIJ_SOCKET_DIR=<root>/socket
```

The harness drives the attached Zellij client with literal bytes only:

```sh
tmux -L "$tmux_server" send-keys -l -t "$pane" -- "$bytes"
tmux -L "$tmux_server" capture-pane -p -t "$pane"
```

Use `capture-pane` to assert the visible result. Use Zellij's native state separately to assert the machine-readable result:

```sh
zellij --session "$session" \
  --config-dir "$ZELLIJ_CONFIG_DIR" \
  --config "$ZELLIJ_CONFIG_FILE" \
  --data-dir "$ZELLIJ_DATA_DIR" \
  action list-panes --json -a -g -t

zellij --session "$session" \
  --config-dir "$ZELLIJ_CONFIG_DIR" \
  --config "$ZELLIJ_CONFIG_FILE" \
  --data-dir "$ZELLIJ_DATA_DIR" \
  action dump-layout
```

The isolated persistent config contains valid quoted-brace KDL, an existing
`Alt /` policy, and an unrelated `Alt Shift z` native `NewTab` shortcut for one
fixed layout. The selected-checkout script validates but never rewrites that
config or its sentinel `layouts/zaphod.kdl`; it passes its repository-owned
layout directly through `new-tab --layout-string`. A tiled rail requests a
runtime-only `MessagePluginId <resident-id>` route after its ordinary
`Reconfigure` permission. That request has no acknowledgement: a received
literal keybind pipe at the active tiled resident—not a local "installed"
boolean—settles that the route is usable. The route is never written to
persistent config.

The smoke script proves the following with literal tmux keys and native Zellij
state:

- The direct script creates exactly one initialized Zaphod tab with the
  candidate WASM at its returned stable tab ID, confirmed in the live pane/tab
  inventory and dumped layout. The fixed global `Alt Shift z` route is left
  byte-identical and is not used as selected-checkout evidence.
- One literal `Alt /` in that tab moves the candidate rail from its known
  28-column docked shape to the 1-column sliver, while native pane identity,
  command, focus, and candidate URL remain unchanged.
- After that observed route, a native tab switch returns the same tmux client
  to a sidebar-less foreign tab. Literal `Alt /` leaves its pane inventory,
  focus, layout, and candidate count byte-identical.

Run the permission-cache upgrade boundary with:

```bash
ZAPHOD_PERMISSION_FIXTURE=upgrade ./tests/zellij-tmux-smoke-test.sh
```

This mode seeds the old grant set without `ReadCliPipes`, waits until the
native prompt is visible in the focused candidate pane, and sends one literal
`y`. It then requires the persisted `ReadCliPipes` grant and the same session
row, layout, toggle, and cleanup checks as the pre-granted run.

## Tab-recipient smoke

Run the companion two-rail check with:

```bash
./tests/zellij-two-rail-recipient-smoke-test.sh
```

It creates two same-CWD rails in one isolated Zellij session. Deterministic
Codex-schema SessionStart fixtures register distinct provider IDs with the two
live terminal pane IDs. An AgentsView-compatible server exposes those two
top-level records plus one unregistered child and logs every request.

Two private sidecars deliberately share one recipient token, so both rails
receive each named-pipe broadcast and stable-tab admission—not channel
isolation—must reject the foreign snapshot. They produce row cardinality
`1/1/0`: each rail shows only its registered top-level session, and neither
rail shows the child. A literal mouse click on the target row must focus its
registered pane rather than the same-CWD spare. The request log contains only
the two exact
`/api/v1/sessions/{codex:<UUID>}` lookups; a global list request fails the
test. The harness then clears one authoritative snapshot, restarts that
sidecar, and requires one-row rehydration without another registration. This
proves stable recipient routing, exact pane membership and focus, child
exclusion, snapshot removal, and restart behavior through real Zellij named pipes. It
then closes the registered native terminal while leaving a spare terminal and
resident rail alive; the next sidecar generation must prune the claim and
render zero stale rows.

Go loopback tests separately hold the SSE stream quiet while committing a
late registration, turning an exact 404 into an indexed record, and moving a
registered pane. The bounded membership refresh must project, recover, or
remove the row without a `data_changed` event.

For headless coverage, the script gives Zellij a disposable `HOME` and writes
a deliberate pre-grant to its temporary permission cache. The cache key is the
raw WASM path, not the `file:` URL rendered into layouts. This is a fixture,
not injected consent: the normal attached-client drill still shows and requires
the ordinary prompt. The script does not attach a second client; same-tab
second-client delivery is a named follow-up, not a Sprint 1 gate.

The test passes only when the screen, pane list, tab inventory, and dumped
layout agree on the candidate WASM URL and expected state transition. It
compares standing `config.kdl` and `layouts/zaphod.kdl` hashes before and after
the run, deletes the isolated Zellij session and tmux server, and removes the
temporary root on every exit path.

## Cleanup

Always delete the disposable Zellij session, kill the dedicated tmux server, remove the temporary root, and then check the standing-config hash. Cleanup runs on success, failure, and interruption.

## Deliberate exclusions

Do not build a custom PTY controller. Do not add raw-input canaries, profile leases, process-group coordination, signal-monitor choreography, or readiness races. tmux supplies the terminal boundary; Zellij's visible output and native actions supply the evidence.
