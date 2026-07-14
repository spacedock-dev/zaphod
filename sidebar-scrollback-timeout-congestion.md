---
id: 441wyy3208egsxq5az3yfzy1
title: Five-second scrollback lookup must not congest Zellij
status: implementation
source: live nautical-cuckoo diagnosis 2026-07-14
sprint: s1-managed-tab-safety
group: release-blocking-hotfix
sprint-readiness: ready
started: 2026-07-14T05:54:11Z
completed:
verdict:
score: 1.0
worktree: .worktrees/spacedock-ensign-sidebar-scrollback-timeout-congestion
issue:
pr:
mod-block:
---

## Problem

With the managed sidebar loaded, its two-second status timer synchronously calls Zellij's pane-scrollback host API. Zellij 0.44.3 can hold that call for five seconds, multiplying queue congestion: `Alt p` fails to create a pane promptly and `Alt n` can leave a half-created empty tab whose queued actions arrive later. The immediate outcome is that the main sidebar may remain loaded while ordinary pane and tab actions stay responsive; degraded metadata is preferable to blocking Zellij.

## Required outcome

Remove or circuit-break the five-second scrollback lookup from the periodic plugin path without redesigning the whole metadata architecture. Reproduce the slow/non-shell-pane case and prove pane creation, tab creation, and tab switching complete within a tight independent deadline with no delayed burst after the sidebar is closed.

## Follow-up boundary

All other synchronous pane metadata calls and the durable cache/sidecar design belong to `nonblocking-pane-metadata-architecture`.

## Proposed approach

Make one tactical deletion from the two-second status refresh: do not call
Zellij's `get_pane_scrollback` host export from `Sidebar::refresh_statuses`.
Do not replace it with an in-WASM timeout or a circuit breaker. The export is
synchronous; a timer that notices the five-second return afterward cannot
protect Zellij's screen queue while the call is blocked.

The refresh continues to obtain the running command and CWD exactly as it does
today. It passes an unavailable viewport to the existing pure
`agent::enrich_fields` function. That function already implements the desired
degradation: a known command or title can still identify the agent kind, while
the last known state/status remains stale-not-blank; a new pane with no usable
signal is `unknown . unknown`. Session association continues through the
existing pure `bind_session` function: exactly one CWD match binds, while
missing or ambiguous CWD renders `unbound` and is never guessed.

This is intentionally not the final metadata architecture. In particular,
`get_pane_running_command` and `get_pane_cwd` are also synchronous host calls,
but have separate 100 ms server budgets and existing wedge/backoff handling.
Removing them, introducing an external collector, coalescing work, defining
cache freshness, and owning collector cancellation all remain in
`nonblocking-pane-metadata-architecture`. This task adds no process, worker,
watcher, channel, or persisted cache and therefore needs no operator-started
watcher. Its cancellation behavior is the absence of scrollback work: hiding
or closing the sidebar leaves no outstanding lookup to cancel and no result
that can arrive later.

The implementation should create a small host-call seam around one periodic
refresh pass so the contract can be tested without sleeping inside a Rust
test. The seam supplies command and CWD results and includes a scrollback trap
which would block if invoked; production must never invoke that trap. It
extends `agent::enrich_fields` and `bind_session` only through their existing
inputs and behavior, not by teaching either pure function about host calls.

### User-visible documentation diff

- `README.md`'s feature list will stop promising a latest prompt/status
  refresh every two seconds. It will say the periodic sidebar refresh uses
  best-effort command/CWD metadata, preserves the last known status when live
  viewport data is unavailable, and shows a new unresolved pane as unknown.
- `SPEC.md` will retain the prototype's two-second scrollback behavior as
  historical evidence but add a safety note that the shipped path retired it
  because Zellij 0.44.3 can synchronously hold that export for five seconds.
- `docs/docking-approach.md`'s polling record will name scrollback as retired
  from the periodic WASM path and point the remaining synchronous metadata
  risk at `nonblocking-pane-metadata-architecture`.

