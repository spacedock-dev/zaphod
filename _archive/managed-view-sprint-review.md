---
id: 68mb9yq4v90fnb8gwhe1kjwy
title: Senior staff review of the managed-view implementation sprint
status: done
source: captain-requested full-sprint review of managed-view ideation and dispatch sequence, 2026-07-11
started: 2026-07-11T04:31:09Z
completed: 2026-07-11T12:57:09Z
verdict: REJECTED
score: 1.0
worktree:
issue:
pr:
mod-block:
archived: 2026-07-11T12:57:04Z
---

## Problem

Three related ideation packets now define the managed-view contract, Zellij controller, and explicit pane adoption, while a live drill exposed a foreground-client defect in the canonical disposable profile. Review the work as one sprint before implementation begins so local task quality does not hide a broken critical path, misplaced gate, unsafe parallelism, or unowned integration risk.

## Review scope

Review `managed-view-driver-contract.md`, `zellij-managed-tab-controller.md`, `zellij-pane-adoption.md`, `docs/zaphod-workspace-architecture.md`, `docs/plan-agent-rail.md`, and `scripts/zellij-worktree-test-profile.sh`. Treat the proposed order as: repair interactive test infrastructure; freeze the portable contract; run contract implementation and the controller invocation-witness spike in parallel; integrate and validate the controller; run the adoption transport invalidation drill; implement adoption; then proceed to hub, dock, tmux, and providers.

## Expected outcome

Return an independent senior-staff recommendation: approve the sprint, approve with concrete reframing, or reject. Name the minimum task splits or dependency changes, the exact critical path and safe concurrency, required gates and evidence, sprint exit criteria, and any work that must be parked. Do not edit product code or product documentation.

## Senior staff review

### Recommendation

**Approve with concrete reframing; do not dispatch the three implementation packets as currently sliced.** The managed-view direction, fail-closed identity rules, invocation-witness invalidation, and adoption reconciliation are strong. The combined sprint nevertheless has four entry blockers and several missing recovery/API decisions that would otherwise be discovered inside implementation branches.

### Material findings

1. **P0 — the disposable profile cannot host an interactive gate.** `scripts/zellij-worktree-test-profile.sh:133-160` backgrounds the attached Zellij client and polls it from a non-interactive shell. An asynchronous terminal client does not reliably retain readable controlling input or foreground ownership; session existence and lifecycle tests do not prove it can receive a human key. Controller AC-1/6/7 and adoption AC-8 must not consume captain time until this is repaired independently.
2. **P0 — no persistent Zellij object owns the binding marker.** The contract requires a tab marker to close the create/persist crash window (`managed-view-driver-contract.md:47-73`) and assigns marker reporting to the controller (`:112`). The controller packet launches a fresh actor and closes it after every success/no-op (`zellij-managed-tab-controller.md:32-55`). tmux window-option evidence does not prove a Zellij marker carrier, lifetime, query path, or crash recovery.
3. **P0 — adoption targets a controller that the preceding task removes.** Adoption sends to an “existing marked controller” without auto-launch (`zellij-pane-adoption.md:57,112,122-124`), while the controller design leaves no controller pane. Adoption also assumes that controller may already hold `ChangeApplicationState` because it focuses/steers (`:75-79`), but the key controller requests only `ReadApplicationState` and `RunCommands` and delegates mutations to the CLI helper. A persistent service, a separately addressed marker actor, or another transport must be chosen and invalidated before adoption implementation.
4. **P0 — the native `zaphod` executable has no build/install owner.** The controller calls an installed `zaphod zellij key-request` helper and adoption exposes `zaphod adopt-pane`, but this repository currently packages one `zellij-sidebar` WASM crate plus `grout`; no packet chooses the native CLI language/module, artifact path, installer transaction, version handshake, or worktree-profile input. This is sprint infrastructure, not incidental controller code.
5. **P1 — the controller protocol drops the contract's failure semantics.** The shared contract requires stable error kinds plus `Unchanged|Indeterminate` (`managed-view-driver-contract.md:95-108`); `KeyResult` carries only a coarse outcome and free-form message (`zellij-managed-tab-controller.md:83-90`). Timeouts and disconnects therefore cannot drive safe reinspection or distinguish closed failures from partial mutation.
6. **P1 — Zellij session incarnation is a proposed representation, not evidence.** Device/inode/mtime of a session socket (`managed-view-driver-contract.md:57`) has not been proved stable under normal activity, rename, detach/reattach, or replacement. The native evidence cited immediately below is tmux-only. Do not freeze `bindings-v1.json` around this representation before a disposable Zellij replacement drill.
7. **P1 — binding recovery is unowned.** The design references explicit `--rebind` but excludes automatic rebind and defines no inspect/unbind/repair surface for corrupt registries, stale dead-session bindings, duplicate markers, or an indeterminate create. Fail-closed without an operator recovery path turns a safe error into a permanently wedged workspace.
8. **P1 — request identity is narrower than binding identity.** The contract distinguishes server namespace, native ID, and incarnation; the controller request sends only `session_id` plus pane/tab/cwd and lets the helper derive the root (`zellij-managed-tab-controller.md:61-93`). Two namespaces can carry the same session name, and cwd can change between witness and helper execution. Pass an opaque binding/request handle or the full revalidated session identity; never select a server from display identity.
9. **P2 — normalize vocabulary and scope before code.** `IgnoredForeignView`, `NoopForeign`, and `noop_foreign` describe one outcome, while the generic contract advertises future `open_surface`, tmux, and adoption capabilities not needed to land create/focus/toggle. Freeze one result envelope and implement the minimum exercised capability set; leave extension points typed but unimplemented.

