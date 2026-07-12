---
title: Sprint 2 walking-skeleton staff coherence review
status: ideation
source: captain direction 2026-07-12
completed:
verdict:
score: 1.0
worktree:
issue:
pr:
mod-block:
sprint: s2-dependable-per-tab-attention-loop
group: release-gate
sprint-readiness: ready
blocked-on:
id: 9j214d92fg2vxa5wmapxxxbj
started: 2026-07-12T01:58:05Z
---

## Problem

Before Sprint 2 moves from coherent task designs to implementation, an independent staff review must establish whether its session-ingestion, gate-projection, and review-surface tasks compose into one end-user walking skeleton. The review treats the expected fully validated result of Sprint 1 task `7h` as an explicit conditional contract, while preserving the fact that `7h` is currently rejected at validation.

## Required outcome

Produce a cited staff assessment of the Sprint 2 release-gate record plus `bb`, `s9`, and `qt`: their end-user sequence, ownership boundaries, dependency order, and a clear assumption ledger for what a fully validated `7h` does and does not grant. Recommend the smallest coherent Sprint 2 outcome and any deferrals needed to protect it.

## Review boundaries

Do not alter code, roadmap, or any reviewed task. Do not alter `7h` or `4d`. Do not treat `ProfileLeaseV1` as canonical tab identity, session-incarnation proof, a second-client grant, or gate-skill authority unless the reviewed records independently establish that claim.

## Acceptance criteria

**AC-O1 — The review maps each Sprint 2 task to one operator-visible step and identifies every cross-task prerequisite.**

Verified by: a cited task-to-outcome/dependency matrix whose sources are the current task records and roadmap, not assumptions in the review itself.

**AC-O2 — The review separates the counterfactual fully-validated `7h` contract from `7h`'s current rejected validation state.**

Verified by: citations to `7h`'s acceptance criteria, implementation report, and validation report, including both its claimed ProfileLeaseV1 boundaries and its unresolved AC-O1 readiness failure.

**AC-O3 — The review gives one actionable coherence recommendation, including what may progress and what remains deferred.**

Verified by: a proposed demo sequence that names its independent proof and explicitly prohibits unsafe identity, origin, and review-routing inferences.

## Out of scope

Implementing any Sprint 2 task, approving a gate, changing Sprint 1, or replacing the gate skill's external v1 contract.

## Staff Review

### Verdict

Sprint 2 is conditionally coherent as one walking skeleton, but it is not
implementation- or release-ready today. Keep the three delivery legs and `e6`
as the final operator-loop proof; repair and fully validate `7h` first, then
admit each leg only when its independent contract proof is available. The
roadmap makes the three tasks one journey rather than a component sequence and
keeps `e6` deferred until their validated behavior exists
(`docs/roadmap.md:121-146`).

### Task-to-operator-outcome and dependency matrix

| Record | Operator-visible step | Prerequisites and ownership boundary |
| --- | --- | --- |
| `e6` — `first-dependable-per-tab-attention-loop` | The final journey shows one real session and one pending review in the current tab, focuses the session or opens the provider UI, and later shows truthful provider state. | It remains backlog until `7h` and all three delivery tasks validate; it may not introduce a hub, managed tab, launcher, adoption, or inline review controls (`first-dependable-per-tab-attention-loop.md:19-37`; `docs/roadmap.md:145-164`). |
| `bb` — `live-current-tab-sessions` | A profile-scoped subscriber shows metadata-proven top-level current-tab sessions, focuses only a unique local pane, and expires stale rows. | It consumes the passed lease's attach object verbatim, has no cleanup/controller authority, and depends only on passed `7h` for its first live spike and implementation (`live-current-tab-sessions.md:37-50,104-124,149-172,227-241`). It owns neither gates nor review controls (`live-current-tab-sessions.md:279-289`). |
| `s9` — `pending-gates-appear-where-the-work-came-from` | A gate with exact verified origin appears only in its origin rail; absent, malformed, stale, or unsupported origin remains globally visible; later complete provider snapshots update or remove rows. | The present baseline has no authoritative origin carrier, open-gate snapshot, or safe raw-tab substitute (`pending-gates-appear-where-the-work-came-from.md:29-61`). Full bound placement requires the configured provider, `GateOriginFixtureBundleV1`, and authoritative canonical identity; reconciliation and projection never acquire review authority (`pending-gates-appear-where-the-work-came-from.md:95-168,202-244`). |
| `qt` — `v1-gate-review-surface-handoff` | From the rail the operator selected, one accepted handoff opens one visible receiver-local reviewer and returns cleanly; the gate skill validates and routes the result. | It needs a real pending-gate projection, passed `7h`, and an offer/result-capability contract that the current v1 probe lacks (`v1-gate-review-surface-handoff.md:42-47,70-112`). Zaphod owns only exact runtime-surface lifecycle; the gate skill owns result validation and every decision (`v1-gate-review-surface-handoff.md:114-161,240-251`). |

The journey therefore composes as: safe disposable profile -> `bb` session
attention and local focus -> `s9` provider-truth projection -> `qt` accepted
surface handoff -> provider resolution -> `e6` truthful post-resolution proof.
`bb` and `s9` are independent delivery legs, but `qt` cannot demonstrate the
operator journey before a pending-gate projection exists, and `e6` depends on
all three.

