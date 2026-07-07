---
title: Plugin rows section
status: ideation
source: plan sprint 0 (docs/plan-agent-rail.md)
score: 0.8
id: 5tdw8rckcrsx1qfytsytgp9m
started: 2026-07-07T05:32:39Z
---

## Problem

The rail renders pane rows only; it has no rows section for agent sessions or
pending gates, and its pipe handler traces payloads without parsing them. The
walking skeleton needs the display half.

## Proposed approach

The rail gains two sections below the pane rows — AGENTS and GATES — fed by
the `agent-event` pipe, with identity binding computed in the plugin and all
decision logic pure. Server-source citations below are zellij-server-0.44.1
(crates.io), the tag CL runs (plan header: zellij CLI 0.44.1).

### Pipe handler: the two pinned kinds, tolerant, traced

`pipe()` gains an `agent-event` branch ahead of the `toggle` branch
(`src/main.rs:373-402`), keeping the existing `pipe recv name=` trace line
FIRST — the grout slice's AC-4 fallback and its payload-size probe observe
receipt through that exact trace. The branch only mutates plugin state (no
host calls — SPEC landmine #4) and returns true when state changed; CLI
callers are released by auto-unblock on return (landmine #5).

Payloads are the grout skeleton's two pinned kinds, verbatim from plan
decision 3 (`docs/plan-agent-rail.md`; field sets pinned in
`grout-skeleton.md` "Row protocol"):

```
{"kind":"session","id","cwd","agent","state","summary","ts"}
{"kind":"gate","log_path","workflow","entity","entity_title","stage","round","recommendation","ts"}
```

Parsing: serde_json into `#[serde(tag = "kind")] enum AgentEvent { Session,
Gate }` with `#[serde(default)]` on every field — unknown extra fields are
tolerated (grout pins exact sets today; tolerance is the plugin's concern per
the grout entity), missing fields default (a session without `cwd` simply
renders unbound). serde + serde_json become direct deps — both already in
the tree via zellij-tile, so no new transitive code. Malformed JSON, unknown
`kind`, or a missing payload is dropped with a trace line naming the reason;
rail state is untouched and the handler never panics.

Pure functions: `parse_agent_event(&str) -> Result<AgentEvent, String>`;
`apply_agent_event(&mut sessions, &mut gates, AgentEvent)` upserts —
sessions keyed by `id`, gates keyed by `log_path`, insertion order kept, no
expiry (grout is one-shot in sprint 0; lifecycle is sprint 1+).

### Binding: cwd match, in the plugin, never guessed

Ideation finding, on the record: **`PaneInfo` carries no cwd** — decision 1's
"matches against its PaneManifest" cannot be satisfied by the manifest alone
(`zellij-utils-0.44.3 data.rs:2296`, no cwd field). The manifest supplies the
pane set; the `get_pane_cwd(PaneId)` host call (zellij-tile 0.44,
`ReadApplicationState` — already granted) supplies each pane's cwd from the
OS via the pane's child pid (`pty.rs:2181-2210`, `os_input_output.rs:512-547`,
sysinfo `process.cwd()`). The decision's intent is preserved: binding lives
in the plugin; grout never learns zellij exists.

- `Row` gains `cwd: Option<PathBuf>`, filled in the `refresh_statuses` pass
  (`src/main.rs:668-690`) — one `get_pane_cwd` per due row, riding the
  existing `PollBackoff` gate unchanged and preserved across PaneUpdate by
  `preserve_agent_fields` (`src/main.rs:303`). A failed cwd call keeps the
  previous value (stale-not-blank, the shipped poll posture). The pass's
  changed flag also covers cwd changes so a binding flip re-renders.
- `bind_session(session_cwd, rows) -> Option<u32>` (pure): exactly one row
  whose polled cwd matches → that pane id; zero or two-plus matches → None.
  **Unbound renders as unbound, never guessed** — an unbound session row
  shows a ` ·unbound` suffix and its click is a no-op.
- Scope: sprint 0 binds within the rail's own tab (the row set it already
  lists). Cross-tab binding goes to sprint 1 with the session-wide pool;
  enabling fact recorded now: a plugin `focus_terminal_pane` on another
  tab's pane switches the client's view there — `Screen::focus_pane_with_id`
  calls `go_to_tab` first (`screen.rs:3517-3552`).
