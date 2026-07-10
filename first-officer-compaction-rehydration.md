---
id: njr36mfyhbafy8zx9ydks8ep
title: Deterministic first-officer rehydration after compaction
status: ideation
source: captain finding — first-officer critical-path wait contract degraded after context compaction, 2026-07-11
started: 2026-07-10T23:27:31Z
completed:
verdict:
score: 0.95
worktree:
issue:
pr:
mod-block:
---

## Problem

Conversation compaction can preserve the story of a workflow while dropping the obligation that drives its next action. In the v9 replay, the compacted summary mentioned Codex wait semantics, but the first officer dispatched a critical-path feedback repair and returned before it consumed the completion signal, verified the committed report, and re-ran validation. The worker remained live; the first officer lost the continuation.

The current contracts already state the right behavior:

- `skills/first-officer/references/codex-first-officer-runtime.md` requires `wait_agent(timeout_ms)` whenever an unresolved Codex worker is the only remaining work, and requires wait to be reinstalled after operator interruption.
- `skills/feedback-rejection-flow/SKILL.md` requires a critical-path follow-up to complete through `«completion-signal»`, then requires durable verification and reviewer re-run.
- `skills/first-officer/references/first-officer-shared-core.md` forbids stopping after a completed, non-gated, non-terminal stage; the first officer must advance and dispatch the next stage.
- `skills/first-officer/references/fo-dispatch-core.md` owns dispatch identity, assignment epochs, and reuse.

Compaction does not deterministically re-read those files. `AGENTS.md` and the skill catalog may be reinjected by the host, but they carry static project and discovery instructions, not the active worker, assignment epoch, report baseline, or required continuation. A generated summary is advisory. Durable workflow state proves the entity's stage and committed reports, but it does not presently identify an unresolved worker or distinguish `await completion` from `verify`, `re-review`, or `present gate`.

Codex 0.144.1 exposes stronger local surfaces. `codex app-server generate-json-schema --experimental` declares stable `preCompact` and `postCompact` hook events, a `ContextCompaction` thread item, the deprecated compatibility notification `thread/compacted`, and `thread/inject_items` for adding model-visible history without a user turn. These surfaces can force a rehydration fence on Codex v2, but the shared contract cannot depend on them because other runtimes may expose no compaction event.

## Proposed approach

Adopt a durable continuation fence as the cross-runtime contract, then strengthen it with host lifecycle events where available.

### Options considered

1. **Instruction-only re-read.** Add "after compaction, re-read the skill" to the runtime adapter. This is small, but it depends on the compacted summary preserving both the event and the instruction. It does not recover an omitted worker identity or continuation step.
2. **Durable continuation fence.** Record each unresolved critical-path obligation outside model context and require turn-entry reconciliation while any obligation exists. This survives summary omission and works without a compaction event. This is the minimum correctness contract and the recommended shared design.
3. **Lifecycle/event injection only.** Use `postCompact` or app-server notifications to inject instructions. This gives Codex a precise trigger, but an event can be missed, an adapter can lack the event, and injected prose alone does not prove that the model re-read or reconciled state.

The design combines option 2 with option 3 as an enforcement adapter. Option 1 remains a readable statement of the rule, not its safety mechanism.

### Host-neutral continuation record

The launcher creates one session-scoped record at `${SPACEDOCK_FO_CHECKPOINT}`, defaulting beneath the user's state directory, not in the workflow's git checkout. The launcher passes the absolute path to the host. The record is atomic, versioned, and disposable after the session; it is durable across context compaction and turn interruption.

Each critical-path obligation contains:

- workflow directory and entity reference;
- stage, feedback cycle, assignment epoch, and worker identity when bound;
- the entity commit and Stage Report count observed before dispatch;
- one continuation state: `await_completion`, `verify_report`, `route_feedback`, `rerun_review`, `present_gate`, or `terminalize`;
- the next required durable boundary; and
- the contract version and digests last re-read.

The first officer records `await_completion` before spawning or routing work, then binds the returned worker identity. If it stops between those writes, reconciliation treats the unbound dispatch as unresolved rather than complete. It clears an obligation only after it has observed the expected assignment epoch, verified a newer committed report against the checklist, and durably recorded the next continuation state. A completion from an older epoch cannot satisfy the record.

### Rehydration procedure

`spacedock dispatch rehydrate --checkpoint "$SPACEDOCK_FO_CHECKPOINT" --json` is the proposed integration boundary. It reads the record and returns the exact sources and reconciliation actions; it does not infer completion from prose. When the record contains an obligation, the first officer's first lifecycle action in a turn must pass this fence:

1. Re-read `skills/first-officer/SKILL.md`, `references/first-officer-shared-core.md`, and the active runtime adapter.
2. Re-read `references/fo-dispatch-core.md` for every unresolved dispatch. Re-read `skills/feedback-rejection-flow/SKILL.md` when the continuation is a fix or re-review. Load other deferred modules only when the recorded continuation names their trigger.
3. Read the workflow README's active stage definition, the entity's latest frontmatter, latest Stage Report, and `### Feedback Cycles` when present.
4. Reconcile the recorded assignment epoch with the live roster/mailbox and durable report baseline.
5. If no matching completion exists, restore the runtime wait or poll. If completion exists, verify the report and continue to re-review, gate presentation, or terminal handling without an intermediate completion-only return.

An explicit compaction event sets `needs_rehydrate`. A runtime with no event uses the bounded fallback: while the record has any unresolved obligation, every new first-officer turn passes the fence before worker lifecycle, state mutation, status-finalization, or user-facing completion. This costs one deterministic read per active turn and makes summary omission irrelevant.

### Codex app-server v2 enforcement

The Codex plugin registers a synchronous `postCompact` hook. The hook marks the checkpoint `needs_rehydrate` and emits a context entry that names the checkpoint and the rehydrate command. An app-server client may also observe an `item/completed` notification whose item type is `contextCompaction`; it accepts legacy `thread/compacted` only as a compatibility signal. It injects the same directive with `thread/inject_items` when the hook's context cannot be carried directly.

A `preToolUse` hook enforces the fence: while `needs_rehydrate` is set, it permits reads and the rehydrate command but blocks worker lifecycle and workflow mutation calls. The rehydrate command streams the authoritative files, performs reconciliation, records their digests and context epoch, then clears the sentinel. A client-side `turn/completed` watchdog refuses to accept a completion-only stop while the checkpoint still requires continuation; it injects the rehydrate directive and starts the continuation turn. This is the stronger v2 guarantee: post-compaction behavior cannot silently finish or reach guarded lifecycle and mutation calls before rehydration. The spike must prove which collaboration calls traverse `preToolUse`; any uncovered call remains guarded by the shared turn-entry contract.

The app-server schema, not a Codex version label, is the capability probe. The exact local evidence is generated from `codex app-server generate-json-schema --experimental`: `v2/HookStartedNotification.json`, `v2/HookCompletedNotification.json`, `v2/ItemCompletedNotification.json`, `v2/ContextCompactedNotification.json`, and `v2/ThreadInjectItemsParams.json`.

Workflow mods remain separate. They govern workflow lifecycle such as merge; they must not carry context-lifecycle state.

## Acceptance criteria

### Offline

**AC-1: Critical-path continuity.** Across manual compaction, automatic compaction, a summary that omits every wait instruction, a queued completion, and operator interruption, the replay records zero premature worker shutdowns, replacement dispatches, completion-only final responses, or state advances before the matching committed Stage Report. After the report arrives, the entity reaches its next gate or terminal continuation in the same drive.
Verified by: an external event trace and git/state assertions from the replay harness, compared with a deliberately faulty driver that reproduces the v9 premature return.

**AC-2: Authoritative re-read.** Before the first post-compaction lifecycle or mutation action, the trace contains a successful rehydrate epoch whose source digests equal the files on disk for the first-officer entry skill, shared core, active runtime adapter, dispatch core, applicable feedback skill, workflow stage definition, and current entity evidence.
Verified by: the harness changes one contract fixture immediately before compaction and checks the rehydrate result against an independently computed digest manifest.

**AC-3: Durable obligation recovery.** Removing the worker and wait obligations from the replacement summary does not change the recovered entity, stage, cycle, assignment epoch, report baseline, continuation state, or next action.
Verified by: round-trip tests that reconstruct the driver from only a checkpoint, workflow checkout, roster fixture, and mailbox fixture, then compare its typed action with the pre-compaction baseline.

**AC-4: Event detection and bounded fallback.** An event-capable adapter runs one rehydrate fence after each observed compaction epoch. An eventless adapter runs it at the first subsequent first-officer turn and before any lifecycle or mutation action while an obligation remains.
Verified by: adapter table tests for `postCompact`, `contextCompaction`, legacy `thread/compacted`, duplicate events, absent events, and delayed next turns.

**AC-5: Completion attribution.** A stale completion from assignment epoch N cannot satisfy epoch N+1; a matching completion cannot clear the obligation until a newer committed Stage Report passes checklist verification.
Verified by: adversarial mailbox/report ordering tests with stale, duplicate, uncommitted, malformed, and matching evidence.

**AC-6: Wait and feedback guarantees.** A timeout or operator interruption returns control without failing, closing, or redispatching the worker. The next idle action reinstalls wait; a rejected fix proceeds through repair completion, durable verification, reviewer re-run, and normal gate flow.
Verified by: a fake-clock runtime test and the full v9 feedback replay, asserting checkpoint transitions and worker identity.

