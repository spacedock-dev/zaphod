# Validation: Grout SSE daemon — sessions for real

Entity: `docs/agent-rail-dev/.spacedock-state/grout-sse-daemon.md`
Worktree at review: `.worktrees/spacedock-ensign-grout-sse-daemon` @ `d08d8b55205bc637a61e5659089268752293f321`
Refutation checkout (throwaway, never the implementation worktree): a fresh
`git clone` of the repo, `git checkout` of the same commit, in an isolated
scratch directory — discarded clean after this review (`git status --short`
empty afterward).

## Offline AC verdicts (independently re-run, not re-read from the stage report)

| AC | Verdict | Command | Evidence |
|---|---|---|---|
| AC-1 — appear/update/stop-refresh driven by events | PASS | `cd grout && go test -run TestWatchSessionsFlow -v ./...` | `--- PASS: TestWatchSessionsFlow (0.47s)` |
| AC-2 — attach-or-start and reconnect | PASS | `cd grout && go test -run 'TestWatchAutoStart\|TestWatchReconnect' -v ./...` | `TestWatchAutoStart`, `TestWatchReconnect`, and the bonus `TestWatchReconnectOnSilence` all PASS |
| AC-3 — wedged rail cannot wedge or flood the watcher | PASS | `cd grout && go test -run TestWatchWedgedPipe -v ./...` | `--- PASS: TestWatchWedgedPipe (0.91s)` |
| AC-4 — cross-tab bind and click decisions | PASS | `cargo test bind_cross_tab` (full: `cargo test && cargo check --tests`) | 4/4 `bind_cross_tab_*` tests PASS; full suite 136/136 PASS; `cargo check --tests` clean |
| AC-5 — one-shot regression-free | PASS | `cd grout && GOPROXY=off go test -count=1 ./... && go vet ./...` | `ok zaphod/grout` (fresh, no cache), `go vet` silent |