## Acceptance criteria

### Offline

**AC-O1 — the periodic WASM path cannot start a pane-scrollback lookup.** A
due status refresh with visible terminal rows completes without invoking its
scrollback dependency, even when that dependency is a trap that would block
longer than Zellij's five-second host timeout. Command/title classification
still updates; prior state and status remain unchanged; a fresh unresolved
shell row renders `unknown . unknown`.

Verified by: a focused Rust test drives one refresh through the injected host
seam on a worker thread, gives it an independent one-second watchdog (the
repository's existing `WEDGE_THRESHOLD`, not a value chosen by the
implementation), and asserts zero scrollback calls. Existing and extended
`agent::enrich_fields` table tests assert the exact degraded fields for prior,
known-agent, and fresh-shell rows. A deliberately bad fixture that invokes the
trap must time out, proving the watchdog can detect the regression.

**AC-O2 — ordinary pane and tab actions remain responsive with a long-running
non-shell pane present.** In an attached disposable Zellij 0.44.3 session with
the managed sidebar and a long-running non-shell fixture, each of three literal
`Alt p` inputs creates exactly one terminal pane within one second; literal
`Alt n` creates exactly one complete tab containing a terminal pane and makes
it active within one second; literal `Alt 1` and `Alt 2` each activate the
requested tab within one second.

Verified by: an extension of `tests/zellij-tmux-smoke-test.sh` and its real PTY
input path. The isolated config fixture owns those exact bindings. After every
input, a separately bounded native `list-panes`/`list-tabs` observation checks
terminal count, tab count, nonzero pane count, and active tab ID. The one-second
deadline comes from `WEDGE_THRESHOLD` and is independently far below the
observed five-second scrollback hold. Success means the resulting native state
changed on time, not merely that an action command returned.

**AC-O3 — closing the sidebar cannot release a delayed action burst.** After
the AC-O2 sequence, closing the exact plugin pane changes only plugin
cardinality. With no further input, terminal count, tab count, pane IDs, and
active tab remain byte-for-byte stable for six seconds, exceeding the old
five-second host timeout.

Verified by: the same tmux/Zellij test records native state immediately after
`zellij action close-pane --pane-id plugin_<id>` and again after an external
six-second wait. It rejects a half-created zero-pane tab, extra panes/tabs, an
active-tab change, or any other queued-action arrival.

**AC-O4 — failure and cleanup are contained.** A missed action deadline or
failed native observation terminates the disposable test without acting on an
ordinary development session. The tmux server, Zellij session, non-shell
fixture, temporary config/data/socket roots, and any test-owned process are
gone; standing `config.kdl` and `layouts/zaphod.kdl` remain in their exact
pre-test present/missing state.

Verified by: existing smoke-harness traps plus an injected timeout case. The
test retains its phase/evidence packet, kills only recorded test-owned process
and session identities, verifies their absence with bounded checks, and
compares pre/post file-state digests. No user-started watcher is part of setup
or cleanup.

### Interactive

**AC-I1 — ordinary development remains responsive in the real slow-pane
scenario.** In a fresh managed tab using the candidate WASM and the captain's
ordinary Zellij config, a real `subspace-tui` pane is open. After at least one
two-second sidebar timer tick, three `Alt p` presses each visibly create one
pane in under one second, `Alt n` visibly creates and focuses a complete tab in
under one second, and `Alt 1` then `Alt 2` visibly switch tabs in under one
second. Closing the sidebar's exact plugin pane and waiting six seconds causes
no delayed pane/tab creation or focus jump.

Verified by: the captain performs the literal-key demo only after AC-O1 through
AC-O4 are green. An external control terminal captures bounded native pane/tab
state after each key and before/after the six-second no-input window. The demo
uses a fresh managed tab because hot-reloading or retrofitting the existing
plugin instance belongs to `managed-main-wasm-reload-loop`; it does not edit
standing KDL and does not reinstall the sidebar after closing it.

