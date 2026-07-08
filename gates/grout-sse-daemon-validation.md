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

---

## Cycle 2 (2026-07-08) — revalidation after same-tab-only rewrite

Cycle 1 was REJECTED at the live demo: CL wants projects scoped strictly by
tab (no cross-tab bind/click) and subagent sessions must never be listed.
Implementation cycle 2 (commit `8b04a4f`) reverted cross-tab binding
entirely and dropped `--include-automated --include-children`. This section
independently re-validates that commit; it supersedes cycle 1's AC-4/AC-7
verdict above and the cross-tab demo steps, which are stale by design
(cross-tab binding/click no longer exist in the code).

Worktree at review: `.worktrees/spacedock-ensign-grout-sse-daemon` @ `8b04a4f7`.
Refutation checkout (throwaway, never the implementation worktree): fresh
`git clone` + `git checkout 8b04a4f`, discarded after this review.

### Offline AC verdicts (independently re-run)

| AC | Verdict | Command | Evidence |
|---|---|---|---|
| AC-1 (unaffected) | PASS | `cd grout && GOPROXY=off go test -count=1 -run TestWatchSessionsFlow ./...` | `ok zaphod/grout` |
| AC-2 (unaffected) | PASS | `cd grout && GOPROXY=off go test -count=1 -run 'TestWatchAutoStart\|TestWatchReconnect' -v ./...` | `TestWatchAutoStart`, `TestWatchReconnect`, `TestWatchReconnectOnSilence` all PASS |
| AC-3 (unaffected) | PASS | `cd grout && GOPROXY=off go test -count=1 -run TestWatchWedgedPipe -v ./...` | `--- PASS: TestWatchWedgedPipe (0.89s)` |
| AC-4 (rewritten: same-tab-only, zero cross-tab rows/clicks) | PASS | `cargo test -- bind_never_crosses_tabs sessions_outside_own_tab_scope_are_filtered_entirely out_of_scope_sessions_do_not_occupy_a_click_line` (full: `cargo test && cargo check --tests`) | 3/3 named tests PASS; full suite 133/133 PASS; `cargo check --tests` clean |
| AC-5 (unaffected) | PASS | `cd grout && GOPROXY=off go test -count=1 ./... && go vet ./...` | `ok zaphod/grout`, `go vet` silent |

Independent counts: Rust 133/133 (matches the implementer's reported
136→133, net −3). Go test count unchanged at 16 top-level funcs (matches
implementer's claim of an argv-only change, no new/removed Go test funcs).

### Refutation audit — targeting the two cycle-2 behavior changes

**Same-tab-only filtering (`sessions_in_own_tab`, `main.rs:295`):**

- **Attack: cross-tab click/bind via an ambiguous multi-tab cwd match.**
  Read `bind_session`/`decide_rail_click`'s only inputs: `self.rows` /
  `self.pane_cwds`, both rebuilt on every manifest event from
  `rows_for_own_tab(&manifest, self.plugin_id)` (`main.rs:542`), which
  resolves the plugin's own tab via `own_tab_position` (`main.rs:2026`) and
  only ever collects panes from `manifest.panes[own_tab]` (`main.rs:2038`).
  No foreign-tab pane id or cwd can enter these structures at all.
  **REFUTED as structurally impossible** — not "usually excluded", there is
  no code path through which a foreign-tab pane id reaches `bind_session` or
  `decide_rail_click`.
- **Attack: a session whose cwd is shared by a pane in two different
  tabs.** Wrote a throwaway test (discarded with the checkout) giving two
  independent tab instances one pane each at the identical cwd
  `/shared/proj`, and one session at that cwd. Result: **survives** — the
  session is in-scope and uniquely bound in *both* tabs' independent
  evaluations (each tab's rail only ever sees its own single-pane match, so
  each legitimately binds locally). This does not violate AC-4's letter (no
  cross-tab data flow occurs — see above) but is a real residual scope
  ambiguity: the design narrative's "a session belongs to whichever tab's
  pane cwd it matches" assumes cwd-to-tab uniqueness that isn't enforced
  anywhere. Worth flagging to CL as a known edge case (two tabs opened at
  the same path double-list a session), not blocking — CL's own workflow
  structures distinct projects into distinct tabs, so non-overlapping cwds
  is the expected case.