- Wedge posture: the export bounds its wait at 100ms
  (`zellij_exports.rs:4142-4177`) — but its twin `get_pane_running_command`
  carries the identical guard (`:4104`) and still produced the sibling
  slice's observed ~13s wedges, so the guard is not trusted: the cwd call is
  treated as wedge-class and rides the same poll pass the pipe-unblock
  slice's wedge budget bounds (one more timed call under that budget).

### Rendering: two sections, zero footprint when empty

Below the pane rows: an `AGENTS` header then session rows, a `GATES` header
then gate rows — each row two lines like pane rows, truncated to `cols`.
A section with no rows renders nothing at all, so a rail that never receives
an event renders byte-identically to today. Session row: line 1 = state
marker + agent + ` ·unbound` when unbound; line 2 = dim summary. Gate row:
line 1 = marker + entity_title; line 2 = dim `{stage} r{round} ·
{recommendation}`. New pure `marker_for_state(&str) -> AgentState` maps
grout's state strings (`blocked|working|idle|done`, else Unknown) onto the
existing `state_marker` glyphs — giving `AgentState::Done` its first
producer and retiring its `#[allow(dead_code)]` (`src/agent.rs:16`).

### Clicks: one shared layout, per-kind actions

A pure `section_layout(pane_count, session_count, gate_count)` describes the
line map once; both `render` and the extended `target_for_line`
(`src/main.rs:1491`) derive from it so click math cannot drift from pixels.
`LineTarget` gains `SessionRow(idx)`/`GateRow(idx)`; section headers map to
None. `decide_click` (`src/main.rs:963`) stays pure: session row →
`FocusPane(bound id)` or None when unbound; gate row → new
`ClickAction::FloatGate { brief, log }`.

`handle_click` executes FloatGate as `open_command_pane_floating(
CommandToRun { path: "subspace-tui", args: [brief, "--log", log], cwd: None },
None, ..)` — from `update()` (Mouse), a permitted context (landmine #4 bars
`pipe()`/`load()` only). `brief` comes from pure
`brief_path_for_log(log_path)`: TrimSuffix `.decisions.jsonl` + `.md`, the
same inversion grout uses; a log_path without that suffix renders the row
but its click is a no-op with a trace (never float a wrong file).
`subspace-tui <brief> --log <log>` verified against
`spacedock-subspace/cmd/subspace-tui/main.go:103-129`: `--log` implies
persist, so verdicts land on the gate's real decision log — the rail itself
never writes the record.

F8 rider (`docs/review-findings-2026-07-07.md`): `handle_click` FocusPane
leaves nav mode stuck on. This slice rewrites those exact lines and its new
actions would inherit the bug, so it folds in the one-line fix — FocusPane /
FloatGate call `exit_nav(false)` when `nav_mode`. Gate may strike it back to
the F5-F8 triage task. No other open finding touches this region (F2/F3/F5-F7
are toggle-path, F9/F10 chrome-path).

### Permissions and blast radius

