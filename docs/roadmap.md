# Zaphod roadmap

This roadmap defines the delivery sequence for the managed workspace product.
The architecture lives in
[`docs/zaphod-workspace-architecture.md`](zaphod-workspace-architecture.md).
`docs/plan-agent-rail.md` and the root prototype documents remain historical
build records; they do not override this sequence.

## Delivery rules

- Prove native identity and transport before building behavior on top of them.
- Keep portable policy outside multiplexer-specific controllers.
- Treat stable native IDs as locators, not ownership proof.
- Fail closed with an operator-visible recovery path.
- Run interactive captain drills only after independently reproducible
  infrastructure and offline checks pass.
- Never mutate standing Zellij or tmux configuration during development tests.

## Sprint 1 — managed-view foundation and feasibility

### Goal

Establish trustworthy test infrastructure, one owned native `zaphod`
executable, a recoverable driver-neutral binding core, and proved Zellij
identity mechanisms. This sprint does not ship production keybindings or pane
adoption.

### Filed tasks

1. **`foreground-attached-client-profile`** — make the disposable Zellij
   client own the foreground terminal; prove PTY input, cleanup, signal
   handling, temporary-root removal, and unchanged global files.
2. **`zaphod-native-cli-skeleton`** — choose the package owner and artifact
   path; add build and test-profile wiring, a version handshake, and typed
   command/result plumbing without managed-tab behavior.
3. **`managed-view-driver-contract`** — implement full session identity,
   atomic binding storage, reverse uniqueness, mutation-aware results, fake
   adapters, and inspect/unbind/rebind/repair operations. Keep the persisted
   Zellij representation provisional until the native feasibility gate.
4. **`zellij-managed-identity-feasibility`** — prove or reject the exact
   multi-client invocation witness, persistent marker owner and query path,
   session replacement/incarnation, native ID reuse, and
   create-before-persist recovery. Spike code remains disposable.

### Dispatch and merge order

1. Complete and merge `foreground-attached-client-profile` first. No other
   task may schedule an interactive Zellij drill before its validation gate.
2. Ideate the native CLI and reframed binding core together, then freeze their
   artifact path, version handshake, full session identity, and typed result
   envelope at a shared contract gate.
3. After that freeze, run three safe lanes in parallel:
   - native CLI artifact and profile wiring;
   - portable registry, recovery commands, and fake-adapter suite;
   - throwaway Zellij identity and marker feasibility.
4. Reconcile all three lanes at the native identity gate. Revise the contract
   if witness, marker, incarnation, namespace targeting, or crash recovery
   fails; do not compensate with active-tab, pane, cwd, or session-name
   guessing.
5. Merge only after the integrated contract suite, process tests, disposable
   native drills, and global-isolation checks pass together.

### Sprint gates

- **Infrastructure:** the attached client owns the PTY foreground process
  group and raw input reaches a terminal canary; normal exit and
  `INT`/`TERM`/`HUP` clean up without changing standing files.
- **Contract:** concurrent and killed writers, schema handling, reverse
  uniqueness, stale/corrupt recovery, typed `Unchanged`/`Indeterminate`
  results, and explicit operator repair are independently reproduced.
- **Native identity:** two-client invocation witness, persistent marker
  ownership, detach/reattach, session replacement, ID reuse, and the
  create/persist crash window are proved against disposable Zellij 0.44.3.
- **Integration:** one test-profile `zaphod` artifact owns the versioned
  interfaces, all required suites pass on the merged branch, and no standing
  multiplexer state changes.

### Exit criteria

- A human can type and exercise real keys in the disposable profile.
- The repository owns one buildable, versioned native `zaphod` artifact.
- Bindings can be created, inspected, reconciled, repaired, and removed
  without trusting display names or reused native IDs.
- Zellij invocation, marker, incarnation, and crash-recovery mechanisms are
  either proved and frozen or rejected with the controller architecture sent
  back for revision.
- No captain demo is used to discover test-infrastructure failure.

## Sprint 2 — managed Zellij entry and guarded toggle

Task `zellij-managed-tab-controller` consumes the accepted Sprint 1 contract.
It ships idempotent `Alt Shift z` create-or-focus and makes `Alt /` issue no
native layout action outside the recorded managed tab. Its first check proves
the real key invocation witness; implementation cannot fall back to active
client, tab, pane, cwd, or display-name guessing.

Sprint 2 exits after repeated and concurrent entry leaves exactly one managed
tab, foreign views remain unchanged, structured failures reconcile safely,
and the captain accepts real-key focus, permission, flicker, and latency in
the repaired disposable profile.

## Sprint 3 — explicit pane adoption

Task `zellij-pane-adoption` consumes the persistent marker/controller target
and mutation-aware result envelope accepted in Sprint 2. Pure request
validation, permission transitions, and reconciliation may begin once shared
types stabilize, but native movement waits for the transport invalidation
gate.

Sprint 3 first proves one correlated existing-instance request across the
permission gap without auto-launch. It then adds exactly one native move with
post-permission revalidation, pane/PID preservation, unrelated-pane
preservation, explicit empty-source behavior, and visible recovery.

## Later delivery

After managed-view entry and adoption are stable:

1. Define the workspace hub's canonical item and local socket contracts.
2. Build the portable dock TUI against that protocol.
3. Add the tmux managed-window driver and run the shared driver suite against
   both multiplexers.
4. Adapt AgentsView session ingestion, then gate and review providers.
5. Add registered `zaphod notify` ingress and complete create, adopt, attach,
   detach, and reattach acceptance drills.

## Parked outside the release path

Keep the foreign-tab retrofit tasks `j5`, `eh`, `fw`, and `4d`, the upstream
transactional retained-pane request, automatic adoption or consent, global
installer rollout, and any Zellij fork outside these sprints. Treat `yb`,
`7v`, and `pz` as prototype evidence to extract only after their behavior fits
the new hub, driver, and provider contracts.
