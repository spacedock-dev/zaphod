# Validation: Grout session-state mapping + default-path fix

Entity: `docs/agent-rail-dev/.spacedock-state/grout-session-states.md`
Worktree at review: `.worktrees/spacedock-ensign-grout-session-states` @ `c2954d23b61941d6b2b754f325c74180638d37c8`
Refutation checkout (throwaway, never the implementation worktree): a fresh
clone at the same commit, discarded after this review.

## Offline AC verdicts (independently re-run, not re-read from the stage report)

| AC | Verdict | Command | Evidence |
|---|---|---|---|
| AC1 — 4/4 survey statuses render inside marker vocabulary | PASS | `cd grout && go test ./... -run TestSurveyStatusesRenderInsideMarkerVocabulary -v` | `--- PASS` (0 unknowns), confirmed against `src/agent.rs:24-28` read directly (blocked/working/idle/done → Unknown fallback matches the test's `markerVocabulary`) |
| AC2 — mapping table across recency boundaries | PASS | `go test ./... -run TestMapSessionState -v` | 17/17 subtests PASS, including the `workingWindow`/`idleWindow` boundary cases |
| AC3 — recorded-fixture path decodes to `blocked` | PASS | `go test ./... -run TestSessionRowFromFixture -v` | PASS; fixture's `termination_status` independently confirmed as `"awaiting_user"` by reading `testdata/session-get.json` directly |
| AC4 — vocabulary drift passes through verbatim | PASS | `go test ./... -run TestMapSessionState/drift_status_stale -v` | PASS (`future_status` stale → `"future_status"` verbatim, not a crash or guess) |
| AC5 — cwd-independent CLI, exit 2 + usage | PASS | `go test ./... -run TestCLIRequiresBothPositionals -v` | PASS from both `grout/` and repo-root cwd; also drove the built binary by hand from both cwds — same exit 2 + `usage: grout <session-id> <gate-log>` |
| AC6 — wire-level: fake agentsview + fake zellij, `state` is `blocked` not `awaiting_user` | PASS | `go test ./... -run TestEmitEndToEnd -v` | PASS; recorded pipe payload's `state` field is `blocked` |

Full suite: `go test ./...` → `ok zaphod/grout` (11 top-level funcs, 35 PASS
lines total top-level+subtests — the stage report's "59 total" over-counts;
noted below, not AC-blocking). `go vet ./...` clean. `git diff --name-only
main...HEAD` touches only `grout/*.go`, `grout/README.md`, `README.md` — no
`src/` file, confirming the plugin's marker vocabulary and rendering are
untouched (read `src/agent.rs:23-31` directly to confirm, not just diffed).

## Refutation audit (throwaway checkout, never the implementation worktree)

Checkout: fresh `git clone` of the repo, `git fetch` + `git checkout` of
`c2954d23b61941d6b2b754f325c74180638d37c8` in an isolated scratch directory.
Attacks were injected as temporary `zz_*_test.go` files, run, then deleted —
`git status --short` confirms the checkout is clean afterward.

- **Attack: negative/zero `age` (future `lastActivity`, clock skew).**
  `MapSessionState("tool_call_pending", now+5m, now)` → `"working"` (negative
  age satisfies `age < workingWindow`). Survives — a clock-skewed future
  timestamp degrades to "looks fresh," never a crash or a false `blocked`.
- **Attack: `clampSummary` with `maxBytes = -1`.** Panics:
  `runtime error: slice bounds out of range [:-1]` (rows.go:64, the
  rune-boundary countdown starts at a negative `cut`). **REFUTED as a live
  attack, confirmed as a latent defect**: `clampSummary` is unchanged by
  this diff (pre-existing sprint-0 code) and its only caller,
  `BuildSessionRow(si, now, cfg.SummaryClampBytes)`, is fed `cfg.SummaryClampBytes`,
  which is hardcoded to `512` in `defaultConfig()` (main.go:34) with no CLI
  flag or code path that can make it negative today. Not reachable through
  any surface this task's ACs cover — flagging for a future hardening pass,
  not a validation blocker.
- **Attack: `clampSummary` with `maxBytes = 0` and a multi-byte rune at the
  boundary.** Returns `""` cleanly, no panic — survives.
- **Attack: `decodeSession` fed garbage (`"not json"`) and an empty
  reader.** Both return a plain Go decode error (`invalid character 'o'...`,
  `EOF`) — no panic, caller (`FetchSession`) already wraps and returns it.
  Survives.
- **Attack: `GateFromLog` against a missing decision-log/brief pair.**
  Returns a clear `open ...: no such file or directory` error, no panic —
  matches the documented "source failures are fatal before anything is
  piped" posture. Survives.
- **Attack: does the fixture's internal timestamp inconsistency break AC3?**
  `testdata/session-get.json` has `created_at` (`05:34:59Z`) *after*
  `ended_at` (`05:00:05Z`), and `TestSessionRowFromFixture`'s `now`
  (`05:00:00Z`) is itself 5s *before* `ended_at` — i.e., negative age by the
  coalesced timestamp. Traced through `MapSessionState`: `awaiting_user`
  short-circuits at row 1 before age is ever computed, so this quirk cannot
  affect AC3's `blocked` result. Survives.
- **Attack: caller impact — is `BuildSessionRow`'s `State` field read by
  anything besides the plugin's `marker_for_state`?** `grep -rn
  "\.State\b" grout/*.go` (non-test) shows only `main.go`'s row-building
  call site; the row is serialized straight to JSON and piped. No second Go
  consumer to break. Survives.
- **Attack: live-data spot check.** Pulled a real, currently-`awaiting_user`
  session (`c0593f5d-0b2c-429b-aa52-b62860fec745`, age ~6.5 min at fetch)
  from the local agentsview daemon's HTTP API (`GET
  /api/v1/sessions?...`, bypassing this sandbox's block on the `agentsview`
  CLI touching `~/.agentsview` directly — the same OS restriction the
  ideation survey hit). Decoded it through the real `decodeSession` +
  `BuildSessionRow` path in the throwaway checkout: state = `blocked`. This
  is a second, independent baseline beyond the recorded fixture, using
  genuinely live production data, and it confirms the mechanism.

No REFUTED verdict against any of AC1-AC6. One pre-existing, unreachable
panic path (`clampSummary` negative-bytes) surfaced by the audit; it predates
this diff and is out of this task's surface.

## Demo script for AC-I1 (interactive, CL drives live)

**Setup.** The plugin (`src/agent.rs`) is untouched by this task, so no wasm
rebuild is needed — use your normal daily-driver zellij session with the
zaphod sidebar already docked. All commands below run from a pane *inside*
that session (grout's default pipes to `$ZELLIJ_SESSION_NAME` — it must be
set, i.e. you must be inside zellij, not merely have it running).

1. **Find a real live session id** (run this in your own shell, not this
   ensign's — the CLI is sandbox-blocked from `~/.agentsview` in this
   review environment, but works normally in your terminal):

       agentsview session list --resume --format json | jq -r '.sessions[] | "\(.id)  \(.termination_status)  \(.project)  \(.cwd)"'

   Pick one that's `awaiting_user` (a pane genuinely waiting on you right
   now) to see `blocked`, or one mid-stream to see `working`. (Session ids
   churn — this is why AC5 killed the hardcoded demo default; there is no
   fixed id to hand you.)

2. **Run grout from the worktree**, inside the zellij session, from
   `/Users/clkao/git/zaphod/.worktrees/spacedock-ensign-grout-session-states/grout`:

       go run . <session-id> testdata/playground-gate.decisions.jsonl

   (The vendored playground fixture is fine for the gate-row half; AC-I1
   only concerns the session row's marker.)

3. **What you should see:** the AGENTS row for that session in the sidebar
   now shows a real `blocked`/`working` marker — never the blank Unknown
   marker it showed before this fix, for a session that is in fact live.

4. **Re-run the original repro** — from `grout/`, no args:

       go run .

   Expected: `usage: grout <session-id> <gate-log>` on stderr, exit 2 —
   not the old silent cwd-dependent open failure you hit live.

AC-I1 is settled by your live observation at steps 3 and 4, not by this
report.

## Note on the stage-report metric discrepancy

The implementation stage report claims "subtests 59 total"; the actual
`go test ./... -v` run in this review counts 35 `--- PASS`/`--- FAIL` lines
(11 top-level + 24 subtests). This does not change any AC verdict — every
test that exists passes — but the implementer's self-reported count is
inaccurate and worth a correction for the record.
