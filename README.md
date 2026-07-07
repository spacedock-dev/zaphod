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
  rows into the rail — agent sessions with state, and pending gate
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

### Toggle / navigate keybinds (config.kdl)

```kdl
keybinds {
    shared {
        bind "Alt /" {
            MessagePlugin "file:/path/to/zellij-sidebar.wasm" {
                name "toggle"
                floating true
                skip_cache true   // dev only: zellij's plugin cache is path-keyed
                rail "1"          // identity key; see SPEC.md learnings #2-3
            }
        }
        bind "Alt ." {
            MessagePlugin "file:/path/to/zellij-sidebar.wasm" {
                name "navigate"
                floating true
                skip_cache true
                rail "1"
            }
        }
    }
}
```

### Docked sidebar in every new tab (layout file)

`default_tab_template` works **only in layout files** (it is silently ignored
in `config.kdl`):

```kdl
// ~/.config/zellij/layouts/default.kdl
layout {
    default_tab_template {
        pane size=1 borderless=true { plugin location="zellij:tab-bar" }
        pane split_direction="vertical" {
            pane size=26 borderless=true {
                plugin location="file:/path/to/zellij-sidebar.wasm"
            }
            children
        }
        pane size=1 borderless=true { plugin location="zellij:status-bar" }
    }
}
```

### Permissions

On first launch the pane shows a permission prompt (`ReadApplicationState`,
`ChangeApplicationState`, `ReadPaneContents`) — focus it and approve once;
zellij caches the grant.

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
