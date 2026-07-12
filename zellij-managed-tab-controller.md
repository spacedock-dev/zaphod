---
id: fp8pn84km859qges2s2ffp5h
title: Safe managed-tab entry and guarded keybindings
status: validation
source: outcome-first roadmap Sprint 1 safe managed-tab onramp; captain correction 2026-07-12
started: 2026-07-12T06:45:43Z
completed:
verdict:
score: 0.98
worktree: .worktrees/zellij-new-tab-entry
issue:
pr:
mod-block:
sprint: s1-safe-managed-tab-onramp
group: walking-skeleton
sprint-readiness: ready
---

## Problem

Before Sprint 2 can surface real sessions and gates, an operator needs one
safe, direct way into a Zaphod-owned tab. The value is deliberately small:
from an ordinary Zellij tab, create a fresh tab made from the selected
checkout's WASM; then use `Alt /` to open or collapse that tab's rail without
ever changing a foreign tab.

The prior controller/portable-CLI proposal is rejected. It put a controller,
stable binding record, helper protocol, invocation witness, and `Run`-adjacent
machinery ahead of the first usable action. It must not be repaired or used as
an implementation dependency.

The existing implementation evidence lives in
`.worktrees/zellij-new-tab-entry` on `feature/zellij-new-tab-entry` at
`c34436f`. It has the right narrow seam—an entry script, native `NewTab`, an
absolute rendered layout, persistent `Alt /` `NoOp`, and a tmux-hosted
fixture—but is not accepted as-is. In particular, it treats a fire-and-forget
`reconfigure()` call and a local `toggle_route_installed` flag as authorization
for a route that has not actually been observed. It has not proved a literal,
authorized positive `Alt /` path in the managed tab.

## Required end value

**Trigger:** The operator runs `scripts/zellij-new-tab.sh --session <name>` or
presses native `Alt Shift z` after activating the selected checkout's Zellij
configuration.

**Visible result:** A fresh Zaphod tab appears with that checkout's candidate
WASM. In that initialized tab, `Alt /` changes the rail's known docked/sliver
state. In every foreign tab it does nothing: it creates no pane, does not load
the WASM, and does not change layout, focus, geometry, process, or tab state.

**Shared-tab rule:** The rail and swap state belong to the managed tab, not to
one attached terminal. Two clients viewing the same initialized managed tab
are both allowed to toggle that shared rail. A client-local runtime binding may
be required by Zellij as delivery plumbing, but it must never become a
client-private authorization scheme or require a client token. Delivering and
proving that behavior across attached clients is a named follow-up, not this
Sprint 1 gate.

**Reproducible proof:** A real Zellij client inside an isolated tmux server
sends literal `Alt Shift z` and `Alt /` bytes, captures the visible screen, and
uses native Zellij pane/layout state to prove the positive managed-tab path,
post-route foreign-tab no-op, and disposable cleanup. It proves standing
Zellij config/layout hashes did not change.

## Proposed approach

Continue in the existing worktree; do not create a controller worktree or
rewrite the entry path:

```text
.worktrees/zellij-new-tab-entry
feature/zellij-new-tab-entry @ c34436f
```

1. **Fresh native entry.** `scripts/zellij-new-tab.sh` derives its repository
   from its own physical path, builds that checkout's WASM, renders
   `layouts/zaphod.kdl` with its canonical file URL, and atomically installs
   the selected config root's layout and bindings. The explicit command creates
   one fresh managed tab per invocation. It is intentionally not a
   create-or-focus controller.

2. **Persistent fail-closed bindings.** In the Zaphod-owned keybinding scopes,
   activation leaves `Alt /` as `NoOp` and binds `Alt Shift z` to native
   `NewTab` with the selected absolute rendered layout path. It refuses an
   unrelated `Alt /` binding rather than rewriting it. It never uses `Run`, a
   `MessagePlugin` keybind that can load a missing pane, a controller WASM, or
   a `zaphod zellij key-request` helper.

3. **Tab-scoped runtime toggle.** Only a visible, tiled resident rail in an
   initialized Zaphod tab may arrange a runtime `MessagePluginId` route to its
   existing plugin instance, after Zellij's ordinary `Reconfigure` permission.
   The route is runtime-only and must not write the persistent config. A pipe
   can act only when that resident's tab is the active managed tab; a floating,
   absent, stale, or foreign resident ignores it. Receipt of a real direct pipe
   plus the native state transition—not the successful return of
   `reconfigure()` or a local boolean—establishes that a route was usable.

4. **Real, narrow harness.** Extend the tmux-hosted smoke in the same
   worktree. It owns a short-lived tmux server and temporary Zellij
   config/data/socket roots; it sends keys with `tmux send-keys`, captures
   panes with `tmux capture-pane`, and queries Zellij with native actions.
   It does not build a custom PTY controller, lease, raw-input canary,
   process-group monitor, or readiness choreography.

