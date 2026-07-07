---
title: Plugin rows section
status: backlog
source: plan sprint 0 (docs/plan-agent-rail.md)
score: 0.8
id: 5tdw8rckcrsx1qfytsytgp9m
---

## Problem

The rail renders pane rows only; it has no rows section for agent sessions or
pending gates, and its pipe handler traces payloads without parsing them. The
walking skeleton needs the display half.

## Proposed approach (seed — ideation refines)

An `agent-event` pipe handler parsing the two typed row kinds; a minimal rows
section rendered in the rail; identity binding in the plugin (match the
session row's cwd against the PaneManifest; unbound renders as unbound, never
guessed); click a session row → focus the bound pane; click a gate row →
float `subspace-tui` on the artifact. Parsing and click-decision logic pure
and TDD-able offline.