### Foreground-profile repair task

Create one separate task: **“Foreground attached-client disposable Zellij profile.”** Its product change is limited to the profile script and process-level tests. After printing metadata, the script must run the attached client synchronously in the terminal foreground; remove the background `CLIENT_PID`/poll loop. Automation owns any outer PTY, while the profile retains cleanup traps and global-file hash checks.

The first proof is a PTY probe that fails on the current script: the attached client process group must equal the PTY foreground process group, and raw input sent through that PTY must reach a terminal canary visible through `dump-screen`. Then rerun normal exit, session deletion, TERM/INT/HUP cleanup, temporary-root removal, and standing config/layout hash checks. A fresh validator must reproduce this on a clean checkout. Only after that gate passes may controller or adoption interactive acceptance be scheduled; CL is not the test harness for this repair.

### Reframed critical path and safe concurrency

1. **Merge the foreground-profile repair.** Gate on the PTY input probe and existing cleanup/global-isolation suite.
2. **Reconcile and freeze the logical contract, not unproved Zellij representation.** Add the native CLI/package owner, opaque ownership proof, full session identity, typed result envelope, and inspect/rebind/unbind recovery operations. Keep the persisted Zellij incarnation/marker encoding provisional until native evidence exists.
3. **Run two lanes in parallel after that freeze.** Lane A implements registry atomicity, uniqueness, recovery commands, driver-neutral types, the native CLI artifact, and fake adapter suite. Lane B is a disposable/throwaway Zellij feasibility spike for multi-client invocation witness, persistent marker carrier/query, create-before-persist recovery, and session replacement/incarnation. Lane B must not grow into production controller code.
4. **Integration gate: bind the Zellij adapter to proved identity.** Reject or revise the contract if witness, marker, or incarnation fails. Freeze the helper's namespace targeting, artifact/version handshake, and mutation-aware result envelope here.
5. **Implement and validate managed entry/toggle.** Build the controller and helper against the integrated contract; extend the repaired profile; run pure, process, disposable native, concurrency, foreign-no-op, and failure drills. Only then present real `Alt Shift z`, `Alt /`, permission, focus, flicker, and latency to CL.
6. **Parallel lane after shared error types stabilize:** adoption may implement pure command validation, permission-state transitions, and reconciliation tables. It may not choose transport, launch a controller, or call the native move yet.
7. **Run the adoption transport invalidation gate after controller integration.** Name the persistent target, prove one correlated existing-instance request across the permission gap, mutate/close the target before grant, and require zero move/new plugin pane. Failure returns to controller architecture, not to move logic.
8. **Implement and validate native adoption.** Add exactly one move, post-permission revalidation, reconciliation, separate focus, empty-source behavior, and visible recovery. Run disposable identity/PID drills before CL's denial/success demo.
9. **Integrate and document.** Rebase the adoption pure lane onto the accepted controller transport, run the full branch suite and global-isolation checks, then update architecture/plan/README only for behavior that passed live gates.

