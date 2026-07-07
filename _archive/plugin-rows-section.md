---
title: Plugin rows section
status: done
source: plan sprint 0 (docs/plan-agent-rail.md)
score: 0.8
id: 5tdw8rckcrsx1qfytsytgp9m
started: 2026-07-07T05:32:39Z
worktree: .worktrees/spacedock-ensign-plugin-rows-section
verdict: passed
completed: 2026-07-07T15:48:37Z
archived: 2026-07-07T15:48:37Z
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

## Stage Report: implementation

- DONE: Red-first evidence recorded for every new pure fn (parse_agent_event, apply_agent_event, bind_session, brief_path_for_log, section_layout, marker_for_state) and the extended click math — each failure output with its predicted reason
  13 commits 9f4f679..b83b302 on spacedock-ensign/plugin-rows-section, each red-first; red outputs listed below.
- DONE: Zero-footprint AC held: every pre-existing test passes unmodified, and a no-events rail renders byte-identically to today
  104 tests before → 130 after, all green (cargo test + cargo check --tests + cargo check, 0 warnings); `git diff 09a4710..HEAD` removes zero test lines; render's section blocks are non-empty-guarded after the untouched pane-region prints, and section_layout(P,0,0).target ≡ target_for_line on every line (test empty_sections_map_like_the_pane_only_rail).
- DONE: Binding lands behind a normalization seam (exact-match now; the cwd-shape probe is unrun — the seam takes a rule without redesign) and the added get_pane_cwd call rides the merged wedge budget; F8 exit-nav fix included per CL
  normalize_cwd is the seam — exact match today, probe pending, seam ready: a probe-pinned rule slots in without touching bind_session's callers. The cwd call is one more timed call in refresh_statuses under WEDGE_THRESHOLD: wedge-classified → backoff recorded + pass aborted; failed call keeps the previous entry (b2151d8). F8: FocusPane and FloatGate both exit nav on click-through (9f4f679, a03a20e).

### Red evidence (test → failure, each observed before its fix)

- row_click_during_nav_mode_exits_nav → panicked "click-through must drop nav mode" (handle_click never exited nav)
- parses_both_pinned_row_kinds (+2 parse tests) → E0425/E0422: cannot find parse_agent_event / AgentEvent / SessionEvent / GateEvent
- session_upsert_replaces_by_id, gate_upsert_replaces_by_log_path_keeping_insertion_order → E0425: cannot find apply_agent_event
- grout_state_strings_map_onto_marker_states → E0425: cannot find marker_for_state
- single_match_binds, no_match_renders_unbound, ambiguous_cwd_renders_unbound, stale_cwd_entries_outside_the_row_set_never_bind → E0425: cannot find bind_session
- brief_path_inverts_the_decision_log_suffix → E0425: cannot find brief_path_for_log
- sectioned_lines_map_headers_rows_and_beyond_for_p2_s1_g1 (+2 layout tests) → E0425 section_layout; E0599 no variant SessionRow/GateRow on LineTarget
- session_row_click_focuses_only_when_bound, gate_click_floats_tui_on_brief, bad_log_suffix_never_floats → E0425 decide_rail_click; E0599 no variant FloatGate on ClickAction
- agent_event_lines_land_as_session_and_gate_rows, unknown_kind_dropped, malformed_json_dropped, missing_payload_dropped → E0609: no field sessions/gates on Sidebar
- cwd_map_prunes_to_live_rows → E0609: no field pane_cwds on Sidebar
- pinned_protocol_lines_render_and_bind, unbound_session_rows_carry_the_unbound_tag → E0425: cannot find session_row_line / gate_row_line / gate_row_detail / state_glyph
- session_row_click_through_the_rail_focuses_and_exits_nav → panicked "the session click must reach FocusPane" (handle_click still routed pane-only)

### Deviations from the entity body