## Test plan

The riskiest unproven claim is that scrollback is the only synchronous call
responsible for the observed five-second congestion in the real
`subspace-tui` case. Command and CWD calls remain deliberately in scope of the
broader architecture task, so this claim must be invalidated before the hotfix
is allowed to expand.

1. **Run the invalidating spike first.** Make the minimum candidate that omits
   `get_pane_scrollback`, load it in a fresh attached disposable Zellij 0.44.3
   session, open a real `subspace-tui` pane, wait through a timer tick, and
   drive the AC-O2 literal-key sequence with one-second native-state deadlines.
   Repeat up to three fresh sessions. If any candidate action misses its
   deadline or bursts later, the tactical design is invalid: preserve the
   event/native-state evidence and route it to
   `nonblocking-pane-metadata-architecture`; do not add command/CWD redesign,
   a watcher, or a cache to this hotfix. If the current build cannot reproduce
   a slow event in three attempts, record that the real-world trigger is not
   deterministic and rely on the blocking host trap for the regression, not a
   false claim that the live baseline was reproduced.
2. Add the host seam and AC-O1 Rust regression. Run the focused test, then
   `cargo test` and `cargo check --tests`. The scrollback trap is the
   deterministic five-second-risk reproduction; the long-running non-shell
   pane exercises the real Zellij scheduling surface.
3. Extend the isolated tmux/Zellij fixture with `Alt p`, `Alt n`, `Alt 1`, and
   `Alt 2`; add the non-shell fixture and exact state/deadline assertions for
   AC-O2. Keep every native observer bounded so a congested control plane
   produces a diagnostic failure rather than hanging the test runner.
4. Add AC-O3's close-and-six-second no-input window and AC-O4's injected
   failure cleanup. Preserve phase logs, before/after native snapshots, and
   standing-file digests as the evidence packet.
5. Apply the README, SPEC, and docking-approach diff, then run the focused
   shell suite and the relevant full shell suite.
6. Only after the offline packet is green, give the captain the exact AC-I1
   demo. A failing live demo blocks the hotfix and hands evidence to the
   broader metadata task; it does not trigger an in-session retrofit or manual
   watcher workaround.

## Out of scope

Removing or restructuring `get_pane_running_command` or `get_pane_cwd`;
external collectors, watchers, sidecars, caches, freshness protocols,
concurrency limits, or durable cancellation; changing the two-second timer;
redesigning agent-state inference; guessing a session when CWD is unavailable
or ambiguous; retrofitting or hot-reloading an already-running managed tab;
changing the captain's standing Zellij config; and broad performance claims
beyond the pane, tab, switching, and delayed-burst behavior measured above.

## Stage Report: ideation

- DONE: Defined the smallest tactical boundary: the periodic WASM status pass
  never starts `get_pane_scrollback`; unavailable viewport data degrades
  through `agent::enrich_fields`, while all other synchronous metadata and
  durable collection architecture remain in the named follow-up.
- DONE: Specified independently timed, native-state acceptance for pane
  creation, complete tab creation, tab switching, and the absence of a delayed
  action burst after the exact sidebar plugin pane closes.
- DONE: Put the real `subspace-tui` invalidating spike first. A miss routes
  evidence to the broader architecture task instead of expanding this hotfix;
  a deterministic blocking-host trap guards the five-second regression when
  the environmental trigger cannot be reproduced reliably.
- DONE: Covered stale/unknown/unbound behavior, bounded failure cleanup,
  standing-config preservation, the concrete documentation changes, and the
  captain's exact post-offline live demo. No manual watcher is required.

### Feedback Cycles

#### Convergence round 4 authorized — 2026-07-14

- The captain authorized one bounded implementation repair and one exact-head
  fourth `code_completion` panel. This does not reset the stable-contract
  three-round budget or authorize broader command/CWD metadata redesign.
