# Isolated Zellij smoke harness

Use this harness for live managed-tab checks without changing the operator's
standing Zellij files:

```bash
./tests/zellij-tmux-smoke-test.sh
```

The test creates a short root with isolated config, data, socket, permission,
tmux, and Zellij state. It drives the attached client with literal tmux keys
and checks visible output against native pane, tab, and layout records.

The test proves that:

- direct entry creates one fresh tab from the selected checkout and leaves
  standing KDL byte-identical;
- direct entry injects the watcher route but starts no helper;
- a command entered in the selected terminal starts `zaphod watch-tab`;
- an exact SessionStart and later AgentsView event render the expected row;
- `Alt /` changes only the managed rail's docked/sliver layout;
- `Alt /` in a foreign tab leaves its layout and pane identity unchanged;
- success, failure, interruption, and injected hangs remove the disposable
  Zellij session, tmux server, and temporary root.

Run both caller boundaries with:

```bash
./tests/zellij-watcher-lifecycle-smoke-test.sh
```

This wrapper runs the same journey first outside Zellij and then from a
loaded-client environment. The direct-entry script must clear the caller's
ambient client identity before it addresses the disposable server.

## Two-rail identity proof

```bash
./tests/zellij-two-rail-recipient-smoke-test.sh
```

The companion test creates two same-CWD tabs and starts one manual watcher
for each terminal. Both rails share a recipient token, so stable tab admission
must reject the foreign snapshot. The test requires `1/1/0` cardinality: one
top-level row in each tab and no child row.

It then clicks the target row from a same-CWD spare pane and requires focus on
the exact watched terminal. Stopping and restarting that watcher must produce
an empty projection until another SessionStart arrives. Closing the watched
terminal must clear the row while the spare terminal and original rail remain
alive. Silent terminal loss does not poll native pane state or promise
immediate daemon exit: the rail clears the unbound projection from its exact
manifest, and the harness explicitly terminates the owned daemon and verifies
socket cleanup.

The AgentsView fixture logs every request. Only exact
`/api/v1/sessions/{codex:<UUID>}` requests pass; list queries and child fetches
fail the test.

## Stress and cleanup evidence

```bash
./tests/zellij-watcher-layout-stress-test.sh
./tests/zellij-stress-evidence-test.sh
```

The stress wrapper repeats the outside/inside journey serially and
concurrently under owned deadlines. The evidence test injects failures and
slow native calls, then requires bounded logs plus proof that the disposable
session, tmux server, processes, sockets, and root were removed.

tmux supplies the terminal boundary. Zellij's visible output and native
actions supply the evidence; the harness uses no custom PTY controller.
