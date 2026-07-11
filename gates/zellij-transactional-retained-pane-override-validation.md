# Validation: Zellij transactional retained-pane override

Entity: `zellij-transactional-retained-pane-override.md`

Implementation worktree commit: `3a92ae8948c97d6f094bd260a9ff3f948077c469`
(clean before validation). All behavioral replay and attacks used disposable
paths under `/tmp/4dt6-validation-*`; the implementation worktree was not
modified.

## Recommendation

**REJECTED.** The ordered series applies and its narrow unit filters turn RED
to GREEN, and the proof-kit directory is inert to Zaphod. It does not implement
or prove the durable server contract in AC-1 through AC-5: there is no server
instruction or tab operation, no plugin event delivery, no CLI command, no
staging/replan race, and no process-level pane/PID matrix.

The apparent `pane-snapshots.json` evidence is not an observation. The
verifier hashes three KDL files and writes a hard-coded list of claimed
invariants at `verify.sh:233-243`; it never starts Zellij, creates a pane, reads
`list-panes`, records a PID, or consumes the fixtures.

## Independent replay

- Run 1, `--fetch`: exact base
  `55a2121b73dce4be624cda425a960e893000777c`; patch IDs
  `ab01695a`, `ffd08f9b`, `65c0cea`; RED exit 101 with expected `E0432`;
  planner filter GREEN 1/1; result helper filter GREEN 3/3; cleanup complete.
- Run 2, clean official `--source`: same base, patch IDs, phase exits, and
  cleanup result. No implementation artifact log was reused.
- Wrong-base control at upstream `v0.44.2` exited 1 before Cargo with `source
  HEAD is not BASE_COMMIT`; every phase exit remained null.
- A copied kit with a deliberately corrupted 0002 context failed in the
  ordered-series preflight before Cargo; every phase exit remained null.

## Offline AC verdicts

| AC | Verdict | Independent evidence |
|---|---|---|
| AC-1 | **REFUTED** | The only server test invoked constructs `RetainedPane` values in memory and compares planned IDs. No terminal, plugin, child PID, server instruction, or `list-panes` snapshot exists in any patch. |
| AC-2 | **REFUTED** | The uninvoked geometry unit test proves only that two input Rust values remain equal after a pure planner error. There is no tab snapshot and no PTY/plugin instruction recorder, so zero spawn/close/unload/resize is not observed. |
| AC-3 | **REFUTED** | `commit_retained_override` has no test. No test stops after commit, compares actual pane maps, or injects a swap-selection failure. Base/swap fields are merely stored in the plan and never installed. |
| AC-4 | **REFUTED** | The N=1/N=2/N=3 unit test has no cwd, argv, hold metadata, process, or discovery-call denial. Absence of launch identity from a synthetic struct is not the required five-fixture process matrix. |
| AC-5 | **REFUTED** | Patch 0003 adds a serializable utility type and two CLI-formatting helpers only. It does not add a plugin command/event, server routing, exactly-once delivery, focus-switch race, CLI parser/action, timeout, stdout/stderr process test, or permission check. |
| AC-6 | **PASS** | `c66400d..3a92ae8` is empty for protected production paths; normalized `cargo metadata --locked --no-deps` is identical; base/candidate installer output normalizes to identical SHA-256 `72ac8698...a5261b`; only `verify.sh` is executable; Rust 132, Go 35, Go vet, and shell syntax passed from a disposable candidate with the kit removed. |
| AC-7 | **REFUTED** | Exact-base/apply controls and the narrow RED/GREEN exits pass, but the identical test filter covers only one planner test. `pane-snapshots.json` contains fixture hashes and declared strings, not before/after pane state; no process-focused test runs. |

## Adversarial refutation audit

- **False-positive proof — SURVIVES.** `verify.sh` returns `verdict: passed`
  while exercising one pure planner test and three result-type tests. The
  hard-coded process-matrix JSON makes the artifact look broader than the
  commands in `commands.tsv`.
- **N=2 atomic rejection — SURVIVES.** The 79+79 test is present but the
  verifier's server filter does not run it. Even if run, it has no live tab or
  side-effect recorder and therefore cannot detect spawn, drop, resize, dirty,
  name, base, or swap mutations.
- **Stale/wrong tab identity — SURVIVES.** `plan.tab_id` is returned by the
  planner but `commit_retained_override` never accepts or checks a current tab
  ID. A plan can therefore be handed to any matching `TiledPanes`; no focus or
  stable-target routing exists.
- **Panic/indexing path — SURVIVES AS UNGUARDED.** Public plan fields are not
  revalidated as a complete ID set before `drain`; commit uses
  `existing.remove(...).expect(...)`. A malformed in-server plan can enter the
  irreversible region. No commit test or type boundary proves construction is
  planner-only.
- **Spawn/drop side effects — SURVIVES AS UNOBSERVABLE.** The patch defines a
  `StagedPlugin` value but no staging function. It cannot prove unload on
  second-plan rejection, and it never exercises a PTY/plugin channel.
- **Caller-visible rejection — SURVIVES.** `cli_exit_status()` returning 1 in
  a utility test does not make a CLI process exit nonzero, and serialization
  round-trip does not establish plugin event delivery or exactly-once
  semantics.
- **Wrong base/apply drift — DOES NOT SURVIVE.** Both controls failed closed
  before compilation, with cleanup complete.
- **Zaphod runtime consumption — DOES NOT SURVIVE.** Protected paths,
  metadata, installer bytes, ordinary suites, and executable inventory all
  confirm that the kit is inert and no fork is selected.

## Demo script and outcome

There is no interactive AC for this inert deliverable, so no live demo was
run and no patched binary was installed or selected. A current live script
cannot be executable: the promised `override-layout --transactional --tab-id`
CLI does not exist in the patch series.

The later activation gate must first identify an upstream release exposing the
actual command/event contract, or record an explicit captain-approved fork
gate. Only then may it pin the exact binary and run this evidence sequence in
a fresh isolated config/data root:

1. Create N=1, N=2 stacked, and N=3 mixed-split tabs; save `list-panes --json`
   plus each terminal child PID and `dump-layout` before invoking the released
   transactional command against the stable tab ID.
2. Require `Applied` with the matching nonce/tab, identical terminal IDs and
   live PIDs, exactly one new rail, and valid non-overlapping base geometry.
3. Repeat N=2 with 79+79 geometry; require a nonzero CLI exit and typed
   `Rejected`, byte-equivalent pane/tab/base/swap snapshots, and zero new or
   closed PIDs/plugins.
4. Hold plugin staging, change the target geometry and then switch focus;
   require rejection/unload for the stale plan and no mutation of either tab.
5. Kill all disposable sessions and prove the isolated roots are removed.

Until the actual upstream surface exists, exact executable commands belong to
that activation entity; inventing them here would falsely claim a shipped
fork. Demo outcome for this gate: **not applicable, offline deliverable
rejected before any activation gate**.