- Row gains no cwd field: pane cwds live in a Sidebar-level pane_cwds map keyed by pane id. Pre-existing tests build Row with full struct literals, so a new field would force modifying them — the zero-footprint checklist item wins. Semantics preserved: survives PaneUpdate by keying, pruned to live rows each poll, stale-not-blank on failure, and bind_session matches listed rows only (stale map entries never bind).
- decide_click/target_for_line keep their shipped signatures (pre-existing tests call them); the extension is SectionLayout::target + decide_rail_click, which delegate the pane region to the shipped fns so the two maps cannot diverge.
- Gate rows wear the Blocked marker — the entity pinned no gate marker; a pending gate waits on a verdict.

### Summary

The display half is in: pipe() parses grout's two pinned kinds ahead of toggle handling with the recv trace kept first, upserts into AGENTS/GATES sections that occupy zero lines while empty, binds sessions by exact cwd match behind the normalize_cwd seam fed by a budget-bounded get_pane_cwd poll, and acts per kind — bound session → focus_terminal_pane, gate → float `subspace-tui <brief> --log <log>` under the newly requested RunCommands grant (one new prompt, per blast radius). The cwd-shape probe (test plan item 1) remains CL's live step; a divergent shape lands as a rule in normalize_cwd. The cwd wedge-abort branch mirrors the adjacent command wedge branch line for line; the drill knob aborts the pass before reaching it, so it is verified by inspection plus the existing budget tests. Wasm not built — no live demo in this stage.

## Refutation audit — validation, throwaway checkout @b83b302

Throwaway: `git clone` of the repo into the session scratchpad, detached at
b83b3024e675f6748745418fd393d6b7467430ec (identity = the implementation
worktree's HEAD, re-checked clean before and after the audit — never the
worktree itself). Baseline there first: pre-slice suite at 09a4710 is
104/104 and the slice tip is 130/130, both my own runs. Probes live in five
throwaway `audit_*` tests appended to `mod tests`, plus two audit-only
seams; all pass. No REFUTED findings; one cross-slice gap and two caveats.

1. **The cwd wedge-abort branch (by-inspection in implementation).** Why it
   cannot be exercised unmodified: the shipped drill knob sleeps at the
   *status* call, which wedge-classifies and breaks the pass before the cwd
   call is reached; and the real `get_pane_cwd` is a wasm host import
   (`#[link(wasm_import_module = "zellij")]`, zellij-tile-0.44.1
   shim.rs:2823-2826) with no native host behind it. Throwaway seam: a
   `wedge_cwd_secs` knob substituting only the host call (branch under test
   verbatim), with `wedge_poll_secs = Some(0)` making the status call an
   instant no-abort Err. Probe: 3 rows, pane 1 pre-seeded cwd `/prev`, cwd
   call wedges → pane 1's backoff records the failure, panes 2-3 get no
   backoff entry (abort spares untried panes), `/prev` survives
   (stale-not-blank), the pass reports no render-worthy change; pass 2 skips
   backed-off pane 1 and wedges pane 2. Identical posture to the status-call
   twin. SURVIVES.
2. **FloatGate argument handling under hostile log_path.** Probe: six
   hostile paths (embedded spaces, `'` and `"`, `;|&`, `$( )` and
   backticks, KDL `{}`+backslash, a raw newline) fed through the real
   `pipe()` → `decide_rail_click` — every one lands in
   `ClickAction::FloatGate` as two discrete verbatim strings (brief
   inverted, log untouched). Host path source-verified end to end: the shim
   protobuf-encodes `CommandToRun` (zellij-tile-0.44.1 shim.rs:622-631), the
   server unpacks `args` as a `Vec<String>` into `RunCommandAction`
   (zellij-server-0.44.1 zellij_exports.rs:2221-2259; permission gate
   `OpenCommandPaneFloating → RunCommands` at :5183-5190), and the exec is
   `std::process::Command::new(cmd.command).args(&cmd.args).spawn()`
   (os_input_output_unix.rs:211-232) — an argv vector, no shell at any hop.
   SURVIVES. Caveat: a hostile path also rides into the `float gate brief=`
   trace line and the floating pane's title — cosmetic surfaces only.
