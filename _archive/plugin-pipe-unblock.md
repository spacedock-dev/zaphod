---
title: Plugin pipe-unblock fix
status: done
source: plan sprint 0 (docs/plan-agent-rail.md)
score: 0.9
id: 2n00q0w1g33531ewx1azgw4e
started: 2026-07-07T04:49:25Z
worktree: .worktrees/spacedock-ensign-plugin-pipe-unblock
verdict: passed
completed: 2026-07-07T09:29:44Z
archived: 2026-07-07T09:29:44Z
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

## Validation evidence (fresh agent, 2026-07-07)

All re-execution and probes ran on a throwaway detached checkout of
0153787 (never the implementation worktree), since removed. Worktree
identity-checked at raw SHA 0153787354aacdd53e034606ed18b2061f62fcdf
(knob 4b9f098, fix 9332436 beneath, tree clean).

Per-AC verdicts, from my own runs:

- **AC-1 PASS.** Baseline drill at 4b9f098: `real 87.00` (bound ≥ 60s;
  implementer 87.02). Fix drill at 0153787: `real 11.97` (bound < 20s;
  implementer 11.96). Releases coincide with the wedge-rail drain
  instants in zellij.log to ~10ms (vbase release 14:01:54.598 vs pass
  end 14:01:54.59; vfix release 14:03:53.135 vs abort 14:03:53.133) —
  the rail was provably the last holder in both timed runs.
- **AC-2 PASS.** Suite re-run: 103 passed, 0 failed. Drill: baseline
  logged six back-to-back `wedge drill: sleeping 15s` per pass with no
  abort; fix logged exactly one sleep then
  `status pass aborted: pane 2 wedge-classified after 15005ms` per pass.
- **AC-3 PASS.** Suite green; plus a throwaway integration test driving
  `refresh_statuses` itself through two aborted passes (wedge knob = 1s):
  only the wedged pane's backoff entry exists after an abort, untried
  panes get no entry, pass 2 moves to the next due pane. Corroborated in
  wall clock: knob-0 passes recur at +4s/+8s/+16s — exactly
  `backoff_skips` growth — and the fix drill's pass 2 wedged pane 3, not
  pane 2.

Refutation audit — probes and outcomes:

1. **1s threshold, fast end (false positive):** 24 knob-0 drill calls
   (four full passes) and 60s of real `get_pane_running_command` polls
   over 6 sleep panes (~30 passes) produced zero abort traces. SURVIVES.
2. **Boundary:** unit tests pin 999ms-continue/1s-abort; drill margin is
   15s vs 1s; a hypothetical misclassification costs one stale pass plus
   one backoff entry that the next success clears. SURVIVES (bounded
   harm, self-healing).
3. **Starvation claim (queued pipe waits ≤ one wedge):** a second pipe
   fired mid-steady-state (backoffs grown, pass 2 in-flight) returned in
   `real 11.98` — the wedge remainder, not N wedges. SURVIVES.
4. **Backoff interaction on aborted passes:** integration probe above;
   `break` precedes the untried panes' `due()` calls, so their skip
   counters are not consumed. SURVIVES.
5. **Born-green test's contract claim:** mutation probe — removing
   `state.record(true)` from the wedge branch leaves all 103 shipped
   tests green; only the throwaway integration probe fails. The shipped
   test pins PollBackoff's contract, not the production wiring (the
   wiring is proven by drill + probe). FINDING: promote the probe test
   (`aborted_pass_leaves_untried_panes_backoff_untouched`, ~25 lines,
   no host calls) into the suite as a cheap follow-up.
6. **Doc diffs vs the refutation:** SPEC #5 append and plan sprint 0(a)
   replacement match the ideation text verbatim; all citations verified
   against upstream v0.44.1 source fetched at tag (pipes.rs:107-127
   release-requires-empty-set, zellij_exports.rs:795-796 caller-only
   insert, wasm_bridge.rs:1174-1184 dispatch marking, :602 unload-only
   cross-instance release, both drain sites) and the local
   zellij-tile-0.44.1 registry (shim.rs:1539-1546). ACCURATE.