### Rejected gaps to finish in the existing worktree

- Replace the optimistic fire-and-forget route authorization. A local
  `toggle_route_installed` write immediately after `reconfigure()` cannot make
  a later unobserved route safe or prove that it reached the intended client.
- Add a literal, permission-authorized managed-tab `Alt /` proof. The current
  smoke proves entry and foreign inertness but not the positive key path.
- Retain the current-checkout identity proof through script, native binding,
  live pane inventory, and dumped layout. A stale global `zaphod.kdl` or WASM
  URL is a failure.

## Riskiest unproven mechanism

**A runtime `MessagePluginId` route can support a literal authorized `Alt /`
from the initialized managed tab while a literal `Alt /` from a foreign tab
remains inert.** Unit tests can prove decision guards but not Zellij's actual
runtime delivery. This needs a narrow tmux-hosted spike before further polish.

If Zellij cannot provide that behavior with the existing resident layout and
runtime route, stop at the observed limitation and return it for a captain
decision. Do not revive the rejected controller/CLI/lease architecture merely
to force the test through.

## Acceptance criteria

### Offline

**AC-O1 — Explicit native entry produces a fresh tab from the invoking checkout.** From an isolated root whose initial config contains a stale
Zaphod URL, the entry script is run from the selected worktree, Zellij is
restarted to load the generated native binding, and literal `Alt Shift z`
creates exactly one new managed tab. The live tab contains the canonical WASM
URL from that worktree and no stale URL.

Verified by: compare native pre/post `list-panes --json --all` tab inventory
(one additional tab); inspect the candidate plugin URL in both that inventory
and `action dump-layout`; and compare it to the canonical URL derived from the
selected worktree's built artifact. `tmux capture-pane` must show the
candidate rail or its normal permission prompt. The expected candidate URL and
tab-count delta come from the selected checkout and Zellij state, not from a
generated config string.

**AC-O2 — A literal authorized `Alt /` changes only the initialized managed tab's known rail state.** With the normal `Reconfigure` permission granted in
the isolated test profile, a literal `Alt /` sent while the managed tab is
active changes its native swap state between the layout's `docked` and
`undocked` forms. It does not add a pane, replace a process, or change the
candidate rail URL.

Verified by: take native pane, process/PID, geometry, focus, and dumped-layout
snapshots immediately before and after the literal key; require the reported
swap state/managed layout shape to change as expected while the rail plugin
identity and all existing pane identities remain equal. Capture the visible
tmux pane for the same transition. A helper call or direct plugin method does
not satisfy this criterion.

**AC-O3 — Foreign-tab `Alt /` is a real no-op even after runtime routing.**
After AC-O2 has established a usable managed route, switch the attached client
to a sidebar-less foreign tab and send literal `Alt /`. The foreign tab does
not create or focus a candidate pane and its pane inventory, PID, geometry,
focus, active-tab selection, and dumped layout are unchanged.

Verified by: normalized native `list-panes --json --all --command --geometry
--state --tab` and `dump-layout` snapshots before/after the key must be byte
equal for the foreign tab; the tmux capture must show no Zaphod launch or
layout transition; and candidate plugin count must remain unchanged. This is
run after—not before—the positive route so it catches a leaked runtime route.

**AC-O4 — The smoke is disposable and preserves standing state.** Every smoke
outcome, including permission refusal, a failed candidate build, and
interruption, removes its Zellij session, dedicated tmux server, and temporary
root. It leaves the operator's standing config and `layouts/zaphod.kdl` hashes
unchanged.

Verified by: the test records pre/post SHA-256 states for both standing files,
checks the tmux server and isolated Zellij session are gone in cleanup, and
asserts the temporary root does not exist. Failure paths use an independent
sentinel standing root to prove cleanup rather than only a happy-path log.

### Interactive

**AC-I1 — The operator can use the first Sprint 1 journey without tab hunting.** In an attached disposable session, CL invokes the selected
checkout's entry script or presses `Alt Shift z`, approves the ordinary
permission prompt, sees the selected candidate rail, toggles it with `Alt /`,
and confirms `Alt /` is inert from a foreign tab.

Verified by: CL's live drill follows the printed tmux-harness commands, with
native pane/layout snapshots retained beside the captured screen. The drill
records the candidate checkout, managed swap transition, and unchanged foreign
baseline. It is not settled by unit tests or configuration inspection.

## Test plan

1. **Run the smallest real delivery spike first.** In the existing worktree,
   start Zellij in an isolated tmux server, activate the selected layout,
   complete the normal permission path, and drive literal `Alt Shift z` and
   `Alt /`. Prove AC-O2 and AC-O3 before refactoring any remaining branch
   code. Do not add a controller, CLI protocol, lease, or custom PTY
   workaround.