`OpenCommandPaneFloating` requires the **`RunCommands`** grant
(`zellij_exports.rs:5181-5191`) — not in the rail's current set, so the
first-render request expands and CL sees **one new permission prompt** per
plugin identity in sessions started after deploy. `get_pane_cwd` rides the
already-granted `ReadApplicationState`. Running sessions are untouched until
restart (path-keyed in-memory plugin cache, landmine #1). With no
agent-event traffic the rail renders and behaves exactly as today; keybinds,
toggle, and layout machinery are untouched.

### Pure functions: extended vs new

Extends: `target_for_line`/`decide_click` (section-aware click math),
`preserve_agent_fields` (carries cwd), the `state_marker` family (via
`marker_for_state`), `refresh_statuses` + `PollBackoff` (cwd call rides the
pass; `due`/`record`/`should_poll_statuses` unchanged). New:
`parse_agent_event`, `apply_agent_event`, `bind_session`,
`brief_path_for_log`, `section_layout`, `marker_for_state`.

### Coordination with sibling slices

- **Consumes:** grout's two pinned row kinds on pipe name `agent-event`
  (plan decision 3; `grout-skeleton.md` "Row protocol"), exact field sets,
  `round` a JSON number, summary pre-clamped grout-side. Unknown kind or
  malformed JSON: dropped with a reason-naming trace, state untouched.
- **Preserves:** the `pipe recv name=` trace ahead of parsing — grout's
  AC-4 fallback observation and payload-size probe depend on it.
- **Rides:** the `refresh_statuses` pass the pipe-unblock slice is
  instrumenting (in implementation now) — the added `get_pane_cwd` call must
  land under its wedge budget; whichever slice merges second applies the
  budget to the added call.
- **Demo dependency:** the sprint-0 exit demo is joint — grout (slice b)
  emits the rows, this slice renders and acts on them, pipe-unblock (slice a)
  protects CL's session. Prerequisites: `subspace-tui` on the zellij server's
  PATH; a live agentsview session for grout; the demo click runs from the
  tab holding the agent's pane (binding is per-tab in sprint 0).

## Riskiest unproven mechanism

**The cwd read path for binding.** `get_pane_cwd` has never been called in
this codebase, and its answer is the OS-resolved physical path (sysinfo
resolves symlinks — `/tmp` → `/private/tmp` on macOS), while agentsview
records whatever the session reported. If the two shapes cannot be matched
textually, binding-in-plugin (plan decision 1) has no data source and this
design is invalid. The smallest end-to-end check costs minutes and no build:
`zellij action list-panes --all --json` uses the identical server path —
`enrich_pane_with_cwd` sends the same `PtyInstruction::GetPaneCwd`
(`route.rs:2794-2814`) the plugin export sends — so comparing its `pane_cwd`
against `agentsview session get --json`'s cwd for a session running in one
of those panes settles shape, symlink resolution, and availability at once.
Test plan item 1; CL runs it (the sandbox blocks agentsview and live
sessions). Divergent-but-normalizable shapes pin a textual normalization
rule from the probe's real outputs; absent/garbage cwd → back to ideation.

Everything else rides proven mechanisms: pipe transport end to end
(2026-07-07 spike, 1-6s), rail row rendering + click delivery (shipped,
landmine #20), `focus_terminal_pane` (shipped click path),
`open_command_pane_floating` from `update()` (mainstream plugin surface;
permission mapping verified at source this ideation), auto-unblock on
`pipe()` return (SPEC #5, verified at source by the sibling slice).

## Acceptance criteria

Offline (agent-reproducible):

**AC-1 — value, wire-to-action: the pinned protocol lines drive a bound,
clickable model.** Feeding the two example lines transcribed from plan
decision 3 (outside this plugin's source) plus a manifest fixture whose one
matching pane cwd equals the session line's cwd yields: a session row whose
marker reflects the line's `state`, bound to that pane —
`decide_click` on its line = `FocusPane(that pane id)` — and a gate row
whose click = `FloatGate` with `brief` = the log_path with
`.decisions.jsonl` → `.md` and `log` = the line's log_path verbatim.
Re-applying the same session line upserts (one row, updated fields), not
duplicates.
Verified by: `cargo test` (proposed `pinned_protocol_lines_render_and_bind`,
`session_upsert_replaces_by_id`, `gate_click_floats_tui_on_brief`).

**AC-2 — binding never guesses.** Zero panes matching the session cwd, and
two panes sharing it, both render ` ·unbound` and click to None; only an
exactly-one match binds. A row whose cwd poll failed binds by its last known
cwd (stale-not-blank, the shipped poll posture).
Verified by: `cargo test` (`ambiguous_cwd_renders_unbound`,
`no_match_renders_unbound`, `single_match_binds`).

**AC-3 — protocol robustness.** Unknown `kind`, malformed JSON, and missing
payload are each dropped with a reason-naming trace: sessions/gates state
before == after, no panic, `pipe()` returns false (no re-render). A gate
`log_path` not ending `.decisions.jsonl` renders but clicks to None.
Verified by: `cargo test` (`unknown_kind_dropped`, `malformed_json_dropped`,
`bad_log_suffix_never_floats`).

**AC-4 — zero footprint while empty.** With no agent-event received, render
output and click mapping are identical to pre-slice behavior: every existing
render/click/toggle test passes unmodified (expected values written before
this slice — a baseline outside it), and the full suite stays green.
Verified by: `cargo test` + `cargo check --tests` (95+ existing tests
untouched).

Interactive (settled only by CL's live demo — the sprint-0 exit, joint with
the grout slice):

**AC-I1 — value: both rows rendered, both clicks land.** In CL's fresh
session, one grout run renders both row kinds in the rail within 10s (spike
baseline 1-6s). Clicking the session row moves focus to the cwd-bound agent
pane — the focused pane after the click is the independent baseline, and it
can move the wrong way (focus elsewhere, or nothing). Clicking the gate row
floats `subspace-tui` showing the playground brief; a verdict issued in that
TUI appends to the gate's `.decisions.jsonl` (on-disk baseline) — written by
the TUI, never by the rail.
Verified by: CL's live demo per the validation stage's script.

**AC-I2 — blast radius: one prompt, nothing else moved.** The first
post-deploy session shows exactly one new permission prompt (RunCommands
added to the set); pane rows, statuses, Alt-/, nav, and layout behavior are
unchanged with and without agent-event traffic.
Verified by: CL confirms during the same demo.

## Test plan

1. **First — smallest end-to-end check that would invalidate the design:
   the cwd-shape probe.** CL, in any live session with an agent running:
   `zellij action list-panes --all --json` (carries `pane_cwd` via the same
   server path as the plugin's `get_pane_cwd`) and
   `agentsview session get <id> --format json`; compare the two cwd strings
   for the same directory. Minutes, no build. Match → binding is proven
   data. Normalizable divergence (e.g. `/private` prefix) → pin the textual
   rule from the real outputs into `bind_session`'s spec. No usable cwd →
   stop; back to ideation (plan decision 1 loses its data source).
2. TDD the pure fns red-first: `parse_agent_event` (both pinned lines,
   unknown kind, malformed, missing fields), `apply_agent_event` upserts,
   `bind_session` (0/1/2 matches; normalization per probe),
   `brief_path_for_log` (inversion + bad-suffix), `marker_for_state`,
   `section_layout` + extended `target_for_line`/`decide_click` (concrete
   line numbers for a P=2,S=1,G=1 fixture), F8 exit-nav-on-click.
3. Implement: `pipe()` branch, `Row.cwd` + poll-pass call under the
   sibling's wedge budget, render sections, `handle_click` FloatGate.
   `cargo test` + `cargo check --tests` after each step; suite stays green.
4. Hermeticity: all offline tests run with no zellij server, no network, no
   agentsview — fixtures are string literals from the plan plus constructed
   manifests (the shipped test pattern, `src/main.rs:1577+`).
5. Live (sprint-0 exit, joint with grout): deploy via `./build.sh` +
   fresh session; AC-I1/AC-I2 per demo script; grout's payload probe rides
   the same session.

## Proposed doc diffs (applied at implementation, reviewed at this gate)

- `README.md` Features, add bullet: "**Agent & gate rows**: a companion
  `grout` process pipes `agent-event` rows into the rail — agent sessions
  with state, and pending gate decisions. Click a session row to focus its
  cwd-bound pane (unbound is shown, never guessed); click a gate row to
  float `subspace-tui` on the gate's artifact with `--log` pointed at its
  decision log. Requires the `RunCommands` permission (prompted once)."
- `SPEC.md` "What it is", append: "The rail also renders agent-session and
  pending-gate rows fed over the `agent-event` pipe by grout — protocol and
  binding rules in `docs/plan-agent-rail.md` (decisions 1-3): two typed JSON
  kinds, cwd binding in the plugin via `get_pane_cwd`, unbound rendered as
  unbound, never guessed."

## Out of scope

- SSE-fed live updates, many sessions, session-wide (cross-tab) binding and
  the go_to_tab click-through, richer state mapping, row expiry/lifecycle
  (sprint 1).
- Glob-config gate discovery, folding, pending/resolved filtering, parked
  rows (sprint 2); verdict actions from the rail (sprint 3 / M2).
- Keyboard nav over agent/gate rows (nav stays pane-rows-only); scrolling
  when sections overflow the rail (existing roadmap gap, now nearer).
- Duplicate-float guarding (clicking a gate twice floats two TUIs).
- Payload-size handling in the plugin (grout clamps; ceiling is grout AC-5).
- The wedge budget itself (sibling slice); grout emit semantics (sibling).
- Review findings other than F8 (F5-F7, F9, F10 — none touch this region).

## Stage Report: ideation

- DONE: ACs split offline vs interactive, with at least one value-measuring AC (e.g. click-to-focus lands on the cwd-bound pane in the live demo)
  AC-1-4 offline, AC-I1/AC-I2 interactive; AC-1 (pinned protocol lines drive a bound clickable model, plan-fixture baseline) and AC-I1 (click-to-focus lands on the cwd-bound pane; post-click focus and the gate log's on-disk append are the independent baselines) measure the end value.
- DONE: Binding-in-plugin design concrete: cwd match against PaneManifest, unbound renders as unbound never guessed; parsing and click-decision logic specified as pure functions
  Ideation finding on the record: PaneInfo carries no cwd — the manifest supplies the pane set, the get_pane_cwd host call (ReadApplicationState, already granted) supplies per-pane cwds under the sibling's wedge budget; bind_session is pure (exactly-one match binds; zero or many render " ·unbound", click no-op); parse_agent_event / apply_agent_event / brief_path_for_log / section_layout / extended decide_click all pure and TDD-able offline.
- DONE: Coordination named explicitly: which agent-event payload shapes this slice consumes (grout's two pinned kinds), what happens on unknown kind/malformed JSON, and the demo dependency on the sibling slices
  Consumes grout's two pinned kinds verbatim (plan decision 3), tolerating unknown extra fields; unknown kind/malformed JSON dropped with a reason-naming trace, state untouched, no panic; joint sprint-0 exit demo with grout (rows) under pipe-unblock's protection, the "pipe recv name=" trace preserved for grout's AC-4/probe, and the added cwd call riding the sibling's budgeted poll pass.

### Summary

Designed the rail's display half: an `agent-event` pipe branch parsing
grout's two pinned row kinds into AGENTS/GATES sections below the pane rows,
cwd binding computed in the plugin, and per-kind clicks (bound session →
focus_terminal_pane; gate → float `subspace-tui <brief> --log <log>`,
verified against subspace-tui's parseArgs). Key mechanism facts verified at
zellij-server-0.44.1 source: PaneInfo has no cwd (get_pane_cwd fills it, same
server path as `list-panes --all --json` — the build-free probe), and
OpenCommandPaneFloating needs the RunCommands grant (one new prompt named in
blast radius). Riskiest unproven mechanism: the cwd shape match between the
OS-resolved get_pane_cwd answer and agentsview's recorded cwd — the probe is
test plan item 1 and a CL step (sandbox blocks agentsview/live sessions).
Folds in the one-line F8 fix on the exact click lines this slice rewrites;
gate may strike it.
