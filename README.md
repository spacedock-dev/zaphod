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
destination. It also requires every Zaphod `MessagePlugin` in the effective
`config.kdl` to use the primary checkout's canonical WASM URL and `rail "1"`.
On a mismatch, it reports each offending URL and writes nothing. It never
rewrites keybinds.

For the one-time cleanup, build the primary checkout and deliberately repoint
each Zaphod keybind to
`file:<PRIMARY_CHECKOUT>/target/wasm32-wasip1/release/zellij-sidebar.wasm`.
Each message block must include `rail "1"`. This is the reference shape, not a
worktree-install recipe:

```kdl
keybinds {
    shared {
        bind "Alt /" {
            MessagePlugin "file:<PRIMARY_CHECKOUT>/target/wasm32-wasip1/release/zellij-sidebar.wasm" {
                name "toggle"
                floating true
                skip_cache true
                rail "1"
            }
        }
        bind "Alt ." {
            MessagePlugin "file:<PRIMARY_CHECKOUT>/target/wasm32-wasip1/release/zellij-sidebar.wasm" {
                name "navigate"
                floating true
                skip_cache true
                rail "1"
            }
        }
    }
}
```

After cleanup, rerun `./install.sh`. The installer renders and parses the
layout in a disposable Zellij 0.44.3 session, renames it atomically, and
rechecks identity. A failed postflight restores the previous layout bytes.
Use live state—not the rendered file—as the final oracle:

```bash
ZELLIJ_SESSION_NAME=<session> zellij action list-panes --json -a -g -t
ZELLIJ_SESSION_NAME=<session> zellij action dump-layout
```

Exactly one sidebar should remain before and after `Alt /`; every sidebar URL
in the dump must name the primary checkout artifact.

### Test an unmerged worktree

Run the profile from the checkout under test. `--cwd` sets the terminal leaf
used by the retrofit drill:

```bash
./scripts/zellij-worktree-test-profile.sh --cwd "$PWD"
```

The command builds that checkout, creates isolated config, layout, data, and
permission state, and launches an attached session from `explicit-cwd.kdl`.
It prints the profile root, session, commit, candidate URL, and exact
`list-panes`/`dump-layout` inspection commands. In another terminal, start a
resident control from the same profile:

```bash
PROFILE_ROOT=<printed-profile-root>
SESSION_NAME=<printed-session-name>
zellij --config-dir "$PROFILE_ROOT/config" --data-dir "$PROFILE_ROOT/data" \
  --session zaphod-control --new-session-with-layout zaphod
```

Press the profile's real `Alt /` and use the printed live-state commands to
verify one candidate identity. Clean up the control before the attached drill:

```bash
zellij delete-session --force zaphod-control
zellij delete-session --force "$SESSION_NAME"
```

The profile's exit and signal traps delete its session, config, data, and
permission state, then fail if the standing global `config.kdl` or
`layouts/zaphod.kdl` existence or SHA-256 changed. The profile never copies,
rewrites, or restores global files.

### Permissions

On first launch in each disposable profile, the pane shows a permission prompt
(`ReadApplicationState`, `ChangeApplicationState`, `ReadPaneContents`) — focus
it and approve once; Zellij caches the grant only inside that profile's data
root.

## Status

Working prototype (zellij 0.44.1): per-tab toggle, click/keyboard switching,
plugin-local agent awareness, state/status lines, docked/sliver toggle. A
tab without a sidebar gets one on its first `Alt /`: a one-time layout
retrofit docks the rail, installs the swap set, and preserves the tab's
pane arrangement (falling back to stacking the panes when the tab's
layout cannot be dumped).

[SPEC.md](SPEC.md) carries the full validated spec, a 23-entry map of
zellij-plugin landmines this prototype paid for, and the from-scratch v2
design (manifest-derived state, two-phase command execution, layout-first
placement).

## Development

- `cargo test` — pure-function tests (row building, click mapping, toggle
  decisions); a host-side stub satisfies the wasm import at link time
- The zellij server log is the oracle:
  `$TMPDIR/zellij-<uid>/zellij-log/zellij.log` (permission denials, real
  compiles vs cache hits, wasm crashes)
- Headless bench: `zellij attach bench --create-background`, then drive it
  with `ZELLIJ_SESSION_NAME=bench zellij action …`
