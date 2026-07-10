---
subspace: v0
gate:
  workflow: agent-rail-dev
  entity: canonical-install-and-worktree-test-profile
  entity-title: Canonical install and isolated worktree test profile
  stage: validation
  round: 4
recommendation:
  verdict: REJECTED
  rationale: "Cycle-4 targeted poll repair passes and cold 11/11 passes, but timeout diagnostics still block on an unread stderr pipe for 5s under a 1s budget; AC-6 remains pending CL and AC-7 belongs to the post-v9 j5 gate."
artifact:
  kind: draft
  path: ./canonical-install-and-worktree-test-profile-validation.md
criteria:
  bar: |
    A fresh validator must reproduce AC-1 through AC-5, refute the change from
    a throwaway checkout, and leave interactive observations to CL. A readiness
    deadline must bound the complete poll, including transcript reads.
  acceptance:
    - id: AC-1
      text: "A linked worktree cannot change a global Zaphod install."
      evidence: "PASS: linked installer refused before writes; primary control installed against the same sentinel fixture."
    - id: AC-2
      text: "Canonical installation refuses split URL or configuration identity."
      evidence: "PASS: foreign, mixed, missing-keybind, and missing-rail inputs preserved the sentinel; coherent and commented-example controls installed."
    - id: AC-3
      text: "The worktree profile is self-contained and preserves global bytes."
      evidence: "PASS when warm: live dump reported only the candidate URL; normal, TERM, and INT cleanup preserved sentinel hashes and removed profile sessions."
    - id: AC-4
      text: "Regression coverage is red first and rejects false confidence."
      evidence: "REJECTED at 16b4faa: cold 11/11 and built-in polling pass, but a blocked stderr consumer makes the 160-line timeout diagnostic return after 5s under a 1s budget."
    - id: AC-5
      text: "Operator and delivery documentation states executable boundaries."
      evidence: "PASS: README, docking, workflow, and PRD diffs match the executable install/profile boundaries and required delivery order."
    - id: AC-6
      text: "One profile has one candidate identity in a real session."
      evidence: "NOT RUN: CL must drive the real Alt-/ observation; exact script is in the artifact."
    - id: AC-7
      text: "The profile supports the j5 red-baseline/candidate drill."
      evidence: "DEFERRED TO j5 GATE: v9 must land first, then j5 rebases and CL drives baseline/candidate; exact script and prerequisites are in the artifact."
---

Cycle-2 validation recommends rejection at
`8c2fe5ecbdccaa822d0bd9a305638384638a26fe`. A fresh detached clone passed
all eight shell groups; Rust 132/check, Go 35/vet, and missing-Zellij
fail-loud evidence also passed. A target-free build took 2m05s and completed
inside the liveness-aware readiness window; a dead launcher failed in 1s.

The full default-expiry attack found a narrower cleanup defect. At timeout,
the parent, session, and profile disappeared and both global hashes matched,
but a TERM-ignoring child remained alive. Lines 28-33 escalate to KILL only
while `PROFILE_LAUNCHER_PID` lives, so a child that outlives its parent
escapes. The committed timeout fixture has no child and cannot catch this.

Route back to implementation with that exact probe. AC-6 remains pending CL's
real keypress. AC-7 is explicitly carried to j5's gate: v9 must land before
j5 rebases and runs the two-checkout demo. No interactive observation is
claimed in this round.

## Cycle 3 current recommendation

Cycle-3 validation rejects and escalates at
4957f6c515ed803607802dfacae03118e4e3f867. The exact descendant attack now
passes, an unrelated canary survives cleanup, and the shell suite passes 10/10
with a fresh target-free detached lifecycle. Rust 132/check, Go 35/vet, and
missing-Zellij fail-loud evidence pass.

The hard wall-clock claim remains false at a blocking poll boundary. With a
1-second timeout and a tr shim that delays the transcript read by 2 seconds,
wait_for_profile_value returns after 3 seconds. Lines 77-78 check SECONDS only
before the unbounded tr/awk command substitution. The normal 10-second test
passes because its poll is fast. Metadata written exactly at the 1-second
boundary is rejected at 1 second, consistent with an exclusive deadline.

This is rejection cycle 3, so escalate rather than auto-route another ordinary
implementation cycle. AC-6 remains pending CL's real keypress. AC-7 remains at
j5's later gate after v9 lands and j5 rebases. No interactive result is claimed.

## Cycle 4 current recommendation

Cycle-4 validation recommends rejection at
16b4faa4cd1a26ad21ee9c39493ba228dfcebd47. The exact delayed-tr regression
passes within one second, descendant/process-group cleanup removes all state,
an unrelated canary survives, and the complete shell suite passes 11/11 with
its fresh target-free detached lifecycle. Rust 132/check, Go 35/vet, missing
Zellij, signal cleanup, hash preservation, and leak checks pass.

Built-in transcript parsing handles a partial line after its newline arrives,
file replacement, and a 1 MiB line within budget. The remaining diagnostic path
is unbounded: with a 1-second timeout and stderr connected to a consumer that
holds the pipe open without reading, print_transcript_excerpt blocks until the
consumer exits. Exact result was status 141 after 5 seconds. The timeout cannot
meet its hard return budget while reporting 160 lines to a blocked sink.

AC-6 remains pending the captain's real Alt-/ observation. AC-7 remains
deferred to j5 after v9 lands and j5 rebases. No human observation is claimed.