- **Attack: symlink/path-shape mismatch hiding an in-scope session.**
  `normalize_cwd` (`main.rs:268`) is an identity function; the adjacent
  comment already documents that `get_pane_cwd` returns the OS-resolved
  physical path (symlinks resolved) while agentsview reports whatever the
  session recorded, and pins exact-match as provisional. A mismatch here
  fails **safe** (session goes invisible, not cross-tab) — consistent with
  the anti-leak intent, not a new regression from cycle 2. Survives, not
  blocking.

**Dropped `--include-automated --include-children` (`grout/list.go:33`):**

- **Attack: a subagent session reaching a row through some other code
  path.** Grepped every `exec.Command` site touching agentsview
  (`grout/agentsview.go:34`, `grout/list.go:34`, `grout/watch.go:82`).
  `agentsview.go`'s `FetchSession` (`session get <id>`, no
  automated/children flags exist for `get`) is the *only* other session
  source, but it is one-shot mode's explicit-ID lookup
  (`grout/main.go:44`), never a discovery/list query — a different threat
  model from the cycle-1 feedback ("we shouldn't list subagents", aimed at
  watch mode's periodic discovery) and explicitly out of scope per AC-5
  (one-shot untouched). Considered and set aside, not a leak of the kind
  AC-6 guards against.
- **Attack: no redundant check if agentsview's own default has a gap.**
  `grout`'s `sessionInfo` struct (`grout/agentsview.go:18`) carries no
  automated/child field at all, and `grep -n "automated|subagent|is_child"
  src/main.rs` in the plugin returns nothing real (only unrelated KDL-tree
  "children"). Confirmed: the *entire* enforcement is the omitted argv,
  pinned by `TestListSessionsArgv` (`grout/list_test.go:39`). This is the
  design's stated trust boundary ("agentsview's own default"), not a code
  defect, but it means zero defense-in-depth exists if that default is
  ever wrong.
- **Attack: empirically probe agentsview's own child-session classifier.**
  Started a real, env-isolated `agentsview` v0.36.1 daemon (own scratch
  `AGENTSVIEW_DATA_DIR`/`CLAUDE_PROJECTS_DIR`, port 18099 — never CL's real
  data) and wrote a synthetic session JSONL with interleaved
  `"isSidechain":true` messages, mimicking a Claude Code subagent
  transcript. Result: **inconclusive** — the synthetic session synced as an
  ordinary top-level session (`"is_automated":false`, no child-classifying
  field in the API response at all) and appeared identically with or
  without `--include-children`. My synthetic construction did not exercise
  agentsview's real child-session marker (most likely a wrong guess at its
  format — sidebar messages may not become a separate "child session" the
  way I assumed), and this sandbox has no filesystem access to
  `/Users/clkao/git/agentsview`'s source to confirm the actual mechanism.
  Flagging this honestly rather than claiming verification: `--help`'s
  documented default ("Include subagent/child sessions" / "excluded by
  default") is taken on trust, unconfirmed by this audit at the mechanism
  level.

No REFUTED against the shipped behavior. One real residual (shared-cwd
double-listing) named for CL's awareness; one verification gap (agentsview's
own child-session classifier) named rather than papered over.

### Revised demo script for AC-6 (AC-7 superseded/dropped, AC-8 separate)

**Setup — build and install from this worktree's cycle-2 commit:**

    cd /Users/clkao/git/zaphod/.worktrees/spacedock-ensign-grout-sse-daemon
    ./build.sh          # → target/wasm32-wasip1/release/zellij-sidebar.wasm
    ./install.sh        # writes ~/.config/zellij/layouts/zaphod.kdl pointing at that wasm

