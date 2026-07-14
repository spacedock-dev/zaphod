---
id: 441wyy3208egsxq5az3yfzy1
title: Five-second scrollback lookup must not congest Zellij
status: ideation
source: live nautical-cuckoo diagnosis 2026-07-14
sprint: s1-managed-tab-safety
group: release-blocking-hotfix
sprint-readiness: ready
started: 2026-07-14T05:54:11Z
completed:
verdict:
score: 1.0
worktree:
issue:
pr:
mod-block:
---

## Problem

With the managed sidebar loaded, its two-second status timer synchronously calls Zellij's pane-scrollback host API. Zellij 0.44.3 can hold that call for five seconds, multiplying queue congestion: `Alt p` fails to create a pane promptly and `Alt n` can leave a half-created empty tab whose queued actions arrive later. The immediate outcome is that the main sidebar may remain loaded while ordinary pane and tab actions stay responsive; degraded metadata is preferable to blocking Zellij.

## Required outcome

Remove or circuit-break the five-second scrollback lookup from the periodic plugin path without redesigning the whole metadata architecture. Reproduce the slow/non-shell-pane case and prove pane creation, tab creation, and tab switching complete within a tight independent deadline with no delayed burst after the sidebar is closed.

## Follow-up boundary

All other synchronous pane metadata calls and the durable cache/sidecar design belong to `nonblocking-pane-metadata-architecture`.