This ordering avoids unnecessary serialization: portable registry/fake work and native identity feasibility run together; adoption's pure state machine overlaps controller implementation after shared types settle. It preserves necessary serialization at native identity, controller UX, and adoption transport boundaries.

### Required gates and sprint exit

- **Infrastructure gate:** foreground PTY/input proof, cleanup, signal behavior, and zero standing-state mutation.
- **Contract gate:** killed/concurrent writers, schema/version handling, corrupt/stale registry recovery, reverse uniqueness, typed mutation results, and explicit operator repair.
- **Native identity gate:** two-client invocation witness, durable Zellij marker ownership, session replacement, ID reuse, and crash-window recovery.
- **Controller gate:** exact one-tab delta, repeated/concurrent entry, stable-ID focus, targeted toggle, zero foreign native action, artifact/version failures, and CL's real-key UX.
- **Adoption gate:** no-auto-launch correlated transport, stale target after consent, denial/timeout reconciliation, pane/PID preservation, unrelated-pane preservation, empty-source semantics, and CL's explicit adoption UX.
- **Integration gate:** clean merged branch, all offline/process/live checks, documentation consistency, no standing global mutation, and an operator-visible recovery path for every fail-closed binding state.

The sprint exits only when those gates pass and one installed/test-profile `zaphod` artifact owns the contract/helper commands. A managed view survives retry without duplicate creation; foreign keys are mutation-free; adoption preserves native identity; indeterminate operations reconcile before retry; and no captain demo is used to discover infrastructure failure.

### Explicitly parked

Park hub/socket protocol, portable dock, production tmux driver, providers, `grout` extraction, notify ingress, global installer rollout, automatic adoption/consent, foreign-tab retrofit (`j5/eh/fw/4d`), transactional retained-pane research, and any Zellij fork. tmux fixtures may test driver-neutral shapes, but tmux product code is not part of this sprint.

## Stage Report: ideation

- DONE: Adversarially review all three ideation packets as one sprint, tracing contract, controller, adoption, architecture, and delivery-plan dependencies; cite every material contradiction, missing owner, unsafe overlap, or unnecessary serialization.
  The review identifies four P0 integration gaps, five bounded P1/P2 contract risks, and a dependency graph that permits registry/native-spike and adoption-pure/controller parallel lanes without crossing native gates.
- DONE: Evaluate the foreground-client test-profile defect as a sprint-entry blocker and specify the smallest separate repair task, proof, and gate needed before any interactive acceptance consumes captain time.
  The repair makes the attached client foreground, proves PTY process-group/input delivery plus cleanup/global isolation on a clean checkout, and blocks CL demos until independently validated.
- DONE: Return one senior-staff recommendation with an exact critical path, safe parallel lanes, required invalidation and integration gates, sprint exit criteria, and explicit parked work.
  Recommendation is approve with concrete reframing: nine ordered steps, six evidence gates, measurable sprint exit, and a parked list exclude downstream product work and obsolete retrofit research.

### Summary

The sprint's end-state direction is credible, but the current packets do not compose into an implementable sequence. Repair the foreground profile first, prove Zellij marker/incarnation/invocation identity beside the portable core, assign the native CLI and recovery surfaces, integrate the controller, and only then invalidate and implement adoption transport. This preserves useful parallelism while keeping captain time behind independently reproducible gates.
