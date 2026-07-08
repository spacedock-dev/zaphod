---
id: 7vdbkanwev18zxfkpgkvhvja
title: Agent kind/state classification regresses to Unknown on partial poll failure, masking working/blocked signals
status: backlog
source: finding — live session dogfooding, 2026-07-08
started:
completed:
verdict:
score: 0.75
worktree:
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

Ideation should confirm and detail the exact fix (the mechanism is already
pinned down; this is design-and-test work, not further root-causing):

1. `enrich_fields` should fall back to `previous.kind` when the freshly
   computed `kind` is `Unknown` but `previous.kind` was not — mirroring the
   fallback `previous.state`/`previous.status` already get in the
   viewport-absent branch. Open question for ideation: should this fallback
   have a staleness ceiling (e.g. don't trust a `previous.kind` from more
   than N polls ago, in case the pane's actual running program genuinely
   changed to something unrecognized)?
2. `detect_viewport` (or its caller) should still evaluate
   `blocker_line`/`has_working_signal` against the viewport even when kind
   is only known via the fallback in (1), not just when freshly detected —
   otherwise a stale-but-still-correct kind still blinds state detection.
3. Check `preserve_agent_fields` (`src/main.rs:1965`) and
   `rows_for_own_tab` for a related gap: does a `Row` losing its `pane_id`
   match (e.g. after a layout rebuild) reset straight to
   `AgentFields::default()` (Unknown) with no analogous grace period? If
   so, name whether that's in scope here or a separate finding.

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
identity-loss question named in Proposed approach step 3, unless ideation
finds it's the same fix.