2. Add failing tests for the narrow code decisions: a persistent `NoOp` route;
   a native absolute-path `NewTab` route; no optimistic local authorization;
   direct-pipe handling only by an active tiled resident; and foreign/floating/
   absent state being inert. Extend the existing pure helpers around
   `runtime_toggle_keybind_kdl`, `should_route_toggle_to_self`, and
   `decide_toggle`; do not add a new driver layer.
3. Complete `scripts/zellij-new-tab.sh` and its config transformer with
   current-checkout artifact identity, atomic write/rollback, conflicting
   non-Zaphod binding refusal, and isolated-root fixture coverage.
4. Make the tmux smoke automate AC-O1, AC-O2, AC-O3, and cleanup. It may use a
   deliberately pre-authorized permission fixture for headless coverage, but
   it must never fake consent with injected permission keystrokes. Retain the
   normal consent flow for AC-I1.
5. Run the focused shell suites, Rust tests, `cargo check --tests`, the
   tmux-hosted smoke, and `git diff --check`. Review the actual Zellij/tmux
   state before presenting the captain-live drill.

## Documentation diff proposed at this gate

- `docs/roadmap.md` names this as Sprint 1's first walking skeleton: safe
  door, visible candidate tab, tab-scoped toggle, and tmux proof. It does not
  put it behind `7h`.
- `README.md` documents the stable entry command, the native `Alt Shift z`
  binding, `Alt /`'s managed-tab-only meaning, and the selected-worktree
  identity expectation. It removes instructions that recommend first-toggle
  retrofit of an ordinary tab.
- `docs/zellij-tmux-smoke-harness.md` records literal key delivery, positive
  managed toggle, post-route foreign no-op, and disposable cleanup. It
  explicitly excludes custom PTY/lease machinery.
- `docs/zaphod-workspace-architecture.md` replaces the stale controller/CLI
  account with this Sprint 1 onramp. The broad future launcher/driver remains
  architectural direction, not a prerequisite to the first operator journey.

## Out of scope

- `7h`, `4d`, ProfileLeaseV1, custom PTY control, leases, raw-input canaries,
  process-group choreography, or another foreground-profile test system;
- a new controller WASM, `Run` pane, portable `key-request` CLI, persistent
  managed-tab record, client tokens, or active-client identity guessing;
- create-or-focus/adoption semantics, pane moves, workspace hub, tmux product
  driver, provider ingestion, gate pooling, or review resolution;
- retrofitting a foreign tab, spawning a rail from `Alt /`, or changing a
  foreign layout; and
- **Follow-up — same-tab multi-client runtime-route delivery:** prove that a
  second client can use the same initialized managed rail without changing the
  tab-scoped semantics. Do not file or dispatch it until this Sprint 1 journey
  exposes a need; it is not an AC, smoke requirement, or captain drill here;
- rewriting the existing entry worktree into a new branch or worktree.

## Stage Report: ideation

- DONE: Reframed the entity around the approved Sprint 1 walking skeleton:
  explicit fresh entry, visible candidate tab, tab-scoped toggle, and real
  tmux proof. The old controller/CLI design is explicitly rejected.
- DONE: Reconciled the existing implementation evidence without treating it as
  authority. Implementation continues in
  `.worktrees/zellij-new-tab-entry` on `feature/zellij-new-tab-entry` at
  `c34436f`; the exact gaps are optimistic route authorization, literal
  positive toggle, and identity proof.
- DONE: Made foreign-tab immutability and current-checkout WASM identity
  end-value criteria rather than helper mechanics. The required tmux proof
  uses real key bytes, visible capture, native state, and disposable cleanup.
- DONE: Kept `7h` and `4d` untouched and excluded the rejected lease/custom
  PTY/controller path. No implementation code or separate implementation plan
  was created in this stage.
- DONE: Mapped every acceptance criterion to its independent planned evidence;
  these are design citations, not claims that the live drill has run.
  AC-O1 → *Fresh native entry*, `layouts/zaphod.kdl`, and test-plan steps 1,
  3, and 4: native tab inventory plus the rendered candidate URL supply the
  externally observable tab-count and artifact-identity oracle.
  AC-O2 → *Tab-scoped runtime toggle*, `layouts/zaphod.kdl`'s named swaps, and
  test-plan steps 1, 2, and 4: literal tmux key bytes and native layout/pane
  snapshots settle the positive transition rather than a helper call.
  AC-O3 → *Persistent fail-closed bindings*, `docs/zellij-tmux-smoke-harness.md`,
  and test-plan steps 1 and 4: a post-route foreign snapshot is compared with
  its independent baseline, so a leaked runtime route cannot pass by merely
  leaving the initial foreign test inert.
  AC-O4 → *Real, narrow harness* and test-plan step 4: pre/post standing-file
  hashes plus tmux/session/root teardown are the independent cleanup oracle.
  AC-I1 → *Required end value* and test-plan steps 4–5: CL's normal-consent
  single-client disposable drill remains the explicit live acceptance, not a
  substitute for offline evidence.