- Repair only the three surviving proof-truth gaps from parent `1001`:
  preserve every pane tuple and sidebar identity across actions; bind the
  native `tail` fixture to `TAB_ID` and prove sidebar receipt plus a completed
  periodic refresh; and start each one-second deadline before `send_literal`
  so delivery latency counts.
- A PASS parent may advance to validation. Another FAIL returns to the captain
  with the frozen head and surviving findings; do not launch a fifth panel.

#### Convergence round 5 proof reframe authorized — 2026-07-14

- Parent `1073` showed that incremental assertion repair is preserving the
  wrong proof shape: whole-screen text, partial tuple predicates, and broad log
  greps can all pass without proving the claimed pane or refresh event.
- Keep the product behavior and operator-visible end value frozen. Reframe the
  native proof around structured, pane-targeted observations: compare the full
  relevant pane/sidebar state, parse an exact refresh record whose `pane_ids`
  contains the fixture, and use a deterministic barrier that holds refresh
  work in flight while measured keys are sent under the pre-send deadline.
- The four parent-1073 findings are one proof-authority cluster and must be
  eliminated atomically, including adjacent wrong-pane/wrong-field variants.
  Run one final exact-head `code_completion` panel after the reframe. PASS may
  advance to validation; FAIL returns to the captain and no further panel is
  authorized.

## Stage Report: implementation

- DONE: Prove red-first that a periodic refresh never invokes pane scrollback, while command/title classification and exact stale/unknown/unbound degradation remain correct.
  Red `93b681a` failed with `left: 2, right: 0`; green `65923a3` removes the periodic call, and `e8906e5` plus `a1c0389` prove linear command/CWD work, zero viewport calls, and a source boundary against direct bypass.
- FAILED: Prove in isolated real Zellij that literal pane creation, complete tab creation, tab switching, and the six-second post-close window meet their native-state deadlines without delayed bursts, including failure cleanup.
  The success and injected-timeout runs pass, but parent `1001` found three remaining false-positive paths in pane identity, fixture/timer attribution, and the end-to-end deadline.
- FAILED: Ship the scoped documentation and exact-head verification evidence required by the implementation stage, without absorbing command/CWD redesign or managed reload work.
  README, SPEC, and docking docs are scoped and green; exact-head native checks pass, but all three required `code_completion` parents failed, so implementation exit evidence is incomplete.
- SKIPPED: Captain AC-I1 live `subspace-tui` demonstration.
  `subspace-tui` is unavailable on this host, and the offline convergence gate blocks the captain demo before validation.
- DONE: Frozen candidate and exact green checks recorded.
  Head `a1c0389ee20eae3df9ae0bbaee1cf4be60d87dad` passes Rust 142/142 plus validator 6/6, `cargo check --tests`, fresh-build responsiveness and injected cleanup, docs, artifact, new-tab, and layout-capture suites.
- DONE: Before/after native counts and standing-state evidence recorded.
  Plugin tests rose from 138 to 142; standing config remains `8ce2a42d...a196` and layout remains `f1004741...d6e`.
- DONE: Parent `958` findings dispositioned; members `955`, `956`, and `957` all failed.
  Fresh-state redesign was rebutted and explicitly accepted by quick `997`; README promises, tab-local proof, active new-tab identity, and the shell fixture were fixed in `56ea9f4`, `4d8677d`, `5a2aba2`, `525870c`, and `c6f1f82`.
- DONE: Parent `987` findings dispositioned; members `984`, `985`, and `986` all failed.
  Commit `525870c` binds the added terminal to the new active tab, and `a1c0389` prevents direct scrollback bypass inside both periodic refresh entry points.
- FAILED: Parent `1001` remains authoritative FAIL; member `998` passed, while `999` and `1000` failed.
  Existing pane tuples/sidebar identity, fixture-to-sidebar timer attribution, and pre-send absolute deadlines remain unresolved and are all `MUST FIX NOW`.
