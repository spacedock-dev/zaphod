---
id: qbf5syzpwvpggp2xgnd5asvf
title: Managed-view binding and shared driver contract
status: ideation
source: evergreen workspace architecture delivery step 1, captain-authorized 2026-07-11
started: 2026-07-11T04:15:01Z
completed:
verdict:
score: 0.99
worktree:
issue:
pr:
mod-block:
---

## Problem

The managed-view spike proved Zellij mechanisms, but production work has no portable contract for binding one workspace to one multiplexer session and one stable managed view. Define the smallest contract that lets the launcher, Zellij controller, future tmux driver, and shared test suite agree without importing hub, dock, or provider concerns.

## Seed direction

Ideation must define binding identity and persistence, `ensure_managed_view`, list/focus/open behavior, driver-neutral errors and capabilities, stable-ID invalidation, fake-driver fixtures, and the boundary between portable launcher logic and native controller operations. It must propose the corresponding revision to the logical dispatch sequence in `docs/plan-agent-rail.md`.

## Dependency boundary

This task is the foundation. It must not implement the Zellij controller, pane adoption, hub, dock, providers, or tmux. Controller implementation may begin only after this contract's ideation gate is approved.

## Proposed approach

### Binding identity and persistence

Store a small versioned registry at the platform runtime directory under `zaphod/bindings-v1.json`. Create the directory with mode `0700`, the registry with mode `0600`, and guard every read-modify-write with one registry lock. Write a temporary file, sync it, rename it atomically, and sync the directory. One registry makes the two uniqueness rules transactional: one active binding per canonical root and one canonical root per live native session.

```text
Binding {
  schema: 1,
  binding_id: UUID,                 # random generation marker
  canonical_root: absolute real path,
  workspace_key: sha256(canonical_root bytes),
  mux: zellij | tmux,
  session: SessionRef {
    server_namespace,
    native_id,
    incarnation,
    display_name,
  },
  managed_view: ManagedViewRef? {
    native_id,
    marker: binding_id,
    reserved_name,
  }
}
```

Resolve a workspace by taking the Git top level when present, then resolving symlinks; otherwise resolve the requested directory. The path, not the hash, remains the authoritative identity. The hash only names runtime artifacts.

`server_namespace` separates independent native servers. `incarnation` prevents a new same-name session from inheriting an old binding. The Zellij adapter uses the session socket's canonical directory plus file identity (`device`, `inode`, and modification time); a socket rename can recover a renamed session by that identity. The tmux adapter uses the canonical socket identity, native session ID such as `$0`, and `session_created`.

`binding_id` also marks the managed view. tmux stores it in the window option `@zaphod_binding`. The Zellij controller returns it with its stable tab ID when queried. A reserved display name aids discovery but never proves ownership.

### Portable convergence

Implement `ensure_managed_view(binding)` once over a native adapter:

1. Acquire the registry lock and re-read the binding.
2. Resolve the session by namespace, native ID, and incarnation. Reject a replaced session.
3. List native views and validate the recorded view by stable ID, marker, and reserved name.
4. If the ID is stale but exactly one view carries the marker, recover that view and persist its ID.
5. If no marked or reserved view exists and the session incarnation matches, create one view with the existing `binding_id`, persist its returned stable ID, and focus it.
6. If the exact view exists, focus it only when needed.
7. Reject every duplicate marker, unmarked reserved name, reused stable ID, or indeterminate native result without creating another view.

The marker closes the crash window between native creation and registry persistence. A retry discovers the marked view rather than creating a duplicate. The registry lock serializes two local launchers.

### Native adapter and public operations

The portable layer exports these logical operations:

```text
ensure_workspace(root, mux, requested_session?) -> Binding
ensure_managed_view(binding, invocation?) -> Created | Recovered | Focused | AlreadyFocused
current_view(invocation) -> NativeViewRef
list_panes(binding) -> [PaneInfo]
focus_pane(binding, pane_id) -> Focused
adopt_pane(binding, pane_id, explicit_intent) -> Adopted
toggle_managed_layout(binding, invocation) -> Toggled | IgnoredForeignView
open_surface(binding, trusted_launch) -> Opened
session_exists(session_ref) -> bool
```