3. **Upsert unbounded growth (no expiry).** Probe, measured: 10,000 distinct
   session lines through `pipe()` ingest in 634ms total (~63µs each; the
   O(n) find makes ingest O(n²) overall), worst-case single re-upsert
   (last id) 129µs, and one render-shaped bind pass (10k sessions × 6
   listed rows) 12.4ms — native debug build; wasm will be slower by a
   small factor. Nothing hangs or panics; memory is a few MB of small
   structs. SURVIVES for sprint-0 scope (grout is one-shot, a handful of
   rows), with the caveat that a chatty sprint-1 grout makes per-render
   bind cost linear in sessions × rows and render output linear in rows —
   the entity's row-expiry/lifecycle line item is load-bearing, not
   optional polish.
4. **Binding flips under stale pane_cwds entries.** Probe: exhaustive
   oracle over listed rows {1,2} × cwd assignments from a 3-value pool × a
   stale map entry for closed pane 99 × 4 session cwds (108 cases +
   broadened equivalence) — `bind_session` returns a pane iff it is a
   LISTED row and the ONLY listed match; the stale 99 entry never binds and
   never breaks a legitimate single match (it can only force ambiguity when
   its cwd is irrelevant — it is filtered by the row set first). SURVIVES.
   Residual flip surface is spec'd behavior: a live row's stale cwd value
   (failed poll) binds by last known value per AC-2, and a second pane
   entering the session's cwd flips bound → unbound at the next poll
   (never-guess wins).
5. **Click math vs render order (drift attack).** Probe: an independent
   render-order walk (header, 2-line pane rows, conditional AGENTS/GATES
   headers + 2-line rows) rebuilt per shape and compared against
   `section_layout().target()` on every line, all P/S/G in 0..4, lines
   -3..40, plus zero-footprint equivalence broadened to P in 0..8, lines
   -5..60 (the shipped test covers P=2 only). Byte-for-byte agreement,
   including out-of-range and negative lines. SURVIVES.

