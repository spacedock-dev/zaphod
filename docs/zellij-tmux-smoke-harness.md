# Isolated Zellij smoke harness

Use this harness for the live Zellij gate. It proves the candidate layout and real key path without changing the operator's standing Zellij state.

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
ZELLIJ_SESSION_NAME="$session" zellij \
  --config-dir "$ZELLIJ_CONFIG_DIR" \
  --config "$ZELLIJ_CONFIG_FILE" \
  --data-dir "$ZELLIJ_DATA_DIR" \
  action list-panes --json -a -g -t

ZELLIJ_SESSION_NAME="$session" zellij \
  --config-dir "$ZELLIJ_CONFIG_DIR" \
  --config "$ZELLIJ_CONFIG_FILE" \
  --data-dir "$ZELLIJ_DATA_DIR" \
  action dump-layout
```

The generated persistent config must bind `Alt /` to `NoOp`, then bind `Alt Shift z`
to `NewTab { layout "zaphod"; }`. After the fresh rail appears, approve its
`Reconfigure` prompt. The rail may change only that attached client's runtime
`Alt /` binding to `MessagePluginId <resident-id>`; it must not write that
route to disk.

The smoke must prove all of the following with literal tmux keys and native
Zellij state:

- `Alt Shift z` creates one initialized Zaphod tab with the candidate WASM.
- `Alt /` changes the initialized rail's known swap state without changing its
  pane identity.
- From a sidebar-less foreign tab, `Alt /` leaves the tab, pane inventory,
  focus, and processes unchanged.
- A newly attached client with no initialized rail safely no-ops until its own
  rail installs the temporary route.

The test passes only when the screen, pane list, and dumped layout agree on the candidate WASM URL and expected tab behavior. It must also compare the standing `config.kdl` hash before and after the run.

## Cleanup

Always delete the disposable Zellij session, kill the dedicated tmux server, remove the temporary root, and then check the standing-config hash. Cleanup runs on success, failure, and interruption.

## Deliberate exclusions

Do not build a custom PTY controller. Do not add raw-input canaries, profile leases, process-group coordination, signal-monitor choreography, or readiness races. tmux supplies the terminal boundary; Zellij's visible output and native actions supply the evidence.
