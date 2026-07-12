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

The generated persistent config binds `Alt /` to `NoOp`, then binds `Alt Shift z`
to native `NewTab` with the selected `layouts/zaphod.kdl` **absolute path**.
Do not use `layout "zaphod"`: Zellij resolves that named form from its standing
default config directory, not necessarily the selected isolated root. A tiled
rail requests a runtime-only `MessagePluginId <resident-id>` route after its
ordinary `Reconfigure` permission. That request has no acknowledgement: a
received literal keybind pipe at the active tiled resident—not a local
"installed" boolean—settles that the route is usable. The route is never
written to persistent config.

The smoke script proves the following with literal tmux keys and native Zellij
state:

- `Alt Shift z` creates exactly one initialized Zaphod tab with the candidate
  WASM, confirmed in the live tab inventory and dumped layout.
- One literal `Alt /` in that tab moves the candidate rail from its known
  28-column docked shape to the 1-column sliver, while native pane identity,
  command, focus, and candidate URL remain unchanged.
- After that observed route, a native tab switch returns the same tmux client
  to a sidebar-less foreign tab. Literal `Alt /` leaves its pane inventory,
  focus, layout, and candidate count byte-identical.

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
