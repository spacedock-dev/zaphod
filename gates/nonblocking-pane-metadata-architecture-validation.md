# Validation: Remove synchronous pane metadata calls from the plugin hot path

Frozen code head: `45719f4aaf545213b2a623871892ec87bde800ca`  
Merge base: `999ba8ab06af8c09a736aed98db21c0d70e341a0`  
Implementation worktree: `.worktrees/spacedock-ensign-nonblocking-pane-metadata-architecture`

The implementation worktree was clean before and after validation. Product
files were not changed. A detached shared clone under the assigned worktree
was used only for negative correctness tests and was removed afterward.

## Recommendation

**OFFLINE PASS — PRESENT AC-I1 TO THE CAPTAIN.** AC-O1 through AC-O6 pass
independent reproduction. The interactive floating-TUI observation remains
pending for the captain and is not inferred from the offline harnesses.

## Frozen review integrity

- Worktree `HEAD` is the required `45719f4aaf545213b2a623871892ec87bde800ca`;
  `merge-base(main, HEAD)` is `999ba8ab06af8c09a736aed98db21c0d70e341a0`.
- Quick synthesis `182` names panel `quick`, exact head `45719f4`, status
  `done`, verdict `P`, and retry count zero.
- Synthesis `188` names `code_completion`, exact range
  `999ba8a..45719f4`, status `done`, verdict `P`, and retry count zero.
- Required members appear exactly once: correctness `185`, journey `186`,
  and proof `187`. Each covers the same exact range, is `done/P`, and has
  retry count zero. The parent output is `No issues found.`

## Offline AC verdicts

| AC | Verdict | Independently reproduced evidence |
|---|---|---|
| AC-O1 | PASS | `tests/zellij-pane-metadata-congestion-test.sh` passed the exact `SMOKE_SECOND_ROW` focus through the fixed 28-column rail while the six-second native metadata barrier remained held. Literal `Alt p` x3, `Alt n`, `Alt 1`, and `Alt 2` reached complete native state within one second; early-release, expired-barrier, late-observation, and action-timeout controls failed visibly; the six-second quiet window stayed unchanged. |
| AC-O2 | PASS | Fresh `cargo test` passed 80/80. The executable source/permission assertion found zero command, CWD, or scrollback host calls and no `ReadPaneContents`; timer, `PaneUpdate`, render, click, key, and pipe matrices passed exact row, focus, and fail-close outcomes. `cargo check --tests` passed. |
| AC-O3 | PASS | Fresh uncached `go test -count=1 ./...` and `go vet ./...` passed. Coordinator tests covered 100-request coalescing, two-worker/one-publisher bounds, one pending refresh/snapshot, supersession, cancellation, ignored cancellation until deadline, exact cache pruning, timeout, and no late publication. |
| AC-O4 | PASS | `tests/zellij-two-rail-recipient-smoke-test.sh` passed two same-CWD manual watchers, exact `1/1/0`, exact row focus, child/history/foreign exclusion, bounded exact-ID fetch, zero idle native polls, restart-empty, later fresh SessionStart recovery, manifest fail-close, and no durable authority record. |
| AC-O5 | PASS | Rust atomic snapshot/projection matrices passed malformed UTF-8/JSON, duplicate identity, exact and over-limit size, unhealthy source, stale lease, old generation, repeated/out-of-order sequence, retained last-good data, missing/moved/suppressed/unselectable panes, and one-shot healthy recovery. Go outage/recovery tests passed uncached. |
| AC-O6 | PASS | Congestion failure injection proved timeout evidence plus absent disposable Zellij session, tmux server, and root. `tests/zellij-stress-evidence-test.sh` passed forced failure, native hang with inner deadline, vanished startup, and explicitly inconclusive cleanup-probe cases while preserving standing config/layout bytes. |

## Local negative correctness audit

The detached checkout was reset to the frozen SHA before each mutation and
the implementation worktree was never edited.

- **Forbidden permission regression:** restoring `ReadPaneContents` made
  `forbidden_pane_metadata_host_apis_are_absent_from_runtime` fail with exit
  101. The incorrect permission cannot pass the executable proof.
- **Pre-diff hot-path regression:** adding `get_pane_cwd` to the timer handler
  made the same source assertion fail with exit 101. The old synchronous
  behavior cannot silently return.
- **Identity false positive:** accepting any present row instead of the exact
  pane made the full Rust suite fail the absent/stale and lease projection
  matrices. A wrong live pane cannot authorize the session.
- **Identity false negative:** inverting exact pane equality made the focused
  lifecycle test fail immediately (`visible_sessions` 0 instead of 1).
- **Indexing path:** replacing the iterator join with `rows[0]` made the full
  suite panic on the empty-manifest lifecycle case; the committed iterator
  implementation and 80/80 suite remain panic-free.
- **Caller timing and cleanup:** the congestion wrapper's early release,
  owner-timeout, post-observation deadline, and injected action-timeout cases
  all rejected incorrect UI/timing state and cleaned their disposable roots.

No negative local UI-state, timing, identity, lifecycle, or cleanup case
survived. The detached checkout and its logs were removed after recording
the results.

