# Agent rail PRD

This document records the rail's current delivery boundary and the dependency
order for the remaining demoable slices. `docs/plan-agent-rail.md` defines the
sprint work; `docs/docking-approach.md` defines the shipped Zellij container.

## Product boundary

Current main contains the per-tab Zaphod plugin and one-shot grout ingestion.
The yb validation branch adds the AgentsView SSE and session-watch path. The pz
validation branch adds gate discovery, server addressing, and the
review/verdict return path. Durable workflow truth remains in
Spacedock/subspace decision logs. Grout is the external adapter, and each tab's
WASM is an ephemeral view and action surface.

## Architecture and data flow

```text
Agent transcripts -> AgentsView ---------------------\
                                                        -> grout (external adapter)
Spacedock gates + durable decision logs -------------/          |
                                                             agent-event pipe
                                                                    v
Zellij pane/tab events ---------------------> per-tab ephemeral Zaphod WASM
                                                -> render | focus | review | verdict
                                                                               |
waiting agent resumes <- decision log <- subspace verdict endpoint <-----------+
```

## Verified delivery order

First land canonical install and the isolated worktree test profile. Then land
j5's no-terminal-leaf retrofit after the profile records current main red and
the candidate green. Resolve eh's leaked-instance and chrome corruption on top
of j5, then land 7v's partial-poll classification preservation.

Rebase yb's SSE daemon onto that result and repeat its live validation before
merge. Land hj only after yb because row expiry consumes yb's fresh-`ts` and
stop-refresh seam. Finally, rebase pz onto yb and confirm the gate walking
skeleton again.

Before pz becomes a standing action, land a confirmation affordance in front
of its irreversible one-click approve POST. The required drill must also fail
loudly when the pinned `spacedock-subspace` dependency is absent; a skipped
test with a green suite is not a gate.

This order is load-bearing. Canonical install prevents every later live gate
from testing mixed artifacts. j5 establishes the deterministic one-terminal
baseline before eh investigates population and race corruption. yb spans the
plugin, grout, and docs, and must rebase after 7v. hj depends on yb's timestamp
seam. pz predates yb, overlaps grout and rail code, and must prove its
cross-repo drill after the rebase.
