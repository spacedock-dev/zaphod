# Validation: Safe managed-tab entry and guarded keybindings

Entity: `docs/agent-rail-dev/.spacedock-state/zellij-managed-tab-controller.md`
Implementation worktree: `.worktrees/zellij-new-tab-entry`
Independently tested candidate: `fabfc73d78e5f9ad6cbc96111f0121bf6c61e041`
(clean before and after validation).

## Gate recommendation

**Offline acceptance is green. Sprint 1 is ready for CL's short,
ordinary-consent live drill; it is not done until CL completes that drill.**

The two prior proof gaps are closed without changing product architecture:

1. The positive-key baseline now waits for the visible, active, tiled,
   28-column candidate after its pre-granted permission result has settled
   (`is_selectable: false`, no prompt). Ten consecutive real smokes passed.
2. The post-route foreign-tab capture is now compared byte-for-byte, in
   addition to the native pane/layout comparisons. A deliberately changed
   screen self-test was rejected at that exact assertion.

No 7h/4d code, ProfileLease, controller, custom PTY, or lease work was used.

## Offline AC verdicts

| AC | Verdict | Fresh independent evidence |
|---|---|---|
| AC-O1 | PASS | All ten literal-key smokes added exactly one active `zaphod` tab from `Alt Shift z`; native pane inventory and `dump-layout` contained the candidate worktree's canonical WASM URL, not the stale fixture URL. |
| AC-O2 | PASS | The settled-resident predicate required active tiled/non-suppressed candidate, 28 columns, `is_selectable: false`, no prompt, and a visible rail before literal `Alt /`. All ten runs changed the known rail 28 → 1 columns while normalized identity, command, focus, and URL stayed equal; layout and visible screen changed. |
| AC-O3 | PASS | After the observed managed route, all ten runs sent literal `Alt /` on the sidebar-less foreign tab. Native pane/layout snapshots and the visible tmux screen were byte-equal, and candidate count stayed one. |
| AC-O4 | PASS | Each green run's trap verified session, dedicated tmux server, temporary root, and standing config/layout hashes. A no-pregrant failure probe also left no named root, tmux server, or Zellij process. |
| AC-I1 | HELD FOR CL | The headless disposable pre-grant is intentionally not ordinary consent. The exact attached-session drill below is the remaining acceptance activity. |

## Fresh command packet

From `.worktrees/zellij-new-tab-entry` at the raw SHA above:

```bash
./tests/zellij-new-tab-test.sh
cargo test --release
cargo check --tests --release
for i in $(seq 1 10); do ./tests/zellij-tmux-smoke-test.sh || exit $?; done
```

Results: entry shell suite 8/8; release Rust tests 134/134; release
test-check clean; real tmux/Zellij smoke 10/10. `git diff --check` was
clean and the candidate remained at the raw SHA.

## Refutation audit

The audit used a disposable detached checkout at the same SHA and never
modified the candidate worktree. Its shared-`target` symlink initially
failed the pre-grant: the audit's textual symlink path differed from the
physical raw WASM path that Zellij uses as its permission-cache key. That is
an **audit-fixture-only** distinction; the candidate's ordinary target path
has no symlink and all ten candidate smokes passed.

With the detached audit's cache keyed to the physical raw path, it reached the
foreign check. Appending one sentinel line only to
`foreign-after.screen` produced:

```text
FAIL: post-route foreign Alt / visibly changed the tmux client
```

and its `/tmp/zs.*` root was removed. A separate deliberately wrong
no-pregrant key failed closed at the settled-resident wait and left no
`/tmp/zaphod-fp-no-pregrant-root`, named tmux server, or named Zellij
process. These are assertion/cleanup probes, not product changes.

## Captain manual drill — actual WORK session

This is the usable end-value path. It creates a fresh tab and never retrofits
the tab from which it is run.

```bash
/Users/clkao/git/zaphod/.worktrees/zellij-new-tab-entry/scripts/zellij-new-tab.sh \
  --session WORK --name Zaphod
```

1. The command prints `TAB_ID=` and a `WASM_URL=` containing
   `.worktrees/zellij-new-tab-entry`. A fresh **Zaphod** tab becomes active.
2. In that tab, approve the ordinary Zellij permission prompt using its
   visible consent control. Do not inject consent keys. Wait until the rail
   displays its normal content instead of the prompt.
3. Press **Alt + /** once. The visible rail collapses from its normal dock to
   the one-column sliver; press it once more to restore the dock. No pane is
   created or replaced.
4. Click the existing **Chaplin** tab. Press **Alt + /** there. Nothing should
   move, open, focus, or change; in particular no Zaphod rail appears there.
5. Return to **Zaphod** and press **Alt + /** once more. It should still
   toggle the existing managed rail.

Optional identity check while Zaphod is active:

```bash
zellij --session WORK action list-panes --json --all --command --geometry --state --tab |
  jq -r '.[] | select(.is_plugin and .tab_name == "zaphod") |
    [.plugin_url, .pane_columns, .is_selectable] | @tsv'
```

The script-created tab and its `Alt /` behavior work in the running
session now. The script also writes the native `Alt Shift z` fresh-tab
binding for this configuration, but Zellij reads persistent keybindings at
server start. Do **not** restart WORK merely to prove it. At the next
ordinary restart, press **Alt Shift z** from any tab: it must create another
fresh Zaphod tab and leave the originating tab unchanged.

## Subspace review instruction

After CL reports the live drill result, present this one artifact with:

```bash
subspace-tui gates/zellij-managed-tab-controller-validation.md --gate-review \
  --log gates/zellij-managed-tab-controller-validation.decisions.jsonl
```

The emitted fold—not chat prose—selects completion, revision, or rejection.