- DONE: Re-ran the ideation AC scan after adding those mappings. The task body
  and report are its sole design record; no separate implementation plan was
  written.
- DONE: Captain-directed scope correction: preserve tab-scoped semantics, but
  remove executable same-tab two-client delivery from Sprint 1's acceptance
  gate. It is now the named, unfiled follow-up in `Out of scope`; no code,
  frontmatter, 7h, or 4d state changed.
- DONE: Scanner-heading repair: `spacedock status --read
  zellij-managed-tab-controller --stage ideation --ac-scan --json` now reports
  AC-O1, AC-O2, AC-O3, AC-O4, and AC-I1, each with `unevidenced: false`.

### Summary

`fp` is now the Sprint 1 safe-door task, not a controller prerequisite. It
owns only the path an operator can actually use: fresh native tab entry,
managed-tab-only `Alt /`, and proof that foreign tabs remain unchanged.

## Stage Report: implementation

- DONE: Finish the existing worktree's one-client managed-tab entry → visible tab → managed Alt-/ behavior → foreign-tab no-op slice.
  `88fac36` makes receipt of an active tiled `PipeSource::Keybind`, not the unacknowledged `reconfigure()` request flag, the literal-toggle authorization; `d5137e6` proves the real path.
- DONE: Drive AC-O1 through AC-O4 with the isolated tmux-hosted Zellij smoke; do not revive 7h/4d, custom PTY, lease, or controller scope.
  `./tests/zellij-tmux-smoke-test.sh` passed four times (three consecutive plus final): literal `Alt Shift z` adds one candidate tab; literal `Alt /` changes its rail 28→1 columns without identity change; post-route foreign state is byte-identical; cleanup verifies session, tmux, root, and standing hashes.
- DONE: AC-O1 — Explicit native entry produces a fresh tab from the invoking checkout.
  The smoke's literal `Alt Shift z` tab inventory changes by exactly one and its live pane inventory plus `dump-layout` contain the selected worktree's canonical candidate WASM URL, never the fixture's stale URL.
- DONE: AC-O2 — A literal authorized `Alt /` changes only the initialized managed tab's known rail state.
  With the disposable raw-WASM-path `Reconfigure` pre-grant, one literal key moves the candidate rail 28→1 columns; normalized native identity, command, focus, and candidate URL snapshots remain equal while screen and dumped managed layout change.
- DONE: AC-O3 — Foreign-tab `Alt /` is a real no-op even after runtime routing.
  After AC-O2's observed route, the smoke returns the same tmux client to its sidebar-less tab and proves byte-identical native pane and layout snapshots plus unchanged candidate count around a literal `Alt /`.
- DONE: AC-O4 — The smoke is disposable and preserves standing state.
  The cleanup trap verifies the isolated Zellij session, dedicated tmux server, and temporary root are gone and compares pre/post standing `config.kdl` and `layouts/zaphod.kdl` hashes on success, failure, and interruption paths.
- DONE: Commit the smallest fix set and record exact red/green and smoke evidence in the fp implementation stage report; surface any non-core gap as a follow-up.
  Code commits: `88fac36 fix: authorize toggles from observed keybind pipes`; `d5137e6 test: prove managed tab key path`. Red: missing receipt helper failed with `E0425`; the first pre-grant fixture prompted until its key was corrected from the `file:` URL to Zellij's raw WASM path. Green: shell entry suite 8/8; `cargo test --release` 134/134; `cargo check --tests --release`; smoke 4/4; `git diff --check`.
- SKIPPED: Debug-profile `cargo test` / `cargo check --tests`.
  The initial debug `cargo test` stopped at ENOSPC with 171 MiB free; only its generated worktree `target/debug` was removed, then the complete release-profile test and check commands above passed.
- SKIPPED: AC-I1 normal-consent captain live drill.
  Validation must run the attached disposable journey with the ordinary permission prompt; the headless pre-grant fixture is deliberately not a substitute.
- SKIPPED: Same-tab second-client runtime-route delivery.
  This remains the entity's named, unfiled follow-up; it is not part of the approved Sprint 1 walking skeleton.

### Summary

The entry worktree now proves the first usable operator journey end to end
without controller, lease, or custom PTY machinery. The only remaining
acceptance work is the captain's normal-consent live drill; 7h and 4d were not
read, changed, or used as dependencies.
