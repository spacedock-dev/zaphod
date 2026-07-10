---
subspace: v0
gate:
  workflow: agent-rail-dev
  entity: canonical-install-and-worktree-test-profile
  entity-title: Canonical install and isolated worktree test profile
  stage: validation
  round: 1
recommendation:
  verdict: REJECTED
  rationale: The runtime boundaries pass when warm, but the committed process suite fails from the required fresh throwaway checkout because its 10-second metadata wait expires during the profile's clean build; interactive AC-6 and AC-7 were not run.
artifact:
  kind: draft
  path: ./canonical-install-and-worktree-test-profile-validation.md
criteria:
  bar: |
    A fresh validator must reproduce AC-1 through AC-5, refute the change from
    a throwaway checkout, and leave AC-6 and AC-7 to CL's attached-session
    observations. A warmed implementation worktree cannot substitute for the
    required cold-checkout regression run.
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
      evidence: "REJECTED: a fresh clone at 7cfd3a9 timed out after 10s waiting for PROFILE_ROOT while the mandatory clean build took 1m42s; the same test passes only after warming the checkout."
    - id: AC-5
      text: "Operator and delivery documentation states executable boundaries."
      evidence: "PASS: README, docking, workflow, and PRD diffs match the executable install/profile boundaries and required delivery order."
    - id: AC-6
      text: "One profile has one candidate identity in a real session."
      evidence: "NOT RUN: CL must drive the real Alt-/ observation; exact script is in the artifact."
    - id: AC-7
      text: "The profile supports the j5 red-baseline/candidate drill."
      evidence: "NOT RUN: CL must drive both baseline and candidate; exact script is in the artifact."
---

Validation recommends rejection at implementation commit
`7cfd3a9b0a9bdf81ecb6c95fa806956799c006f1`. AC-1, AC-2, AC-3, and AC-5
survive independent execution, but AC-4's committed process suite depends on
a warmed build tree and fails in the mandatory fresh audit checkout.

The cold audit reached five passing behavior groups, then emitted
`FAIL: timed out waiting for profile value PROFILE_ROOT`. The profile runs
`build.sh` before it creates or prints `PROFILE_ROOT`; the test polls only
`100 × 0.1s`. A clean build took 1m42s on the validation host. The lifecycle
group passed immediately after that explicit build, proving a warm-cache
dependency rather than a runtime identity failure.

Route back to implementation. Extend readiness to cover a clean build (and
terminate/wait the launcher on timeout), then rerun the full process suite from
a new clone at the candidate SHA. AC-6 and AC-7 remain pending CL; no human
keypress or visual observation is claimed.