The shared implementation calls narrow adapter primitives: inspect session, list views, create marked view, focus stable view, resolve invoking pane/view, list/focus/move panes, toggle native layout, and open a trusted surface. Native adapters return observations; they do not choose rebind, repair, or conflict policy.

Capabilities are explicit: `CreateManagedView`, `ResolveInvocation`, `FocusPane`, `AdoptPane`, `ManagedLayoutToggle`, `PopupSurface`, and `SplitSurface`. Zellij advertises layout toggle and pane adoption. tmux may omit `ManagedLayoutToggle`; the first release does not invent a cross-multiplexer toggle. Unsupported capabilities return `Unsupported` without fallback mutation.

### Error and mutation model

Every error has a stable kind and a mutation state, `Unchanged` or `Indeterminate`:

```text
SessionMissing | SessionReplaced | ViewMissing | ViewIdentityConflict
ReservedNameConflict | DuplicateManagedView | PaneMissing | StaleInvocation
ControllerUnavailable | PermissionDenied | Unsupported | Busy
PersistenceFailure | NativeFailure
```

Preflight and identity errors must report `Unchanged`. A timed-out or disconnected native mutation reports `Indeterminate`; the caller re-inspects state before retrying and never issues a blind second mutation. Native stderr and exit status remain diagnostic fields, not portable error kinds.

Stable IDs are necessary but insufficient. An ID with the wrong marker is `ViewIdentityConflict`, even when its name matches. A missing ID plus one matching marker is recoverable. A missing ID plus no marker and no reserved name is repairable only in the same session incarnation. A same-name session with a different incarnation requires explicit `--rebind`.

### Ownership boundary

The portable launcher owns canonical roots, the registry, locking, uniqueness, convergence, recovery, capability checks, and user-facing errors. Native drivers own observation and the smallest multiplexer mutations. The Zellij controller owns invocation context, tab marker reporting, guarded swap steering, and `break_panes_to_tab_with_id`. The tmux adapter owns socket/session/window/pane commands and native marker options. Hub items, provider parsing, review policy, dock rendering, and permission consent remain outside every driver.

## Riskiest mechanism and live evidence

The riskiest mechanism is crash-safe recovery when a native stable ID disappears or is reused. The managed-tab spike observed Zellij return tab ID `2` for a failed creation and later reuse `2` for a different tab. Therefore, a stable ID or reserved name alone cannot prove ownership.

A disposable tmux 3.6a server exercised the marker model. Session `$0`, managed window `@1`, and pane `%0` survived a rename, a move from index `1` to `5`, focus by stable ID, and `join-pane`; pane `%0` retained PID `26671`. Killing `@1` and creating the same reserved name produced `@2` without a marker. Adding marker `7df0f1d7-test-generation` made exactly one recovery candidate: `@2|zaphod-managed|7df0f1d7-test-generation`. Killing same-name session `$0` and recreating it on the same socket produced `$2` with a different `session_created`. These observations come from tmux's native IDs, PIDs, timestamps, and window options, not contract prose.

## Acceptance criteria

### Offline

**AC-1 — Registry uniqueness and atomicity.** Concurrent attempts to bind one root to two sessions, or one session to two roots, yield one committed binding and one conflict; a killed writer leaves either the old complete registry or the new complete registry.
Verified by: process-level tests against a temporary runtime directory, with two independently started writers and JSON parsing after forced termination.

**AC-2 — Exactly one managed view.** Two concurrent `ensure_managed_view` calls against an unbound fake session call `create_marked_view` exactly once and return the same stable view ID and marker.
Verified by: a barrier-controlled fake adapter whose call log and final inventory are owned by the test harness.

**AC-3 — Crash recovery and ID reuse fail closed.** A stale recorded ID plus one matching marker recovers without creation; a reused ID with the wrong marker, an unmarked reserved name, or duplicate markers returns the specified conflict with zero mutation calls.
Verified by: table-driven fake inventories plus the native Zellij ID-reuse and tmux `@1` to `@2` observations above.

