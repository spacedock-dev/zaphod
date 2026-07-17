---
id: xec4hxprebk8zagjwsg4975s
title: Executable render spec pins rail output and click targets
status: backlog
source: zj-radar comparative research finding (2026-07-16)
sprint:
group:
sprint-readiness:
started:
completed:
verdict:
score:
worktree:
issue:
pr:
mod-block:
---

## Problem

The rail's rendered output and click-target mapping have no machine-checked specification; drift lands as defects (sliver-click-target-mapping and dock-floating-leak-and-chrome-misplacement are open render-contract regressions). zj-radar demonstrates the fix at production quality: docs/rail-reference.md is an executable spec — include_str!'d by crates/plugin/src/reference_tests.rs, each scenario a state block plus the exact expected grid — and a lockstep invariant derives the emitted ANSI and the click-target map from one Vec<Line> so they structurally cannot drift (CONTEXT.md:184-192, in /Users/clkao/git/spacedock-research/spaceterm/zj-radar).

## Proposed approach

Author a rail reference doc where each scenario pairs a state fixture with the exact expected rendered grid and click-target map; a test includes the doc verbatim and asserts the renderer reproduces every scenario. Per this workflow's code-gate-over-prose rule, the doc is the spec only because the test enforces it.

## Acceptance criteria

**AC-1 — Spec is machine-checked.**
A test reads the reference doc and asserts every scenario's expected output. Verified by: the test file path plus a green run.

**AC-2 — Drift fails the build.**
A deliberate one-glyph render mutation fails the spec test. Verified by: mutation run recorded in the stage report.

**AC-3 — Baseline coverage.**
Scenarios cover the shipped baseline: pane rows with agent classification glyphs, click-to-focus targets, sliver mode, and the gate row. Verified by: scenario list in the doc.

## Test plan

The spec test is the deliverable; host-side only, no live drill needed.

## Out of scope

Changing render behavior; fixing the open dock/click defects (scenarios pin current-correct behavior only).
