---
title: Plugin pipe-unblock fix
status: implementation
source: plan sprint 0 (docs/plan-agent-rail.md)
score: 0.9
id: 2n00q0w1g33531ewx1azgw4e
started: 2026-07-07T04:49:25Z
worktree: .worktrees/spacedock-ensign-plugin-pipe-unblock
---

## Problem

The `zellij pipe` CLI never exits after delivery when a wedged plugin
instance exists (2026-07-07 spike: verbatim receipt rendered by the live
rail, then 13s+ hang until killed). Mechanism, confirmed in zellij v0.44.1
server source during this ideation: every running instance a CLI pipe is
directed at is marked into the pipe's `currently_being_processed_by` set at
dispatch (`zellij-server/src/plugins/wasm_bridge.rs:1174-1184`), and the CLI
is released only when that set empties (`plugins/pipes.rs:107-127`). An
instance wedged in a `GetPaneRunningCommand` host call (~13s per call)
blocks its pinned thread and `RunningPlugin` lock, so its queued pipe-apply
cannot run; with several stuck panes one status-poll pass strings wedges
back to back (N×13s), and the grout piping rows into the rail — or any CLI
pipe — waits out the whole pass. The wedge bites CL's real sessions during
ordinary status polls, which is why this slice leads sprint 0. (The slice
title names the outcome — CLI pipes unblock promptly — not the mechanism,
which ideation revised below.)

## Proposed approach

**The planned explicit-unblock mechanism is refuted; a wedge budget on the
poll path is the fix.**

The seed approach (reinstate the `ReadCliPipes` grant + explicit
`unblock_cli_pipe_input` on every receipt) cannot protect against a wedged
instance, per v0.44.1 server source:

- The host call only inserts the pipe id into the **calling** instance's
  `input_pipes_to_unblock` (`plugins/zellij_exports.rs:795-796`), drained
  only when that instance's current handler returns (`plugins/pipes.rs:221-253`;
  drain sites: `pipes.rs:179-197` on pipe-return, `wasm_bridge.rs:2232-2248`
  on event-return).
- The drained Unblock clears the explicit-block flag and removes only the
  reporting instance from the processing set; release still requires the
  whole set empty (`pipes.rs:113-126`).
- So a healthy sibling cannot release a pipe a wedged instance holds, and in
  the self-wedged case the explicit unblock drains at the same instant
  auto-unblock fires — a no-op either way. The only cross-instance release
  is plugin unload (`wasm_bridge.rs:602`).

The 2026-07-07 spike corroborates this behaviorally: the healthy rail
rendered the payload (its `pipe()` returned, removing it from the set) and
the CLI still hung — exactly what the bookkeeping predicts. Dropping the
grant also removes the seed's one user-visible side effect (a permission
re-prompt in every session, since the cached grant would not cover the
expanded set).

What ships instead — a **wedge budget** in `refresh_statuses`
(`src/main.rs:668-690`), the plugin's only blocking-host-call site:

1. Time each `get_pane_running_command` call (`Instant`, already in use for
   `toggle_cooldown`, `src/main.rs:34,288`).
2. A call whose elapsed time reaches 1s is wedge-classified — new pure fn
   `wedge_aborts_pass(elapsed) -> bool`. Threshold from outside the file:
   healthy polls run every 2s with no visible lag (sub-timer returns), the
   observed wedge is ~13s; 1s sits well clear of both.
3. On a wedge-classified call: record the failure into that pane's
   `PollBackoff` (existing, `src/main.rs:88-116`) and abort the rest of the
   pass — the remaining panes keep their previous status (stale-not-blank,
   the same posture `should_poll_statuses` already chose) with their backoff
   state untouched, and a trace line marks the abort.
4. Queue-order consequence (the starvation killer): the per-instance pinned
   executor runs tasks FIFO and `set_timeout` re-arms only inside the Timer
   handler (`src/main.rs:362`), so a queued CLI pipe-apply sits immediately
   behind the in-flight pass. Bounding the pass at one wedge bounds any CLI
   pipe's wait at ≈ one wedge (~13-15s) instead of N×13s.

Drill knob (config-gated, ships in the wasm like `debug`):
`wedge_poll_secs "N"` sleeps N seconds in place of each
`get_pane_running_command` — the identical blocking layer (plugin pinned
thread under the `RunningPlugin` lock) — so the budget path is exercisable
end to end without waiting to catch a wild wedge. Parsed by a pure helper
mirroring `debug_enabled`.

