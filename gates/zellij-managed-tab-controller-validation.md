# Validation: Safe managed-tab entry and guarded keybindings

Entity: `docs/agent-rail-dev/.spacedock-state/zellij-managed-tab-controller.md`
Implementation worktree: `.worktrees/zellij-new-tab-entry`
Raw candidate SHA: `d5137e602dcef91a721852bffa099bb21380102c` (clean before
validation; no candidate files were changed by this validator).

## Gate recommendation

**REJECTED — return two confined tmux-smoke proof repairs to implementation.**
The fresh managed-tab journey has successful real runs, but AC-O2 is not
reproducible yet: an unmodified repeated smoke observed a native rail-state
change unrelated to the literal toggle between its before and after snapshots.
AC-O3 also retains but does not assert its visible unrelated-tab safety
evidence. Do not spend CL's normal-consent drill time until the offline proof
is stable.

## Why the unrelated-tab check belongs in this journey

The product path is only a **fresh Zaphod-managed tab**. An existing tab is
never adopted, retrofitted, displayed as managed, or otherwise made part of
the feature.

AC-O3 is the fail-closed negative proof for that promise. Once the fresh tab
has installed its temporary runtime `Alt /` route, the attached client can
still switch to an ordinary pre-existing tab. Because Zellij's runtime
keybinding is in a shared scope, pressing the same keys there must do nothing:
no pane, focus, layout, process, or visible screen change. This protects
existing work in the session; it is not a second product flow.

## Offline AC verdicts

| AC | Verdict | Independent evidence |
|---|---|---|
| AC-O1 | PASS in successful runs | `./tests/zellij-new-tab-test.sh` passed 8/8. Fresh tmux/Zellij runs observed literal `Alt Shift z` add exactly one active `zaphod` tab, with the selected worktree's `file:` WASM URL in native pane state and `dump-layout`. |
| AC-O2 | **REFUTED** | An unmodified repeated `./tests/zellij-tmux-smoke-test.sh` run failed at `:346-349`: only the candidate rail's `is_selectable` changed `true → false` after literal `Alt /`. That breaks the unchanged pane/process/focus identity proof. A later unmodified 10-run control passed, establishing an intermittent readiness race rather than a deterministic toggle result. |
| AC-O3 | **REFUTED as visible safety evidence** | Successful runs proved byte-equal native pane/layout state and one candidate rail after literal `Alt /` on an unrelated tab. The harness captures `foreign-before.screen` and `foreign-after.screen` at `:362-365`, but `:366-376` never compares or asserts them. A terminal-visible-only mutation could therefore pass. |
| AC-O4 | PASS for executed paths | Successful smoke preserved standing hashes and removed tmux/Zellij/root. Independent forced-build failure, TERM after the tmux server was live, and a detached no-pregrant failure all cleaned their root/session/server and preserved standing sentinels. |

Supporting reruns at the raw SHA: `cargo test --release` 134/134;
`cargo check --tests --release` clean; entry shell 8/8; successful real smoke
runs covered the four offline paths. A host ENOSPC event aborted one attempted
repeat before `mktemp`; it is environment noise, not candidate evidence.

## Reproduced AC-O2 failure

```diff
-    "is_selectable": true,
+    "is_selectable": false,
FAIL: managed Alt / replaced a pane, process, focus, or candidate identity
```

The smoke discovers the candidate at
`tests/zellij-tmux-smoke-test.sh:310-312`, snapshots it immediately, and
sends its literal key at :336. The rail requests permission on first render
(`src/main.rs:630-646`). Its later
`PermissionRequestResult(Granted)` calls `set_selectable(false)` and
requests the runtime route (:429-438, :699-720); the key pipe is correctly
rejected until that grant is present (:610-627, :1075-1087). A pre-grant cache
prevents human input but does not prove the resident processed that event
before the baseline.

## Required narrow repair and revalidation

Do not change the managed-tab architecture, revive the optimistic route flag,
or touch 7h, 4d, custom PTY, or lease code.