- FAILED: Preserve every existing pane's `(id, is_plugin, tab_id, plugin_url)` and the sidebar identity across each action.
  Estimated scope: harness-only tuple projections before and after every literal pane/tab action.
- FAILED: Bind the native `tail` fixture to `TAB_ID` and prove the sidebar received it and completed a periodic refresh before keys.
  Estimated scope: a small test/debug refresh marker plus exact fixture and sidebar observations.
- FAILED: Start each one-second deadline before `send_literal` and carry the same absolute deadline through native observation.
  Estimated scope: harness-only absolute deadline plumbing; tmux delivery time must count.
- FAILED: Three-round review convergence gate blocks round four.
  Parents `958`, `987`, and `1001` consumed the stable-contract budget; no fourth panel or post-gate code change was launched.

### Summary

The frozen candidate removes periodic scrollback and passes its offline behavior and cleanup suites without command/CWD redesign or managed reload work. Authoritative review remains blocked on three proof-truth gaps, all classified `MUST FIX NOW`; the convergence gate now requires captain direction before any round four.

## Stage Report: implementation (cycle 2)

- DONE: Preserve each existing pane's full native tuple across every literal
  action.
  Red `db08feb` failed because the tuple predicate did not exist. Green
  `6c0d2e5` adds focused moved-terminal and replaced-sidebar attacks, preserves
  `(id, is_plugin, tab_id, plugin_url)` for `Alt p` and `Alt n`, and requires
  exact tuple-inventory equality for `Alt 1` and `Alt 2`.
- DONE: Carry one pre-send absolute deadline through key delivery and native
  observation.
  Red `4a5951a` failed with `zaphod_action_deadline_ms: command not found`.
  Green `dec7508` starts each measured one-second deadline before
  `send_literal`. The full suite later exposed the exact correct new-tab state
  after the deadline; `6bbcf72` kept the same deadline and collected the two
  independent native inventories concurrently.
- DONE: Bind the long-running native fixture to the managed tab and require
  sidebar and timer acknowledgments before the measured keys.
  Red `8d526da` failed with `sidebar did not complete a periodic refresh
  containing tail pane 2`. Green `157addf` asserts the fixture's `TAB_ID`,
  title, command, and live terminal identity; sees its title in the docked
  client; and records a debug-gated completed refresh containing its pane ID.
  The debug gate and Zellij log remain inside the disposable smoke profile.
- DONE: Run the semantic adversarial pass over the repaired proof.
  The tuple matrix covers an allowed added terminal, terminal tab movement,
  sidebar replacement, exact equality, and changed equality. The asynchronous
  path names the isolated Zellij server as owner, the stable managed tab and
  terminal pane as recipients, visible receipt and refresh completion as
  acknowledgments, one absolute action deadline, and conclusive Zellij/tmux
  cleanup after an injected timeout.
- DONE: Record frozen-head green evidence.
  Head `6bbcf721919111d086e5a110b97ff9202134b39c` passes Rust 142/142,
  `cargo check --tests`, the focused tuple/deadline test, docs, artifact,
  selected-checkout new-tab, native layout-capture, and the fresh-build
  responsiveness plus injected-timeout cleanup wrapper. Standing config
  remains `8ce2a42d...a196`; standing layout remains `f1004741...d6e`.
- DONE: Clear the exact-tip quick cost gate.
  `roborev wait HEAD` reviewed `6bbcf72` and reported no issues before the
  fourth panel launched.
- FAILED: The single authorized fourth `code_completion` panel did not pass.
  Synthesis parent `1073` reviewed
  `a5fc0f3649ac903bc45c737f188e50808a101a71..6bbcf721919111d086e5a110b97ff9202134b39c`
  with panel `code_completion`. Required members `1070` (`correctness`), `1071`
  (`journey`), and `1072` (`proof`) each ran once without execution failure and
  returned FAIL; the authoritative parent returned FAIL.
