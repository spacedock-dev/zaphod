---
id: 1s5n4b0vq2mqcvv8j8gzzfn9
title: Explicit Zellij pane adoption into the managed tab
status: backlog
source: managed-view roadmap Sprint 3 pane adoption; ideation evidence preserved 2026-07-11
started:
completed:
verdict:
score: 0.95
worktree:
issue:
pr:
mod-block:
---

## Problem

Users need an explicit way to bring an existing terminal into the managed tab without restarting its process or retrofitting its foreign tab. Design pane adoption around the proved `break_panes_to_tab_with_id` seam while preserving pane identity, process identity, unrelated panes, and user-controlled permission consent.

## Seed direction

Ideation must define the explicit command/controller request, source and target validation, permission-result state machine, empty-source-tab behavior, visible failure cases, and reusable driver acceptance tests. It must use the existing spike evidence and identify only the remaining production integration drill.

## Dependency boundary

This task follows the stable managed-view identity and controller message path. It must not automate permission responses, auto-adopt panes, revive foreign-tab retrofit, or add hub, dock, provider, or tmux behavior.

The binding generation, managed-view marker, error kinds, and mutation states below consume the proposal in `managed-view-driver-contract.md`. That proposal remains pending approval; implementation must use its approved successor rather than copying these types independently.

## Proposed approach

### Explicit request

Expose one deliberate command:

```text
zaphod adopt-pane [PANE_ID]
```

Inside Zellij, an omitted ID means the terminal pane named by `ZELLIJ_PANE_ID`. Outside a Zellij pane, `PANE_ID` is required. The command resolves exactly one approved workspace/session binding, rejects every plugin pane and every pane from another session, and treats a pane already in the managed tab as `AlreadyManaged`. There is no implicit “current active pane” lookup through a transient CLI client.

The portable side creates one request:

```text
AdoptPaneRequest {
  request_id: UUID,
  binding_id: UUID,                 # proposed binding generation
  session_incarnation,
  expected_target_tab_id: u64,
  source_pane_id: Terminal(u32),
  expected_source_tab_id: u64,
  explicit_intent: true,
  expires_at_monotonic,
}
```

It snapshots the source pane ID, source tab ID, process PID, and the source and target pane inventories. It then sends the request through the controller's approved existing-instance message path. The transport must correlate one response to `request_id`, must not broadcast adoption to sibling plugins, and must not launch a controller into the invoking foreign tab when the expected controller is absent.

### Controller state machine

The controller serializes adoption. A second request receives `Busy/Unchanged`; it never shares a permission result with the pending request.

```text
Idle
  -> Preflight(request, source snapshot, target snapshot)
  -> AwaitPermission(request)
  -> PermissionDenied -> Respond(PermissionDenied/Unchanged) -> Idle
  -> PermissionGranted
       -> Revalidate(binding, marker, target ID, source pane, expiry)
       -> Move(source pane -> exact target ID)
       -> Reconcile(native result, fresh inventories)
       -> Respond(Adopted | unchanged error | Indeterminate) -> Idle
```

Preflight requires the proposed session incarnation, binding generation, controller marker, and target tab ID to agree. It also requires one live terminal source with the expected source tab and a distinct target. Only then does the controller request `ChangeApplicationState` from Zellij.

`PermissionRequestResult` has no request ID, so the controller permits one pending permission request. A cached grant still enters `PermissionGranted`; it does not skip revalidation. A denial ends the request without a native move. A grant after `expires_at_monotonic` ends as `StaleInvocation/Unchanged`.

Zellij grants `ChangeApplicationState` to the plugin, not to each adoption. The permission may already be cached because the same controller focuses tabs or steers layouts. In that case, the explicit `zaphod adopt-pane` command is the per-action consent and the cached grant is only the host capability check. The UI must not promise a new Zellij prompt for every move.

Immediately before `break_panes_to_tab_with_id`, revalidation repeats every identity check. A replaced binding generation, reused target ID, wrong marker, duplicate managed view, closed source pane, changed source tab, or expired request fails before mutation. The controller calls:

```text
break_panes_to_tab_with_id([Terminal(source_pane_id)], target_tab_id, false)
```

