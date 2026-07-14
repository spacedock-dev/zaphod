---
id: 6s0s2704zrms3med9n04mm4y
title: Automate tab-local agent watcher lifecycle and multi-pane recovery
status: backlog
source: KJ walking-skeleton scope split 2026-07-14
sprint: s1-managed-tab-safety
group: post-walking-skeleton
sprint-readiness: defer
started:
completed:
verdict:
score: 0.72
worktree:
issue:
pr:
mod-block:
---

## Problem

The KJ walking skeleton deliberately requires one manually launched watcher in each managed agent terminal. It does not automatically start watchers, discover agents in later panes, preserve registrations across watcher restart, or protect durable state across same-named Zellij session reincarnation.

## Required outcome

After the manual watcher walking skeleton is proven, design the smallest secure automation that starts and discovers tab-local watchers, supports multiple agent panes, and rehydrates registrations without allowing stale session, pane, rail, or recipient authority. Reuse the walking skeleton's private hook protocol and exact identity checks; add durability or incarnation identity only when restart recovery requires it.

## Out of scope

Changing the manual KJ walking-skeleton delivery contract before it is validated; inference from CWD, titles, timing, prompts, or newest-session order.
