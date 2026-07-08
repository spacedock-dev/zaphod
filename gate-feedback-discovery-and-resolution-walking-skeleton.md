---
id: pzw1ctjmej8bjy9t4td91yy8
title: Gate feedback discovery and resolution automation — walking skeleton across the sprint 2/3 seam
status: validation
source: finding — live session dogfooding, 2026-07-08 (CL left subspace-tui feedback that sat unread with no automated consumer)
started: 2026-07-08T08:48:18Z
completed:
verdict:
score: 0.85
worktree: .worktrees/spacedock-ensign-gate-feedback-discovery-and-resolution-walking-skeleton
issue:
pr:
mod-block:
---

## Problem

Live tonight: CL clicked a gate row, floated `subspace-tui`, left a comment,
quit. The feedback landed correctly in
`dock-toggle-restructures-panes.decisions.jsonl` — but nothing consumed it.
It sat unread until CL explicitly asked "who consumes the response?" and the
FO went and manually `cat`'d the file. There is no automated discovery or
resolution path today; this is the documented, intentional current state,
not a regression:

- `docs/plan-agent-rail.md`'s **Dogfood posture** (line 92-94): "Artifact
  review moves into the fresh zellij session starting now: pre-sprint-2
  gates reviewed there manually via `subspace-tui`; sprint 2 automates the
  discovery."
- **Sprint 2 — "gates for real + review dogfood"** (line 67-76): "Glob
  watching per the config; fold via the subspace binary — never reimplement
  fold... Exit: a real zaphod dev gate resolved via a rail-surfaced review."
- **Sprint 3 — "M2 seam"** (line 78-85): "The subspace gate server writes
  `<log>.addr` beside the log (additive, no model change); rail verdict
  actions — approve / revise / reject / hold — POST to it. Direct log
  append stays forbidden (single-writer flock). Exit: a verdict issued from
  the rail lands on the record and unblocks the waiting agent."

Both sprints are prose in a plan document, not filed tasks. This entity
files the work and designs it as a **walking skeleton**: the thinnest slice
that proves the *entire* loop — glob-watch → fold → surface in the rail → a
rail verdict action → POST to a gate server → unblock the waiting/polling
agent — closes end-to-end, before either sprint's full generality (many
concurrent gates, all four verdict types, arbitrary glob config) is built.

## Recon findings — the plan vs. subspace at HEAD 9be5fbc