The move itself does not change client focus. After reconciliation proves success, the portable layer focuses the managed view as a separate, non-destructive operation. This keeps identity proof independent from focus behavior and suppresses focus on denial or indeterminate state.

### Reconciliation and outcomes

Treat `Some(target_tab_id)` as a native acknowledgement, not sufficient proof. Re-read both inventories and require:

- the source pane ID appears exactly once in the expected managed tab;
- its process PID equals the preflight PID;
- every pre-existing target terminal/plugin ID and PID remains present;
- every unrelated source ID and PID remains present;
- no replacement terminal appears; and
- the only missing tab is an emptied source tab.

Geometry may reflow in the source and target. The managed swap layout may become dirty. Neither permits identity loss or an extra process.

If the native call returns `None`, times out, or disconnects, reconcile before replying. Return `Adopted` when all success postconditions hold, the appropriate unchanged error when the source remains in its original tab and inventories match, or `NativeFailure/Indeterminate` for any other state. Never retry the move blindly.

An emptied source tab closing is normal Zellij behavior. Report `source_tab_closed: true`; do not recreate the foreign tab. If unrelated panes remain, the source tab and their identities must remain.

### Visible failures

Map failures to the proposed portable kinds: `SessionReplaced`, `ViewIdentityConflict`, `DuplicateManagedView`, `PaneMissing`, `StaleInvocation`, `ControllerUnavailable`, `PermissionDenied`, `Busy`, and `NativeFailure`. Print the request ID, source pane ID, and corrective action on stderr. Identity and permission failures report `Unchanged`. Missing-controller preflight must complete before a Zellij plugin launch; raw Zellij launch otherwise leaves an error float.

### Ownership boundary

The portable command owns binding resolution, explicit intent, request IDs, the preflight baseline, response rendering, and post-success focus. The existing Zellij controller owns permission state, last-moment identity validation, the one native move, and fresh observations. Neither side owns hub items, dock rows, providers, review policy, or permission consent.

## Proved spike facts

- Zellij 0.44.3 moved terminal pane `0` from foreign tab `0` to managed tab `2` through `break_panes_to_tab_with_id`; pane `0` retained live PID `18723`.
- Unrelated source pane `14` retained its pane ID and PID `45164`, and it expanded back to `80x24` after the move.
- A synchronized permission denial on a distinct controller URL left pane `15`, PID `50156`, and foreign tab `3` unchanged; the temporary controller closed itself.
- A nonexistent source pane produced no move. Moving the sole remaining source terminal caused Zellij to close the empty source tab.
- A raw missing-controller launch returned a plugin ID and left an error float. A denial keystroke sent before the permission prompt appeared did not constitute consent control. Production must preflight the controller and wait for the actual permission event.

## Remaining production integration drill

The unproved mechanism is the existing-controller request/response path across the permission gap. The smallest design-invalidating drill starts from a foreign terminal, sends one correlated request to the already marked controller without launching a plugin, changes or closes the target managed tab while the permission prompt is pending, then grants permission. The required result is `ViewIdentityConflict` or `StaleInvocation`, zero pane movement, zero new plugin pane, and unchanged foreign pane IDs/PIDs. If Zellij 0.44.3 cannot deliver and correlate that request without an auto-launch race, stop and redesign the controller transport before implementing adoption.

## Acceptance criteria

### Offline

**AC-1 — Adoption requires explicit, current intent.** An omitted pane ID resolves only from `ZELLIJ_PANE_ID`; a supplied ID must be a live terminal in the bound session. Plugin, cross-session, expired, and already-managed inputs produce the specified outcomes without a move call.
Verified by: table-driven command tests with externally supplied environments and pane inventories.

**AC-2 — Consent gates the move.** Denial, late grant, and a second concurrent request issue zero `break_panes_to_tab_with_id` calls. One current grant reaches revalidation exactly once.
Verified by: a fake controller event queue that supplies permission events independently and records native call counts.

**AC-3 — Binding and target are revalidated after consent.** Changed generation, reused target ID, wrong marker, duplicate marker, closed source, and changed source tab fail unchanged after permission grant.
Verified by: mutating the fake inventory between preflight and grant and asserting zero move calls.