Pure functions this design extends or joins: `should_poll_statuses`
(`src/main.rs:882`, unchanged — the visibility gate stays), `backoff_skips`
+ `PollBackoff::{due,record}` (`src/main.rs:868-872,94-116`, unchanged), new
`wedge_aborts_pass`, new `wedge_poll_secs` config parse.

## Riskiest-mechanism determination

- Planned mechanism (explicit unblock releasing a held CLI pipe): **refuted
  at source**, citations above, at the exact tag CL runs (zellij CLI 0.44.1
  per the plan header) — and corroborated by the 2026-07-07 spike's
  receipt-then-hang observation. Verified while at it: the shim itself is
  fire-and-forget (`zellij-tile-0.44.1 src/shim.rs:1539-1544`, no
  `bytes_from_stdin`), so landmine #4 was never the obstacle; the server
  bookkeeping is.
- Retained design: **no spike needed** — the proven mechanisms it relies on:
  wall-clock `Instant` timing inside the plugin (ships in `toggle_cooldown`),
  per-pane `PollBackoff` (shipped d9f99e8, tests at `src/main.rs:1674+`),
  the poll visibility gate (`should_poll_statuses`, tests at
  `src/main.rs:1660+`), CLI auto-unblock on `pipe()` return (SPEC landmine
  #5, verified against server source, spike-consistent), and the pipe
  transport end to end (2026-07-07 spike: daemon → SSE → grout →
  `zellij pipe` → rendered row, 1-6s, `docs/prd-agent-rail.md` §4).
- Residual uncertainty: whether a headless background session
  (`zellij attach --create-background`) renders the rail and fires polls
  (the drill needs render width ≥ 8 and TabUpdate). Settled first and
  cheaply by test-plan item 1; fallback named there.

## Acceptance criteria

Offline (agent-reproducible):

**AC-1 — value: bounded pipe-CLI exit time against the current hang.**
With a wedge rail (`wedge_poll_secs "15"`) over 6 long-lived panes
(`pane command="sleep"`) in its tab plus a healthy rail in a second tab of a
scratch session, piping 5s after the wedge tab is created:
`time zellij --session <drill> pipe --name agent-event -- '{"kind":"session","id":"drill"}'`
exits in under 20s at the fix commit (pass aborts at the first wedge); the
identical drill at the pre-fix commit (knob patch only) takes ≥ 60s — the
first pass alone is 6×15s — recorded as the measured time or "killed at
120s". The baseline is an independent timing that can move the wrong way: a
regression re-lengthens or re-hangs the exit.
Verified by: the scripted drill of test-plan items 1/4, with both `time`
outputs recorded in the implementation stage report.

**AC-2 — mechanism serving AC-1: at most one wedge per poll pass.**
A status-poll pass issues no further pane-status host calls after a
wedge-classified call.
Verified by: `cargo test` on the pure decision (proposed names
`wedge_classified_call_aborts_the_pass`, `fast_call_continues_the_pass`)
plus, in the drill, the abort trace line greppable in the drill session's
`zellij.log` (evidence external to the file under test).

**AC-3 — mechanism: backoff still owns the wedged pane.**
The wedge-classified pane's failure lands in its `PollBackoff` (skips grow
per `backoff_skips`) and a later success clears it; the untried panes'
backoff state is untouched by an aborted pass.
Verified by: `cargo test` (existing `poll_backoff_skips_grow_and_cap` plus a
wedge-outcome case), full suite green (95+ tests).

