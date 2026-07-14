---
id: 441wyy3208egsxq5az3yfzy1
title: Five-second scrollback lookup must not congest Zellij
status: ideation
source: live nautical-cuckoo diagnosis 2026-07-14
sprint: s1-managed-tab-safety
group: release-blocking-hotfix
sprint-readiness: ready
started: 2026-07-14T05:54:11Z
completed:
verdict:
score: 1.0
worktree:
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
