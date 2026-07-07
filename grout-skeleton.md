---
title: Grout skeleton
status: ideation
source: plan sprint 0 (docs/plan-agent-rail.md)
score: 0.8
id: fvfk1a5c1xcrbcjp6ap0z157
started: 2026-07-07T04:49:25Z
---

## Problem

The walking skeleton needs the glue half: nothing today turns agentsview
sessions and subspace decision logs into rail rows. Grout (Go, new `grout/`
dir) is that binding — and sprint 0 only needs its thinnest thread.

## Proposed approach (seed — ideation refines)

Hardcoded config; one agentsview session via one-shot `session get --json`;
one gate log (the subspace playground fixture); emit both typed row kinds
({kind:"session"…} and {kind:"gate"…}) as JSON lines on pipe name
`agent-event` via `zellij pipe`, fire-and-forget with a kill timer (the pipe
CLI can hang — see plugin-pipe-unblock). Verify row payload size limits while
the skeleton is up (untested in the spike).
