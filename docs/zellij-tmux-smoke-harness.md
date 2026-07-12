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
default config directory, not necessarily the selected isolated root. After
the fresh rail appears, approve its `Reconfigure` prompt. The rail may change
only that attached client's runtime `Alt /` binding to `MessagePluginId
<resident-id>`; it must not write that route to disk.

The smoke script proves the following with literal tmux keys and native Zellij
state:

- `Alt Shift z` creates one initialized Zaphod tab with the candidate WASM.
- From a sidebar-less foreign tab, `Alt /` leaves the tab, pane inventory,
  focus, and processes unchanged.

The script deliberately does not auto-approve plugin permissions or attach a
second client. The Rust tests prove that a pre-grant pipe and an unrouted
header click are inert. A live two-client route-isolation drill remains manual:
attach the second client, do not approve its rail, and confirm `Alt /` leaves
its native pane state unchanged.

The test passes only when the screen, pane list, and dumped layout agree on the candidate WASM URL and expected tab behavior. It must also compare the standing `config.kdl` hash before and after the run.

## Cleanup

Always delete the disposable Zellij session, kill the dedicated tmux server, remove the temporary root, and then check the standing-config hash. Cleanup runs on success, failure, and interruption.

## Deliberate exclusions

Do not build a custom PTY controller. Do not add raw-input canaries, profile leases, process-group coordination, signal-monitor choreography, or readiness races. tmux supplies the terminal boundary; Zellij's visible output and native actions supply the evidence.