## Exact captain-live AC-I1 script

The captain drives the keys and TUI. The validator records native state; this
offline report does not claim those observations.

### 1. Cheap preflight in a control terminal

```bash
set -euo pipefail
WT=/Users/clkao/git/zaphod/.worktrees/spacedock-ensign-nonblocking-pane-metadata-architecture
cd "$WT"
test "$(git rev-parse HEAD)" = 45719f4aaf545213b2a623871892ec87bde800ca
test "$(git merge-base main HEAD)" = 999ba8ab06af8c09a736aed98db21c0d70e341a0
test "$(zellij --version)" = 'zellij 0.44.3'
./build.sh
./tests/zellij-pane-metadata-congestion-test.sh
```

The preflight must print `PASS: native metadata congestion preserves pane/tab
responsiveness and quiet cleanup`. Do not spend captain time if it fails.

### 2. Start services and one fresh managed tab

```bash
set -euo pipefail
WT=/Users/clkao/git/zaphod/.worktrees/spacedock-ensign-nonblocking-pane-metadata-architecture
cd "$WT"
AGENTSVIEW_URL=http://127.0.0.1:8080
agentsview serve status
until curl -fsS "$AGENTSVIEW_URL/api/v1/sessions?limit=1" >/dev/null; do sleep 2; done
./scripts/zellij-new-tab.sh --session WORK --name 'Task 91 captain live' \
  --agentsview-url "$AGENTSVIEW_URL" | tee /tmp/task91-captain-entry.out
```

Record the printed `TAB_ID`, exact permission command, and watcher command.
Approve the native permission prompt if shown. The new rail must be exactly 28
columns wide and the selected terminal must remain the direct-entry terminal.

### 3. Start the manual watcher and a real top-level Codex session

In the selected direct-entry terminal:

```bash
set -euo pipefail
DEMO_ROOT="$(mktemp -d /tmp/zaphod-task91-captain.XXXXXX)"
export DEMO_ROOT
export ZAPHOD_METADATA_BARRIER_ENABLE="$DEMO_ROOT/metadata-enable"
export ZAPHOD_METADATA_BARRIER_ENTERED="$DEMO_ROOT/metadata-entered"
export ZAPHOD_METADATA_BARRIER_RELEASE="$DEMO_ROOT/metadata-release"
printf 'DEMO_ROOT=%s\n' "$DEMO_ROOT"
./target/zaphod watch-tab
codex
```

If Codex first asks to trust the hook, trust
`scripts/zaphod-codex-session-hook.sh`, exit that process, and start a new
Codex process in the same watched terminal. Send:

```text
TASK91_CAPTAIN_REAL: inspect README.md without edits, then wait.
```

The rail must show exactly that top-level session and clicking its delivered
row must focus this exact terminal. An `unbound` row, child/history row, or a
row in another tab fails.

### 4. Open a real Subspace TUI beside the watched terminal

CL presses literal `Alt n`, then in the new pane runs:

```bash
subspace-tui present --transport direct -- --advisory \
  --actor person:reviewer --approver agent:invoking-session \
  /Users/clkao/git/zaphod/docs/agent-rail-dev/.spacedock-state/gates/nonblocking-pane-metadata-architecture.md
```

Keep the TUI open. The original terminal remains the watched exact Codex pane;
the Subspace pane is the real slow/non-shell neighbor and must not acquire the
session row.

### 5. Hold enrichment and perform the measured actions

In a control terminal, record baseline state and enable the next exact refresh:

```bash
TAB_ID="$(sed -nE 's/^TAB_ID=([0-9]+)$/\1/p' /tmp/task91-captain-entry.out | tail -1)"
DEMO_ROOT=<value printed/exported in the watched terminal>
date +%s%3N > "$DEMO_ROOT/actions.started-ms"
zellij --session WORK action list-panes --json --all --command --geometry --state --tab \
  > "$DEMO_ROOT/before.json"
touch "$DEMO_ROOT/metadata-enable"
until test -s "$DEMO_ROOT/metadata-entered"; do sleep 0.05; done
test ! -e "$DEMO_ROOT/metadata-release"
```

While the barrier is still held, CL clicks the delivered session row, presses
literal `Alt p` three times, `Alt n`, `Alt 1`, and `Alt 2`. After each action,
the validator immediately captures `list-panes --json --all --command
--geometry --state --tab` and a millisecond timestamp. Each action must reach
the intended exact focus/pane/tab state within 1000 ms from before key
delivery. Every capture must still show the original rail at width 28 and the
barrier must still lack `metadata-release` and `metadata-entered.completed`.

The exact row click must focus the registered pane ID, not the Subspace pane.
The three pane actions must create the expected distinct live terminal IDs;
`Alt n` must create one complete tab; `Alt 1` and `Alt 2` must activate the
corresponding stable tab. Any wrong pane, incomplete lifecycle tuple, late
observation, early barrier completion, or width change fails AC-I1.

### 6. Release, restart-empty, recover, and prove quiet cleanup

