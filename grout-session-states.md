---
title: Grout session-state mapping + default-path fix
status: ideation
source: plan sprint 1 + sprint-0 gate findings
score: 0.9
id: kawd2h37e2rhn9t9ynf61ppe
started: 2026-07-07T23:00:49Z
---

## Problem

Live AGENTS rows carry no signal: grout emits agentsview's
`termination_status` verbatim (e.g. `awaiting_user`) while the plugin maps
only `blocked|working|idle|done`, so every real session wears the blank
Unknown marker (rows-section validation, cross-slice gate finding). And
grout's default gate-log path is cwd-relative — it never resolves under
`go run .` (CL hit it live; absolute argv[2] was the workaround).

## Survey — agentsview's real `termination_status` vocabulary (executed 2026-07-08)

The riskiest unproven mechanism was the vocabulary itself, so the survey ran
during ideation, before any design commitment.

- **The producer pins no enum.** `agentsview openapi` (v0.36.1) types
  `termination_status` as a bare string on every schema carrying it
  (DbSession, DbSidebarSessionIndexRow, DbTopSession, ServiceSessionDetail).
- **Full-corpus survey** over the live daemon
  (`GET http://localhost:8080/api/v1/sessions?include_one_shot=true&include_automated=true&include_children=true&limit=500`,
  paginated by `next_cursor`; 41 pages, 20,126 sessions):

  | termination_status | sessions |
  |---|---|
  | `awaiting_user` | 10,546 |
  | `clean` | 8,748 |
  | `tool_call_pending` | 728 |
  | absent/null | 104 |

  The seed's guess (`running`, `completed`) does not exist in the data.
  (Direct reads of `~/.agentsview` and the `~/git/agentsview` source were
  OS-blocked in this session; the daemon's HTTP API carried the survey.)
- **agentsview's own liveness derivation** (decoded from the served UI bundle
  `/assets/index-yvzhQyOd.js`, v0.36.1): last-activity =
  `ended_at ?? started_at ?? created_at`, max over children; then, first
  match: `awaiting_user` and age < 600s → *waiting*; age < 60s → *working*;
  age < 600s → *idle*; `tool_call_pending` or `truncated` → age < 3600s ?
  *stale* : *unclean*; else *quiet*. Two facts fall out: the code's
  vocabulary includes a fourth value **`truncated`** (unobserved in this
  corpus), and **status alone cannot carry liveness — the proven mechanism is
  status + transcript recency**. That is how the corpus holds 10,546
  `awaiting_user` rows while only a handful of agents are actually waiting on
  CL; a static `awaiting_user → blocked` map would be as wrong as today's
  blank marker.

## Proposed approach

Two grout-side changes; the plugin contract (`marker_for_state`,
`src/agent.rs:23` — `blocked|working|idle|done`, anything else renders the
Unknown marker) stays untouched (plan decision 1).

### 1. Session-state mapping (status + recency)

A pure function in new `grout/state.go`:

    MapSessionState(status string, lastActivity, now time.Time) string

`lastActivity` coalesces `ended_at ?? started_at ?? created_at` (agentsview's
own rule); `sessionInfo` (`agentsview.go:18`) gains those three fields, all
already present in the recorded `session get` shape; `BuildSessionRow`
(`rows.go:33`) swaps `State: si.TerminationStatus` for the mapped value.
First match wins:

| # | condition | state | provenance / rationale |
|---|---|---|---|
| 1 | status == `awaiting_user` | `blocked` | transcript ends with the agent waiting on the user. **Named deviation:** agentsview decays waiting→quiet at 10 min; grout does not decay, because the rail's defining scenario is returning after an hour to find who is still blocked (PRD §3). Staleness of the *population* belongs to the emitter (sibling SSE task), not per-session state. |
| 2 | age < workingWindow (60s) | `working` | agentsview's 60s window — transcript still moving; covers fresh `tool_call_pending` (tool mid-execution), fresh absent-status (mid-turn), and a just-`clean` session that may still be appending (same ordering as agentsview) |
| 3 | status == `clean` | `done` | clean shutdown |
| 4 | age < idleWindow (10 min) | `idle` | agentsview's 600s window — recent but quiet (long tool run, lull between turns) |
| 5 | otherwise | verbatim status | stale `tool_call_pending`/`truncated`/absent and any future vocabulary → the plugin's Unknown marker, deliberately: visible-not-guessed (decision 1's posture) and graceful under vocabulary drift (no enum pinned upstream) |

