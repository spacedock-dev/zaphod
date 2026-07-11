---
id: 7hm8rw9kzp9m2chdmbe721qr
title: Foreground attached-client disposable Zellij profile
status: ideation
source: managed-view roadmap Sprint 1 entry gate, senior staff review 2026-07-11
started: 2026-07-11T05:09:21Z
completed:
verdict:
score: 1.0
worktree:
issue:
pr:
mod-block:
---

## Problem

The canonical disposable profile backgrounds its attached Zellij client, so it can render but cannot reliably own or read the controlling terminal. Repair the profile before any real-key acceptance drill consumes captain time.

## Sprint role

This is Sprint 1's mandatory entry task and merges before other live Zellij work. Limit changes to the disposable profile and process-level tests. Prove foreground process-group ownership, raw PTY input reaching a terminal canary, normal and signal cleanup, temporary-root removal, and unchanged standing config/layout hashes.