Independent counts: 16 top-level Go test funcs (`grep -c '^func Test' *.go`,
matches the stage report's 6→16 claim), 136 Rust tests (matches 130→136).
`git diff --stat 87e180c..d08d8b5` confirms `emit.go`'s only change is
threading `ctx` through `EmitRow` (cancellation on watch shutdown); `gate.go`,
`rows.go`, `agentsview.go` are untouched — read directly, not just diffed.

## Refutation audit (throwaway checkout, never the implementation worktree)

- **Attack: truncated final SSE frame — server closes the stream right
  after a `data:` line with no trailing blank-line terminator.** Injected a
  throwaway `TestRefuteSSETruncatedFinalFrame` calling `parseSSEEvents` on
  `"event: data_changed\ndata: {...}\n"` (no trailing `\n\n`). Result: **0
  events dispatched** — the in-flight frame is silently dropped
  (`sse.go:27` only calls `dispatch()` on a blank line; EOF mid-frame never
  flushes it). This is a real false-negative at the parser layer. **Does not
  survive as an end-to-end defect**: any stream close forces a reconnect
  (`connectLoop`), and reconnect always runs a full refresh
  (`runWatch`'s `case <-connected:` starts a refresh regardless of what was
  lost) — independently confirmed by `TestWatchReconnect`'s own assertion
  (`waitInvocations(t, argvLog, 2)`, i.e. reconnect really does re-list).
  The 60s tick is a second independent backstop. Documented, not blocking.
- **Attack: malformed/garbage JSON from `session list --server`.**
  `decodeSessionList` fed `"not json"` and an empty reader both return a
  plain decode error; `refreshSessions` traces it to stderr and returns —
  no panic, no partial-row corruption. Survives.
- **Attack: negative/zero-width render bounds.** `SectionLayout::target`
  and `target_for_line` both guard `line <= 0 → None` before any
  arithmetic; `cols.saturating_sub` throughout the render path. Traced by
  reading, not just trusting the doc comment — no panic path found for any
  `line`/`cols` input. Survives.
- **Attack: caller impact — `os.Args[1] == "watch"` now shadows one-shot's
  positional `SessionID` override.** Before this diff, `grout watch` in
  one-shot mode would have tried to fetch a session literally named
  `"watch"`. Session ids are UUIDs (`defaultConfig`'s demo id, agentsview's
  `session get <uuid>`), so no real caller could collide with the literal
  string `"watch"` — the shadowing is total in practice, not just in
  today's tests. Survives (no real caller impact).
- **Attack: own-tab duplicate-match coverage gap.** `bind_cross_tab_*`
  tests cover own=1/foreign=1 (own wins) and own=0/foreign=2 (unbound), but
  no test drives own=2 directly. Read `bind_session` (`main.rs:290`): the
  second `own.next()` check is scope-symmetric with the tested foreign-pair
  branch — same `.next().is_none()` idiom, same data shape. Not a distinct
  code path, just an untested input combination. Noting as a coverage gap,
  not a defect — no REFUTED.
- **Attack: does watch mode's daemon-shared posture ever call `serve
  stop`?** `grep -n "serve" grout/*.go` (non-test): only `serve
  --background` in `startDaemon`. No stop call anywhere in the diff.
  Survives (matches "grout never stops the shared daemon").
- **Live spot-check beyond the offline suites (real `agentsview` v0.36.1,
  not fakes).** Env-isolated daemon (`AGENTSVIEW_DATA_DIR`/
  `CLAUDE_PROJECTS_DIR` at scratch dirs, port 18099), `go run . watch
  -server http://127.0.0.1:18099 -tick 5s` against it with a recording
  `zellij` stub on PATH (no real zellij session available in this review
  sandbox). Wrote a synthetic session JSONL + `agentsview session sync`:
  the row was piped within ~1s (well inside AC-6's ≤30s), re-emitted every
  tick with a strictly fresh `ts`, and `agentsview session list --server
  ... --json` reported `"total":1` — exact count parity with the 1 row
  grout piped. This exercises the real shipped binary end-to-end
  (attach, SSE-or-poll-driven refresh, enrichment, emit), not just the
  test fakes.

No REFUTED verdict against AC-1..5. One real parser-level false negative
found (truncated final SSE frame) — mitigated end-to-end by the
reconnect-always-refreshes design, not a blocker.

## Demo script for AC-6 / AC-7 (interactive, CL drives live) and AC-8 framing

**Setup — build and install the plugin from this worktree** (this task
changes `src/main.rs`, unlike the sibling `grout-session-states` task, so a
rebuild is required before anything cross-tab can be observed):

    cd /Users/clkao/git/zaphod/.worktrees/spacedock-ensign-grout-sse-daemon
    ./build.sh          # → target/wasm32-wasip1/release/zellij-sidebar.wasm (verified clean in this review)
    ./install.sh        # writes ~/.config/zellij/layouts/zaphod.kdl pointing at that wasm

Start a **fresh** zellij session on the installed layout (AC-6 requires a
session CL hasn't been driving grout in yet):

    zellij --session grout-validate --layout zaphod

1. **Baseline count** (run in a pane inside that session):

       agentsview session list --include-one-shot --include-automated \
         --include-children --active-since "$(date -u -v-30M +%FT%TZ)" --json | jq '.total'

   Note this number as **N**.

2. **Start the watcher** (same session, any pane):

       cd /Users/clkao/git/zaphod/.worktrees/spacedock-ensign-grout-sse-daemon/grout
       go run . watch

   (Defaults: `-server http://127.0.0.1:8080 -since 30m -tick 60s`; attaches
   to your normal daemon or starts one — it never stops it.)

3. **AC-6, first half (fill + count parity).** Within 30s, the sidebar's
   AGENTS section should show **N** rows (same count as step 1's baseline).
   A mismatch (grout showing fewer or more) is the AC-6 baseline moving the
   wrong way — REJECTED with the observed vs. expected counts.

4. **AC-6, second half (a brand-new session appears unattended).** In
   another tab, start a new Claude/agent session and send its first
   message. Within ≤30s of that message — no grout interaction — a new
   AGENTS row should appear for it.

5. **Test-plan item 1 / AC-7 (cross-tab click).** From the rail's tab,
   click the new session's row. Expected: the view switches to the tab
   holding its bound pane, with that pane focused, and the row shows the
   bound marker (not unbound). This is the one mechanism that could not be
   reproduced offline or in this review's headless sandbox (no live zellij
   session was available here — `focus_terminal_pane`'s tab-switch is
   inherently visual). On refute (view does not switch), the fallback is a
   `go_to_tab` implementation change — file it as a REJECTED finding with
   what was observed instead, not a workaround applied on the spot.

6. **Cleanup.** `Ctrl-C` the `go run . watch` process — expect a clean
   exit (no orphaned `agentsview` process; grout never stops the shared
   daemon so it keeps running for your normal use).

**AC-8 — dogfood-exit framing (not a single-sitting demo).** This settles
from CL's lived experience, not a script: after running with `grout watch`
across a real working day with ≥2 concurrent agent workflows, the question
at the gate is simply — *did finding the next blocked agent go through the
rail, or did you still alt-tab to scan for it?* A "still alt-tabbed"
answer is REJECTED with what the rail failed to surface (missed session,
stale row, wrong state marker), not a soft pass. If CL has already been
running this branch's `grout watch` day-to-day before this gate, that lived
window counts — no separate synthetic dogfood exercise is needed.