Known blind spot, accepted: a Claude Code permission prompt and a
long-running tool are both `tool_call_pending` with growing age — the
transcript cannot distinguish them; rows 2/4/5 treat them as
working→idle→Unknown exactly as agentsview does. The pane-side viewport
detector already catches permission prompts for pane rows.

`workingWindow`/`idleWindow` are named constants with provenance comments
citing the decoded derivation. Timestamps that fail to parse → zero time →
age effectively infinite → rows 3/5, never a guessed live state.

### 2. Gate-log default-path fix: make both positionals required

Of the two sanctioned options, resolve-from-executable breaks under `go run`
(`os.Executable()` is a temp build dir) and would embed a machine path at
build time; the cwd-relative default is what bit CL live. So: no defaults —
usage `grout <session-id> <gate-log>`, exit 2 with usage on stderr when
either is missing. The hardcoded demo `SessionID` default falls with it (a
UUID that exists only in CL's DB — the same hidden machine dependency). M1's
glob config (plan decision 4) is the removed default's real successor.

## Acceptance criteria

### Offline (agent-reproducible)

- **AC1 — the end value, counted against independent baselines.** For each
  of the four survey-observed `termination_status` values (`awaiting_user`,
  `clean`, `tool_call_pending`, absent), a session row built from a
  fresh-activity session carries a state inside the plugin's marker
  vocabulary: the count of survey statuses rendering the Unknown marker
  drops 4/4 → 0/4. Verified by: Go test whose two baselines are transcribed
  from outside the code under test — the survey table above and the plugin
  vocabulary at `src/agent.rs:24-28` (the `rows_test.go` transcription
  pattern).
- **AC2 — mapping semantics.** `MapSessionState` reproduces every row of the
  mapping table across the recency boundaries (just-inside/just-outside 60s
  and 10 min, per status). Verified by: table-driven Go test with injected
  `now`; expected values trace to the decoded agentsview derivation recorded
  above.
- **AC3 — recorded-fixture path.** `testdata/session-get.json` decoded via
  `decodeSession` (not a synthetic struct) yields state `blocked`
  (`awaiting_user`, row 1, any age). Verified by: extending
  `TestSessionRowFromFixture` (`agentsview_test.go`).
- **AC4 — vocabulary drift.** A status outside the surveyed vocabulary with
  stale activity passes through verbatim (e.g. `future_status`), which
  `marker_for_state` renders Unknown — no crash, no guess. Verified by: Go
  test on the fallback row.
- **AC5 — cwd-independent CLI.** `grout` with fewer than two positionals
  exits 2 and prints usage naming both; behavior is identical from the repo
  root and from `grout/` (the cwd CL was bitten in). Verified by: test or
  scripted check running the built binary from two different cwds, asserting
  exit code and stderr.
- **AC6 — wire-level.** A full offline run — fake `agentsview` returning the
  recorded fixture, fake `zellij` recording argv (the `emit_test.go`
  `writeScript` harness) — pipes a session row whose `state` is `blocked`,
  not `awaiting_user`. Verified by: extending the emit e2e test; the
  assertion reads the recorded pipe payload.

### Interactive (settled only by CL's live demo)

- **AC-I1.** In a fresh zellij session, grout run against a currently-live
  agent session renders a real marker on the AGENTS row — blocked while the
  agent waits on CL, working while it streams — never the blank Unknown
  marker for a live session; and re-running the original repro (`go run .`
  from `grout/` without a gate-log) yields the usage error instead of the
  silent cwd-dependent open failure.

## Test plan (riskiest first)

1. **Vocabulary survey — executed 2026-07-08 during ideation** (results
   above; the smallest end-to-end check, paid before any code). Refresh
   procedure recorded: paginate the sessions API against the local daemon,
   count `termination_status`. Residual risk — no upstream enum, `truncated`
   unobserved — guarded by AC4's verbatim fallback.
2. TDD the mapping: `state_test.go` table (AC2) red → green; then the AC3
   fixture path; then AC4 drift.
3. Cross-baseline count (AC1) — red 4/4 against today's verbatim
   pass-through, green 0/4 after.
4. CLI args (AC5) red → green (today: silent machine-specific defaults, no
   usage).
5. Offline wire e2e (AC6) on the existing emit harness.
6. CL live demo (AC-I1) — the interactive exit.

## Doc diff (user-visible: live rows change from blank Unknown to real markers)

`grout/README.md` usage block:

    -    go run ./grout [session-id [gate-log]]
    -
    -Both positionals optional; defaults live in `main.go` (`defaultConfig`).
    +    go run ./grout <session-id> <gate-log>
    +
    +Both positionals required — no default session or gate log, so runs
    +behave identically from any cwd; missing arguments exit 2 with usage.

`grout/README.md`, new section after Usage:

    ## Session state

    The row's `state` maps agentsview's `termination_status` plus transcript
    recency (`ended_at ?? started_at ?? created_at`) onto the plugin's
    marker vocabulary; first match wins: `awaiting_user` → `blocked`; last
    activity < 60s → `working`; `clean` → `done`; < 10 min → `idle`;
    otherwise the status passes through verbatim and the rail shows the
    unknown marker. Thresholds mirror agentsview v0.36.1's own liveness
    derivation; vocabulary surveyed 2026-07-08 over a 20k-session corpus
    (awaiting_user, clean, tool_call_pending, absent; `truncated` exists in
    code, unobserved).

Top-level `README.md`, AGENTS bullet:

    -  rows into the rail — agent sessions with state, and pending gate
    +  rows into the rail — agent sessions with state (blocked / working /
    +  idle / done, mapped from agentsview status + activity), and pending gate

## Out of scope

- Which sessions get emitted (population, SSE `data_changed`, active-since
  filtering) — the sibling sprint-1 task `grout-sse-daemon`; this task owns
  only per-session state. Row 1's no-decay deviation leans on that boundary
  explicitly.
- Any plugin change — `src/agent.rs` vocabulary and rendering untouched.
- Gate-log glob config (plan decision 4) — the removed default's successor.
- Distinguishing permission-prompt from long-running tool inside
  `tool_call_pending` (transcript-invisible; pane-side detection covers it).

## Stage Report: ideation

- DONE: Acceptance criteria written as end-state properties with Verified by clauses, split into offline (agent-reproducible) and interactive (CL live-demo) ACs
  AC1-AC6 offline + AC-I1 interactive; every expected value is transcribed from outside the code under test (survey table, src/agent.rs:24-28, recorded fixture, decoded agentsview derivation).
- DONE: Riskiest unproven mechanism named (agentsview's real termination_status vocabulary) with the smallest end-to-end survey/check listed first in the test plan, or a no-spike-needed record naming the proven mechanism
  Survey executed during ideation over the live daemon API: 20,126 sessions -> awaiting_user 10,546 / clean 8,748 / tool_call_pending 728 / absent 104; `truncated` plus the 60s/600s thresholds decoded from agentsview's own UI liveness derivation; test plan step 1 records results and the refresh procedure.
- DONE: Doc diff proposed in the body if the state-vocabulary mapping changes what a row displays (user-visible)
  Concrete diffs in the body: grout/README.md usage block + new Session state section, top-level README AGENTS bullet.

### Summary

Surveyed agentsview v0.36.1's real termination_status vocabulary end to end (full-corpus API survey plus the UI bundle's own liveness derivation): status alone cannot carry liveness, so the design maps status + transcript recency onto the plugin's four states, with one named deviation (awaiting_user never decays — the rail's return-after-an-hour scenario) and verbatim pass-through for vocabulary drift. The default-path fix resolves as "both positionals required" (resolve-from-executable breaks under go run). Direct DB/source reads were OS-blocked this session; the daemon's HTTP API carried the survey.
