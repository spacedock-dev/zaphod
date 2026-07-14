---
id: qjfzjxjhdky2phw78fyb6axg
title: Develop against main WASM with an exact close-and-reload loop
status: backlog
source: captain direction 2026-07-14
sprint: s1-managed-tab-safety
group: developer-experience
sprint-readiness: ready
started:
completed:
verdict:
score: 0.96
worktree:
issue:
pr:
mod-block:
---

## Problem

Zaphod development needs a normal Zellij session whose managed rail loads the WASM built from `main`, plus a repeatable upgrade path. Closing `plugin_<id>` is a reliable emergency stop, but current generic plugin launch commands do not prove that they reconstruct the managed tab contract; using them ad hoc risks entering the unsupported retrofit path.

## Required outcome

Provide a supported development command or documented executable workflow that identifies and closes only the current managed Zaphod plugin, waits for its sidecar/polling activity to stop, rebuilds the WASM from `main`, and reloads it into the same managed tab contract with its stamped version and configuration intact. It must refuse ambiguous or unmanaged targets, preserve terminal panes and tab layout, and prove two consecutive upgrade cycles in an isolated Zellij session before a live demo.

## Dependency

Do not dogfood this loop in an important session until the five-second congestion hotfix is merged. Its design must not silently bless the general retrofit/adoption path.