```bash
touch "$DEMO_ROOT/metadata-release"
until test -s "$DEMO_ROOT/metadata-entered.completed"; do sleep 0.05; done
```

Back in the watched terminal, exit Codex, stop the PID printed by `watch-tab`, and start
`./target/zaphod watch-tab` again. The rail must publish empty and remain empty
until a new top-level Codex process emits a fresh matching SessionStart; that
fresh process must restore exactly one focusable row.

Before closing the disposable tab, resolve and close the exact rail, wait six
seconds, and capture native state:

```bash
zellij --session WORK action list-panes --json --all --command --geometry --state --tab \
  > "$DEMO_ROOT/before-rail-close.json"
RAIL_ID="$(jq -er --argjson tab "$TAB_ID" \
  '[.[] | select(.tab_id == $tab and .is_plugin and (.plugin_url | endswith("/zellij-sidebar.wasm")))] | if length == 1 then .[0].id else error("expected one exact rail") end' \
  "$DEMO_ROOT/before-rail-close.json")"
zellij --session WORK action close-pane --pane-id "$RAIL_ID"
sleep 6
zellij --session WORK action list-panes --json --all --command --geometry --state --tab \
  > "$DEMO_ROOT/after-quiet.json"
```

No delayed pane, tab, focus, or snapshot delivery may appear during that
quiet window. Then close the remaining disposable panes/tabs, stop the
replacement watcher, remove `DEMO_ROOT`, and confirm no task-91 tmux/Zellij
helper remains. Standing `~/.config/zellij/config.kdl` and
`~/.config/zellij/layouts/zaphod.kdl` hashes must match their preflight values.

## Interactive outcome

**PENDING CAPTAIN.** AC-I1 has not been run or claimed by this validator. The
offline packet is green and the exact live script is ready for the captain.

## Live setup checkpoint — 2026-07-18

The captain authorized setup through the first human observation. Validation
reran the exact frozen-head preflight: head `45719f4`, merge base `999ba8a`,
Zellij `0.44.3`, `build.sh` PASS, and the full metadata-congestion wrapper
PASS.

Because the standing default AgentsView path was inaccessible to this worker,
the validator started v0.37.5 with the explicit disposable data root
`$WT/.task91-agentsview`; its initial 2,620-session sync completed and
`http://127.0.0.1:8080/api/v1/sessions?limit=1` is healthy. No standing
AgentsView path was changed.

Fresh managed stable tab `4` is named `Task 91 captain live`. Native setup
state is retained at `/tmp/task91-captain-native-before.json` and records
candidate rail pane `plugin_33` at `x=0,y=1,28x58`, selected terminal pane
`26` at `x=28,y=1,210x58`, and intact `238x1` tab/status bars. Entry output is
at `/tmp/task91-captain-entry.out`; it records recipient token
`zaphod-24730-169-1784385683`, watcher root
`/tmp/zaphod-watch-tab-v1-501`, the exact permission command targeting
`plugin_33`, and the candidate `target/zaphod watch-tab` command.

Standing KDL hashes before and after setup are identical:
`config.kdl` `398ff6d6…be316` and `layouts/zaphod.kdl`
`bb9e8e21…3980e`. AC-I1 remains pending.

**Captain cue:** open the `Task 91 captain live` tab and only look. Report
PASS if one fixed 28-column rail is visible at left with one selected terminal
to its right; report FAIL with what differs. Also report whether a permission
prompt is visible. Do not approve the prompt or start the watcher yet.

## Live visual checkpoint 1 — captain PASS

The captain reported PASS: one fixed 28-column rail is visible at left and one
selected terminal is visible to its right. The captain did not report whether
a permission prompt is visible, so validation does not infer that observation.

Native checkpoint `/tmp/task91-captain-native-checkpoint-1.json` still records
stable tab `4`, candidate rail `plugin_33` at `x=0,y=1,28x58`, selected
terminal `26` at `x=28,y=1,210x58`, and intact 238-column chrome. The rail is
now nonselectable, which is consistent with the plugin's granted-state
behavior but does not prove what the captain can see. AgentsView remains
healthy and standing KDL hashes remain byte-identical.

**Captain cue:** is a permission prompt visible right now? Reply only `YES` or
`NO`; do not press a key, approve anything, or start the watcher yet.

## Live permission checkpoint — captain YES

The captain reported `YES`: a native permission prompt is visibly present.
The captain also explicitly accepted the visible `FIXED` label as a
non-blocking follow-up, not a task-91 rejection. Follow-up entity
`1zhcvrj8727eez45mdj6ec3j` owns removing that debug-only label without changing
behavior; task 91 remains in validation.

Native state remains retained at
`/tmp/task91-captain-native-permission-visible.json`; AgentsView is healthy and
standing KDL hashes remain byte-identical. These facts do not replace the
captain's prompt observation.

**Captain cue:** approve the visible native permission prompt exactly once
using its displayed approval control. Expected result: the prompt closes and
the same `Task 91 captain live` tab returns with the fixed 28-column left rail
and selected terminal to its right; no pane or layout moves. Report PASS or
FAIL, then stop without starting the watcher.