**AC-4 — Adoption preserves identity.** Success places the same pane ID and PID exactly once in the target, preserves every unrelated source and target ID/PID, and creates no terminal process.
Verified by: independent before/after `list-panes --json --all` snapshots and process liveness, using the spike's pane `0`/PID `18723` result as the baseline.

**AC-5 — Empty-source behavior is explicit.** A source tab with another pane remains with that pane intact; a source tab with only the adopted pane may disappear and returns `source_tab_closed: true` without reconstruction.
Verified by: two disposable Zellij fixtures whose expected tab counts are captured before the controller runs.

**AC-6 — Ambiguous native results reconcile safely.** Native `None`, timeout, and disconnect outcomes return `Adopted`, an unchanged error, or `Indeterminate` solely from fresh inventories and never trigger a second move.
Verified by: fake native outcomes combined with externally supplied post-operation inventories and exact move-call counts.

**AC-7 — Missing controller and transport failures do not mutate foreign views.** The approved message path targets the existing marked controller, correlates one response, and fails visibly when it is absent.
Verified by: the production integration drill plus before/after plugin and terminal inventories.

### Interactive

**AC-8 — Captain adoption drill.** From a foreign tab containing two long-running terminals, CL denies one request and observes no change, then grants a fresh request and lands in the managed tab with the selected pane still running under the same PID. The unrelated source terminal remains and no replacement terminal appears.
Verified by: CL's live observation plus captured native pane/PID snapshots.

## Test plan

1. First run the production transport invalidation drill: existing controller only, target changed during consent, no auto-launch, no move.
2. Drive the state machine offline through denial, cached grant, late grant, concurrent request, stale generation, reused target ID, duplicate marker, closed source, native `None`, timeout, and disconnect.
3. Exercise a disposable Zellij session with two source panes and the managed target; compare native pane IDs, PIDs, tabs, and process liveness before and after one move.
4. Repeat with a one-pane source tab and record its allowed disappearance.
5. Run the captain drill only after all offline and disposable-session evidence passes.

## Evergreen documentation change

Add `zaphod adopt-pane [PANE_ID]` to `docs/zaphod-workspace-architecture.md`. State that omission uses only `ZELLIJ_PANE_ID`, permission remains Zellij-controlled, success preserves pane/PID and then focuses the managed view, and an emptied source tab may close. Extend the controller section with post-permission revalidation and mutation-aware reconciliation. Update delivery step 3 in `docs/plan-agent-rail.md` to require the production transport invalidation drill before pane-move implementation.

## Out of scope

Automatic pane adoption; automated consent or denial; foreign-tab layout retrofit; reconstruction of an emptied source tab; plugin-pane adoption; cross-session moves; cancellation UX beyond request expiry; hub, dock, provider, or review integration; tmux implementation; and changes to Zellij upstream.

## Stage Report: ideation

- DONE: Define the explicit adoption request and permission-result state machine against the proposed binding generation, managed-view marker, stable target ID, and controller message path.
  The design records a correlated request, one serialized permission state, post-permission identity revalidation, one native move, reconciliation, and a separate post-success focus. It marks the driver contract as pending approval.
- DONE: Specify identity-preserving and no-unrelated-mutation evidence for source, target, permission denial, stale IDs, closed panes, duplicate views, missing controller, and emptied source tabs.
  Eight ACs compare independent pane/tab/PID inventories and exact move counts across every named success and failure case.
- DONE: Separate proved spike facts from the remaining production integration drill, and keep auto-adoption, automated consent, foreign-tab retrofit, hub, dock, provider, and tmux implementation out of scope.
  Native pane/PID preservation, denial, missing source, empty-tab closure, and raw missing-controller behavior are recorded as proved; existing-controller correlation across the consent gap remains the first drill.

### Summary

Adoption is one explicit, expiring, generation-bound request. The controller revalidates after Zellij's permission result, performs one move, and reconciles native state before reporting success. Implementation must first prove a no-auto-launch request path to the existing marked controller.
