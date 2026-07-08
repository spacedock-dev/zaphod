---
subspace: v0
gate:
  workflow: agent-rail-dev
  entity: grout-session-states
  entity-title: Grout session-state mapping + default-path fix
  stage: validation
  round: 1
recommendation:
  verdict: PENDING-DEMO
  rationale: Offline AC1-AC6 independently reproduced PASS; refutation audit found no valid REFUTED (one pre-existing, unreachable panic path noted, out of this task's surface); AC-I1 awaits your live demo (script in the artifact).
artifact:
  kind: draft
  path: ./grout-session-states-validation.md
criteria:
  bar: |
    The demo is the gate. Offline ACs must be independently re-run by a
    fresh agent (not re-read from the implementer's report), a refutation
    audit must run on a throwaway checkout naming concrete attack scenarios
    attempted, and the interactive AC is settled only by CL's live demo in a
    fresh zellij session.
  acceptance:
    - id: AC1
      text: For each of the four survey-observed termination_status values, a fresh-activity session row carries a state inside the plugin's marker vocabulary (0/4 Unknown, was 4/4).
      evidence: "go test ./... -run TestSurveyStatusesRenderInsideMarkerVocabulary -v: PASS"
    - id: AC2
      text: MapSessionState reproduces every row of the mapping table across the recency boundaries.
      evidence: "go test ./... -run TestMapSessionState -v: 17/17 subtests PASS"
    - id: AC3
      text: The recorded fixture (testdata/session-get.json), decoded via decodeSession, yields state blocked.
      evidence: "go test ./... -run TestSessionRowFromFixture -v: PASS"
    - id: AC4
      text: A status outside the surveyed vocabulary with stale activity passes through verbatim, no crash, no guess.
      evidence: "go test ./... -run TestMapSessionState/drift_status_stale -v: PASS"
    - id: AC5
      text: grout with fewer than two positionals exits 2 and prints usage naming both, identically from the repo root and from grout/.
      evidence: "go test ./... -run TestCLIRequiresBothPositionals -v: PASS from both cwds; also hand-verified against the built binary"
    - id: AC6
      text: A full offline run (fake agentsview + fake zellij) pipes a session row whose state is blocked, not awaiting_user.
      evidence: "go test ./... -run TestEmitEndToEnd -v: PASS"
    - id: AC-I1
      text: In a fresh zellij session, grout against a currently-live agent session renders a real marker (never blank Unknown); re-running the original repro (go run . from grout/ without a gate-log) yields the usage error instead of the silent cwd-dependent open failure.
      evidence: "pending — demo script in the artifact; settled only by CL's live drive"
---

Offline ACs (AC1-AC6) were independently re-run in this review (not re-read
from the implementer's stage report) against the implementation worktree at
`c2954d23b61941d6b2b754f325c74180638d37c8` — all PASS. A refutation audit ran
on a fresh throwaway checkout at the same commit (never the implementation
worktree): eight adversarial probes attempted (clock-skew age, negative/zero
summary-clamp bytes, malformed decode input, missing gate-log/brief pair, a
fixture timestamp-ordering quirk, caller-impact grep, and a spot-check
against a genuinely live agentsview-daemon session pulled via its HTTP API).
Seven survived; one surfaced a real but pre-existing, unreachable panic
(`clampSummary` with negative `maxBytes` — code this task didn't touch, fed
only a hardcoded `512` today) — noted, not a validation blocker.

AC-I1 is the interactive exit: the plugin's marker vocabulary
(`src/agent.rs`) is untouched by this task, so no rebuild is needed — the
demo just needs grout run from the worktree against a real live session
inside your usual zellij session. Full script, including the discovery
command for a live session id (there is no fixed one — that's the point of
AC5's fix), is in the artifact.