**AC-7: Inactive cost.** With no unresolved critical-path obligation, rehydration performs no roster reconciliation, contract streaming, or checkpoint write.
Verified by: an event trace for an idle workflow with an empty checkpoint.

### Interactive

**AC-8: Live Codex demonstration.** During a delayed worker repair, CL triggers `/compact`, interrupts one foreground wait, and sends an unrelated status question. The first officer reports status, restores wait, consumes the worker's final notification, verifies its committed report, and continues to re-review or the next gate without a reminder.
Verified by: CL's observation plus the app-server notification trace, hook trace, checkpoint history, and workflow git log from that session.

## Test plan

The riskiest unproved mechanism is Codex's ordering and coverage: a synchronous `postCompact` context entry or `thread/inject_items` directive must become model-visible before the next first-officer action, and `preToolUse` must cover collaboration lifecycle calls as well as shell mutations.

1. **Invalidate the design first.** Build a temporary Codex plugin with a `postCompact` marker and a blocking `preToolUse` hook. Start a delayed subagent through app-server v2, call `thread/compact/start`, and capture `item/completed`, `hook/started`, `hook/completed`, injected items, and the first post-compaction tool call. Fail if the marker is absent or late, if duplicate events create multiple epochs, or if any collaboration lifecycle call bypasses the hook. Test manual and automatic compaction. If lifecycle calls bypass the hook, retain the cross-runtime fence and narrow the v2 enforcement claim.
2. Add checkpoint serialization, atomic-write, incomplete-bind, digest, epoch, and stale-completion tests under a proposed `internal/rehydrate/` package.
3. Add a split-root replay fixture under `internal/ensigncycle/testdata/compaction-rehydration/`: validation rejects to implementation, a delayed fixer commits a report, and validation must run again before the gate.
4. Add `internal/ensigncycle/compaction_rehydration_test.go`. Drive dispatch, replace context with a summary that omits the obligation, deliver completion before and after the next turn, interrupt wait, verify the committed report, and assert the next gate or terminal state.
5. Table-test event-capable and eventless adapters. Include duplicate notification, no notification, queued completion, operator message, wait timeout, stale epoch, uncommitted report, and host restart with the same checkpoint.
6. Extend `integration/codex_idle_notification_test.go` or add an adjacent contract test to prove the Codex runtime text and live adapter preserve wait restoration and file verification.
7. Run the live Codex scenario for AC-8 and retain the app-server JSONL, hook log, checkpoint transitions, and temp workflow git log as CI artifacts.

## Out of scope

Implementing the protocol in this stage; changing Zaphod product behavior; treating a generated summary as authoritative; changing compaction algorithms; persisting full conversation history; weakening existing wait, feedback-routing, gate, or merge contracts; and using workflow mods as context-lifecycle hooks.

## Stage Report: ideation

- DONE: Define the smallest cross-runtime rehydration contract and a stronger app-server v2 enforcement path, with explicit detection and fallback semantics.
  The design uses a durable continuation fence on every active turn and adds Codex `postCompact`/`contextCompaction`, injection, and pre-tool enforcement where the live schema proves those capabilities.
- DONE: Specify externally verifiable acceptance criteria and a replay harness proving unresolved critical-path dispatch survives compaction through completion, durable verification, and next-gate continuation.
  AC-1 through AC-8 use external event traces, git state, checkpoint epochs, and a split-root v9-style replay rather than prose inspection.
- DONE: Compare AGENTS/skill instructions, durable checkpoints, lifecycle hooks, and app-server event injection; identify the authoritative files reread and the exact integration boundary.
  The body distinguishes static host instructions from active continuation state and specifies `spacedock dispatch rehydrate --checkpoint ... --json` as the boundary.
- DONE: Name the riskiest unproven mechanism and put its smallest invalidating end-to-end check first in the test plan.
  The first test probes post-compaction ordering, model visibility, duplicate events, and pre-tool coverage for collaboration calls.
- DONE: Split acceptance criteria into offline and interactive evidence.
  Seven offline criteria cover correctness and cost; one CL-driven live Codex demonstration covers the real operator path.
- DONE: Propose concrete source paths and tests without editing product or cached plugin files.
  Proposed work targets `internal/rehydrate/`, `internal/ensigncycle/`, and the existing Codex idle-notification integration region.

### Summary

The minimum reliable contract is a durable continuation fence, not a better summary. Every active first-officer turn reconstructs its obligation from the checkpoint and authoritative workflow sources; Codex v2 can additionally force the fence with compaction hooks and app-server injection. The design preserves the existing wait, durable-verification, feedback, and next-gate guarantees and makes the v9 failure replayable.