Start a **fresh** zellij session on the installed layout, with the demo
tab's panes already opened at the project(s) whose sessions should count
(AC-6 is scoped to *this tab's own* project cwds only):

    zellij --session grout-validate-c2 --layout zaphod

1. **Scoped baseline count.** Note the cwd(s) of the demo tab's own panes,
   then compute the baseline restricted to those cwds and excluding
   automated/child sessions (agentsview's own default — no
   `--include-automated`/`--include-children`):

       agentsview session list --include-one-shot \
         --active-since "$(date -u -v-30M +%FT%TZ)" --json \
       | jq --arg cwd "<demo tab's pane cwd>" '[.sessions[] | select(.cwd == $cwd)] | length'

   Note this number as **N**. (Repeat/sum per distinct own-tab pane cwd if
   the tab has panes in more than one project.)

2. **Start the watcher** (same session, any pane):

       cd /Users/clkao/git/zaphod/.worktrees/spacedock-ensign-grout-sse-daemon/grout
       go run . watch

   (Defaults: `-server http://127.0.0.1:8080 -since 30m -tick 60s`; attaches
   to your normal daemon or starts one — it never stops it.)

3. **AC-6, first half (fill + scoped count parity).** Within 30s, the
   sidebar's AGENTS section in the demo tab should show **N** rows — no
   more, no fewer. Sanity check: a session known to live in a *different*
   tab's project must NOT appear here. A subagent/automated session, if one
   is active, must also not appear or count on either side. A mismatch in
   either direction is the AC-6 baseline moving the wrong way — REJECTED
   with the observed vs. expected counts and which session(s) diverged.
4. **AC-6, second half (a brand-new same-tab session appears unattended).**
   In the *demo tab's own project* (a pane already open there, or a new
   pane opened in that same cwd), start a new Claude/agent session and send
   its first message. Within ≤30s of that message — no grout interaction —
   a new AGENTS row should appear in the demo tab.
5. **Negative check (out-of-scope session never appears).** In *another*
   tab's project (a different cwd not matching any of the demo tab's own
   panes), start a new Claude/agent session and send its first message.
   Confirm it never appears as a row in the demo tab, at any point —
   there is no cross-tab click step in this cycle's demo (AC-7 is
   superseded by AC-4's offline test; do not attempt to click into another
   tab's pane).
6. **Cleanup.** `Ctrl-C` the `go run . watch` process — expect a clean
   exit (no orphaned `agentsview` process; grout never stops the shared
   daemon so it keeps running for your normal use).

**AC-8 — unchanged, a separate dogfood-window question** (see the framing
above); not part of this demo script.

---

## Cycle 3 (2026-07-08) — revalidation after the id-prefix subagent filter

Cycle 2 was REJECTED at the live demo: dropping `--include-automated
--include-children` did not stop subagent sessions from accumulating, because
`is_automated` was `false` for every real Task-tool subagent CL captured (59
of them). Implementation cycle 3 (commits `e3619bb`, `153862d`) replaced that
trust boundary with `excludeSubagents`: drop any session whose `id` starts
with the literal prefix `agent-`, the shape every Task-tool-spawned subagent
gets. This section independently re-validates that fix. **Diff scope this
cycle is grout-only** (`git diff --stat 8b04a4f..153862d`: `grout/list.go`,
`grout/list_test.go`, `grout/README.md` — no `src/main.rs`/wasm change), so
unlike cycles 1/2 no plugin rebuild is required for this fix specifically.

Worktree at review: `.worktrees/spacedock-ensign-grout-sse-daemon` @
`153862da9c7409c4533d99c3442894d9c8089b7d` (HEAD, clean).

### Offline AC verdicts (independently re-run, fresh `-count=1`)

| AC | Verdict | Command | Evidence |
|---|---|---|---|
| AC-1 (unaffected) | PASS | `cd grout && GOPROXY=off go test -count=1 -run TestWatchSessionsFlow -v ./...` | `--- PASS: TestWatchSessionsFlow (0.49s)` |
| AC-2 (unaffected) | PASS | `cd grout && GOPROXY=off go test -count=1 -run 'TestWatchAutoStart\|TestWatchReconnect' -v ./...` | `TestWatchAutoStart`, `TestWatchReconnect`, `TestWatchReconnectOnSilence` all PASS |
| AC-3 (unaffected) | PASS | `cd grout && GOPROXY=off go test -count=1 -run TestWatchWedgedPipe -v ./...` | `--- PASS: TestWatchWedgedPipe (0.90s)` |
| AC-4 (unaffected, plugin untouched this cycle) | PASS | `cargo test -- bind_never_crosses_tabs sessions_outside_own_tab_scope_are_filtered_entirely out_of_scope_sessions_do_not_occupy_a_click_line` (full: `cargo test && cargo check --tests`) | 3/3 named tests PASS; full suite 133/133 PASS; `cargo check --tests` clean |
| AC-5 | PASS | `cd grout && GOPROXY=off go test -count=1 ./... && go vet ./...` | `ok zaphod/grout`, `go vet` silent |
| Subagent-id filter | PASS | `cd grout && GOPROXY=off go test -count=1 -run 'TestExcludeSubagents\|TestListSessionsFiltersSubagents' -v ./...` | both PASS; cwd-collision case (`TestExcludeSubagents`) confirmed a shared cwd does not save the `agent-`-prefixed row |