### `7h` conditional ledger

`7h` is currently `implementation` with `verdict: REJECTED`
(`foreground-attached-client-profile.md:1-18`). Its implementation report
claims raw PTY, lease, cleanup, and isolation evidence
(`foreground-attached-client-profile.md:327-338`), but validation refuted
AC-O1 with zero-terminal and absent-canary runs, found a target-free readiness
failure, and did not run AC-I1 (`foreground-attached-client-profile.md:340-375`;
`gates/foreground-attached-client-profile-validation.md:12-46,108-120`). No
later task may consume the claimed result today.

| Assumptions permitted only after full validation | Assumptions still forbidden even then |
| --- | --- |
| A foreground-attached disposable profile; deterministic raw PTY input to one real terminal; an immutable bounded `ProfileLeaseV1`; normal/signal cleanup; and standing-config isolation (`foreground-attached-client-profile.md:59-125,152-231`). | Canonical tab identity or session incarnation; marker ownership; a second client; controller, permission, or candidate-binary authority; gate origin; gate-skill authority; or a review-surface handoff (`foreground-attached-client-profile.md:112-125,278-290`). |

The lease may therefore give `bb` exact disposable attach inputs, but it never
makes its `native_session_id` an origin token. `s9` still needs its external
identity authority, and `qt` still needs a gate-skill-owned accepted
offer/result path.

### Evidence-required correction and staged recommendation

Before `e6`, correct `bb`'s known shared-CWD ambiguity. Its planned test
covers a foreign-only CWD and duplicate CWDs within one tab, but not one source
session whose CWD occurs in two tabs (`live-current-tab-sessions.md:203-213`).
The retained validation audit reproduced the latter: each rail uniquely bound
the same session locally despite no foreign-pane focus path
(`gates/grout-sse-daemon-validation.md:181-206`). Add a two-tab same-CWD case
that leaves the row unbound or omitted in both rails; it must never return
`FocusPane`. If that cannot be established from the available manifest, declare
shared-CWD tabs outside this release claim and exclude them from the `e6` demo.
Do not invent a tab identity to fix it.

The smallest coherent sequence is:

1. Repair `7h` raw-input readiness and cold-run metadata timing, rerun its
   full offline packet repeatedly and target-free, then let CL run AC-I1. Keep
   its passing lease, cleanup, no-TTY, and isolation boundaries intact
   (`foreground-attached-client-profile.md:362-375`; `gates/foreground-attached-client-profile-validation.md:114-120`).
2. After that pass, run `bb`'s lease-to-row root/child invalidator, then its
   offline packet and current-tab focus/staleness drill. Only `bb` may become
   implementation-ready at this point (`live-current-tab-sessions.md:149-172,243-267`).
3. Obtain the external provider snapshot/origin fixture and review
   offer/result contracts. A gate without usable origin may be demonstrated
   only as a global row, and clicking it selects the current receiver rail; it
   is not tab-bound placement (`pending-gates-appear-where-the-work-came-from.md:142-155,263-289`; `v1-gate-review-surface-handoff.md:72-78`).
4. Keep `s9`'s bound branch deferred until exact canonical identity is proven,
   and keep `qt` completion deferred until duplicate delivery proves one pane,
   one gate-skill result, and no fallback (`v1-gate-review-surface-handoff.md:175-192,253-276`).
5. Run `e6` last: one real session, one real pending gate, correct local focus,
   one reviewer, provider-side resolution, and truthful later projection. This
   is the release proof, not a reason to broaden architecture.

## Stage Report: ideation

- DONE: Map the Sprint 2 release gate, session, gate-projection, and review-surface tasks to one operator journey with cited cross-task prerequisites.
  AC-O1 evidence: `Staff Review`'s task-to-operator-outcome and dependency matrix cites `e6`, `bb`, `s9`, `qt`, and `docs/roadmap.md:121-146`; it identifies `7h`, provider-snapshot/origin/identity, pending-gate projection, and offer/result contracts without adding an inferred dependency.
- DONE: Build a strict “if 7h fully validates” assumption ledger that contrasts the claimed ProfileLeaseV1 result with 7h’s current rejected validation evidence.
  AC-O2 evidence: `Staff Review`'s `7h` conditional ledger cites 7h's ACs (`foreground-attached-client-profile.md:59-125,152-231`), implementation report (`:327-338`), validation report (`:340-375`), and independent gate artifact (`gates/foreground-attached-client-profile-validation.md:12-46,108-120`).
- DONE: Give one smallest coherent Sprint 2 recommendation, including explicit deferrals and a proof-bearing demo sequence; modify no reviewed task, code, 7h, or 4d.
  AC-O3 evidence: `Staff Review`'s staged recommendation names the independent 7h, root/child, provider-snapshot/identity, accepted-handoff, and `e6` proofs; it reserves global fallback for absent origin and prohibits identity, origin, and review-routing inference. The shared-CWD audit citation records the only evidence-required correction.

### Summary

The task records form one conditional operator journey: local session attention,
provider-truth gate discovery, one accepted review surface, and truthful return.
`7h` currently blocks live work, external v1 contracts block bound gates and
review completion, and the shared-CWD ambiguity must receive a safe disposition
before the release gate claims correct focus.
