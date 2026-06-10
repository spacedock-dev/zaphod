# zaphod

A per-tab pane-switcher sidebar for [zellij](https://zellij.dev), built for
stacked-pane workflows where many terminal agents (Claude Code, codex, …) run
side by side. It answers one question at a glance: **what is running in this
tab and what does it want from me?**

```
┌ sidebar ──────────────┐
│▾ PANES             ⇄ │
│● ✳ spacedock:officer  │   ← focused (yellow ●), agent title in cyan
│    > approve merge?   │   ← the pane's last terminal line, live
│● ✳ codex literature   │   ← red ● = working ("esc to interrupt")
│    Searching arxiv…   │
│  clkao@mac:~/git/x    │
│    $                  │
└───────────────────────┘
```

## Features

- Lists the current tab's terminal panes; **click a row to focus that pane**
- The sidebar is unfocusable (tab-bar mechanism): clicks are delivered
  without focusing it, so it never steals your keyboard
- Per-pane status line: the last non-empty terminal line, refreshed every 2s
- Agent awareness: `✳`-titled panes (Claude Code/codex) highlighted; red dot
  while the agent is working
- **Keyboard navigation mode**: `j/k`/arrows move a highlight, `Enter` jumps,
  `Esc` returns focus where it was
- Floating "rail" mode (pinned, left edge, exact placement at runtime) and
  docked tile mode (reserves space; placed via layouts); `⇄` in the header
  toggles between them
- Per-tab instances that toggle independently

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
status lines, busy markers, rail and docked modes. Known broken: lazily
spawning an instance into a tab that never had one (crashes the spawning
instance — a response-reading shim call inside a pipe handler; see SPEC.md
law #4). Workaround: open the sidebar via the layout, or press `Alt /` in a
fresh tab before instances exist elsewhere.

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
