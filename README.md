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
- Per-pane status line: detected state, agent kind, and latest prompt/status,
  refreshed every 2s
- Plugin-only agent awareness for Claude, Codex, and Pi panes using zellij's
  running-command and scrollback APIs; shell panes remain visible as
  `unknown . unknown`
- Blocked prompts outrank working prompts, with state markers in the first
  line and details in the dimmed second line
- **Keyboard navigation mode**: `j/k`/arrows move a highlight, `Enter` jumps,
  `Esc` returns focus where it was
- `Alt /` (or the `⇄` header) toggles the docked 28-col rail down to a
  1-col sliver and back by cycling the tab's swap layouts — panes are
  rearranged in place, never spawned or hidden, and a manually re-split tab
  keeps its arrangement (the swap set is regenerated from the live layout)
- Per-tab instances that toggle independently
- **Agent & gate rows**: a companion `grout` process pipes `agent-event`
  rows into the rail — agent sessions with state (blocked / working /
  idle / done, mapped from agentsview status + activity), and pending gate
  decisions. Click a session row to focus its cwd-bound pane (unbound is
  shown, never guessed); click a gate row to float `subspace-tui` on the
  gate's artifact with `--log` pointed at its decision log. Requires the
  `RunCommands` permission (prompted once)

## Build

Requires a Rust toolchain with the `wasm32-wasip1` target
(`rustup target add wasm32-wasip1`).

```bash
./build.sh
# → target/wasm32-wasip1/release/zellij-sidebar.wasm
```

`build.sh` pins rustup's `rustc` explicitly because a homebrew Rust earlier
on `PATH` lacks the wasm std and fails with `can't find crate for core`.

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

To activate this checkout and create a fresh tab in an existing session, run:

```bash
./scripts/zellij-new-tab.sh --session WORK
```

The command builds this checkout, renders its WASM URL into an inline layout,
and creates exactly one new tab. It atomically updates only existing Zaphod
keybind scopes in the selected config root: `Alt Shift z` natively creates
that root's stored layout by absolute path, and persistent `Alt /` is `NoOp`.
It never changes an existing tab. The absolute path matters: Zellij resolves
the named `layout "zaphod"` form from its standing default config root, even
when the session was launched with an isolated config root.

When a tiled Zaphod rail is visible, approve its `Reconfigure` permission.
The rail then installs a temporary, current-client `Alt /` route to its own
already-running plugin. `Alt /` toggles that rail only. In a foreign tab—or in
a newly attached client before its rail has initialized—it safely does nothing
and never creates a pane.

Use `ZELLIJ_CONFIG_DIR`, `ZELLIJ_CONFIG_FILE`, and `ZELLIJ_DATA_DIR` to run it
against an isolated profile. The current invocation creates its tab at once;
restart the Zellij server before relying on a newly written native keybind.

Run the real-key boundary with:

```bash
./tests/zellij-tmux-smoke-test.sh
```

It is the [isolated tmux smoke harness](docs/zellij-tmux-smoke-harness.md).

### Historical worktree profile

`scripts/zellij-worktree-test-profile.sh` is parked experimental evidence. It
is not the candidate test path and does not define `Alt /` safety. Use the
[isolated tmux smoke harness](docs/zellij-tmux-smoke-harness.md) for every
real-key candidate check.

### Permissions

On first launch in each disposable profile, the pane shows a permission prompt
(`ReadApplicationState`, `ChangeApplicationState`, `ReadPaneContents`,
`Reconfigure`, `RunCommands`) — focus it and approve once; Zellij caches the
grant only inside that profile's data root. `Reconfigure` changes only the
current client's runtime keybinds; Zaphod does not save that route to disk.
`RunCommands` is required only when a gate row floats `subspace-tui`.

## Status

Working prototype (zellij 0.44.3): per-tab toggle, click/keyboard switching,
plugin-local agent awareness, state/status lines, and docked/sliver toggle.
Create a rail with `scripts/zellij-new-tab.sh` or the initialized `Alt Shift z`
binding. `Alt /` never creates or retrofits a tab.

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