Interactive (settled only by CL's live demo):

**AC-I1 — live sessions protected, poll value intact.** In CL's fresh
session with the rail docked over live agents: statuses keep updating, and
Alt-/ plus a manual `zellij pipe` stay prompt even while a wedge-knob tab
exists in the session.
Verified by: CL's demo per the validation stage's demo script.

**AC-I2 — blast radius: nothing else moved.** No new permission prompt
after the wasm hot-swap (the grant set is unchanged); rows, keybinds, and
layout behavior unchanged.
Verified by: CL confirms during the same demo.

## Test plan

1. **First — smallest end-to-end check that would invalidate the design:
   the pre-fix baseline drill.** Patch only the wedge knob onto the pre-fix
   tree, build, script the scratch session (`zellij attach --create-background
   <drill>`; `zellij --session <drill> action new-tab --layout` a drill
   layout carrying the knobbed rail + 6 sleep panes; healthy tab second),
   wait 5s, run the timed pipe. Expected: exit ≥ 60s or killed,
   reproducing the spike class and proving the headless drill surface
   (render + polls in a background session) works. ~30 min. If it does NOT
   hang, either the wedge/dispatch model is wrong or the source-level
   refutation is (a healthy sibling released it) — stop and re-analyze
   before implementation spends anything. If polls never fire headless,
   AC-1 degrades to interactive (CL runs the two timed commands live) and
   the offline set keeps AC-2/AC-3.
2. TDD the pure fns (red first): `wedge_aborts_pass` threshold cases,
   `wedge_poll_secs` parse cases, the wedge-outcome backoff case.
3. Implement the budget in `refresh_statuses` (~6 lines) + the abort trace
   line; `cargo test` + `cargo check --tests`.
4. Re-run the drill at the fix commit; record both timings against AC-1.
5. Park demo-ready for CL (AC-I1/AC-I2), per the workflow's park-for-demo
   posture.

## Proposed doc diffs (applied at implementation, reviewed at this gate)

- `SPEC.md` landmine #5, append: "A CLI pipe stays blocked until every
  directed instance's `pipe()` has returned: each instance is marked at
  dispatch (`wasm_bridge.rs:1174-1184`), and an explicit
  `unblock_cli_pipe_input` from a sibling cannot release it — the unblock
  only clears the explicit-block flag and the caller's own membership;
  release requires the processing set empty (`pipes.rs:107-127`). The only
  cross-instance release is plugin unload (`wasm_bridge.rs:602`). Explicit
  unblock is therefore useless against a wedged sibling (verified v0.44.1;
  corroborated by the 2026-07-07 receipt-then-hang spike)."
- `docs/plan-agent-rail.md` sprint 0 (a), replace the final sentence ("It
  reinstates the `ReadCliPipes` grant + explicit unblock …") with:
  "Ideation refuted the explicit unblock (a sibling's unblock cannot
  release a pipe held by a wedged instance — SPEC #5); the fix is a wedge
  budget on the status-poll path: one wedge-classified
  `GetPaneRunningCommand` call aborts the pass, bounding any CLI pipe's
  wait to ≈ one wedge."

## Blast radius on CL's live sessions

- Running sessions: untouched until restart — the compiled-plugin cache is
  in-memory and path-keyed (landmine #1); the new wasm loads only in
  sessions started after `./build.sh`.
- After deploy: no permission re-prompt (grant set unchanged — the seed's
  `ReadCliPipes` would have re-prompted once per plugin identity; dropping
  it removes that). No keybind, row, or layout change.
- Poll behavior: on a wedge, the rest of that pass's statuses go
  stale-not-blank for ≥ 1 timer (2s) — the same degradation direction the
  shipped backoff already chose; wedge-free tabs pay one `Instant` read per
  polled pane and nothing else.
- Failure containment: worst case post-fix, a CLI pipe waits ≈ one wedge
  (~13s) instead of N×13s or never; the rail's own event queue drains
  behind at most one wedge, so toggles and renders stay live.

## Out of scope

- The grout-side kill timer and fire-and-forget posture (sprint 0 slice b).
- `agent-event` row parsing, rendering, and click semantics (slice c).
- Upstream zellij fix (explicit unblock ignoring the processing set
  contradicts `block_cli_pipe_input`'s "this or another plugin" doc,
  `zellij-tile shim.rs:1546`) — goes to the upstream-issues docket
  (task #5), not this slice.
- Reaping wedged or zombie background instances (only plugin unload
  releases their held pipes; unreachable from a sibling plugin).
- Shortening `GetPaneRunningCommand`'s ~13s server-side stall itself.
- Review findings F1-F10 — none touch `refresh_statuses` or `pipe()`
  (checked against `docs/review-findings-2026-07-07.md`).

## Stage Report: ideation

- DONE: ACs split offline vs interactive, with at least one value-measuring AC (e.g. bounded pipe-CLI exit time against the current hang) whose baseline can move the wrong way
  AC-1 times the pipe-CLI exit (<20s fix vs ≥60s/killed baseline, both recorded); AC-2/AC-3 are the mechanism ACs serving it; AC-I1/AC-I2 are CL-demo-only.
- DONE: Riskiest-mechanism determination on the record: spike result or "no spike needed: {proven mechanisms}" — the 2026-07-07 spike evidence counts if cited precisely
  Seed mechanism refuted at v0.44.1 source (pipes.rs:107-127, zellij_exports.rs:795-796, wasm_bridge.rs:1174-1184) and corroborated by the spike's receipt-then-hang; retained design recorded as "no spike needed" over five named proven mechanisms.
- DONE: Wedge mitigation designed against the GetPaneRunningCommand poll path, with the change's blast radius on CL's live sessions named
  Wedge budget in refresh_statuses (1s classification, one wedge aborts the pass, backoff unchanged); blast radius section names cache-gated rollout, no re-prompt, stale-not-blank statuses, and the ~13s worst-case pipe wait.

### Summary

Ideation reversed the slice's seed mechanism: the v0.44.1 server source
shows an explicit `unblock_cli_pipe_input` cannot release a CLI pipe held
by a wedged sibling (release requires the dispatch-time processing set to
empty), so the `ReadCliPipes` grant is dropped and the fix is a wedge
budget bounding the status-poll pass to one wedge-classified
`GetPaneRunningCommand` call. The design extends the shipped PollBackoff
machinery, adds a config-gated drill knob, and leads the test plan with a
pre-fix baseline drill that would invalidate either the wedge model or the
refutation before implementation starts. Doc diffs for SPEC #5 and the
plan are proposed for gate review.

## Stage Report: implementation

- DONE: Red-first evidence recorded for each new pure fn (wedge_aborts_pass, wedge_poll_secs parse) — failure output with the predicted reason in the stage report
  Both went red with the predicted reason (fn not yet written): `error[E0425]: cannot find function `wedge_poll_secs` in this scope` → green in 4b9f098; `error[E0425]: cannot find function `wedge_aborts_pass` in this scope` → green in 9332436.
- DONE: AC-1 drill outcome recorded: pre-fix vs post-fix `time` outputs (or the headless-render fallback taken with reason) — settle test-plan item 1 cheaply FIRST
  Run first, at the knob-only commit 4b9f098: `real 87.02` (≥60s bound met; exactly one 6×15s pass, 6 sleep traces). Post-fix at 9332436: `real 11.96` (<20s bound met). No fallback needed — the headless background session rendered the rail and fired polls.
- DONE: Doc diffs applied as designed: SPEC #5 and docs/plan-agent-rail.md sprint 0(a) corrected off the refuted mechanism
  0153787 applies both diffs verbatim from the ideation body (SPEC #5 append; sprint 0(a) final-sentence replacement).

### Summary

Three commits on spacedock-ensign/plugin-pipe-unblock: 4b9f098 (wedge_poll_secs
drill knob — sleeps in place of get_pane_running_command, plus a per-sleep trace
line), 9332436 (wedge budget — a call stalling past WEDGE_THRESHOLD=1s records
into its pane's PollBackoff and aborts the pass with a trace line), 0153787
(doc diffs). Suite 99 → 103 (`cargo test`), `cargo check --tests` clean; new
tests: wedge_drill_runs_only_with_a_numeric_wedge_poll_secs_value,
wedge_classified_call_aborts_the_pass, fast_call_continues_the_pass,
wedge_outcome_lands_in_the_panes_backoff_and_a_success_clears_it (the last
documents the wedge→backoff contract over shipped PollBackoff behavior, so it
was born green — red-first evidence covers the two new pure fns).

AC-2 drill evidence: post-fix the log gained exactly one "wedge drill: sleeping"
line per pass, and `status pass aborted: pane 2 wedge-classified after 15001ms`
is greppable in the drill session's zellij.log.

Drill notes for validation: the scripted drill created the wedge tab LAST
(focus=true) so its rail owns the visibility-gated poll — creating the healthy
tab last would gate the wedge rail's polls off and void the drill. Session
names must stay short (macOS 103-byte IPC socket cap). The drill wasm path was
pre-granted by seeding permissions.kdl (SPEC #9) and the entry was removed
after both runs — CL's permission cache is as found; a live demo of the wedge
tab needs a re-seed or one interactive grant for the drill wasm path. Parked
demo-ready for AC-I1/AC-I2 per the workflow's posture.
