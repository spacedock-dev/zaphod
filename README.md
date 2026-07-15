# zaphod

A per-tab pane-switcher sidebar for [zellij](https://zellij.dev), built for
stacked-pane workflows where many terminal agents (Claude Code, codex, …) run
side by side. It answers one question at a glance: **what is running in this
tab and what does it want from me?**

```
┌ sidebar ──────────────┐
│▾ PANES             ⇄ │
│  codex literature   │   ← known agent from command/title
│    unknown . codex  │   ← fresh pane has no viewport status
│  claude planner     │   ← title classification still works
│    unknown . claude │
│  clkao@mac:~/git/x    │
│    unknown . unknown  │
└───────────────────────┘
```

## Features

- Lists the current tab's terminal panes; **click a row to focus that pane**
- The sidebar is unfocusable (tab-bar mechanism): clicks are delivered
  without focusing it, so it never steals your keyboard
- The periodic refresh uses best-effort command/CWD metadata and preserves the last known status when live viewport data is unavailable.
- Plugin-only command/title classification identifies Claude, Codex, and Pi;
  a new unresolved shell pane appears as `unknown . unknown`.
- Terminal pane rows classify command/title identity only. Their state and
  status remain stale when previously known and start unknown when unavailable;
  AgentsView session rows separately carry externally supplied workflow state.
- **Keyboard navigation mode**: `j/k`/arrows move a highlight, `Enter` jumps,
  `Esc` returns focus where it was
- `Alt /` (or the `⇄` header) toggles the docked 28-col rail down to a
  1-col sliver and back by cycling the tab's swap layouts — panes are
  rearranged in place, never spawned or hidden, and a manually re-split tab
  keeps its arrangement (the swap set is regenerated from the live layout)
- Per-tab instances that toggle independently
- **Tab-bound session rows**: `scripts/zellij-new-tab.sh` creates a fresh
  managed tab and injects an exact watcher route. The operator runs
  `target/zaphod watch-tab` in the selected terminal. A trusted SessionStart
  hook sends the exact Codex session ID through that terminal's private
  socket. The watcher fetches only that AgentsView ID, and the rail focuses
  only that live pane. CWD, titles, prompts, timestamps, history, and child
  labels never authorize a row.
- **Gate rows**: the rail can display pending decisions and float
  `subspace-tui` on a gate artifact with `--log` pointed at its decision log.
  Gate delivery is separate from the tab-local watcher.
  Floating the review tool requires the `RunCommands` permission (prompted
  once).

## Build

Requires a Rust toolchain with the `wasm32-wasip1` target
(`rustup target add wasm32-wasip1`).

```bash
./build.sh
# → target/wasm32-wasip1/release/zellij-sidebar.wasm
# → target/zaphod
```

`build.sh` pins rustup's `rustc` explicitly because a homebrew Rust earlier
on `PATH` lacks the wasm std and fails with `can't find crate for core`.
The second artifact is the checkout-local manual watcher and hook receiver.

## Usage

### Canonical global install

Build and install only from the primary checkout:

```bash
./build.sh
./install.sh
```

The installer derives the primary checkout from Git's common directory and
refuses linked worktrees, even when `ZELLIJ_CONFIG_DIR` points to a writable
destination. It also requires every remaining Zaphod `MessagePlugin` in the
effective `config.kdl` to use the primary checkout's canonical WASM URL and
`rail "1"`; on a mismatch, it reports each offending URL and writes nothing.
It never rewrites keybinds. Configure persistent `Alt /` as `NoOp` before
running it.

For the one-time cleanup, build the primary checkout and point the Zaphod
navigation route at
`file:<PRIMARY_CHECKOUT>/target/wasm32-wasip1/release/zellij-sidebar.wasm`.
Each message block must include `rail "1"`; leave `Alt /` fail-closed. This is
the reference shape, not a worktree-install recipe:

```kdl
keybinds {
    shared {
        bind "Alt /" { NoOp; }
        bind "Alt ." {
            MessagePlugin "file:<PRIMARY_CHECKOUT>/target/wasm32-wasip1/release/zellij-sidebar.wasm" {
                name "navigate"
                floating true
                skip_cache true
                rail "1"
            }
        }
        bind "Alt Shift z" {
            NewTab { layout "<ABSOLUTE_ZELLIJ_CONFIG_ROOT>/layouts/zaphod.kdl"; }
        }
    }
}
```

After cleanup, rerun `./install.sh`. The installer renders and parses the
layout in a disposable Zellij 0.44.3 session, renames it atomically, and
rechecks identity. A failed postflight restores the previous layout bytes.
Use live state—not the rendered file—as the final oracle:

```bash
zellij --session <session> action list-panes --json -a -g -t
zellij --session <session> action dump-layout
```

On an initialized Zaphod tab, exactly one sidebar should remain before and
after `Alt /`; every sidebar URL in the dump must name the primary checkout
artifact.

### Create a fresh managed tab

To build the selected checkout and create a fresh tab in an existing session,
run:

```bash
./scripts/zellij-new-tab.sh --session WORK
```

The command validates the selected profile, builds this checkout, renders its
canonical WASM URL into a disposable inline layout, and creates one new tab.
It leaves standing `config.kdl` and `layouts/zaphod.kdl` bytes unchanged. It
then verifies one tiled rail at the new stable tab ID. A missing or ambiguous
rail reports `rail-target-unready` and leaves the new tab available for
inspection.

Direct entry starts no watcher. It injects the exact AgentsView URL, rail URL,
recipient token, Zellij profile, binary, and private socket root into the new
terminal. In that terminal, run:

```bash
./target/zaphod watch-tab
```

The command uses one native inventory to prove the terminal, stable tab, and
original rail, then returns after its background watcher proves the AgentsView
event stream, recipient, socket, and initial empty lease. It prints the watcher
PID and log path. Start AgentsView before this command. The default endpoint is
`http://127.0.0.1:8080`; direct entry's `--agentsview-url URL` selects another
endpoint.

The trusted `.codex/hooks.json` command invokes
`target/zaphod register-agent-session`. It accepts only Codex `SessionStart`
events from `startup` or `resume` and sends one bounded record to the live
socket derived from `ZELLIJ_SESSION_NAME` and `ZELLIJ_PANE_ID`. Outside
Zellij, the hook exits successfully before it requires a built receiver.

The watcher keeps one registration in memory. It fetches only
`/api/v1/sessions/{codex:<SessionStart session_id>}` and sends leased snapshots
to the exact recipient tab. Recipient- and generation-checked heartbeats renew
the cached projection every 1.4 seconds without waiting for plugin output;
new hook records and AgentsView data changes trigger exact fetches and
acknowledged snapshots. A later SessionStart in the same terminal replaces
the old row. Restart starts empty and requires a new SessionStart.

Socket loss, watcher loss, terminal/tab/rail manifest loss, source failure, or
rejected delivery makes the row and focus fail closed through the plugin's
current manifest and lease. After ready, SessionStart/data-change delivery,
heartbeats, and cleanup make zero native pane-inventory or cleanup-probe calls.
A watcher whose pane or rail disappears silently may remain until explicit
manual cleanup. It never follows a replacement pane or rail, consults the
global session list, writes durable session authority, or guesses from CWD.
Follow the [chat-guided AgentsView demo](docs/zellij-agentsview-live-demo.md)
for the two-tab journey.

`Alt Shift z` remains a separately configured tab-only shortcut. It opens the
fixed layout named by the operator's global config; it does not select a
checkout or launch a watcher. Zellij pipes remain session-wide broadcasts,
so every snapshot carries the recipient token, stable tab ID, watcher
generation, and lease.

When a tiled Zaphod rail is visible, approve its `Reconfigure` permission.
The rail requests a temporary runtime `Alt /` route to its own already-running
plugin; the persistent binding remains `NoOp`. `reconfigure()` has no
acknowledgement, so only a received literal keybind pipe at the active tiled
rail is allowed to toggle the docked/sliver layout. Sprint 1 proves this
ordinary journey for one attached client; second-client delivery is a named
follow-up. The separately tracked `v3` hardening task must still prove that a
tiled sidebar-shaped unmanaged resident cannot qualify for that route; visual
shape or a URL substring is not managed ownership.

Use `ZELLIJ_CONFIG_DIR`, `ZELLIJ_CONFIG_FILE`, and `ZELLIJ_DATA_DIR` to run it
against an isolated profile. The command creates its inline tab at once and
does not install or update a native keybind.

Run the real-key boundary with:

```bash
./tests/zellij-tmux-smoke-test.sh
./tests/zellij-watcher-lifecycle-smoke-test.sh
./tests/zellij-two-rail-recipient-smoke-test.sh
```

They use the [isolated tmux smoke harness](docs/zellij-tmux-smoke-harness.md).
The two-rail test creates two same-CWD tabs, starts one watcher per terminal,
and exposes one unregistered child. It proves `1/1/0` cardinality, exact-ID
requests, direct focus, restart-empty behavior, lease expiry, and failure on
terminal loss.

### Historical worktree profile

`scripts/zellij-worktree-test-profile.sh` is parked experimental evidence. It
is not the candidate test path and does not define `Alt /` safety. Use the
[isolated tmux smoke harness](docs/zellij-tmux-smoke-harness.md) for every
real-key candidate check.

### Permissions

Tokenless installed-layout rails request `ReadApplicationState`,
`ChangeApplicationState`, `ReadPaneContents`, `Reconfigure`, and `RunCommands`.
A token-bound direct-entry rail also requests `ReadCliPipes` for its private
watcher. On the first request or grant expansion, focus the pane and
approve its native prompt once.
Zellij's grant cache is keyed by the raw WASM path. By default, the smoke
harness redirects `HOME` to a temporary root and uses a pre-granted fixture.
It never writes the operator's cache. `ReadCliPipes` carries the watcher's
private readiness probes, leased snapshots, and acknowledgments.
`Reconfigure` changes only runtime keybinds; Zaphod does not save that route
to disk. `RunCommands` is required only when a gate row floats `subspace-tui`.

## Status

Working prototype (zellij 0.44.3): per-tab toggle, click/keyboard switching,
plugin-local agent awareness, tab-bound session rows from the direct script,
exact SessionStart-to-pane admission, state/status lines, and docked/sliver toggle. Use
`scripts/zellij-new-tab.sh` to create a rail from a selected checkout. A
separately installed `Alt Shift z` binding opens only its fixed configured
layout and does not select a checkout or launch a watcher. `Alt /` never
creates or retrofits a tab.

[SPEC.md](SPEC.md) records the shipped prototype and its numbered Zellij
plugin landmines, including the historical rebuild guidance. For the evergreen
product direction—one managed tab or window, portable dock, workspace hub,
and multiplexer drivers—see
[docs/zaphod-workspace-architecture.md](docs/zaphod-workspace-architecture.md).

## Development

- `cargo test` — pure-function tests (row building, click mapping, toggle
  decisions); a host-side stub satisfies the wasm import at link time
- The zellij server log is the oracle:
  `$TMPDIR/zellij-<uid>/zellij-log/zellij.log` (permission denials, real
  compiles vs cache hits, wasm crashes)
- Headless bench: `zellij attach bench --create-background`, then drive it
  with `zellij --session bench action …`
