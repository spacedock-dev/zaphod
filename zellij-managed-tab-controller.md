---
id: fp8pn84km859qges2s2ffp5h
title: Zellij managed-tab controller and guarded keybindings
status: backlog
source: managed-view roadmap Sprint 2 controller delivery; ideation evidence preserved 2026-07-11
started:
completed:
verdict:
score: 0.98
worktree:
issue:
pr:
mod-block:
---

## Problem

Zaphod has no shipped entry point that creates or focuses one managed Zellij tab, and the current `Alt /` prototype still mutates ordinary tabs. Design the thin native controller and portable launcher seam that make `Alt Shift z` idempotent and make `Alt /` a no-op outside the recorded managed tab.

## Seed direction

Ideation must reuse the completed live spike evidence, define controller messages and permission flow, specify artifact preflight and visible failures, extend the disposable Zellij profile, and split offline from interactive acceptance. The CLI `Run` path remains a proving harness, not the production keybinding mechanism.

## Dependency boundary

Ideation may proceed in parallel with the driver contract to expose interface pressure. Implementation must consume the approved managed-view contract and must not add pane adoption, hub, dock, provider, or tmux behavior.

## Proposed approach

Use a separate, thin Zellij WASM controller and a fixed portable-launcher helper. The controller captures trustworthy key invocation context, owns permission and visible-error UX, and calls one installed command. The helper owns workspace resolution, binding locks, artifact checks, and Zellij CLI convergence. Neither component parses providers, renders dock rows, or moves user panes.

The production keybindings use `MessagePlugin`, never Zellij's `Run` action:

```kdl
bind "Alt Shift z" {
    MessagePlugin "file:<CONTROLLER_WASM>" {
        name "ensure-managed-view-v1"
        launch_new true
        floating true
        controller "1"
    }
}
bind "Alt /" {
    MessagePlugin "file:<CONTROLLER_WASM>" {
        name "toggle-managed-layout-v1"
        launch_new true
        floating true
        controller "1"
    }
}
```

`launch_new true` is deliberate. In Zellij 0.44.3, `PipeSource::Keybind` carries no invoking pane, tab, or client. A fresh controller actor can map its plugin ID through `PaneManifest` to the tab in which Zellij launched it, translate that tab position through `TabUpdate` to a stable tab ID, and cross-check `get_focused_pane_info()` for the associated client. It must recover the pre-key focused terminal pane ID and a source cwd that equals that terminal's cwd; the actor's inherited cwd is corroborating evidence, not a substitute for pane identity. A missing, stale, or contradictory pane/tab/cwd witness rejects the request. The actor closes after success or a foreign-tab no-op, so no controller pane persists.

The controller's event loop extends the prototype's pure `own_tab_position` and `active_tab_for_decision` seams. `pipe()` only validates and records one key intent. A later `PaneUpdate`, `TabUpdate`, or zero-delay `Timer` drains it after invocation evidence and permission state exist; blocking host calls never run from `load()` or `pipe()`.

### Controller/launcher protocol

After it has an invocation witness, the controller runs only the configured canonical executable with controller-generated arguments:

```text
zaphod zellij key-request --protocol 1 --request-id ID --action ACTION \
  --session-id SESSION --actor-pane-id ACTOR --invoking-tab-id TAB \
  --invoking-tab-position POSITION --invoking-pane-id TERMINAL \
  --invoking-cwd CWD
```

`run_command` has no standard-input channel, so request fields travel only in this fixed argument shape; key payloads never become commands or arguments. The helper writes one JSON result to standard output, and the controller correlates the `RunCommandResult` by `request_id`:

```text
KeyRequest {
  protocol: 1,
  request_id,
  action: ensure_managed_view | toggle_managed_layout,
  session_id,
  actor_pane_id,
  invoking_tab_id,
  invoking_tab_position,
  invoking_pane_id,
  invoking_cwd
}

KeyResult {
  protocol: 1,
  request_id,
  outcome: created | focused | toggled | noop_foreign | rejected,
  managed_tab_id?,
  binding_generation?,
  message?
}
```

The controller supplies identity, not policy. The helper derives the canonical root from the invoking tab's terminal context, acquires the binding lock, and validates the session, reserved name, stable tab ID, and binding generation. `ensure_managed_view` creates the managed layout and records its returned stable ID when no binding exists, or focuses the recorded ID. `toggle_managed_layout` compares `invoking_tab_id` with the recorded managed ID before issuing any Zellij command; a mismatch returns `noop_foreign`. Both paths use stable-ID CLI actions. The helper serializes repeated requests, so concurrent `Alt Shift z` presses cannot create two tabs.

This command is not the rejected option-2 keybinding harness: Zellij opens no terminal `Run` pane. The native controller launches the helper in the background and receives its result as an event.

### Permissions, preflight, and failures