- FAILED: Parent `1073` found four surviving Medium proof gaps.
  The whole-screen title check can match the terminal border; tuple predicates
  omit exited, suppressed, floating, and selectable sidebar state; the log
  grep can match the pane number outside `pane_ids`; and the measured actions
  begin after refresh completion instead of overlapping deterministic refresh
  work. Each finding needs a captain disposition because the authorized repair
  round is exhausted.
- SKIPPED: Captain AC-I1 live `subspace-tui` demonstration.
  `subspace-tui` remains unavailable, and the failed offline convergence panel
  blocks validation.

### Summary

The bounded fourth round repaired parent `1001`'s three named gaps and froze a
green candidate at `6bbcf72`. Parent `1073` nevertheless found four new Medium
false-positive paths. The branch remains frozen, no fifth panel was launched,
and implementation returns to the captain for a new convergence decision.

## Stage Report: implementation (cycle 3)

- FAILED: Replace broad screen/log matches with structured pane-targeted
  observations that atomically prove the exact terminal, sidebar, full relevant
  state, and refresh `pane_ids` membership.
  Commits `f6f146d..b4e4940` add wrong-pane and wrong-field attacks, and
  `dd91519..0d41aa7` replace text greps with exact JSON refresh records; parent
  `1152` found that action preservation still omits four terminal state fields.
- FAILED: Hold deterministic periodic refresh work in flight while sending each
  measured action under one pre-send absolute deadline, with adjacent
  wrong-pane and wrong-field variants failing.
  Red `4a3da23` failed with `sidebar did not hold a fixture refresh in flight`;
  green `7898a3b` adds per-action refresh IDs and a 400 ms disposable barrier,
  but parent `1152` found that completion before native observation can pass.
- FAILED: Keep product behavior and scope frozen, rerun the complete relevant
  green packet, then launch exactly one final exact-head `code_completion`
  panel and stop on either verdict.
  Product layouts remain unchanged and the full packet is green, but the one
  authorized final panel returned FAIL; no further panel was launched.
- DONE: Parse each refresh lifecycle atomically.
  Red `dd91519` rejected the new record, green `0d41aa7` binds exact event,
  plugin, refresh ID, and unique numeric `pane_ids`; `8e1dce3..b3cce7f` makes
  abort a terminal event.
- DONE: Bind the native fixture's exact command.
  Red `c69ec7` accepted `tail -f /tmp/wrong`; green `79245eb` requires the
  exact `tail -f /dev/null` argv shape and passes the live smoke.
- DONE: Record frozen-head green evidence.
  Head `79245ebc16d891640a6885a9770e63c39bb0b33f` passes Rust 143/143,
  `cargo check --tests`, focused structured proof, docs, artifact, new-tab,
  layout capture, and live responsiveness plus injected cleanup. Standing
  config remains `8ce2a42d...a196`; layout remains `f1004741...d6e`.
- DONE: Clear the final exact-tip quick gate.
  `roborev wait HEAD` reviewed `79245eb` and reported no issues.
- FAILED: The single final `code_completion` panel did not pass.
  Parent `1152` reviewed
  `a5fc0f3649ac903bc45c737f188e50808a101a71..79245ebc16d891640a6885a9770e63c39bb0b33f`;
  members `1149` (`correctness`), `1150` (`journey`), and `1151` (`proof`) each
  ran once without execution failure and returned FAIL. The parent returned
  FAIL with two Medium findings: overlap ends too early, and terminal lifecycle
  fields are absent from action-preservation tuples.
- SKIPPED: Captain AC-I1 live `subspace-tui` demonstration.
  `subspace-tui` remains unavailable, and the failed offline panel blocks
  validation.

### Summary

The proof reframe removed broad screen and log matches, added exact refresh
authority, and froze a green candidate at `79245eb`. Parent `1152` still found
two false-positive paths. The final-panel authority is exhausted, so the branch
remains frozen and returns to the captain.