**New finding (methodology + upstream docket candidate):** in client-less
scratch sessions (`attach --create-background`), CLI pipes that arrive
while all plugin instances are idle hang past 60s even though every
`pipe()` returned (4/4 probes: both rails' `pipe recv` traced, no release
ever logged; `--plugin`-targeted pipes hang identically); pipes that
arrive while the wedge rail is mid-sleep release exactly at its drain
(3/3, the AC-1 runs). `route.rs:80: Action CliPipe did not complete
within 1s timeout` logs in both cases. Not a fix defect — attached
sessions release idle-arrival pipes in 1-6s (2026-07-07 spike, PRD §4) —
but scripted drills MUST fire pipes ~5s after wedge-tab creation so they
land mid-first-pass (the AC-1 recipe already does). Second scratch quirk:
a wedge tab created as the session's first `new-tab` never renders or
polls (2/2); the AC-1 shape (healthy tab first, wedge tab last) renders
and polls (5/5). Both quirks belong in the upstream-issues docket
(task #5).

## Demo script (CL's live gate: AC-I1 / AC-I2)

Pre-demo, no CL time (~3 min):

1. Deploy the candidate at the production path — this is what makes
   AC-I2's no-prompt check real (same path, cached grant, new bytes):

       cd /Users/clkao/git/zaphod/.worktrees/spacedock-ensign-plugin-pipe-unblock
       ./build.sh
       cp target/wasm32-wasip1/release/zellij-sidebar.wasm \
          /Users/clkao/git/zaphod/target/wasm32-wasip1/release/zellij-sidebar.wasm

   Running sessions are untouched (in-memory path-keyed plugin cache);
   only sessions started afterward load the candidate. Revert on
   rejection: `./build.sh`.
   Interactive grant needed: **none** — the production path is already
   granted in permissions.kdl. (Only if CL insists on not touching the
   production artifact: point the layouts below at the worktree wasm
   instead, which then needs one interactive `y` in the wedge tab or a
   permissions.kdl re-seed for that path, restored after.)

2. Write the two demo layouts. `/tmp/demo-healthy.kdl` — same as
   `/tmp/demo-wedge.kdl` below but with the six sleep panes replaced by
   one, and no `wedge_poll_secs` line. `/tmp/demo-wedge.kdl`:

       layout {
           tab name="wedge" focus=true {
               pane split_direction="vertical" {
                   pane size=28 borderless=true name="sidebar" {
                       plugin location="file:/Users/clkao/git/zaphod/target/wasm32-wasip1/release/zellij-sidebar.wasm" {
                           rail "1"
                           debug "1"
                           wedge_poll_secs "15"
                       }
                   }
                   pane stacked=true {
                       pane command="sleep" {
                           args "3600"
                       }
                       // ... five more identical sleep panes ...
                   }
               }
           }
       }

3. Spot-check (~60s, scripted, zero interaction — proves drill infra at
   the production path before CL sits down). Short session name; healthy
   tab first; pipe 5s after the wedge tab so it lands mid-first-pass
   (both scratch-session quirks above):

       S=spot1
       zellij attach --create-background $S; sleep 1
       zellij --session $S action new-tab --layout /tmp/demo-healthy.kdl; sleep 1
       zellij --session $S action new-tab --layout /tmp/demo-wedge.kdl; sleep 5
       time zellij --session $S pipe --name agent-event -- '{"kind":"session","id":"spot"}'
       zellij kill-session $S; sleep 1; zellij delete-session $S --force
       grep -c "status pass aborted" \
           /var/folders/h1/vnssm1dj6ks4nzzvx8y29yjm0000gn/T/zellij-501/zellij-log/zellij.log

   Expect: `real` ≈ 12s and a nonzero abort count (log path is this
   machine's zellij log: `$TMPDIR`-based `zellij-<uid>/zellij-log/`).

Live demo (CL, ~5 min):

1. Start a fresh session as usual. **AC-I2:** no permission prompt as
   the rail loads; rows, clicks, and Alt-/ behave exactly as before.
2. Watch statuses refresh (~2s cadence) over live agents for a moment.
3. From any pane: `zellij action new-tab --layout /tmp/demo-wedge.kdl`.
   The wedge tab lands focused; its rail starts 15s-wedge passes.
4. **AC-I1 bounded-wait:** from a pane in that session (or outside with
   `--session <name>`):
   `time zellij pipe --name agent-event -- '{"kind":"session","id":"demo"}'`
   → returns in ≈ 12-16s (one wedge), where pre-fix the identical setup
   held it ~90s (first pass) — the offline AC-1 numbers, live.
5. **AC-I1 protected session:** switch back to the agents tab — statuses
   keep updating (the backgrounded wedge rail stops polling; give the
   in-flight wedge ≤ 15s to drain), Alt-/ toggles promptly, and a repeat
   manual pipe now returns in ~1-2s. Press Alt-/ from the agents tab —
   the toggle pipe's config identity (`rail "1"`) does not match the
   wedge rail's, so it can never queue behind a wedge.
6. Cleanup: close the wedge tab (sleeps die with it);
   `rm /tmp/demo-wedge.kdl /tmp/demo-healthy.kdl`. Approved → merge (the
   production path already carries the candidate). Rejected → revert per
   step 1.

## Stage Report: validation

- DONE: Every offline AC (AC-1, AC-2, AC-3) re-verified by re-execution: re-run the suite yourself AND re-run the scripted drill end-to-end (knob commit baseline + fix commit) recording YOUR OWN time outputs — the implementer's drill notes (wedge tab created last/focus=true, short session names, permissions.kdl seed + restore) are the recipe, not the evidence
  Suite 103/103 on a throwaway checkout of 0153787; my drill timings `real 87.00` (4b9f098) vs `real 11.97` (0153787), releases matching the rail-drain instants in zellij.log to ~10ms; permissions.kdl seeded from a backup and restored byte-identical.
- DONE: Refutation audit on a THROWAWAY checkout: attack the 1s threshold classification (boundary/jitter), the abort's starvation claim (does a queued pipe really wait ≤ one wedge), backoff interaction on aborted passes, the born-green test's contract claim, and the doc diffs' accuracy vs the refutation — each probe and outcome recorded
  Six probes recorded above: threshold/boundary/starvation/backoff SURVIVE (24 knob-0 calls + 60s real polls, zero aborts; steady-state pipe 11.98s; integration probe); born-green mutation exposed a shipped-suite wiring gap (finding: promote the probe test); all eight doc-diff citations verified against upstream v0.44.1. Plus one new finding: client-less scratch sessions flakily hang idle-arrival CLI pipes (zellij-side, 4/4 vs 3/3 separation) — drill methodology hazard and upstream docket candidate, not a fix defect.
- DONE: Demo script prepared for CL's live gate covering AC-I1/AC-I2: exact wedge-tab setup, the one interactive grant (or re-seed) it needs, what stays prompt (Alt-/, manual pipe, statuses), and a cheap spot-check that drill infra works before CL's time is spent
  Demo script section above: production-path deploy needs no grant at all (the honest AC-I2 shape); exact KDL + a 60s scripted spot-check that must run before CL sits down; prompt-surface checklist covers statuses, Alt-/ (config-identity isolation), and manual pipes in both wedge-tab postures.

### Summary

Re-executed everything independently on a throwaway checkout: suite
103/103, baseline 87.00s vs fix 11.97s with releases provably bound to
the wedge rail's drain, and a six-probe refutation audit in which every
attacked claim survived. Two findings for the gate: the born-green
backoff test does not pin the production wiring (a ready-made ~25-line
integration test closes it), and client-less scratch sessions carry a
zellij-side pipe-release flake plus a first-new-tab render quirk — both
documented for the upstream docket and encoded as constraints in the
demo script. Demo is parked ready for CL with a zero-interaction
spot-check ordered before any CL time.
