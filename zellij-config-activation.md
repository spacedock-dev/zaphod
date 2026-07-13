---
id: fqjswvmd93vek5y12zemf1k2
title: Activation preserves valid Zellij KDL
status: backlog
source: staff review of fp merge 2026-07-12; captain direction
sprint: s1-managed-tab-safety
group: hardening
sprint-readiness: ready
started: 2026-07-12T14:33:06Z
completed:
verdict:
score: 0.98
worktree:
issue:
pr:
mod-block:
---

## Problem

The live managed-tab entry rewrites standing Zellij KDL through a 437-line AWK brace counter. A valid unrelated value such as `WriteChars "{"` causes activation to fail before write. It is fail-closed, but an operator with a normal config can be unable to create a managed tab.

## Required outcome

An operator can run the managed-tab entry against valid supported Zellij configuration, including quoted braces and unrelated bindings; activation either produces a validated config atomically or leaves the standing config and layout byte-for-byte unchanged with a clear error.

## Proposed approach

Replace the hand-rolled AWK structural transformer with a small internal native
Rust helper, `zellij-config-activate`, invoked only by
`scripts/zellij-new-tab.sh` from the selected checkout. It is not a public
`zaphod` command. Add `kdl = "=4.7.1"` as a direct dependency (that exact
lossless parser is already resolved in `Cargo.lock`) and give the helper only
`--wasm-url` and `--layout-path` inputs plus config bytes on stdin; it writes a
candidate config to stdout and never opens, replaces, or deletes a standing
file.

The helper parses a `KdlDocument`, requires exactly one `keybinds` block, and
locates only its direct scopes that already contain a Zaphod `MessagePlugin`
route. It changes only the documented Zaphod `MessagePlugin`, `Alt /`, and
`Alt Shift z` nodes, then serializes the same document. Existing node trivia is
retained by the parser; newly inserted/replaced Zaphod nodes use one canonical
form. It rejects malformed KDL, ambiguity, a missing Zaphod route, or a
conflicting non-Zaphod `Alt /` route before emitting a candidate. The shell
entry continues to render both candidate configs, run `zellij setup --check`,
and atomically install config and layout with its existing rollback discipline.

There is no Go activator in main, any retained worktree, or reachable branch.
The maintained Go candidate `github.com/sblinch/kdl-go` parses KDL v1 but its
generator normalizes comments, indentation, and inline bindings, so it cannot
meet AC-2. A lossless Go/tree-sitter route would require vendored C bindings
and source-span rewriting; it is larger and less proven than the already
available Rust parser.

## Acceptance criteria

### Offline

**AC-1 — Valid quoted KDL activates.**
Verified by: a black-box entry-script fixture containing `WriteChars "{"` activates successfully and the resulting config passes `zellij setup --check`.

**AC-2 — Unrelated config survives exactly.**
Verified by: fixture assertions compare unrelated keybind and config blocks before and after activation; only the documented Zaphod bindings and layout reference may differ.

**AC-3 — Invalid input fails without mutation.**
Verified by: malformed fixture input makes activation fail and preserves pre-run config/layout hashes.

### Interactive

No new live interaction is introduced: the existing `Alt Shift z` captain
drill remains the user-facing check after offline proof. It must still create a
fresh tab, and activation failure must still leave the standing config/layout
unchanged; this task does not add a new key, prompt, or provider flow.

## Test plan

The riskiest mechanism was whether a parser edit retains human-authored bytes.
The spike passed: Rust `kdl` 4.7.1 exactly round-tripped
`tests/fixtures/zellij-tmux-smoke-config.kdl` and a comment-bearing fixture
with `WriteChars "{"`; changing one parsed `MessagePlugin` URL changed only
that URL. By contrast, `sblinch/kdl-go` parsed the same fixture but rewrote its
comments and inline binding into normalized multiline KDL.

1. Add helper unit tests first: exact no-op round-trip of the real smoke
   fixture, quoted braces/comments, a URL edit that preserves an unrelated
   sentinel block byte-for-byte, and malformed KDL rejection.
2. Extend `tests/zellij-new-tab-test.sh` with a real helper fixture whose
   unrelated `WriteChars "{"` block and comment-bearing normal scope are
   compared to a reviewed golden output. The fake Zellij log must show one
   `new-tab`; `zellij setup --check` validates the emitted candidate in the
   real-config test.
3. Add malformed and conflicting-route black-box cases that compare
   pre/post config and layout hashes, verify no `new-tab`, and verify temp
   cleanup. Re-run the existing entry suite and the tmux-hosted isolated smoke;
   no custom PTY or standing-root mutation test harness is introduced.

Doc diff proposed: amend the README fresh-managed-tab section to say the same
entry command accepts supported KDL including quoted braces and leaves standing
config/layout unchanged on activation failure. The user-facing command and
keybindings do not change.

## Out of scope

Concurrent-activation locking, multi-client behavior, managed-tab route
authorization, a public Go or `zaphod` CLI, Sprint 2 bb/session work, gate
pooling, 7h, 4d, leases, and custom PTYs.

## Stage Report: ideation

- DONE: Prove whether a Go/KDL parser replacement is available and round-trips fixtures.
  Main, all retained worktrees, and reachable refs contain only `scripts/zellij-config-activate.awk`; the Go `sblinch/kdl-go` spike parsed quoted braces but normalized the document, while Rust `kdl` 4.7.1 losslessly round-tripped the real smoke fixture and a quoted-brace fixture.
- DONE: Specify the smallest atomic activation contract without AWK brace parsing.
  The selected-checkout internal Rust helper is stdin-to-stdout only; the existing shell script remains the sole writer and retains candidate validation, atomic replace, and rollback.
- DONE: Keep the existing entry command and no Sprint 2 dependency.
  `scripts/zellij-new-tab.sh`, `Alt Shift z`, and the Sprint 2 entry gate are unchanged; no controller, lease, custom PTY, 7h, 4d, or public CLI is introduced.

### Summary

The AWK transformer is a real operator-blocking compatibility defect, but it
is not a Sprint 2 dependency: a valid user config can make the current entry
fail closed before any mutation. No existing Go implementation was found. The
smallest proven replacement is the already-resolved lossless Rust KDL parser,
kept behind the existing shell entry and its atomic write boundary.