The controller requests `ReadApplicationState` and `RunCommands` on first render, after its pane is registered. It waits for `PermissionRequestResult`; it never sends a consent keystroke. Denial renders `Zaphod controller permission denied; no changes made` and waits for explicit dismissal.

Installation and every external `zaphod` entry preflight the exact canonical controller WASM, helper executable, managed layout, and keybinding artifact identity before changing a binding or session. A direct keypress proves the controller WASM loaded; the helper repeats layout and binding preflight before mutation. Missing helper, malformed result, stale binding, duplicate reserved name, command failure, or timeout leaves the error in the controller float. A missing controller artifact produces Zellij's own visible error float; installation must refuse to write such a keybinding. Successful and `noop_foreign` results close the actor without a persistent pane.

### Pressure on the shared driver contract

The parallel driver contract must support these Zellij requirements without adopting their transport:

- `InvocationContext { session_id, stable_view_id, native_actor_id }` accompanies key-originated actions.
- `ensure_managed_view` returns `Created { stable_view_id }` or `Focused { stable_view_id }`, not a Boolean.
- `toggle_managed_layout` returns `Toggled`, `NoopForeign`, or `Rejected` and guarantees `NoopForeign` issued no native layout command.
- Binding convergence is serialized and compare-validates the stable ID, reserved name, and generation before mutation.
- Driver preflight reports missing controller, helper, layout, or unsupported stable-ID action separately.

The shared contract need not expose `MessagePlugin`, permissions, plugin pane IDs, or `RunCommandResult`. Those remain Zellij adapter details.

## Riskiest unproven mechanism

The completed option-2 spike proved stable-ID convergence and guards only when a transient CLI `Run` pane supplied `ZELLIJ_PANE_ID`. It did not prove that a `MessagePlugin launch_new true` actor always lands in the keypressing tab, can recover the pre-key terminal pane ID and workspace cwd after its float opens, can derive that tab's stable ID before acting, and closes without persistent focus, pane, geometry, or process changes. This invocation-witness mechanism must be tested first. If it cannot distinguish two attached clients on different tabs, loses source identity when its float takes focus, or Zellij routes the message to an older instance, the design is invalid; do not fall back to active-tab, pane, or cwd guessing.

## Acceptance criteria

### Offline

**AC-1 — Native key invocation has a trustworthy pane, cwd, and stable-tab witness.** In a disposable Zellij 0.44.3 session, fresh `launch_new` controller actors invoked from three tabs with distinct terminal and cwd canaries, including two attached clients focused on different tabs, report the exact pre-key terminal pane ID, keypressing tab's independent stable ID, and source cwd on every attempt and leave no controller pane after exit.
Verified by: before/after `list-tabs --json` and `list-panes --json --all` snapshots over at least five invocations per tab; actor and focused pane IDs mapped through `PaneManifest` and `TabUpdate`; reported cwd compared with the pre-key focused terminal baseline; terminal pane IDs, PIDs, focus, and geometry compared after actor exit. Any cross-client or cwd mismatch invalidates the design.

**AC-2 — Managed entry is idempotent under repetition and concurrency.** The first `Alt Shift z` request creates one managed tab; three sequential requests and two near-simultaneous requests focus the same stable tab ID and create no additional tab or terminal.
Verified by: an independent pre-key tab/pane baseline, helper outcomes, recorded binding ID/generation, returned stable IDs, final tab count, pane IDs, and PIDs. The expected delta is exactly one managed tab on the first request and zero thereafter.

**AC-3 — `Alt /` is disabled outside the managed tab.** Requests from every foreign tab return `noop_foreign` without issuing a native layout action; a request from the managed tab advances exactly one known managed swap state.
Verified by: helper action logs plus before/after stable tab IDs, active swap name, pane IDs, PIDs, focus, and geometry. Every foreign snapshot must equal its settled baseline; the managed request changes only its swap state and expected geometry.

**AC-4 — Permission and artifact failures fail visibly and closed.** Permission denial, missing helper, missing or invalid layout, stale managed ID, duplicate reserved name, malformed helper result, and timed-out helper never create a second managed tab or change a foreign layout.
Verified by: exact controller message/outcome and final tab/pane/PID snapshots for each negative case. No test automates a permission response; the denial case waits for `PermissionRequestResult`.

**AC-5 — The controller remains an adapter.** Its pure state machine accepts only the two key intents, waits for permission and an invocation witness, launches only the configured helper, correlates one result, and closes or renders an error.
Verified by: Rust unit tests extending `own_tab_position` and `active_tab_for_decision` behavior with stale/mismatched TabUpdate fixtures, duplicate presses, permission states, unexpected pipe sources, malformed results, and timeouts; host-call fakes prove `load()` and `pipe()` emit no blocking call.

### Interactive

