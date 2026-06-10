# Plugin Agent Awareness Sidebar Design

## Purpose

Zaphod should show which panes in the current zellij tab are agents and what
they need from the user. The first version stays plugin-only: no helper daemon,
no native process tree walker, and no long-lived external companion.

The design copies herdr's useful shape, not herdr's terminal runtime. Zaphod
will render compact two-line agent rows, classify agent state conservatively,
and keep click-to-focus and keyboard navigation behavior from the existing
sidebar.

## Scope

This design covers a richer agent awareness pane inside the existing zellij WASM
plugin. It does not change placement, toggle behavior, docked/floating rail
logic, or layout installation.

The first detector set targets Claude Code, Codex, and Pi. Other agents can be
added later by extending the detector table.

## User Experience

Rows keep the current two-line rhythm but switch from a generic pane list to an
agent-aware presentation.

```text
agents              current
[idle]    spacedock officer
          idle . claude

[blocked] codex literature
          blocked . codex . approve command?

[working] pi reviewer
          working . pi . Searching...
```

Each row remains clickable. Clicking any part of a row focuses that terminal
pane. Keyboard navigation continues to select rows and jump with Enter.

Known agents get state styling. Shell panes render as two-line `Unknown` rows in
the same list for v1. This keeps row height, click mapping, and keyboard
navigation uniform. Unknown rows use lower-emphasis styling, but clicking them
still focuses their pane.

## Architecture

Add a plugin-local state engine around the existing `Row` model.

```rust
enum AgentKind {
    Claude,
    Codex,
    Pi,
    Unknown,
}

enum AgentState {
    Blocked,
    Working,
    Done,
    Idle,
    Unknown,
}

struct Row {
    pane_id: u32,
    title: String,
    focused: bool,
    agent: AgentKind,
    state: AgentState,
    status: String,
    running_command: Option<Vec<String>>,
}
```

The current `PaneUpdate` handler still owns row discovery. It rebuilds the row
list from the tab's terminal panes, preserves prior classification by pane id,
and marks focus from the manifest.

The timer poll enriches each row through a pure function:

```rust
fn enrich_row(
    previous: &Row,
    command: Result<Vec<String>, String>,
    scrollback: Result<Vec<String>, String>,
) -> Row
```

The function owns stale-state behavior. On command failure, it keeps the prior
`running_command`; on scrollback failure, it keeps the prior `state` and
`status`. The timer performs host calls; `enrich_row` stays testable.

The timer poll steps are:

1. Call `get_pane_running_command(PaneId::Terminal(row.pane_id))`.
2. Call `get_pane_scrollback(PaneId::Terminal(row.pane_id), false)`.
3. Identify the agent from the running command first, then from the title.
4. Pass those results to `enrich_row`.
5. Replace the row with the enriched result.

The plugin should not call response-reading zellij APIs from `load()` or
`pipe()`. Those handlers may record intent, but polling and enrichment should
run from timer/update paths that already work in this codebase.

## Detection

Agent identity should prefer structured data. Normalize command names by taking
the path basename, lowercasing it, and stripping one common executable suffix:
`.exe`. Match the normalized basename, not arbitrary substrings in paths or
arguments.

| Source | Claude | Codex | Pi |
| --- | --- | --- | --- |
| Command basename | `claude`, `claude-code` | `codex` | `pi` |
| Title fallback | contains `claude` or `claude code` | contains `codex` | contains `pi` as a word |
| Scrollback fallback | Claude prompt chrome or `esc to interrupt` with Claude title marker | Codex confirmation or Codex header text | `Working...` |

The title marker alone identifies "agent-like" but not the specific agent. If a
title contains only the marker and no agent name, classify it as `Unknown`.
Never classify `pip`, `python`, paths containing `/pi/`, or arbitrary arguments
as Pi.

State detection should prefer visible blockers over activity. The initial order
is:

1. Blocked: approval or confirmation prompts such as "allow command", "allow
   command?", "enter to confirm", "press enter to confirm", "enter to submit
   answer", "enter to submit all", "[y/n]", "yes (y)", or "submit answer".
2. Working: interrupt controls such as "esc to interrupt", "esc to cancel",
   "esc to stop", or Pi's exact `Working...` line.
3. Idle: known agent with no blocker or working signal.
4. Unknown: no known agent or no reliable signal.

`Done` needs a seen/unseen model that zaphod does not yet maintain. The first
implementation may define the enum but render idle until completion tracking is
designed.

The v1 status extractor is exact and testable: trim each viewport line, scan
from bottom to top, and return the first non-empty line. If the detected state is
`Blocked`, prefer the matched blocker line when it is available; otherwise use
the last non-empty line. Truncate the rendered status to fit the row.

## Rendering

Keep the existing terminal-print renderer for now. Add small helper functions:

- `state_icon(state) -> &'static str`
- `state_label(state) -> &'static str`
- `state_color(state) -> ansi style`
- `agent_label(kind) -> &'static str`
- `truncate_text(text, width) -> String`

Render each row as:

1. State marker, primary label.
2. Indented state label, agent label, and optional status.

Focused rows keep a clear highlight. Navigation mode keeps reverse-video
selection. Active state styling should not obscure the state marker.

## Error Handling

If `get_pane_running_command` fails, preserve the previous command value for
that pane and classify from title plus scrollback. If `get_pane_scrollback`
fails, keep the previous status and state. If both calls fail, preserve the
prior enriched row except for manifest-derived fields such as title and focus.
A missing permission should degrade the sidebar, not crash the plugin.

No detector should panic on malformed text or invalid UTF-8 replacement.
Truncation must be character-based, as the current renderer already does.

## Testing

Keep tests pure where possible. Add unit tests for:

- command-to-agent classification
- viewport-to-state classification
- status-line extraction
- row preservation across `PaneUpdate`
- `enrich_row` failure behavior for command failure, scrollback failure, both
  failures, and stale-state preservation
- click line mapping if row height changes
- rendering helpers, especially truncation

Manual verification remains necessary in zellij 0.44.1:

1. Build with `./build.sh`.
2. Launch the sidebar in a tab with Claude, Codex, Pi, and a shell pane.
3. Confirm blocked, working, idle, and unknown states render correctly.
4. Confirm click-to-focus and keyboard navigation still work.
5. Confirm the sidebar does not steal focus outside navigation mode.

## Open Questions

The first implementation should answer these with code, not speculation:

- Does `get_pane_running_command` return the foreground agent command for normal
  zellij terminal panes?
- Does polling it every two seconds create visible lag or log noise?
- Are Claude and Codex titles enough to identify agents when command detection
  is unavailable?

## Recommendation

Build the plugin-local state engine first. It gives zaphod the herdr-like agent
awareness pane without adding install steps or a new runtime. External probes,
helper daemons, and long-lived native companions are out of scope for this
design. If zellij's own command and scrollback APIs prove too weak, that needs a
separate design and approval.
