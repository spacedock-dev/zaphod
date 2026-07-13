---
title: Managed-tab indicator and plugin provenance hotkey
status: ideation
source: captain direction 2026-07-13
sprint:
group: managed-tab-ux
sprint-readiness: defer
score: 0.7
started: 2026-07-13T12:30:05Z
completed:
verdict:
worktree:
issue:
pr:
mod-block:
id: jrp6se4epjqnnbr6vc29tazm
---

## Problem

Operators cannot tell at a glance whether a Zellij tab is a properly managed
Zaphod tab, and they cannot inspect the provenance of the rail actually loaded
in that tab. A sidebar-shaped or same-WASM lookalike must not be confused with
a tab that satisfies the managed-tab ownership proof.

## Direction

Add one emoji indicator to the title of a tab only after the tab satisfies the
same authoritative managed proof used by V3: the required managed marker plus
the exact canonical loaded WASM identity. The emoji is display state, never an
authorization input. Make title decoration idempotent and define how it behaves
across user rename, tab reorder, plugin reload, proof loss, and tab teardown
without overwriting the operator's underlying title.

Add a runtime-scoped hotkey or equivalent managed-tab action that shows the
loaded plugin's provenance: at minimum the exact `file:` WASM URL/path and a
version/build value stamped into the artifact at build time. Ideation must
choose the smallest legible surface and a non-conflicting key route, and must
fail closed or report "not managed" when no verified managed rail owns the
active tab. Do not infer version from a filename or mutable checkout state.

Reserve the title-indicator model so a later task can add a distinct visual
state when gates are presented in the tab. Gate discovery, delivery, and
notification behavior are not part of this task.

## Acceptance boundaries

- Exactly one visible emoji marks a verified managed tab; ordinary tabs and
  same-shape/same-WASM lookalikes without the full proof never receive it.
- The indicator survives ordinary title changes without accumulating markers
  or destroying the operator's title, and is removed or invalidated when the
  managed proof no longer holds.
- The info action visibly reports the exact loaded WASM path and artifact-
  stamped version/build identifier for the active managed tab.
- The indicator and info route do not weaken V3 authorization, mutate standing
  Zellij config/layout, or treat a title, emoji, geometry, CWD, or URL substring
  as ownership proof.
- Isolated native-state tests cover managed, lookalike, rename/reload, and
  absent-proof cases before any captain-live drill.

## Dependency and scope

Base this work on the integrated V3 managed-tab proof after `kj`
(`managed-tab-safety-session-integration`) lands. This filing does not choose
the emoji or hotkey, implement gate notification, move tabs across sessions,
or repair the deferred complex-layout `Alt /` behavior.
