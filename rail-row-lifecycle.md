---
title: Rail row lifecycle — expiry and unbind debounce
status: backlog
source: plan sprint 1 + rows-section validation audit
score: 0.7
id: hjmtja3p7nyt0qfz4cfmydk5
---

## Problem

Rows never expire (10k-line ingest quantified at O(n^2) with per-render
bind cost linear in sessions x rows — the audit marked expiry load-bearing
before any chatty grout ships), and binding follows raw cwd reads which are
observably flaky per-read (CL's probe: same pane null one read, real value
the next) — a transient miss must not flap a row between bound and unbound.

## Proposed approach (seed — ideation refines)

Row expiry driven by event ts + a staleness horizon; unbind requires N
consecutive missed/changed reads (bind stays eager, unbind gets debounced).
Pure decision fns over the existing sessions/gates state; offline TDD.
