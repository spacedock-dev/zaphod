# zaphod

A per-tab pane-switcher sidebar for [zellij](https://zellij.dev), built for
stacked-pane workflows where many terminal agents (Claude Code, codex, …) run
side by side. It answers one question at a glance: **what is running in this
tab and what does it want from me?**

```
┌ sidebar ──────────────┐
│▾ PANES             ⇄ │
│● codex literature   │   ← blocked/working agent state marker
│    blocked . codex  │   ← state, agent, and latest prompt/status
│✓ claude planner     │   ← idle known agent
│    idle . claude    │
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
- Blocked prompts outrank working prompts, with state markers in the first
  line and details in the dimmed second line
- **Keyboard navigation mode**: `j/k`/arrows move a highlight, `Enter` jumps,
  `Esc` returns focus where it was
- `Alt /` (or the `⇄` header) toggles the docked 28-col rail down to a
  1-col sliver and back by cycling the tab's swap layouts — panes are
  rearranged in place, never spawned or hidden, and a manually re-split tab
  keeps its arrangement (the swap set is regenerated from the live layout)
- Per-tab instances that toggle independently
- **Tab-bound session rows**: `scripts/zellij-new-tab.sh` creates a fresh
  managed tab, verifies its resident rail, and privately starts the
  checkout-local subscriber. It projects AgentsView session state (blocked /
  working / idle / done) into that rail only; a click focuses one exact
  cwd-bound terminal and leaves zero or multiple matches unbound.
- **Gate rows**: the rail can display pending decisions and float
  `subspace-tui` on a gate artifact with `--log` pointed at its decision log.
  Gate delivery is separate from the first tab-bound session subscriber.
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
The second artifact is an internal native sidecar; it is not an installed
command or a second launcher.

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

The command asks Zellij to validate the selected profile, builds this checkout,
renders its canonical WASM URL into a disposable inline layout, and creates
exactly one new tab. It does not parse, rewrite, stage, or restore the standing
`config.kdl` or `layouts/zaphod.kdl`; failures and interruption leave both
byte-identical. Persistent key policy, including `Alt /` and `Alt .`, remains
global profile setup; the direct command neither validates nor retargets those
routes and adds no runtime keybinding. A global `Alt .` route may therefore
remain tied to its fixed installed plugin; it is not a selected-checkout entry
guarantee. The direct command guarantees the fresh tab and private session-row
subscriber described below. It never changes an existing tab.

This direct command is also the current session-row entry point. After
`new-tab` returns, it waits for native `list-panes` state to show exactly one
tiled, non-suppressed Zaphod rail with the returned stable tab ID and this
checkout's canonical WASM URL. Only then does it start one private
`target/zaphod subscribe` process with the same Zellij profile and session.
The inline rail and sidecar also share a fresh per-entry recipient token, so a
second rail in the same stable tab cannot acknowledge or receive its rows.
The sidecar reads AgentsView from `http://127.0.0.1:8080` by default; pass
`--agentsview-url URL` or set `ZAPHOD_AGENTSVIEW_URL` to use another endpoint.
The startup handshake allows 30 seconds for the sidecar to verify the exact
stable-tab target, establish a correctly typed AgentsView SSE response that
remains open through a short stability probe, and deliver one acknowledged
initial snapshot. Changes arriving during that work are fetched and
acknowledged before readiness; the sidecar reports success only after a short
quiet window with no pending change.
Start AgentsView and wait for the sessions endpoint before direct entry because
the sidecar exits on its first later source failure and does not retry. A
failed handshake terminates and reaps the unready sidecar. Do not run the
sidecar yourself. For a live session-row check, follow the
[chat-guided AgentsView demo](docs/zellij-agentsview-live-demo.md).

If that exact rail never appears, the command reports
`sidecar-target-unready`, starts no sidecar, and preserves the newly created
tab for inspection. After it starts, target-tab loss, a delivered SIGINT or
SIGTERM, source EOF, or a source failure ends the sidecar. It does not
restart, retarget, or clean up AgentsView, Zellij sessions, tabs, panes, or
plugins.

`Alt Shift z` remains a separately configured tab-only shortcut. It opens the
one fixed layout already named by the operator's global config; it does not
select an arbitrary checkout and the direct script never repoints it. It also
cannot safely start the subscriber because a native Zellij `Run` keybind
materializes a helper pane.
Zellij named pipes remain session-wide broadcasts. Direct entry therefore
uses a versioned pipe name derived from its fresh recipient token. The rail
also requires a fresh `PaneUpdate` followed by `TabUpdate` and the exact
stable `recipient-tab-id`. CWD is used only after these checks to focus a pane
in the accepted rail.

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
ZAPHOD_PERMISSION_FIXTURE=upgrade ./tests/zellij-tmux-smoke-test.sh
./tests/zellij-two-rail-recipient-smoke-test.sh
```

They use the [isolated tmux smoke harness](docs/zellij-tmux-smoke-harness.md).
The second test deliberately gives two same-CWD rails one recipient token and
proves that the stable-tab guard still delivers only to the target rail.

### Historical worktree profile

`scripts/zellij-worktree-test-profile.sh` is parked experimental evidence. It
is not the candidate test path and does not define `Alt /` safety. Use the
[isolated tmux smoke harness](docs/zellij-tmux-smoke-harness.md) for every
real-key candidate check.

### Permissions

Tokenless installed-layout rails request `ReadApplicationState`,
`ChangeApplicationState`, `ReadPaneContents`, `Reconfigure`, and `RunCommands`.
A token-bound direct-entry rail also requests `ReadCliPipes` for its private
subscriber. On the first request or grant expansion, focus the pane and
approve its native prompt once.
Zellij's grant cache is keyed by the raw WASM path. By default, the smoke
harness redirects `HOME` to a temporary root and uses a pre-granted fixture.
Its `upgrade` mode seeds an old grant without `ReadCliPipes`, focuses the exact
candidate pane, sends one literal `y` through the attached tmux client, and
checks the expanded cache and normal session row. Neither mode writes the
operator's cache. `ReadCliPipes` is used only for the direct-entry
subscriber's private recipient, initial-snapshot, and accepted-row
acknowledgments.
`Reconfigure` changes only runtime keybinds; Zaphod does not save that route
to disk. `RunCommands` is required only when a gate row floats `subspace-tui`.

## Status

Working prototype (zellij 0.44.3): per-tab toggle, click/keyboard switching,
plugin-local agent awareness, tab-bound session rows from the direct script,
state/status lines, and docked/sliver toggle. Use
`scripts/zellij-new-tab.sh` to create a rail from a selected checkout. A
separately installed `Alt Shift z` binding opens only its fixed configured
layout and does not select a checkout or start session rows. `Alt /` never
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
