# Validation: Safe managed-tab entry and guarded keybindings

Entity: `docs/agent-rail-dev/.spacedock-state/zellij-managed-tab-controller.md`
Implementation worktree: `.worktrees/zellij-new-tab-entry`
Raw candidate SHA: `d5137e602dcef91a721852bffa099bb21380102c` (clean before
validation; no candidate files were changed by this validator).

## Gate recommendation

**REJECTED — return two confined tmux-smoke proof repairs to implementation.**
The end-value mechanism has successful real runs, but AC-O2 is not
reproducible yet: an unmodified repeated smoke observed a native rail-state
change unrelated to the literal toggle between its before and after snapshots.
AC-O3 also captures its visible foreign-tab evidence without comparing it.
Do not spend CL's normal-consent drill time until the offline proof is stable.

## Offline AC verdicts

| AC | Verdict | Independent evidence |
|---|---|---|
| AC-O1 | PASS in successful runs | From the selected worktree, `./tests/zellij-new-tab-test.sh` passed 8/8. Fresh real tmux/Zellij smoke runs observed literal `Alt Shift z` add exactly one active `zaphod` tab, with the candidate `file:` WASM URL in both native pane state and `dump-layout`. |
| AC-O2 | **REFUTED** | An unmodified repeated `./tests/zellij-tmux-smoke-test.sh` run failed at `tests/zellij-tmux-smoke-test.sh:346-349`: the candidate rail's otherwise-normalized native state changed only from `is_selectable: true` to `false` after literal `Alt /`. This breaks the criterion's unchanged pane/process/focus identity proof. A subsequent unmodified 10-run control passed, so this is an intermittent readiness race, not a deterministic semantic result. |
| AC-O3 | **REFUTED as the full criterion** | Successful real smoke runs sent the literal foreign-tab `Alt /` only after the observed managed route; normalized native pane state and dumped layout were byte-equal, and candidate count remained one. But the harness captures `foreign-before.screen` and `foreign-after.screen` at :362-365 and never compares or otherwise asserts their visible invariant at :366-376, so a terminal-visible-only foreign effect could pass. |
| AC-O4 | PASS for executed paths | Successful smoke preserved standing config/layout hashes and removed its tmux server, Zellij session, and root. Independent forced-build failure left no new `/tmp/zs.*` root; a TERM sent after the dedicated tmux server was live exited 143, removed that server/root, and preserved both standing hashes. A detached no-pregrant probe removed the disposable permission cache before both attaches; the smoke failed rather than falsely passing, then removed session `zs81556`, tmux server `zs81556`, and `/tmp/zs.svFKAR` while retaining its isolated standing sentinel hashes. |

Supporting reruns at the raw SHA: `cargo test --release` 134/134;
`cargo check --tests --release` clean; one fresh `zellij-new-tab` shell suite
8/8; successful tmux smoke runs exercised all four offline paths. `git diff
--check` was clean before validation. A host ENOSPC event aborted one attempted
repeat before `mktemp`; it is reported as environment noise and is not used as
candidate evidence.

## Reproduced failure and root-cause evidence

The failed native diff was:

```diff
-    "is_selectable": true,
+    "is_selectable": false,
FAIL: managed Alt / replaced a pane, process, focus, or candidate identity
```

The smoke accepts the candidate as soon as its URL exists
(`tests/zellij-tmux-smoke-test.sh:310-312`), then snapshots it at :312 and
presses `Alt /` at :336. In the rail, permission is requested on first render
(`src/main.rs:630-646`). `PermissionRequestResult(Granted)` later calls
`set_selectable(false)` and requests the runtime route (`src/main.rs:429-438`,
`:699-720`); literal key delivery is correctly rejected until that grant is
present (`src/main.rs:610-627`, `:1075-1087`). Thus the pre-grant cache avoids
human input but does not establish that the plugin has processed its grant
event before the test's baseline. A delayed-baseline control passed, which is
consistent with this ordering; it is not accepted as the proof.

## Required narrow repair and revalidation

Do not change the managed-tab architecture, restore the optimistic route flag,
or touch 7h/4d/custom PTY/lease code. In the existing worktree only:

1. Make the real smoke wait for an observable, stable post-grant resident
   before the AC-O2 baseline: candidate URL, active tiled 28-column shape,
   no visible permission prompt, and the native `is_selectable: false` state
   produced by the grant handler. Do not replace the literal key with a helper
   call or weaken the after-key identity comparison.
2. Assert AC-O3's retained foreign tmux screen evidence: compare normalized
   before/after captures or assert an equally stable visible invariant that
   rejects a candidate launch or dock/layout transition. Keep the existing
   native byte-equality checks; they are not a substitute for the visible
   proof.
3. Add the smallest regressions around those two smoke conditions, then rerun
   `./tests/zellij-new-tab-test.sh`, `cargo test --release`,
   `cargo check --tests --release`, and at least ten consecutive unmodified
   `./tests/zellij-tmux-smoke-test.sh` runs. Retain the per-run exit status and
   any native diff on failure.
4. Repeat the failure/TERM cleanup probes and preserve the standing-file hash,
   absent tmux server, absent Zellij session, and absent temporary-root result.

## Detached refutation audit

A throwaway detached checkout at the same raw SHA,
`/tmp/zaphod-fp-refutation-d5137e6`, was used rather than the implementation
worktree. Static attacks found that persistent `Alt /` is `NoOp`, the runtime
route targets an already-running `MessagePluginId`, and a received pipe still
requires granted permission, `Keybind` source, a tiled rail, and matching live
tab (`src/main.rs:607-627`, `:699-720`, `:1047-1087`). The successful real
smoke separately exercises that route before the foreign-tab no-op
(`tests/zellij-tmux-smoke-test.sh:329-376`). No broader routing or foreign-tab
semantic hole was found. The surviving proof attacks are the AC-O2 baseline
ordering and the absent AC-O3 screen assertion above; together they reject the
offline gate.

The detached audit also removed its disposable HOME permission cache immediately
before both Zellij attaches. The smoke correctly failed instead of treating the
ungranted rail as ready, and its cleanup removed the temporary root, tmux
server, and Zellij session while the audit's standing config/layout sentinels
remained byte-identical. This is cleanup evidence only; it does not substitute
for CL's ordinary permission decision in AC-I1.

## Captain-live drill

**Held.** AC-I1 deliberately remains unrun. After the repeated offline packet
is green, the drill must use a disposable config/data/socket root and a real
tmux-hosted attached Zellij client: run the selected worktree's entry command,
restart so the native binding is loaded, press literal `Alt Shift z`, approve
the ordinary prompt without injected consent, press literal `Alt /`, save
native before/after pane+layout snapshots, switch to a foreign tab, and prove
its literal `Alt /` snapshot remains unchanged. The pre-granted smoke does not
settle this AC.

## Subspace review instruction

When implementation returns with the repeated offline packet, review this
single artifact in `subspace-tui --gate-review` and persist its feedback log as
`gates/zellij-managed-tab-controller-validation.decisions.jsonl`. The human
fold—not chat prose—selects approval, revision, or rejection.
