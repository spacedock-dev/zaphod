---
title: Classify Roborev findings by release scope in review output
status: backlog
source: captain request 2026-07-15
sprint: s1-managed-tab-safety
group: review-policy
sprint-readiness: ready
started:
completed:
verdict:
score: 0.95
worktree:
issue:
pr:
mod-block:
id: jb5jzbbwqgmpgw586d070p78
---

## Problem

Zaphod's workflow requires release-scope triage, but `.roborev.toml` does not tell reviewers how classification controls output. Because Roborev treats every entry under `## Review Findings` as a failure, deferred risks or polish can incorrectly block delivery, while the current quick-review instruction to keep Low observations visible does not specify a safe non-blocking location. Adopt material, deferred-risk, polish, and needs-decision classification with complete user/harm/boundary/trigger evidence, preserve Zaphod's stronger acceptance-criterion and non-negotiable boundaries, and align the workflow's blocking language with the review output contract.
