---
title: Remove redundant terminal pane title chrome while Zaphod is docked
status: backlog
source: captain direction 2026-07-13
sprint:
group: future-polish
sprint-readiness: defer
score: 0.45
started:
completed:
verdict:
worktree:
issue:
pr:
mod-block:
id: 68c7t9dhbczs4g69nvnrkhyt
---

When the Zaphod rail is visibly docked, its focused row may already provide the active terminal identity, making duplicated terminal pane-title chrome unnecessary. Ideation must first prove whether Zellij can hide that chrome per managed tab without changing global `pane_frames`, degrading the undocked sliver, or removing focus identity from ordinary tabs. No layout or runtime behavior changes are authorized by this backlog filing.