**AC-6 — Real key UX is acceptable.** In CL's disposable attached profile, real `Alt Shift z` creates then focuses one managed tab, and real `Alt /` toggles there while doing nothing visible to foreign layouts.
Verified by: CL presses both configured keys from managed and foreign tabs, repeats entry three times, and confirms navigation, focus restoration, controller-float flicker, and latency are acceptable. CLI action simulation cannot settle this AC.

**AC-7 — First-run consent and errors are understandable.** The first real keypress presents one controller permission prompt; denial and one missing-helper drill explain that nothing changed and allow dismissal without trapping focus.
Verified by: CL observes and resolves the permission prompt manually, then reviews the denial and missing-helper surfaces in the disposable profile.

## Test plan

1. Build the smallest instrumented controller actor and test AC-1 before implementing the helper. Launch it with `MessagePlugin launch_new true` from three distinct-cwd tabs and two clients; record its plugin ID, manifest tab position, stable TabUpdate ID, inherited or focused source cwd, and `get_focused_pane_info()` result. Stop if any request cannot prove its tab and cwd origin or leaves persistent pane/focus/geometry changes.
2. Define pure request, result, binding-decision, and controller-state types. Add failing tests for foreign no-op, stale evidence, permission denial, duplicate requests, malformed results, and timeout before implementing transitions.
3. Add the fixed helper command and disposable binding root. Prove one first create, three repeated focuses, and two near-simultaneous requests against tab and pane inventories.
4. Add guarded stable-ID toggle. Compare managed and every foreign tab before and after; record native actions so a foreign no-op proves no command ran.
5. Run each failure drill with isolated controller URL, layout, binding, and permission roots. Never alter the standing config, layout, data directory, or WORK session.
6. Extend `scripts/zellij-worktree-test-profile.sh` with controller artifact/config inputs and printed `list-tabs`/`list-panes` inspection commands. Keep real single-line Zellij shapes whenever a dumped fixture is unavoidable; this design should rely on JSON inventories instead of parsing dumps.
7. Run Rust `cargo test` and `cargo check --tests`, the canonical shell profile suite, and the interactive AC-6/AC-7 script in a disposable attached session.

## Documentation diff proposed at this gate

- In `docs/zaphod-workspace-architecture.md`, replace the abstract controller paragraph with the two key intents, fresh-actor invocation witness, fixed helper boundary, exact permissions, foreign `NoopForeign`, and structured failure outcomes.
- In `README.md`, keep the shipped prototype instructions clearly historical and add a managed-controller usage section only after the real key drill passes.
- In the generated/disposable config documentation, show `launch_new true` and canonical controller identity; do not install or recommend the binding until AC-1 and CL's interactive gate pass.

## Out of scope

- pane adoption or `break_panes_to_tab_with_id`;
- workspace hub, dock TUI, providers, `grout`, or item protocols;
- tmux behavior beyond the shared-contract pressure listed above;
- foreign-tab layout retrofit, retained-layout transactions, or a Zellij fork;
- changing the current sidebar renderer or row behavior;
- standing global config/layout edits during implementation; and
- automatic permission or consent responses.

## Stage Report: ideation

- DONE: Turn the completed option-2 spike evidence into a minimal controller protocol for idempotent Alt Shift z and managed-tab-only Alt /, including permissions, artifact preflight, and visible failure behavior.
  The design uses fresh native `MessagePlugin` actors plus a fixed `zaphod zellij key-request` helper, structured outcomes, `ReadApplicationState`/`RunCommands`, canonical artifact checks, manual consent, and closed visible failures.
- DONE: Specify offline and interactive acceptance evidence using the disposable Zellij profile, stable IDs, pane inventories, geometry, and repeated entry without relying on the transient CLI Run path in production.
  AC-1 through AC-5 use independent tab/pane/PID/cwd/geometry baselines and stable IDs; AC-6 and AC-7 reserve real-key flicker, latency, focus, consent, and error UX for CL's attached-profile demo.
- DONE: Identify concrete pressure on the parallel driver contract without absorbing pane adoption, hub, dock, provider, tmux, or foreign-tab retrofit scope.
  The contract needs invocation context, structured create/focus/toggle/no-op results, serialized stable-ID convergence, and typed preflight errors; Zellij messages, permissions, actors, and result events remain adapter-local.
- DONE: Name the riskiest unproven mechanism and put its invalidating check first.
  A fresh controller must recover the exact pre-key terminal pane ID, cwd, and stable tab ID across multiple tabs and clients without persistent mutation; failure forbids active-tab, pane, or cwd guessing.
- DONE: Propose the user-visible documentation diff.
  The gate records precise evergreen-architecture, README, and disposable-config updates, all conditional on AC-1 and CL's interactive acceptance.

### Summary

The design keeps the controller thin: it authenticates native key context, manages permission/error UX, and invokes one fixed portable helper. The first implementation check can invalidate the approach before helper work if Zellij cannot preserve an exact invocation witness without the transient CLI `Run` pane.