**AC-4 — Session incarnation prevents accidental inheritance.** Rename with the same incarnation updates the native name; a same-name session with a different incarnation returns `SessionReplaced` and performs no view mutation.
Verified by: fake socket identities and a disposable native socket/session replacement fixture.

**AC-5 — Capability and error portability.** Each adapter maps native failures to the same stable kinds and mutation states; an unsupported operation performs no native fallback.
Verified by: the shared driver suite run against fake Zellij and tmux adapters with externally supplied exit statuses and inventories.

**AC-6 — Pane and invocation guards.** Foreign invocation returns `IgnoredForeignView` with zero layout calls; managed invocation issues one toggle. Adoption requires explicit intent and, when supported, preserves native pane ID and process identity.
Verified by: fake call counts and the prior Zellij pane `0`/PID `18723` plus tmux pane `%0`/PID `26671` live observations.

### Interactive

This contract changes no production keybinding or view. The later Zellij-controller and pane-adoption tasks own the captain's `Alt Shift z`, guarded `Alt /`, and move-pane demonstrations.

## Test plan

1. First invalidate the design with the crash-window table: stale ID plus matching marker must recover; stale/reused ID without the marker must not focus or create.
2. Exercise registry locking, atomic replacement, process death, reverse uniqueness, and permission modes in a temporary runtime directory.
3. Run the same convergence tables against a deterministic fake adapter: absent, existing, recovered, renamed session, replaced session, reserved-name collision, duplicate marker, native timeout, and persistence failure.
4. Assert exact adapter call sequences and mutation counts, not messages written by the implementation.
5. Run a disposable Zellij profile and tmux socket for stable IDs, markers, focus, replacement, and pane identity; never use `WORK` or standing configuration.

## Proposed delivery-plan revision

Replace the first two items under `docs/plan-agent-rail.md` → `Target delivery order` with:

1. Define and implement the binding registry, driver-neutral types, error/capability model, fake adapter, shared contract suite, and generic managed-view convergence.
2. Build the thin Zellij adapter/controller against that suite. Add `Alt Shift z` create-or-focus and guard `Alt /` by the invoking pane's stable tab ID and binding marker.

Keep current items 3–8 unchanged. This makes the fake contract and crash recovery executable before native controller work begins.

## Out of scope

The Zellij controller implementation; production keybindings; pane-adoption UI; hub, dock, or provider protocols; tmux production code; cross-multiplexer layout toggle semantics; persistent item state; automatic rebind; automated permission consent; a global daemon; and any foreign-view retrofit.

## Stage Report: ideation

- DONE: Define the exact workspace/session/managed-view binding, driver operations, capability and error model, stable-ID invalidation rules, and portable-versus-native ownership boundary.
  The design specifies a locked atomic registry, session incarnation, binding generation marker, logical operations, adapter primitives, optional capabilities, mutation-aware errors, and fail-closed recovery rules.
- DONE: Name and exercise the riskiest unproven mechanism first, then specify fake-driver and live evidence whose expected values come from outside the implementation.
  The crash-recovery marker was exercised on tmux 3.6a: `$0`, `@1`, and `%0` survived rename/reindex/move; `%0` kept PID `26671`; same-name replacements became `@2` and session `$2`; marker `7df0f1d7-test-generation` identified one recovery candidate.
- DONE: Produce bounded acceptance criteria, test plan, out-of-scope list, and an exact proposed revision to the logical dispatch sequence in docs/plan-agent-rail.md.
  Six offline ACs, the no-interactive-proof rationale, an ordered five-step test plan, explicit exclusions, and replacement text for delivery items 1–2 are recorded above.

### Summary

The shared contract treats native stable IDs as locators, not ownership proof. A portable converger owns policy and validates a session incarnation plus managed-view marker; Zellij and tmux adapters expose narrow observations and mutations. The fake contract suite and registry implementation should land before the Zellij controller.
