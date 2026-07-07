---
title: Plugin pipe-unblock fix
status: backlog
source: plan sprint 0 (docs/plan-agent-rail.md)
score: 0.9
id: 2n00q0w1g33531ewx1azgw4e
---

## Problem

The `zellij pipe` CLI never exits after delivery when a config-matched wedged
plugin instance exists (2026-07-07 spike, verbatim receipt then 13s+ hang),
and an instance wedged in `GetPaneRunningCommand` (~13s per host call) stalls
the session's entire CLI pipe path. Grout piping rows into a wedged rail
hangs; worse, the wedge can bite CL's real sessions during status polls. This
is the one change that protects live sessions, which is why it leads sprint 0.

## Proposed approach (seed — ideation refines)

Take the `ReadCliPipes` grant and explicitly `unblock_cli_pipe_input` on every
pipe receipt (reinstating the mechanism the dock rework deleted — auto-unblock
happens only when `pipe()` returns, which is exactly what a wedged instance
never does), plus a `GetPaneRunningCommand` wedge mitigation on the poll path.
TDD offline; the decision logic is pure.
