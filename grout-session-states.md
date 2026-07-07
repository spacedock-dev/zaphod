---
title: Grout session-state mapping + default-path fix
status: ideation
source: plan sprint 1 + sprint-0 gate findings
score: 0.9
id: kawd2h37e2rhn9t9ynf61ppe
started: 2026-07-07T23:00:49Z
---

## Problem

Live AGENTS rows carry no signal: grout emits agentsview's
`termination_status` verbatim (e.g. `awaiting_user`) while the plugin maps
only `blocked|working|idle|done`, so every real session wears the blank
Unknown marker (rows-section validation, cross-slice gate finding). And
grout's default gate-log path is cwd-relative — it never resolves under
`go run .` (CL hit it live; absolute argv[2] was the workaround).

## Proposed approach (seed — ideation refines)

Map agentsview's real state vocabulary (survey it from the DB/source:
awaiting_user, running, completed, ...) onto the four plugin states in
grout's rows.go — the plugin contract stays untouched. Fix the default
gate-log path to resolve from the executable/module location or make the
argument required. Both grout-side; offline-testable end to end.