Function names verified verbatim against source (`grep -n "fn bind_never_crosses_tabs\|fn sessions_outside..."  src/main.rs`), not assumed from the AC text.

### Live-data verification (independently pulled, not the implementer's captured sample)

Queried the currently-running local `agentsview` daemon (pid 66006, v0.36.1)
myself, across three independent slices totaling ~2,500 real sessions (not
reusing the implementer's 500-session capture): most-recent 500
(`--sort started:desc`), oldest 500 (`--sort started:asc`, spans
2025-11-18..2026-01-30, versions 2.1.15..2.1.25), and a mid-range 500
(`--date-from 2026-04-01 --date-to 2026-04-15`). Cross-checked the `id`
prefix against `relationship_type`/`parent_session_id` — an independent
ground-truth field neither the implementer's nor my own filter logic
consults — as the oracle for "is this really a subagent":

- **False positives** (id starts with `agent-` but `relationship_type` ≠
  `subagent`): **0 of ~2,500** sampled sessions, across `claude`/`codex`
  history, multiple CC versions, and one live real-time confirmation (my own
  ensign session, `agent-aspacedock-ensign-grout-sse-daemon-validation-…`,
  classified `subagent` the instant `agentsview` observed it — no
  classification lag).
- **False negatives** (real subagent, `id` does not start with `agent-`):
  **0 for `claude`** across all samples. **Found for `codex`**: real
  Codex-spawned subagents (`relationship_type: "subagent"`, real
  `parent_session_id`) use `codex:<uuid>` ids, never `agent-`-prefixed —
  e.g. `codex:019d4f0d-cf13-7480-ac25-295491b7a9a7`, parent
  `codex:019d4c95-f95c-7ea1-bd9f-792057b3e689`, a real dispatched worker in
  CL's own `spacedock` project. Confirmed on the shipped code, not just by
  inspection: a throwaway `TestRefutation_CodexSubagentIDShapeSurvivesFilter`
  fed that exact captured record through the real `excludeSubagents` — it
  survives the filter (not dropped).

### Refutation audit (throwaway checkout, never the implementation worktree)

Fresh `git clone` of the repo + `git checkout 153862d` in an isolated scratch
dir (`/private/tmp/.../scratchpad/throwaway-clone`), discarded clean after
(`rm -rf`, no working-tree changes left behind).

- **Attack: construct a real non-subagent session whose id starts with
  `agent-` (false positive).** Not found in ~2,500 live samples or by
  construction — the prefix is generated by a single internal code path
  (Task-tool spawn), never user- or session-content-controlled; no other
  agent family (`codex:`, `pi:`, `cowork:`, `gemini:`) or top-level session
  shares the scheme. **Survives** (no false positive found).
- **Attack: find a real subagent whose id does NOT start with `agent-`
  (false negative) — different spawn mechanism.** **Confirmed real and
  reproducible**: Codex's own subagent/exec mechanism produces
  `codex:<uuid>` ids for genuine children (`relationship_type: "subagent"`),
  which `excludeSubagents`' `strings.HasPrefix(id, "agent-")` check cannot
  catch by construction — proven against the shipped code with the captured
  record above, not just reasoned about. **Currently masked end-to-end**:
  `listSessions` never passes `--include-children`, and I independently
  confirmed `agentsview`'s own default already excludes every
  `relationship_type: "subagent"` session server-side for both `codex`
  (807→198 total with the flag dropped) and `claude` (19348→2408) in today's
  live data — so this specific gap has no live path to a rendered row today.
  **Residual risk named for CL's awareness, not blocking this gate**: the
  design narrative and AC frame the id-prefix filter as *the* subagent
  defense (chosen specifically because cycle 2 showed `agentsview`'s
  classifiers "don't reliably catch" Task-tool subagents); for Codex
  sessions the actual defense today is silently the opposite — agentsview's
  own default, the exact kind of layer cycle 2 already caught failing once
  for Claude. CL actively dispatches Codex-based workers (the captured
  example is a real `spacedock` ensign), so if agentsview's default
  classification ever regresses for `codex` the way it did for `claude` in
  cycle 2, no backstop exists. Scope note: CL's cycle-1/2 complaints named
  "Claude subagent sessions" specifically, and the AC's literal wording
  (`id` does not start with `agent-`) only ever promised Task-tool/Claude
  coverage — so this is not a violation of the AC as written, but a gap in
  the AC's coverage relative to the general "we shouldn't list subagents"
  intent.
- **Attack: nested sub-subagent (an Explore/refutation agent spawned from
  within a Task-tool subagent).** Searched ~1,000 of the most recent
  sessions (including today's heavy spacedock dispatch activity) for any
  session whose `parent_session_id` itself starts with `agent-`: **0
  found**. Inconclusive rather than a clean survive/refute — no nested
  example exists in the searched window to test against, and coverage was
  ~1,000 of 20,235 total sessions system-wide. Flagging the gap honestly
  rather than claiming proof either way.
- **Attack: malformed/garbage session id (empty string, no prefix at
  all).** `excludeSubagents` is a plain `strings.HasPrefix`; an empty or
  garbage id simply fails the prefix check and passes through unfiltered —
  no panic, matches every other session's treatment. Survives.

No REFUTED against any stated AC. One real, live-data-confirmed, code-proven
false-negative class found (Codex subagent ids) — currently non-blocking
because a different, already-once-unreliable layer (agentsview's own
default) happens to mask it today; named explicitly rather than left buried
in a PASS.

### Final demo script for AC-6 / subagent-id filter (interactive, CL drives live)

**Setup.** This cycle's fix is grout-only (Go); no plugin/wasm change. If
your zellij layout is already running the plugin build validated at cycle 2
or later, skip straight to the watcher. If unsure, rebuild to be safe:

    cd /Users/clkao/git/zaphod/.worktrees/spacedock-ensign-grout-sse-daemon
    ./build.sh && ./install.sh

Start (or reuse) a zellij session on the installed layout, with the demo
tab's panes already open at the project(s) whose sessions should count.

1. **Scoped, subagent-excluded baseline.** Compute the baseline by the exact
   mechanism grout now uses — cwd scope AND `id` does not start with
   `agent-` — not by trusting `agentsview`'s own default (this is
   deliberate: the default's `--include-children` exclusion would mask a
   regression in grout's own filter, since cycle 2 already showed that kind
   of classifier can be wrong). Pull the superset yourself and filter by id
   prefix in `jq`:

       agentsview session list --include-one-shot --include-children --include-automated \
         --active-since "$(date -u -v-30M +%FT%TZ)" --json \
       | jq --arg cwd "<demo tab's pane cwd>" \
         '[.sessions[] | select(.cwd == $cwd) | select(.id | startswith("agent-") | not)] | length'

   Note this number as **N**. Repeat/sum per distinct own-tab pane cwd if the
   tab has panes in more than one project.
2. **Start the watcher:**

       cd /Users/clkao/git/zaphod/.worktrees/spacedock-ensign-grout-sse-daemon/grout
       go run . watch

3. **Fill + count parity.** Within 30s, the demo tab's AGENTS section shows
   **N** rows, no more, no fewer. A mismatch is REJECTED with the observed
   vs. expected counts and which session(s) diverged.
4. **Unattended appearance.** In the demo tab's own project, start a new
   Claude/agent session and send its first message. Within ≤30s, no grout
   interaction, a new row appears.
5. **Subagent-cwd-collision check (the crux of this cycle's fix).** While
   `grout watch` is running, dispatch a Task-tool subagent (e.g. an `Explore`
   agent, or any sub-agent) whose cwd is the *same* project as the demo tab's
   own panes. Confirm it never appears as a row, at any point, even though
   its cwd is in-scope — a cwd match must not save a subagent from the
   filter (this is the exact scenario `TestExcludeSubagents`/
   `TestListSessionsFiltersSubagents` pin offline; this step is the live
   confirmation).
6. **Out-of-scope negative check.** In another tab's project, start a new
   session; confirm it never appears in the demo tab. No cross-tab click
   step exists anywhere in this script — AC-7 stays superseded by AC-4, and
   binding/rendering never cross tabs by construction (cycle 2's structural
   REFUTED-as-impossible finding still holds; this cycle changed no
   tab-scoping code).
7. **Cleanup.** `Ctrl-C` the `go run . watch` process — clean exit, no
   orphaned `agentsview` process (grout never stops the shared daemon).

**AC-8 — unchanged, a separate dogfood-window question**, not part of this
script.