Ideation reconned `spacedock-subspace` (`~/git/spacedock-research/spacedock-subspace`,
HEAD `9be5fbc`, matches the plan's recon). Three plan assumptions are
inaccurate against the shipped code and reshape the skeleton:

1. **`<log>.addr` does not exist and nothing writes it.** The plan's Sprint-3
   sentence "the subspace gate server writes `<log>.addr` beside the log" is
   unbuilt. A gate server advertises its address **only on its own stderr** as
   one JSON line — `{"status":"open","url":"http://127.0.0.1:PORT/"}`
   (`cmd/spacedock-subspace/main.go:123-128`). The only sidecar file beside the
   log is `<log>.lock` (the flock PID). So the rail cannot address a gate server
   via a `.addr` file today; the address must come from the launching process's
   stderr.

2. **`subspace-tui` has no verdict path — it is feedback-only.** Its sole
   decision write is `AppendFeedback` (`cmd/subspace-tui/pin.go:114`); it never
   produces a verdict and does not take the flock. So the current rail gate-row
   click (float `subspace-tui`, `src/main.rs:740-759`) can leave a *comment* but
   can never *issue a verdict* — exactly what happened live tonight. A verdict
   (approve/revise/reject/hold) is produced **only** by POSTing to a running
   `spacedock-subspace <brief>` gate server (`POST /api/verdict`,
   `internal/server/server.go:140,203-255`). The skeleton therefore introduces a
   genuinely new rail capability distinct from the existing float-review click.

3. **The gate server is ephemeral and in-process, not a daemon.** It *is* the
   `spacedock-subspace <brief>` process: binds `127.0.0.1:0` (random port),
   serves the review UI, **blocks on a Go channel** waiting for its own POST, and
   dies the instant the gate resolves (`main.go:118-152`). A direct file append
   would wake a separate `wait` poller but **not** a live server, and would
   violate the single-writer flock the server holds — so the rail must resolve
   via POST, never by appending (matches the plan's "direct log append stays
   forbidden").

The canonical "waiting agent" the PRD describes ("holds no socket open... polls
that same log") is a `spacedock-subspace wait <gate>` poller (2s fold loop,
`cmd/spacedock-subspace/wait.go`); it is exactly what the FO gate-loop runs
(`docs/dev/gate-loop-recipe.md` step 2), and it resumes on *any* terminal event
in the log regardless of writer. It is the real waiter, not a mock.

## Proposed approach

Thinnest slice that exercises every layer once. Cuts confirmed against the
walking-skeleton principle (not treated as fixed by the brief):

- **One gate, one verdict — `approve`.** Prove approve unblocking a waiter
  before revise/reject/hold.
- **Discovery: naive periodic glob (accepted cost, CL's direction).** grout
  gains a poll loop that re-globs a configured gates location for
  `*/decision-log.jsonl` / `*.decisions.jsonl` on a timer and (re)emits gate
  rows. The accumulating-`.jsonl` cost and the non-event-driven poll are named,
  accepted costs for this pass, not the final mechanism.
- **Fold via the subspace binary as a black box.** grout shells
  `spacedock-subspace wait <gate>` (the black-box fold reader — returns the fold
  instantly once resolved) for resolution status; gate *identity* fields
  (entity/stage/round/recommendation) continue to come from the brief YAML as
  the Sprint-0 grout already does (`grout/gate.go`). No fold logic is
  reimplemented.
- **Address discovery for the skeleton = capture the server's stderr URL.**
  Because `<log>.addr` does not exist, for the one skeleton gate grout launches
  its `spacedock-subspace <brief>` server, captures the `{"status":"open","url":…}`
  stderr line, and carries that URL on the gate row (a new `addr` field). The
  rail POSTs to that URL. `<log>.addr` (so the rail addresses gate servers
  *without* grout owning their launch) is named as a Sprint-3 follow-on, below.
- **The rail issues the verdict by running `curl`, reusing the command
  mechanism it already has.** On the approve action the plugin runs
  `curl -sS -X POST <addr>/api/verdict -d '{"verdict":"approve","note":…}'`
  through the same `RunCommands`-gated command path it already uses to float
  `subspace-tui` (`open_command_pane_floating`, `src/main.rs:740-759`). No HTTP
  stack is added to the wasm plugin and no new permission is requested. (grout
  owning the POST via a reverse channel is the alternative if the plugin-runs-curl
  path proves awkward; the ACs measure the outcome, agnostic to which process
  POSTs.)

### Existing pure functions / structures the skeleton extends

- grout: `GateRow` + `BuildGateRow` (`grout/rows.go:20-31,45`) — add an `addr`
  field; `GateFromLog`/`gateFromBrief` (`grout/gate.go:35-77`) — augment with a
  black-box `wait` fold for resolution status; a new discovery-loop function
  around the existing one-shot `run` (`grout/main.go`).
- plugin: `GateEvent` (`src/main.rs:186-219`) already tolerates unknown JSON
  fields (`#[serde(default)]`), so an `addr` field needs no parser change;
  `decide_rail_click` (`src/main.rs:1343`) and `enum ClickAction`
  (`src/main.rs:395`) — add a verdict action variant; `brief_path_for_log`
  (`src/main.rs:292`), `upsert`/`apply_agent_event` (`src/main.rs:231,248`) —
  unchanged.

No layout-dump fixtures are involved, so the "single-line dump shape" fixture
rule does not apply to this task.

## Riskiest unproven mechanism — SPIKE NEEDED, RUN, PASSED

**Riskiest mechanism:** a rail-issued verdict actually unblocking a *real*
waiting/polling agent via the gate server's POST — not a UI action that writes a
file nobody reads. The recon findings above (no `<log>.addr`; server address
only on stderr; POST-only resolution) meant this was genuinely unproven, so a
spike was required (not the `no spike needed` negative).

**Spike run during ideation (2026-07-08), result PASSED.** Rig
(`scratchpad/spike/run-spike.sh`, using a fresh build of subspace at 9be5fbc and
the `playground-gate.md` verdict-mode fixture; `curl` stands in for the rail's
future POST):

1. Launched `spacedock-subspace <brief>` in the background; discovered its
   address **only** from stderr → `http://127.0.0.1:58111/` (proves the
   no-`.addr` discovery path works).
2. Started a real `spacedock-subspace wait --interval 1s` poller; confirmed it
   was **genuinely blocked** after a poll interval (`alive before verdict? YES`).
3. `curl -X POST <url>/api/verdict -d '{"verdict":"approve","note":…}'` →
   `200 {"status":"recorded"}`.
4. The blocked poller **resumed automatically** — `exit 0`, fold
   `{"verdict":"approve","round":1,…}` — with no human touching the log; resume
   latency ≤ the 1s poll interval (opened `09:03:11Z` → verdict `09:03:12Z`).
5. The log gained **exactly one** fsynced `event:"verdict"` line, written by the
   flock-holding server (lock released on clean exit) — i.e. via POST, not a
   direct rail append.

This de-risks the core loop before any rail/grout code is written. The
implementation stage owns reproducing it driving the rail's real POST path.

## Acceptance criteria

Each AC is an end-state property of the finished skeleton. Split into
**offline** (a fresh agent reproduces headlessly) and **interactive** (settled
only by CL's live zellij demo).

**AC-1 (offline) — grout's periodic glob discovers a newly-opened gate and
emits a gate row, with no manual trigger.**
Verified by: a grout test that places a brief + `decision-log.jsonl` into the
watched glob directory, advances one poll tick, and asserts exactly one
`zellij pipe --name agent-event` gate-row emission carrying the folded fields.
Independent baseline that can move the wrong way: the emission count 0→1 driven
by a filesystem change grout did not itself make (not a re-read of grout's own
output).

**AC-2 (offline) — the value AC: a verdict issued the way the rail will issue
it lands on the record AND resumes a genuinely-blocked real waiting agent, with
no human touching the log.**
Verified by: the mechanism drill (the spike, productionized as a repeatable
test) — launch a `spacedock-subspace <brief>` gate server, start a
`spacedock-subspace wait` poller, assert it is still blocked after ≥1 poll
interval, issue the approve exactly as the rail's action will
(`curl -X POST <captured-url>/api/verdict -d '{"verdict":"approve"}'`), then
assert (a) the poller exits **0** with a `{"verdict":"approve",…}` fold, (b) the
log gains **exactly one** `event:"verdict"` line, (c) resume happens within ≤2×
the poll interval of the POST. Independent baselines from outside any
rail/grout file: the poller's exit code (0 vs 5-timeout) and the log's
terminal-event count (0→1), both produced by the subspace binary.

**AC-3 (offline) — the rail obtains the live gate server's address without a
`<log>.addr` file.**
Verified by: a grout test asserting the emitted gate row's `addr` equals the URL
parsed from a fixture `{"status":"open","url":…}` stderr stream; and AC-2's
drill POSTing to exactly that captured URL. (Mechanism AC serving AC-2.)

**AC-4 (offline) — exactly one verdict type is reachable from the rail
(`approve`); no other verdict is wired.**
Verified by: a test asserting the rail's verdict action produces only an
`approve` POST body and that no revise/reject/hold path exists in the diff (a
code assertion, not a prose check). Scope-deferral naming lives in *Out of
scope* and is checked at this gate, not asserted as an AC.

**AC-5 (interactive) — CL's live scenario closes from the rail.**
Verified by: CL drives it in the fresh zellij session — a real (or fixture)
open zaphod-dev gate appears as a rail row **without CL hunting for it**; CL
approves it from the rail; the FO's `spacedock-subspace wait` loop for that gate
resumes (exit 0) and the entity advances — the exact loop that failed tonight
(feedback left, nothing consumed) now resolves end-to-end. A fresh agent cannot
reproduce a live floating-rail interaction, so CL is the validator here.

## Test plan

Riskiest first, per the workflow's spike discipline:

1. **Riskiest mechanism — DONE in ideation (see above), PASSED.** The
   POST→unblock-a-real-`wait`-poller drill. Cost: minutes. Re-run as an
   automated test at implementation (AC-2).
2. **grout offline (AC-1, AC-3):** Go tests (`go test ./...` + `go vet`) —
   glob-tick discovery emits one gate row; the row carries the stderr-captured
   `addr`. Cost: cheap, hermetic (fake `zellij`/`spacedock-subspace` scripts as
   the existing `emit_test.go` does).
3. **plugin offline (AC-4):** `cargo test` + `cargo check --tests` — the verdict
   action wires only `approve`; `decide_rail_click` returns the new action for
   the gate-row approve affordance. Cost: cheap.
4. **Live e2e (AC-5), interactive:** CL drives the full loop in the fresh
   zellij session (required by the workflow's "live e2e before merge for
   output-shape changes" rule — this changes the gate-row action surface). A
   cheap single-command spot-check (the AC-2 drill) proves the infra before
   CL's live time is spent.

## Scope decision — one thin skeleton task + named follow-ons

**Recommendation: this is ONE thin skeleton task; generalization is separate
named follow-ons.** Rationale: the skeleton's value is proving the whole loop
closes with minimum mechanism; it already spans the Sprint 2/3 seam (discovery +
verdict-unblock), which is itself the thinnest end-to-end slice — splitting it
further (discovery-only, then verdict-only) would defeat "prove the loop
closes." Bundling generalization in would breach the workflow's "a design that
reaches beyond the task's sprint exit criterion" bar. Follow-ons, each its own
future task:

- **F-1 — all verdict types + a verdict picker** (revise/reject/hold;
  revise/reject carry active feedback; hold is the one non-terminal verdict that
  resumes on relaunch). Completes Sprint 3.
- **F-2 — the `<log>.addr` subspace seam** (a subspace-side additive change so
  the rail addresses gate servers without grout owning their launch — the plan's
  actual M2 seam). **Cross-repo (subspace, not zaphod); needs CL's explicit
  go-ahead** before it is filed here.
- **F-3 — real event-driven discovery** replacing naive poll+glob (CL's "a real
  consumer"), removing the accumulating-`.jsonl` cost.
- **F-4 — multi-gate concurrency + arbitrary glob config** (Sprint 2 generality;
  the skeleton hardcodes one gate).

## Doc diff (user-visible behavior change — reviewed at this gate)

The skeleton adds a verdict action to gate rows (new user-visible behavior).
Proposed concrete edits, to land with the implementation:

- **README.md** (the "Agent & gate rows" bullet, lines 39-45): after "click a
  gate row to float `subspace-tui`… decision log", add: "and **approve** a
  pending gate from the rail — the skeleton wires the `approve` verdict only,
  which POSTs to the gate's live review server (`POST /api/verdict`) and resumes
  the waiting agent; other verdicts and a picker are follow-on work. `subspace-tui`
  remains feedback-only; a verdict is never a direct log append (single-writer
  flock — the rail POSTs)."
- **SPEC.md** (the agent-event paragraph, lines 12-15): add one line: "A pending
  gate row also exposes an `approve` action that POSTs a verdict to the gate's
  ephemeral review server, addressed by the URL grout captures from the server's
  stderr (`<log>.addr` is a later seam); direct decision-log append stays
  forbidden."

## Out of scope

Deferred to the named follow-ons above, not silently built into the skeleton:
revise/reject/hold and any verdict picker (F-1); the `<log>.addr` subspace seam
(F-2, cross-repo, needs CL's go-ahead); event-driven discovery replacing naive
poll+glob (F-3); multi-gate concurrency and arbitrary glob config (F-4). Also
out: changing `subspace-tui` (it stays feedback-only); any subspace model change
for the skeleton; retiring the existing float-review gate-row click (the approve
action is additive to it).

## Stage Report: ideation

- DONE: Design the walking skeleton scope precisely: one gate, one verdict type (approve), naive poll+glob discovery (accepted cost, not the final mechanism), fold via the existing subspace binary as a black box.
  See *Proposed approach* — one gate/one approve; grout poll-glob discovery (accepted `.jsonl` accumulation cost named); fold via `spacedock-subspace wait` as a black box; existing pure fns to extend named.
- DONE: Name the riskiest unproven mechanism (a rail-issued verdict actually unblocking a real waiting/polling agent via the gate server's POST) and put its smallest end-to-end proof first in the test plan, per this workflow's spike discipline.
  See *Riskiest unproven mechanism* + *Test plan* item 1: spike RUN during ideation, PASSED (POST → real blocked `wait` poller resumed exit 0, one fsynced verdict line; evidence `scratchpad/spike/run-spike.sh`).
- DONE: State explicitly whether this is one task or should split into a skeleton task plus separate generalization follow-ons (multi-gate, all verdict types, real discovery consumer replacing poll+glob) and why.
  See *Scope decision*: one thin skeleton task; four named follow-ons (F-1 all verdicts, F-2 `<log>.addr` cross-repo, F-3 event-driven discovery, F-4 multi-gate/glob config), with rationale.

### Summary

Reconned subspace at HEAD 9be5fbc and found three plan assumptions inaccurate: `<log>.addr` does not exist (address is stderr-only), `subspace-tui` is feedback-only (no verdict path), and the gate server is ephemeral/in-process. These reshape the skeleton — a verdict must be a POST to a live server addressed via captured stderr, never a direct append. Ran the riskiest-mechanism spike (verdict POST → real blocked `wait` poller resumes) and it PASSED, de-risking the loop before any code. Filled the body with offline/interactive-split ACs (AC-2 is the value AC against independent baselines: poller exit code + log terminal-event count), a riskiest-first test plan, a one-task-plus-follow-ons scope decision, and a concrete README/SPEC doc diff for the new gate-row approve action.

## Stage Report: implementation

- DONE: Implement the walking skeleton exactly as scoped: one gate, approve-only verdict, naive poll+glob discovery in grout, fold via `spacedock-subspace wait` as a black box, rail issues the verdict via curl through the existing RunCommands command path -- reproducing the ideation spike as an automated test (AC-2).
  grout `watch` (poll+glob → launch server → capture addr → emit); rail runs `curl` via `open_command_pane_floating` (the RunCommands path); AC-2 drill re-runs the spike green (commits dd95dca, fd46219, f2a7cc0, de96ffe, 1e390dd). Interpretation to review: the `wait` black-box is the *waiter* the AC-2 drill drives; grout's watch loop does not itself shell `wait` (the FO gate-loop is the real waiter, per recon) — I judged grout-side wait-probing YAGNI for one gate.
- DONE: Write AC-1 (grout offline glob-discovery test), AC-3 (addr captured from server stderr, offline), and AC-4 (only approve is wired, no revise/reject/hold path in the diff) with red-test-first evidence for each.
  AC-1 red: `unknown field GatesDir` + `undefined: discoverAndEmit`. AC-3 red: `undefined: parseServerURL` + `too many arguments in call to discoverAndEmit`. AC-4 red: `struct GateEvent has no field named addr` + `no variant named ApproveGate` + `cannot find function verdict_post_args`. All green after minimal fix.
- DONE: Apply the README.md and SPEC.md doc diff drafted in ideation for the new gate-row approve action, and flag AC-5 (the live interactive close-the-loop demo) as needing CL's live session -- do not attempt to fake it.
  README gate-rows bullet + SPEC agent-event paragraph updated (commit 75fe775). AC-5 NOT attempted — needs CL's live zellij session (see below).

- AC-2 (value AC): PASSED as an automated test — `TestRailVerdictUnblocksRealWaiter` (2.06s): server launched, addr captured from stderr, real `wait` poller genuinely blocked, `curl` approve POST → poller exit 0 with `{"verdict":"approve",...}` fold, exactly 1 verdict line, resume ≤ 2× poll interval.
- AC-5 (interactive): OPEN — requires CL to drive the click in a live zellij session; a fresh agent cannot reproduce a floating-rail interaction.

### Summary

Built the walking skeleton across the sprint 2/3 seam. grout: glob discovery (both decision-log forms), addr captured from the server's stderr (no `<log>.addr`), and a `watch` poll loop that launches each gate's server and emits rows carrying the addr. plugin: the gate row's detail line issues an `approve` verdict by running `curl` through the existing RunCommands path (title line still floats the review TUI — additive); a source scan asserts only `approve` is wired. Test counts: plugin 130→134 (`cargo test` + `cargo check --tests` clean); grout 41 test funcs green (`go test ./...` + `go vet` clean, AC-2 runs with the binary present). Smoke-tested the full live `grout watch` path against a fresh subspace build at 9be5fbc: it emitted a gate row with a captured `http://127.0.0.1:PORT/` addr. Decisions for the gate: (1) approve gesture is the detail-line click with no confirmation step — accidental-approve risk, deferred to CL's AC-5 read; (2) grout owns the server launch for the skeleton (F-2 `<log>.addr` removes it); (3) AC-2 and `grout watch` need the cross-repo `spacedock-subspace` binary (not vendored) — tests skip without it, run via `SPACEDOCK_SUBSPACE_BIN`/PATH.

## Stage Report: validation

Identity: re-ran against worktree HEAD `1e390dd` (clean, no uncommitted changes). subspace built fresh from `spacedock-research/spacedock-subspace@9be5fbc` (matches recon).

- DONE: Independently re-run AC-1/AC-2/AC-3/AC-4's evidence against the implementation worktree's final commit -- re-execute the tests yourself (including the AC-2 drill, TestRailVerdictUnblocksRealWaiter), not a re-read of the implementer's stage report.
  All PASS re-executed by me. AC-1 `TestDiscoverEmitsGateRowOnFilesystemChange` (0.23s: emission 0→1 driven by `writeGate`, a fs change grout did not make). AC-3 `TestParseServerURL`+`TestGateRowCarriesCapturedAddr`. AC-2 `TestRailVerdictUnblocksRealWaiter` (2.04s) with binary; SKIP 0.00s without it → proves the binary is genuinely exercised, not a silent pass. AC-4 four tests + source scan, `cargo test` 134 passed / 0 failed. `go vet` + `cargo check --tests` clean.
- DONE: Review the implementer's flagged interpretation (grout's watch loop does not shell `spacedock-subspace wait`; FO gate-loop is the real waiter; grout-side wait-probing YAGNI).
  CONFIRMED — does NOT undermine AC-2. grout production code never shells `wait`: the only `exec.Command` of the subspace binary is the SERVER launch `spacedock-subspace <brief>` (`grout/watch.go:40`); the sole `wait` shell-out in the whole module is the AC-2 drill test (`grout/verdict_unblock_test.go:127`); the "wait" in `grout/state.go:19,44` is the word "awaiting_user". The waiter is architecturally a separate process (FO gate-loop), so grout not owning it is correct separation, and a grout-side second poller on the same log would be redundant → YAGNI is sound. AC-2's value claim is proven regardless: the drill spawns a REAL `spacedock-subspace wait` poller (same binary+verb the FO runs), confirms it blocked, POSTs approve, asserts exit 0 + one verdict line.
- DONE: Run a refutation audit on a throwaway checkout against the curl verdict path and poll+glob discovery loop, naming concrete attack scenarios; prepare the AC-5 live demo script.
  Audit run on a detached throwaway worktree at `1e390dd` (never the impl worktree; removed after). Attacks + verdicts below. Demo script below. AC-5 NOT faked — it is CL's to drive.

### Refutation audit — attacks attempted

Curl verdict path (`src/main.rs`):
- SURVIVES — existing float-review click unregressed: gate TITLE line still floats `subspace-tui` (`main.rs:1411-1418`); approve is additive on the DETAIL line only (`1419-1421`), asserted by `gate_detail_click_approves_via_captured_addr`.
- SURVIVES — no POST-to-nowhere: an addr-less gate row decides to `ClickAction::None` (`main.rs:1419,1422`), asserted by `gate_detail_click_without_addr_does_nothing`.
- SURVIVES (with named limit) — approve-only: body is the hardcoded literal `{"verdict":"approve"}` (`main.rs:431`); the AC-4 source-scan guard `only_approve_verdict_is_wired` catches any LITERAL non-approve body but has a blind spot for a dynamically-built body (e.g. `format!` with a variable) — no such path exists in this diff, so AC-4 holds; flagged for F-1.
- SURVIVES by construction — no shell injection: curl runs as an argv vector via `CommandToRun{path:"curl", args:…}` (`main.rs:797-805`), not a shell string; addr also originates from a localhost server grout launched, not attacker input. Trailing slashes normalized by `trim_end_matches('/')` (`main.rs:424`).

Poll+glob discovery (`grout/`), probed live in the throwaway:
- CONFIRMED (accepted for 1-gate scope) — one malformed brief poisons the WHOLE pass: `discoverGates` folds every gate and returns on the first error (`grout/discover.go:34-38`), so a single broken sibling suppresses valid gates' rows too (probe: emitted=0, 0 rows piped). Fine for the one-gate skeleton; a multi-gate robustness gap → F-4. Same shape for server-launch failure aborting `runWatch` at startup (`grout/watch.go:68-70`).
- SURVIVES — per-row addr-resolution failure is isolated: inside a tick `discoverAndEmit` logs+skips a failed row and emits the rest (`grout/discover.go:54-72`) (probe: 1 healthy row still piped while the other failed).
- SURVIVES — no double-count (the two glob forms are structurally disjoint); empty decision-log is fine (row identity comes from the brief, not log content).

Semantic drift / cross-checks:
- Sprint-0 one-shot `run` unaffected: `addr` is `json:",omitempty"` (`grout/rows.go:30`) + plugin `#[serde(default)]` — old gate rows serialize unchanged.
- Drift risk flagged: the Go drill's curl argv (`verdict_unblock_test.go:147`) and the Rust plugin's `verdict_post_args` (`main.rs:424-432`) are independently-maintained literals. I ran the rail's EXACT argv-as-written live against a real grout-launched server + real `wait` poller → `{"status":"recorded"}`, waiter EXIT 0, fold `{"verdict":"approve",…}`, exactly 1 verdict line. They currently agree byte-for-byte.
- Minor: the body's recon cites `docs/dev/gate-loop-recipe.md` (lines 82,99) which does not exist in the repo — a recon citation, not a code claim; the `spacedock-subspace wait` mechanism it names is real (exit 0=resolved / 5=timeout, `wait.go`) and matches AC-2's baseline. Does not affect any AC.

### Pre-demo infra spot-check (done — proves the drill before CL's live time)

Drove the full live `grout watch` path with a real subspace server and only zellij faked as the sink: grout launched `spacedock-subspace <brief>`, captured `http://127.0.0.1:56489/` from its stderr, and piped one `agent-event` gate row carrying that addr + folded fields (`kind=gate, entity=playground, stage=review, round=1, recommendation=approve`). Combined with the live rail-argv POST→resume above, the entire headless half of AC-5 is proven; only the live click remains for CL.

### AC-5 demo script (CL drives, live zellij session)

Prereqs: run everything from INSIDE the fresh zellij session that has the rail docked (so grout's `zellij pipe` reaches it). `spacedock-subspace` on PATH or `SPACEDOCK_SUBSPACE_BIN` set; `curl` on PATH; the plugin's RunCommands permission already granted (reuses the subspace-tui float path — no new grant). IMPORTANT coupling: grout owns the server launch, so do NOT also launch a second `spacedock-subspace <brief>` server for the same gate — run only the `wait` poller. Only the detail (2nd) line of the gate row approves; the title line still floats the review TUI.

1. Build grout: `cd <worktree>/grout && go build -o /tmp/grout .`
2. Create a fixture gate dir `GATES=$(mktemp -d)/gates; mkdir -p $GATES/playground`, write `$GATES/playground/brief.md` (verdict-mode: `subspace: v0`; `gate:` with workflow/entity/entity-title/stage/round; `recommendation: {verdict: approve}`; `artifact: {kind: draft, path: ./playground.md}`), a `playground.md`, and an empty `decision-log.jsonl`. (A real open zaphod-dev gate works too.)
3. Start the FO's waiter in a visible pane: `spacedock-subspace wait --interval 2s $GATES/playground/decision-log.jsonl` — it blocks.
4. Start discovery: `SPACEDOCK_SUBSPACE_BIN=… /tmp/grout watch $GATES`. Expect: a gate row appears in the rail WITHOUT CL hunting — line 1 = blocked glyph + "Playground handoff demo", line 2 = "review r1 · approve".
5. CL clicks the gate row's DETAIL line (line 2). Expect: a floating curl pane fires `POST <addr>/api/verdict {"verdict":"approve"}` → `{"status":"recorded"}`.
6. Expect: the waiter from step 3 resumes EXIT 0 with fold `{"verdict":"approve",…}`; the log gains exactly one `"event":"verdict"` line; the entity advances. This is the exact loop that failed live tonight (feedback left, nothing consumed) now closing end-to-end.

Watch-fors during the demo: the row must carry a non-empty addr (else the detail click is a no-op by design); accidental-approve — the detail click approves with no confirmation step (implementer decision (1), CL's call); if CL launched its own server, the flock contends — let grout be the sole launcher.

### Summary

Independently reproduced all four offline ACs against worktree HEAD `1e390dd` by re-executing the tests (AC-1/AC-3/AC-4 hermetic; AC-2 drill 2.04s with a fresh subspace@9be5fbc build, and SKIP-without-binary proving it is genuinely exercised); `go vet`, `cargo check --tests`, `cargo test` (134/0) all clean. Confirmed the flagged interpretation with file:line — grout never shells `wait`, the FO gate-loop is the real waiter, and AC-2's value claim holds because the drill drives a real `wait` poller; the YAGNI call is sound. Refutation audit on a throwaway checkout: the curl verdict path SURVIVES all attacks (additive click, addr-less no-op, argv-vector no-injection, approve-only guard with a named dynamic-body blind spot); the poll+glob loop is per-row resilient but all-or-nothing on a malformed brief (acceptable for one gate, named F-4) — no REFUTED. Also proved the rail's exact argv-as-written unblocks a real waiter live (Go-drill/Rust-literal agree). Prepared the AC-5 live demo script; AC-5 remains OPEN for CL to drive. Recommendation: the offline ACs pass gate; present AC-5 to CL for the live close.

## Stage Report: validation (addendum — two implementer decisions scrutinized)

Two decisions from the implementation summary (entity line 314) elevated to explicit gate findings. Neither refutes an AC; both are reviewer-judgment calls the gate must rule on rather than wave through.

- DONE: Assess "no confirmation step on approve — accidental-approve risk, deferred to CL's AC-5 read."
  FINDING V-1 (real defect for standing use; scope-cut-acceptable only for the gated skeleton). Verified fact: the gate row's DETAIL line click routes to `ClickAction::ApproveGate` (`src/main.rs:1419-1421`) → `open_command_pane_floating` runs `curl -X POST <addr>/api/verdict {"verdict":"approve"}` (`src/main.rs:797-805`). The command AUTO-RUNS on open — the shim (`zellij-tile-0.44.1/src/shim.rs:622-637`) dispatches `OpenCommandPaneFloating` to the host with no suspend/hold flag, and the entity's own live incident confirms the identical mechanism auto-launches subspace-tui with no start gesture. So one click = one immediate, terminal, IRREVERSIBLE verdict that advances the entity past its gate; no confirm, no undo. Blast radius is not cosmetic: it resolves a gate a human was meant to review — the exact failure class the gate mechanism exists to prevent, and the mirror of tonight's incident. Aggravator: click is the rail's primary low-ceremony gesture, and approve sits on line 2 of a 2-line row whose line 1 is the SAFE review-float — an off-by-one-row click flips safe→irreversible. Judgment: does NOT block AC-1..4, and AC-5 is CL approving deliberately (so the demo itself is not endangered); ACCEPTABLE to prove the skeleton. NOT acceptable as a standing action — a confirm affordance (or two-step gesture / brief undo window) must land before approve is used outside a gated demo; pull into F-1 at the latest, arguably sooner given irreversibility. Present to CL at the AC-5 gate as an explicit ruling.
- DONE: Assess "AC-2's test silently SKIPS without the cross-repo spacedock-subspace binary."
  (a) My own AC-2 re-run GENUINELY EXECUTED — not a silent skip. I built the binary from `spacedock-research/spacedock-subspace@9be5fbc`, set `SPACEDOCK_SUBSPACE_BIN`, and `TestRailVerdictUnblocksRealWaiter` ran 2.04s/2.07s (real subprocess run: server launched, real `wait` poller blocked, curl approve POST, resume exit 0, one verdict line); contrast — without the binary it reports `--- SKIP (0.00s)`. I also ran the rail's exact argv (`src/main.rs:424-432`) live against a real server + real waiter → exit 0, one verdict line. So AC-2 is independently confirmed on real evidence.
  (b) FINDING V-2 (real false-confidence gap, separate from the mechanism working). `t.Skip` (`grout/verdict_unblock_test.go:19-28,88-91`) is rolled into `go test`'s green `ok`. The dep is cross-repo and UNVENDORED, so "binary absent → skip → green" is the DEFAULT for a fresh checkout / any teammate / any naive `go test ./...`. There is NO CI in the repo today (no `.github/workflows`, no Makefile), so tests run by hand — meaning the one test proving this entity's entire reason-for-existing does not run by default while the suite passes. This is exactly the false confidence the workflow's own discipline forbids: README `docs/agent-rail-dev/README.md:173-179` ("Live e2e before merge… offline tests prove the logic, never the surface" + "Spike discipline binds infra too"). Recommend before "done": (a) a pre-merge/CI step that pins spacedock-subspace@9be5fbc and RUNS the drill so green means the drill ran; or at minimum (b) a loud, merge-gating opt-in (e.g. `SPACEDOCK_REQUIRE_DRILLS=1` turning the skip into a hard fail in the env that must run it) plus a documented pre-merge checklist item. A silent skip whose only mitigation is a sentence in a stage report is insufficient for a riskiest-mechanism guarantee.

### Summary (addendum)

Both scrutinized decisions produce gate findings, neither an AC refutation. V-1: the no-confirmation approve auto-fires an irreversible verdict on a single detail-line click (verified via the auto-run shim + the entity's own incident) — acceptable to prove the skeleton, but a confirm/undo must land before it is a standing action; CL rules at the AC-5 gate. V-2: my AC-2 re-run genuinely executed (real 2.04s run, cross-checked with the rail's literal argv), but the drill silently skips without the unvendored cross-repo binary and the suite still goes green — a false-confidence gap against the workflow's own spike discipline that should be closed (pinned drill in CI, or a loud merge-gating require-drills switch) before this is called done. Net recommendation unchanged for the OFFLINE gate (AC-1..4 pass on reproduced evidence); V-1 and V-2 are should-address findings to put in front of CL alongside the AC-5 live close.