Cross-slice gap (gate finding, not a defect in this slice's letter): grout
emits agentsview's `termination_status` verbatim as `state`
(grout/rows.go:39 — e.g. `awaiting_user`), while `marker_for_state` maps
only `blocked|working|idle|done`; every real live session row therefore
renders the Unknown marker (two spaces — no dot, no color). Both entities
scoped state-value mapping to sprint 1, so sprint 0 ships an AGENTS marker
that carries no live signal. AC-1's marker clause holds only for the plan's
fixture vocabulary. Named in the demo script so CL is not surprised.

## Demo script — sprint-0 exit gate (AC-I1 + AC-I2), CL drives

Prerequisites (once, before any session):

1. `subspace-tui` is NOT currently on PATH (checked on this machine).
   Install it in the shell that will launch zellij (the server inherits
   that shell's PATH):

       cd ~/git/spacedock-research/spacedock-subspace
       go build -o /opt/homebrew/bin/subspace-tui ./cmd/subspace-tui
       which subspace-tui   # must resolve before starting the session

2. Build + install the plugin from this branch:

       cd /Users/clkao/git/zaphod/.worktrees/spacedock-ensign-plugin-rows-section
       ./build.sh && ./install.sh

   Then add `debug "1"` beside each `rail "1"` in the installed
   `~/.config/zellij/layouts/zaphod.kdl` — the drop-reason traces and the
   AC-5 receipt check need it; rendered rows are the primary observable.
3. Pick a LIVE session id: `agentsview session list | head`. grout's no-arg
   default id is the synthetic fixture id — not in the real DB; always pass
   a real id. Note its cwd:
   `agentsview session get <id> --format json | jq -r .cwd` — the demo tab
   must hold exactly ONE pane whose cwd is that directory (binding is
   per-tab and exactly-one-match in sprint 0).

Spot-check (seconds, proves the infra before CL's time is spent):

4. Start a fresh zellij session on the zaphod layout. EXPECT exactly one
   permission prompt whose set now includes RunCommands — grant it. That
   observation is half of AC-I2; note anything beyond one prompt.
5. Inside the session: `timeout 10 zellij pipe --name agent-event -- ping; echo exit=$?`
   Expect exit=0 quickly (pipe-unblock posture holds), the rail visually
   unchanged, and in `${TMPDIR%/}/zellij-$(id -u)/zellij-log/zellij.log`
   two trace lines: `pipe recv name=agent-event` then
   `agent-event dropped: …` naming the parse reason — the branch is live
   and garbage has zero footprint. Any hang or missing trace: stop, fix
   infra (debug key / stale plugin), do not spend demo time.

cwd-shape probe (test plan item 1 — run BEFORE the click drill; its result
explains the drill's outcome):

6. In the tab holding the agent's pane:

       zellij action list-panes --all --json | jq -r '.[] | "\(.id) \(.pane_cwd)"'
       agentsview session get <id> --format json | jq -r .cwd

   Compare the agent pane's two cwd strings. Match → binding has real data.
   Normalizable divergence (e.g. `/private` prefix) → record BOTH strings
   verbatim in this entity; the rule lands in `normalize_cwd` (the seam
   takes it without redesign) and the session row will show ` ·unbound`
   until it does. No usable cwd → stop; back to ideation (plan decision 1
   loses its data source).

AC-I1 — rows render, clicks land:

7. Inside the session, from the worktree root:
   `time go run ./grout <session-id>` — expect exit 0, wall time ≤10s
   (spike baseline 1-6s). Rail gains within that window:
   - inverse-video `▾ AGENTS` header; session row `claude` (NO state dot —
     known sprint-0 gap: grout sends termination_status verbatim, the
     plugin maps only blocked|working|idle|done, so live rows wear the
     Unknown two-space marker) + dim first-message summary;
     ` ·unbound` appears only on zero/ambiguous cwd match;
   - inverse-video `▾ GATES` header; gate row red ● +
     `Playground handoff demo`; dim detail `review r1 · approve`.
8. Click-to-focus: click the session row → focus moves to the cwd-bound
   agent pane (post-click focus is the baseline; it can move the wrong
   way). If the row shows ` ·unbound`, the click must do NOTHING — that is
   spec'd never-guess behavior; record which outcome with the step-6
   result.
9. Gate-row float: baseline first —
   `wc -l grout/testdata/playground-gate.decisions.jsonl`. Click the gate
   row → a floating `subspace-tui` opens on the playground brief. Issue a
   verdict in the TUI → the log gains exactly one line (`wc -l` again),
   written by the TUI, never the rail. This DIRTIES the vendored fixture:
   restore with `git checkout -- grout/testdata/playground-gate.decisions.jsonl`
   (or demo against a real gate log via `go run ./grout <id> <log>`).

AC-I2 — blast radius (alongside the above):

10. Pane rows, statuses, Alt-/ toggle, nav mode, and layout behave exactly
    as deployed — with and without agent-event traffic; the only new
    prompt was step 4's RunCommands.

grout AC-5 payload probes (parked from the grout slice; ride this session,
one size at a time so receipt is attributable):

11.     timeout 10 zellij pipe --name agent-event -- "$(python3 -c 'print("x"*4096)')"
        timeout 10 zellij pipe --name agent-event -- "$(python3 -c 'print("x"*65536)')"
        timeout 10 zellij pipe --name agent-event -- "$(python3 -c 'print("x"*262144)')"

    After each: one `pipe recv name=agent-event` + one
    `agent-event dropped: …` trace pair (x-runs are malformed JSON — state
    untouched, rail unchanged; AC-3 robustness re-proven live at size).
    Record the ceiling (or "≥256 KiB") in the grout entity per its AC-5.

## Stage Report: validation

- DONE: Every offline AC re-verified by re-execution in the worktree (your own runs: full suite, the zero-footprint diff check, the section_layout/target_for_line equivalence) — never the implementer's numbers
  My runs at b83b302: full suite 130/130; the 13 AC-named tests green individually; `cargo check --tests` + `cargo check` warning-free; `git diff 09a4710..HEAD -- src/` removes zero test lines (all 8 removed lines are extended production lines); pre-slice baseline 104/104 re-run myself at 09a4710; equivalence re-run shipped (P=2) and broadened to P 0..8 × lines -5..60 in the audit — all agree.
- DONE: Refutation audit on a THROWAWAY checkout, priority attacks: the by-inspection cwd wedge-abort branch (find a way to exercise it — e.g. a knob variant or a unit seam — or prove why not), FloatGate argument handling with hostile log_path values (spaces, quotes, KDL/shell metacharacters — args ride as a vec but prove it), upsert unbounded growth (no expiry — what does 10k session lines do), binding flips under stale pane_cwds entries
  Five probes on a scratchpad clone at b83b302, section above: cwd wedge-abort exercised via a cwd-site knob seam (branch verbatim; why-not-unmodified proven — wasm-only host import + the status knob aborts first); hostile log_paths ride verbatim as discrete argv strings with the no-shell exec chain source-cited; 10k sessions = 634ms ingest / 12.4ms per bind pass, quantified not hung; binding survived a 108-case exhaustive oracle. No REFUTED. One cross-slice gate finding: live state strings (termination_status) all map to the Unknown marker — sprint-0 AGENTS markers carry no live signal; both entities scoped the mapping to sprint 1.
- DONE: Demo script for CL's joint sprint-0 exit gate: build+install from this branch, grout run, expected rendered AGENTS/GATES rows, click-to-focus drill, gate-row float drill (subspace-tui on PATH prerequisite), the RunCommands one-prompt expectation, the cwd-shape probe folded in, grout's parked AC-5 payload probes riding along — with a cheap spot-check first so CL's time is never spent on broken infra
  Demo script section above: subspace-tui install verified MISSING from PATH today (build step given); grout must get a REAL session id (its default is the synthetic fixture id, absent from the live DB); expected row text pinned from the fixtures (`Playground handoff demo` / `review r1 · approve`); float drill dirties the vendored fixture (restore step included); cwd probe ordered before the click drill so an unbound outcome is explained, not mysterious; AC-5 sizes verbatim from the grout entity.

### Summary

Independently re-verified all four offline ACs at b83b302 — suite 130/130,
zero-footprint diff and broadened click-map equivalence all from my own
runs. Five-probe refutation audit on a scratchpad clone: no REFUTED; the
previously by-inspection cwd wedge-abort branch now has executed evidence
via a throwaway seam, the FloatGate argv path is shell-free end to end at
source, 10k-row growth is quantified (degradation, not hang — sprint-1
expiry is load-bearing), and binding survived an exhaustive oracle. One
gate-worthy cross-slice finding: live session rows will all wear the
Unknown (blank) state marker because grout emits termination_status
verbatim — named in the demo script (step 7) so the joint demo reads
correctly. Demo is parked ready for CL with prerequisites verified against
this machine (subspace-tui missing from PATH today; grout needs a real
session id) and a seconds-cost spot-check ordered before any real drill.

### Demo outcome (CL, 2026-07-07)

Live demo PASSED; gate approved. Verbatim facts from CL's run:

1. **AC-I1 PASS** — grout run exit 0, AGENTS + GATES rows rendered in the
   rail, session-row click jumped focus to the cwd-bound pane. Gate-row
   float and the AC-5 size probes were skipped by CL (optional extras;
   they ride the sprint-1 dogfood).
2. **cwd-shape probe (test plan item 1) SETTLED**: `pane_cwd` and the
   agentsview cwd are byte-identical `/Users/clkao/git/zaphod` —
   `normalize_cwd` stays identity. ALSO observed: raw per-read `pane_cwd`
   is flaky (same pane null one read, real value the next) —
   stale-not-blank smooths it, but sprint 1 should debounce the unbind
   direction.
3. **AC-I2 PARTIAL**: the expected one permission prompt NEVER RENDERED
   (SPEC #7 landmine confirmed live — the plugin parked silently, all
   events denied); the grant was seeded manually in `permissions.kdl`;
   everything else unchanged post-grant.
4. **Demo-script defects found live, for the record**: the worktree
   `install.sh` split the plugin identity from the `config.kdl` keybind
   (the production-path cp is the correct deploy pattern);
   `go run ./grout` from any root fails (the module lives in `grout/`);
   grout's default gate-log path is cwd-relative and never resolves under
   `go run .` — absolute-path argv[2] is the workaround, a grout fix is
   seeded for sprint 1.
