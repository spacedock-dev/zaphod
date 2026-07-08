---
id: 7vdbkanwev18zxfkpgkvhvja
title: Agent kind/state classification regresses to Unknown on partial poll failure, masking working/blocked signals
status: validation
source: finding — live session dogfooding, 2026-07-08
started: 2026-07-08T08:12:13Z
completed:
verdict:
score: 0.75
worktree: .worktrees/spacedock-ensign-agent-classification-regresses-on-partial-failure
issue:
pr:
mod-block:
---

## Problem

Live in CL's `WORK` session: a pane row correctly classified as `claude` /
`idle` flipped back to `unknown . unknown` with no layout change and no
user action — then, separately, the same pane showed an animated
"thinking" spinner (Claude actively generating) that the rail never
reflected as `working`. Both trace to the same root cause in
`src/agent.rs`'s `enrich_fields`:

```rust
if current_command.is_none() && viewport.is_none() {
    return previous.clone();          // only TOTAL failure preserves prior state
}
let kind = [command_kind, title_kind, viewport_kind]
    .into_iter()
    .find(|k| *k != AgentKind::Unknown)
    .unwrap_or(AgentKind::Unknown);   // never falls back to previous.kind
```

`refresh_statuses` (`src/main.rs:967`) calls `get_pane_running_command` and
`get_pane_scrollback` independently per poll (every `STATUS_POLL_SECS` =
2s, gated to the active tab, with a per-pane exponential backoff after
failures — `PollBackoff`, `src/main.rs:109-137`). Zellij's own log shows
these host calls (`GetPaneRunningCommand`/`GetPaneCwd`) timing out
intermittently and continuously across long-lived sessions. When exactly
one of the two calls fails on a given poll (a **partial** failure — the
common case, not the `enrich_fields` early-return's TOTAL-failure case),
`kind` is recomputed from scratch with no memory of the previous value. If
the surviving data point (usually the viewport) doesn't happen to contain
one of the narrow literal phrases `agent_from_viewport`/`agent_from_title`
key off (`"claude code"`, `"codex"`, `"press enter to confirm"`, a literal
`"Working..."` line) — true for essentially all steady-state UI, not just
a splash screen — `kind` regresses to `Unknown` even though it was
correctly known moments earlier.

This compounds into the second symptom: `detect_viewport`
(`src/agent.rs:148-172`) gates its *entire* state heuristic behind kind
being known —

```rust
pub fn detect_viewport(agent: AgentKind, viewport: &[String]) -> ViewportDetection {
    let status = last_non_empty_line(viewport);
    if agent == AgentKind::Unknown {
        return ViewportDetection { state: AgentState::Unknown, status };
    }
    if let Some(status) = blocker_line(viewport) { ... }       // never reached
    if has_working_signal(agent, viewport) { ... }              // never reached
    ...
}
```

So the instant `kind` regresses to `Unknown` from the bug above,
`blocker_line`/`has_working_signal` never run that tick — even with an
active "esc to interrupt" spinner sitting right there in the viewport
text. The rail can silently blind itself to a genuinely working or
blocked agent for one or more poll cycles, self-correcting only once a
fully-successful poll re-establishes both fields together.

## Proposed approach

Confirmed fix, in `enrich_fields` (`src/agent.rs:174-218`):

Before:

```rust
let command_kind = current_command
    .as_deref()
    .map(agent_from_command)
    .unwrap_or(AgentKind::Unknown);
let running_command = current_command.or_else(|| previous.running_command.clone());
let title_kind = agent_from_title(title);
let viewport_kind = viewport
    .as_deref()
    .map(agent_from_viewport)
    .unwrap_or(AgentKind::Unknown);
let kind = [command_kind, title_kind, viewport_kind]
    .into_iter()
    .find(|kind| *kind != AgentKind::Unknown)
    .unwrap_or(AgentKind::Unknown);
```

After (new lines marked):

```rust
let command_failed = command.is_err();          // NEW
let viewport_failed = viewport.is_err();          // NEW — read before `.ok()` shadows `viewport`
let command_kind = current_command
    .as_deref()
    .map(agent_from_command)
    .unwrap_or(AgentKind::Unknown);
let running_command = current_command.or_else(|| previous.running_command.clone());
let title_kind = agent_from_title(title);
let viewport_kind = viewport
    .as_deref()
    .map(agent_from_viewport)
    .unwrap_or(AgentKind::Unknown);
let mut kind = [command_kind, title_kind, viewport_kind]        // NEW: mut
    .into_iter()
    .find(|kind| *kind != AgentKind::Unknown)
    .unwrap_or(AgentKind::Unknown);
// A partial poll failure (exactly one of command/viewport missing) must not
// by itself erase a previously known kind: the missing source, not a change
// in the pane's program, is why nothing matched this tick. A poll where both
// sources succeeded and still came up Unknown is trusted immediately — that
// is a real classification, not a gap (see
// `successful_unknown_sources_clear_stale_agent_kind`).
if kind == AgentKind::Unknown                                    // NEW
    && previous.kind != AgentKind::Unknown
    && (command_failed || viewport_failed)
{
    kind = previous.kind;
}
```

`command_failed`/`viewport_failed` must be captured from the `Result`s
*before* `command.ok()`/`viewport.ok()` discard the `Err` — the existing
code only keeps the `Ok` payloads. No other lines in `enrich_fields` change;
the rest of the function (the viewport-absent early return, the
`detect_viewport` call) already consumes `kind` by value, so both consumers
automatically see the fallback.

This resolves checklist item 2 (`detect_viewport` running
`blocker_line`/`has_working_signal` off the AC-1 fallback) for free: the
`detect_viewport(kind, viewport.as_slice())` call at the end of
`enrich_fields` already passes the (now possibly-fallback) `kind` — no
separate change is needed there. It only runs when `viewport` is `Some`
(command-failed/viewport-ok, the AC-2 shape); when viewport itself failed,
the function already takes the early-return branch that preserves
`previous.state`/`previous.status` directly, so there is no viewport text to
re-scan that tick regardless of `kind`.

**Staleness ceiling: rejected, no ceiling added.** Two reasons:
1. There is no clock or counter available to a pure function like
   `enrich_fields`, and none of the callers thread poll-tick counts into
   `AgentFields` today. Adding one means growing `AgentFields` with a new
   field purely to support an edge case with no reported symptom — the kind
   of invented mechanism this stage is supposed to avoid.
2. The codebase already has a named, precedented policy for exactly this
   trade-off: "stale-not-blank." `pane_cwds`'s doc comment
   (`src/main.rs:96-99`, "A failed poll keeps the previous entry
   (stale-not-blank)") and `refresh_statuses`'s wedge-abort comments
   (`src/main.rs:989-992`, `1002-1006`) both keep prior data indefinitely
   across consecutive failed/skipped polls rather than blanking it after N
   tries. `PollBackoff::record` (`src/main.rs:128-136`) reinforces this: a
   single successful poll — not a timeout — is what clears staleness. Kind
   should follow the same rule its sibling fields already follow: any
   ceiling that force-expires `kind` after N failed polls would reintroduce
   exactly the flicker this fix exists to remove, on the one signal
   (`kind`) that currently gets *worse* treatment than `state`/`status`/
   `pane_cwds` already get.

**Related gap, confirmed out of scope, separate finding.**
`preserve_agent_fields` (`src/main.rs:1965-1971`) and `rows_for_own_tab`
(`src/main.rs:1985-2004`) are a different code path from `enrich_fields`,
triggered by `Event::PaneUpdate` manifest rebuilds
(`src/main.rs:510-515`), not by the 2s poll timer. `rows_for_own_tab`
always constructs fresh `Row`s with `agent: AgentFields::default()`;
`preserve_agent_fields` restores prior agent fields only via an *exact*
`pane_id` match against the old row set. If zellij ever hands out a new
`pane_id` for what a user perceives as the same terminal across a manifest
rebuild, the match fails and the row resets straight to
`AgentFields::default()` (Unknown/Idle/empty) with no fallback and no
self-correction path — worse than the bug this entity fixes, since nothing
here ever re-derives the old identity from title/position. This is a
distinct trigger (manifest/pane_id churn, not poll `Result::Err`), a
distinct code path, and would need a distinct, riskier remediation
(matching identity across a `pane_id` change needs a heuristic — title or
position — with its own false-positive risk, e.g. mis-attributing one
pane's agent state to a different pane that happens to reuse a
position). It is not the same fix and is **not in scope here**; filed as a
follow-up finding rather than folded into this entity's AC set.

## Acceptance criteria

**AC-1 — A known agent kind survives a partial poll failure.**
Verified by: a unit test calling `enrich_fields` with `previous.kind =
Claude`, a failing `command` (`Err`), and a viewport that contains neither
a kind-identifying phrase nor a working/blocker signal — asserts the
returned `kind` is still `Claude`, not `Unknown`.

**AC-2 — A working/blocked signal is detected even when kind is only known
via the previous-value fallback from AC-1.**
Verified by: a unit test with the same partial-failure setup as AC-1, but
a viewport containing `"esc to interrupt"` — asserts the returned `state`
is `Working`, not `Unknown`.

**AC-3 — The existing total-failure and fresh-detection paths are
unchanged.**
Verified by: the existing `enrich_fields`/`detect_viewport` test suite
(`src/agent.rs` `mod tests`) still passes unmodified, confirming the fix is
additive (a new fallback branch) rather than a rewrite of working paths.

## Test plan

Entirely offline and fast — this is pure-function unit-test territory
(`enrich_fields`/`detect_viewport` take no host calls directly), no live
zellij session or spike needed. Riskiest first: AC-1's fallback test, since
it's the one that changes existing behavior; AC-2 and AC-3 follow directly
once AC-1's fallback shape is settled.

## Out of scope

The `GetPaneRunningCommand`/`GetPaneCwd` host-call timeout frequency itself
(why the poll fails at all) — that's `dock-floating-leak-and-chrome-misplacement`'s
territory (an inconclusive timeout-correlation check already ran there).
This entity is about the classification logic not regressing *when* a poll
fails, regardless of why it fails. Also out of scope: the `Row`/`pane_id`
identity-loss gap in `preserve_agent_fields`/`rows_for_own_tab` — ideation
confirmed (see Proposed approach) it is a distinct trigger and code path
from the `enrich_fields` fix here, not the same fix; tracked as a separate
follow-up finding.

## Stage Report: ideation

- DONE: Confirm the exact fallback fix for enrich_fields (kind falls back to previous.kind on partial poll failure) and decide whether it needs a staleness ceiling, with concrete code before/after.
  Read src/agent.rs:174-218; wrote before/after diff in Proposed approach — capture `command_failed`/`viewport_failed` ahead of `.ok()`, fall back `kind` to `previous.kind` only when fresh classification is Unknown and one poll axis failed. Ceiling rejected: no ceiling added, citing the existing "stale-not-blank" precedent (src/main.rs:96-99, 128-136, 989-1006).
- DONE: Confirm detect_viewport (or its caller) still runs blocker_line/has_working_signal when kind is only known via the AC-1 fallback, not only on fresh detection.
  Traced enrich_fields's existing `detect_viewport(kind, viewport.as_slice())` call: it already consumes the (now fallback-capable) `kind` by value, so no separate change to detect_viewport or its caller is needed — confirmed in Proposed approach.
- DONE: Check preserve_agent_fields/rows_for_own_tab for a related pane_id-identity-loss gap and state explicitly whether it's in scope here or a separate finding.
  Read src/main.rs:510-515, 1965-2004: confirmed a real gap (exact pane_id match, no fallback, full reset to default on manifest rebuild) but a distinct trigger/code path from enrich_fields; recorded as out of scope, filed as a separate follow-up finding, not folded into this entity's ACs.

### Summary

Confirmed the `enrich_fields` fix is a single conditional after the existing three-source `kind` derivation: fall back to `previous.kind` only when the fresh result is `Unknown` AND at least one of `command`/`viewport` failed this tick, leaving the existing all-succeeded-Unknown clearing path (`successful_unknown_sources_clear_stale_agent_kind`) untouched. No staleness ceiling — the codebase's existing `pane_cwds`/`PollBackoff` "stale-not-blank" precedent argues against one, and none of the current ACs need it. The `preserve_agent_fields`/`pane_id` gap is real but confirmed as a separate, out-of-scope finding (different trigger: manifest/pane_id churn, not poll `Result::Err`).

## Stage Report: implementation

- DONE: Implement the enrich_fields fallback exactly as designed in ideation: capture command_failed/viewport_failed before .ok() discards the Err, fall back kind to previous.kind only when fresh kind is Unknown and previous.kind is not, and at least one of command/viewport failed.
  src/agent.rs:180-181 captures `command_failed`/`viewport_failed` from the `Result`s before `.ok()`; src/agent.rs:197-212 adds the `mut kind` fallback conditional exactly matching the ideation diff. Commit 8d754e1.
- DONE: Write AC-1/AC-2/AC-3's three unit tests as designed (kind survives partial failure; working/blocked signal detected via the fallback; existing test suite passes unmodified), red test first, minimal fix, suite green.
  Added `partial_failure_preserves_known_kind_when_fresh_result_is_unknown` (AC-1) and `partial_failure_fallback_kind_still_detects_working_signal` (AC-2) to src/agent.rs mod tests; both failed red before the fix, both pass after. AC-3 verified by the full pre-existing suite passing unmodified (no test bodies changed). Commit 8d754e1.
- DONE: Confirm no staleness ceiling was added, per ideation's explicit rejection of one, and record before/after test counts plus the exact red output in the stage report.
  No clock/counter/new AgentFields field added — fallback is a pure comparison against `previous.kind` and the two `Result`s already in scope. Before: `cargo test` 130 passed (20 tests in agent.rs). After: 132 passed (22 in agent.rs), 0 failed. `cargo check --tests` clean.

  Red output (both new tests, before the fix):
  ```
  test agent::tests::partial_failure_fallback_kind_still_detects_working_signal ... FAILED
  test agent::tests::partial_failure_preserves_known_kind_when_fresh_result_is_unknown ... FAILED

  ---- agent::tests::partial_failure_fallback_kind_still_detects_working_signal stdout ----
  thread '...' panicked at src/agent.rs:516:9:
  assertion `left == right` failed
    left: Unknown
   right: Claude

  ---- agent::tests::partial_failure_preserves_known_kind_when_fresh_result_is_unknown stdout ----
  thread '...' panicked at src/agent.rs:501:9:
  assertion `left == right` failed
    left: Unknown
   right: Claude

  test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 130 filtered out
  ```

### Summary

Added the single fallback conditional to `enrich_fields` designed in ideation: `kind` falls back to `previous.kind` only when the fresh three-source derivation is `Unknown`, `previous.kind` is known, and at least one of `command`/`viewport` failed this poll — leaving the all-succeeded-Unknown clearing path untouched. Two new unit tests (AC-1, AC-2) went red for the predicted reason (`Unknown` where `Claude` expected) before the fix and green after; the full pre-existing suite (AC-3) passed unmodified, 130 to 132 total. No staleness ceiling was added, matching ideation's rejection.