1. Before AC-O2's baseline, wait for an observable settled resident: candidate
   URL, active tiled 28-column shape, no visible permission prompt, and native
   `is_selectable: false`. Keep the literal-key positive path and unchanged
   identity comparison.
2. Assert the retained AC-O3 unrelated-tab screens with a stable comparison or
   visible invariant that rejects a candidate launch or dock/layout change.
   Keep the existing native byte-equality checks; neither observation replaces
   the other.
3. Rerun `./tests/zellij-new-tab-test.sh`, `cargo test --release`,
   `cargo check --tests --release`, and at least ten consecutive unmodified
   `./tests/zellij-tmux-smoke-test.sh` runs. Preserve per-run exit status and
   a native diff on any failure. Repeat failure/TERM cleanup probes.

## Detached refutation audit

A detached checkout at the same SHA,
`/tmp/zaphod-fp-refutation-d5137e6`, never touched the implementation
worktree. It found persistent `Alt /` fail-closed as `NoOp`, a direct
`MessagePluginId` route to the already-running resident, and pipe guards for
granted permission, keybind source, tiled state, and active-tab match
(`src/main.rs:607-627`, :699-720, :1047-1087). No independent unsafe
unrelated-tab toggle was found. Its no-pregrant attack failed closed and
cleaned all disposable state while preserving sentinel hashes.

## Captain-live drill

**Held.** AC-I1 remains unrun until the repaired repeated offline packet is
green. The operator entry point is the existing script—not a gate-specific
bootstrap:

```bash
env ZELLIJ_CONFIG_DIR="$PROFILE_ROOT/config" ZELLIJ_CONFIG_FILE="$PROFILE_ROOT/config/config.kdl" ZELLIJ_DATA_DIR="$PROFILE_ROOT/data" ZELLIJ_SOCKET_DIR="$PROFILE_ROOT/socket" "$REPO/scripts/zellij-new-tab.sh" --session "$SESSION"
```

The script already uses these isolated roots, atomically activates its
config/layout, and creates exactly one fresh managed tab. It deliberately does
not adopt or alter an existing ordinary tab.

For the normal-consent drill, run that command once to activate the disposable
root, restart the disposable Zellij server so it loads the native bindings,
then run the same command again to create the fresh tab CL sees. CL approves
the ordinary prompt, presses literal `Alt /`, retains native+screen
before/after evidence, switches to an unrelated tab, and confirms the
fail-closed no-op there. The hotkey remains covered by AC-O1's literal
`Alt Shift z` smoke; AC-I1 may choose the stable script path.

### Current minimal drill gap

`scripts/zellij-new-tab.sh` safely consumes an existing isolated session; it
does not create or attach that disposable session. The only existing real-key
harness, `tests/zellij-tmux-smoke-test.sh`, intentionally pre-grants
permission and exits after its check. Therefore there is no maintained
normal-consent disposable-session launcher to hand to CL yet. If a one-command
captain drill is required, add only that launcher/documented mode: it must
start an attached temporary config/data/socket root without a permission cache
and then invoke `zellij-new-tab.sh`; it must not duplicate the entry
script's config transformation, layout rendering, or native NewTab logic.

## Revision response to gate feedback (round 1)

- The unrelated-tab assertion remains because it proves the fresh managed
  journey cannot mutate existing work after the shared runtime binding is
  active. It is a safety boundary, not adoption semantics.
- The hand-built captain bootstrap was removed. The drill names the existing
  entry script and its supported isolated-root inputs; the missing
  normal-consent session launcher is stated explicitly rather than recreated
  in validation prose.

## Subspace review instruction

Re-present this single revised artifact with:

```bash
subspace-tui gates/zellij-managed-tab-controller-validation.md --gate-review --log gates/zellij-managed-tab-controller-validation.decisions.jsonl
```

The human fold—not chat prose—selects approval, revision, or rejection.
