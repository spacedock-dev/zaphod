---
title: Grout SSE daemon — sessions for real
status: ideation
source: plan sprint 1 (docs/plan-agent-rail.md)
score: 0.8
id: ybqh2eyp10qjbv05bd7njtbw
started: 2026-07-07T23:00:50Z
---

## Problem

Grout is one-shot: one session, one gate log, run by hand. The rail's
daily value needs every active session appearing, updating, and
disappearing without CL running anything.

## Proposed approach (seed — ideation refines)

Long-running grout: `agentsview serve --background` + SSE `data_changed`
consumption with `session list/get --server` enrichment (events carry no
content), emitting row updates for many sessions; gate-log glob config
replaces the single positional; cross-tab binding gets its enabling zellij
fact re-checked (focus_terminal_pane goes via go_to_tab). Dogfood exit:
CL stops alt-tabbing to find blocked agents.
