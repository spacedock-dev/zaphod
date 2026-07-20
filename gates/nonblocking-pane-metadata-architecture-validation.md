# Validation: Remove synchronous pane metadata calls from the plugin hot path

Frozen code head: `7bdb3d7a5a07b45245b37ee44d80920f673041b4`
Merge base: `999ba8ab06af8c09a736aed98db21c0d70e341a0`
Implementation worktree: `.worktrees/spacedock-ensign-nonblocking-pane-metadata-architecture`

The implementation worktree was clean before and after validation. Product
files were not changed. A detached shared clone under the assigned worktree
was used only for negative correctness tests and was removed afterward.

## Recommendation

**RECIPIENT REPAIR OFFLINE PASS — RESTART AC-I1 IN VALIDATOR TAB 7.** AC-O1
through AC-O6 pass independent reproduction at `7bdb3d7`, including the
post-ready recipient repair and passive fixed-width boundary. The interactive
floating-TUI observation remains pending for the captain and is not inferred
from the offline harnesses.

## Frozen review integrity

- Worktree `HEAD` is the required `7bdb3d7a5a07b45245b37ee44d80920f673041b4`;
  `merge-base(main, HEAD)` is `999ba8ab06af8c09a736aed98db21c0d70e341a0`.
- Synthesis `297` names `code_completion`, exact range
  `999ba8a..7bdb3d7`, status `done`, verdict `P`, and retry count zero.
- Required members appear exactly once: correctness `294`, journey `295`,
  and proof `296`. Each covers the same exact range, is `done/P`, and has
  retry count zero. The parent output is `No issues found.`

## Offline AC verdicts

| AC | Verdict | Independently reproduced evidence |
|---|---|---|
| AC-O1 | PASS | `tests/zellij-fixed-width-pane-creation-test.sh` passed the first literal `Alt p` at width 28. `tests/zellij-pane-metadata-congestion-test.sh` passed three later additions, exact `SMOKE_SECOND_ROW` focus through the fixed 28-column rail while the six-second native metadata barrier remained held, literal action deadlines, negative controls, fullscreen, and quiet cleanup. |
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

## Live permission approval action — captain confirmed

The captain confirmed they approved the visible native permission prompt
exactly once. That records the human action only; prompt closure and unchanged
layout are not inferred without the captain's explicit visual report.

Supporting native checkpoint
`/tmp/task91-captain-native-permission-approved.json` still records tab `4`,
candidate rail `plugin_33` at `x=0,y=1,28x58`, selected terminal `26` at
`x=28,y=1,210x58`, and intact chrome; standing KDL hashes remain identical.
This instrumentation does not replace the visual observation. The disposable
AgentsView daemon is no longer listening, so validation will restore it before
any watcher start after this visual checkpoint.

**Captain cue:** look at `Task 91 captain live` now. Report `PASS` only if the
permission prompt is gone and the same fixed 28-column left rail plus selected
terminal are visible with no pane or layout movement; otherwise report `FAIL`
and what differs. Do nothing else and do not start the watcher.

## Replacement-head revalidation — `b5a379f`

The preserved failed specimen was captured before retirement at
`/tmp/task91-failing-specimen-before-retire.{json,kdl}`. It records tab `4`,
rail `plugin_33` widened to 91 of 181 columns, four terminals, and dumped
`size="50%"`. Exact pane IDs `26,28,29,30,plugin_33` were then closed; no old
rail was hot-reloaded.

Stored parent `239` is `done/P` on exact range `999ba8a..b5a379f`, with
correctness `236`, journey `237`, and proof `238` exactly once at `done/P` and
retry zero. The replacement does not change `src/main.rs` or Go runtime code;
the clean isolated Rust build passed 80/80 plus `cargo check --tests`, including
zero forbidden host calls, no pane-content permission, and no runtime layout
mutation path. Fresh Go plus vet passed after isolated timing retries.

Independent live results:

- `zellij-fixed-width-pane-creation-test.sh`: first literal `Alt p` retained
  the fresh rail at exactly 28 columns.
- `zellij-pane-metadata-congestion-test.sh`: the later three literal pane
  additions retained width 28; exact session focus, fullscreen, all one-second
  pane/tab deadlines, barrier ownership, quiet window, and cleanup passed.
- `zellij-new-tab-test.sh`: all 13 entry/render/isolation cases passed;
  rendered layout contains exactly one passive `fixed-width` swap.
- Manual permission exposure passed without auto-consent; two-rail smoke passed
  exact `1/1/0`, focus, stale retention, restart-empty, zero idle polls, and
  host-default isolation.
- In a detached local checkout, removing the passive swap made the first pane
  addition widen the rail to 80 and exit 1. Changing the swap rail to 29 made
  it render at 29 and exit 1. Both incorrect layouts were rejected by the
  exact-width journey, and the disposable checkout was removed.
- Standing KDL hashes stayed `398ff6d6…be316` and `bb9e8e21…3980e` across all
  replacement tests.

The disposable AgentsView v0.37.5 service is healthy again. Fresh managed tab
`4`, named `Task 91 captain live b5a379f`, uses candidate rail `plugin_38` at
`x=0,y=1,28x49` beside selected terminal `31` at
`x=28,y=1,153x49`; native state is retained at
`/tmp/task91-b5-captain-native-before.json`, entry commands at
`/tmp/task91-b5-captain-entry.out`, and standing hashes are unchanged.

**Captain cue:** open `Task 91 captain live b5a379f` and only look. Report
`PASS` if one fixed 28-column rail is visible at left with one selected
terminal to its right; otherwise report `FAIL` and what differs. Also report
whether a permission prompt is visible. Do not approve anything, press pane
keys, or start the watcher yet.

## Replacement live visual checkpoint — captain PASS

The captain reported both required human observations: the fresh replacement
rail is visibly 28 columns wide, and no permission prompt is visible. No
permission action is required or authorized at this checkpoint.

Supporting native state at `/tmp/task91-b5-captain-visual-pass.json` still
records tab `4`, candidate rail `plugin_38` at `x=0,y=1,28x49`, selected
terminal `31` at `x=28,y=1,153x49`, and intact chrome. AgentsView is healthy
and standing KDL hashes remain byte-identical; this instrumentation supports
but does not replace the captain's visible report.

The validator prepared the owner-held metadata barrier root
`/Users/clkao/git/zaphod/.worktrees/spacedock-ensign-nonblocking-pane-metadata-architecture/.task91-live-b5.Rkweve`.

**Captain cue:** in the selected terminal of `Task 91 captain live b5a379f`,
run this one command exactly:

```bash
env ZAPHOD_METADATA_BARRIER_ENABLE=/Users/clkao/git/zaphod/.worktrees/spacedock-ensign-nonblocking-pane-metadata-architecture/.task91-live-b5.Rkweve/metadata-enable ZAPHOD_METADATA_BARRIER_ENTERED=/Users/clkao/git/zaphod/.worktrees/spacedock-ensign-nonblocking-pane-metadata-architecture/.task91-live-b5.Rkweve/metadata-entered ZAPHOD_METADATA_BARRIER_RELEASE=/Users/clkao/git/zaphod/.worktrees/spacedock-ensign-nonblocking-pane-metadata-architecture/.task91-live-b5.Rkweve/metadata-release /Users/clkao/git/zaphod/.worktrees/spacedock-ensign-nonblocking-pane-metadata-architecture/target/zaphod watch-tab
```

Expected visible result: the command returns `watch-tab ready pid=… log=…`,
the rail stays 28 columns, and its AGENTS projection is empty before any fresh
SessionStart. Report `PASS` or `FAIL` with the exact output, then stop without
starting Codex or pressing pane keys.

## Replacement watcher attempt 1 — setup/evidence failure

The captain ran the supplied command and reported exactly:
`watch-tab daemon exited before readiness: EOF`.

The retained terminal transcript shows the long cue was entered as three
physical commands. The first standalone `env
ZAPHOD_METADATA_BARRIER_ENABLE=…` printed the environment and did not persist
the enable path into `watch-tab`; the daemon log at
`/Users/clkao/Library/Application Support/org.Zellij-Contributors.Zellij/zaphod-watch-tab.2382221432.log`
therefore reports `metadata barrier requires enable, entered, release, and
completed paths`. No barrier evidence, watcher socket, or live watcher process
was created. This immediate fail-close is a cue/setup defect, not a product
defect.

The disposable AgentsView source was also down when inspected, so it could not
have supported a successful watcher journey. The first officer restarted the
same isolated `.task91-agentsview` root; v0.37.5 PID `75707` is healthy and
`/api/v1/sessions?limit=1` returns one session. Standing KDL hashes remain
unchanged.

To remove line-wrap ambiguity, validation created and syntax-checked executable
`.task91-live-b5.Rkweve/start-watcher.sh`, which supplies all three barrier
inputs in one process environment; the completed path is deterministically
derived from the entered path by the candidate.

**Captain cue:** in the same selected replacement terminal, run exactly this
one short command:

```bash
/Users/clkao/git/zaphod/.worktrees/spacedock-ensign-nonblocking-pane-metadata-architecture/.task91-live-b5.Rkweve/start-watcher.sh
```

Expected visible result: `watch-tab ready pid=… log=…`, rail width remains 28,
and AGENTS is empty before SessionStart. Report `PASS` or `FAIL` with the exact
output, then stop without starting Codex or pressing pane keys.

## Replacement watcher retry — readiness confirmed

The captain ran the short wrapper and reported the exact successful readiness
output:

```text
watch-tab ready pid=47030 log=/Users/clkao/Library/Application Support/org.Zellij-Contributors.Zellij/zaphod-watch-tab.2525861830.log
```

The preceding terminal echo was visibly truncated to `tcher.sh`; it is recorded
as display context, not as a second command result. The captain has not yet
reported the post-readiness rail width or AGENTS projection.

Supporting native inspection confirms PID `47030` owns the candidate
`target/zaphod` binary, readiness log, and Unix socket
`/tmp/zaphod-watch-tab-v1-501/w-awdGTI5bgHd1-c25PczLhHg1Jo_QFBHbHcRKQshHZQM.sock`,
with an established connection to the isolated AgentsView endpoint. The log
binds generation `generation-00000000000000000001` to session `WORK`, tab `4`,
terminal `31`, and rail `38`.

A fresh native `list-panes --all --json` records `plugin_38` at
`x=0,y=1,28x49`, focused terminal `31` at `x=28,y=1,153x49`, and intact
181-column chrome. A targeted rail screen dump is zero bytes, supporting an
empty pre-SessionStart projection without substituting for the captain's visual
observation. Standing KDL hashes remain `398ff6d6…be316` and
`bb9e8e21…3980e`.

**Captain cue:** look at the same rail now. Report `PASS` only if it is still
visibly 28 columns wide and AGENTS is empty; otherwise report `FAIL` and what
differs. Do not start Codex or press pane keys.

## Replacement empty-state observation — captain PASS

The captain reported that no AGENTS heading is visible. This is the expected
empty state at frozen replacement head `b5a379f`, not a failure: `src/main.rs`
renders the heading and its rows only inside `if !sessions.is_empty()`. With
zero delivered sessions, the entire section intentionally has zero visual
footprint.

Supporting native state still records `plugin_38` at exactly 28 columns beside
focused terminal `31`; watcher PID `47030`, its generation socket, and its
AgentsView connection remain live. This instrumentation supports the state but
does not replace the captain's observation.

The focused terminal's current native CWD is `/Users/clkao/git/agentsview`, so
the next command explicitly changes to the frozen task worktree before starting
Codex. That ensures `.codex/hooks.json` loads the intended trusted SessionStart
hook while preserving the direct-entry shell's watcher route.

**Captain cue:** in the same focused terminal, run exactly:

```bash
cd /Users/clkao/git/zaphod/.worktrees/spacedock-ensign-nonblocking-pane-metadata-architecture && codex
```

Expected visible result: the Codex TUI opens in that same terminal and the rail
shows an AGENTS heading with exactly one top-level Codex session row. If Codex
instead shows a hook-trust prompt, report `TRUST PROMPT` and stop there. Otherwise
report `PASS`, or `FAIL` with what differs. Do not send the task prompt or press
pane keys yet.

## Recipient repair independent revalidation — offline PASS

Frozen head `7bdb3d7a5a07b45245b37ee44d80920f673041b4` and merge base
`999ba8ab06af8c09a736aed98db21c0d70e341a0` were identity-checked before and
after validation. Stored `code_completion` parent `297` is `done/P`, retry
zero, on exact range `999ba8a..7bdb3d7`; correctness `294`, journey `295`, and
proof `296` each appear exactly once at `done/P`, retry zero, with no unresolved
material finding.

Independent execution passed:

- A clean isolated Rust target with sccache disabled passed 82/82 and
  `cargo check --tests`. The new harmless-refresh positive and
  moved/floating/missing negative tests ran in the full suite alongside foreign
  token/tab/rail and exact lifecycle matrices.
- Fresh uncached `go test -count=1 ./...` plus `go vet ./...` passed the bounded
  scheduler, cancellation, cache, source, and watcher matrices.
- `zellij-two-rail-recipient-smoke-test.sh` passed exact `1/1/0`, exact row
  focus, foreign/child/history exclusion, stale retention, restart-empty,
  fresh SessionStart recovery, zero idle native polls, and host isolation.
- `zellij-fixed-width-pane-creation-test.sh` kept the first literal `Alt p` at
  exact width 28. `zellij-pane-metadata-congestion-test.sh` kept the later
  ordinary pane additions at 28, focused the delivered exact row, held native
  enrichment across every sub-one-second pane/tab action, and passed fullscreen,
  negative timing controls, six-second quiet cleanup, and owned-state removal.
- The 13-case entry suite, explicit native permission exposure, retired-host-call
  docs check, and stress-evidence suite all passed. The latter retained bounded
  evidence for forced failure, native hang, vanished startup, and inconclusive
  cleanup probes without claiming absence.

The throwaway clone attacked both sides of the recipient boundary. Restoring
unconditional recipient clearing made
`harmless_manifest_refresh_keeps_exact_snapshot_recipient_live` fail at its
post-PaneUpdate snapshot assertion with status 101. Over-retaining the recipient
alone did not bypass the downstream current-position/floating guards; after
also removing those live guards,
`moved_floating_or_missing_rail_still_disarms_snapshot_recipient` failed at its
rejection assertion with status 101. The committed narrow carry-forward plus
downstream exact-state checks survived both attacks. The clone was removed.

## Validator-owned live handoff — first captain checkpoint

AgentsView v0.38.1 PID `33168` owns disposable root
`.task91-validator-agentsview`, listens only on `127.0.0.1:18092`, and serves
the exact-session API successfully. It is independent of implementer PIDs
`68294` and `96455`.

Stable proof tab `6` contained tiled rail `plugin_44` at `x=0,y=1,28x49`
beside selected terminal `36` at `x=28,y=1,153x49`. Validator watcher
generation 1, PID `48779`, accepted one
exact SessionStart after an ordinary second terminal was created and remained
live with its socket and healthy exact source for seven seconds beyond the old
five-second acknowledgment timeout. The rail stayed 28 columns. The extra
terminal and generation-1 watcher were then removed.

That tab was not used for captain handoff. After watcher restart, validation
renamed the tab post-readiness; a later source revision then failed with the
exact retained log:

```text
metadata delivery failed phase=post-ready revision=166 session=WORK tab=6 rail=44 socket=/tmp/zaphod-watch-tab-v1-501/w-DYEBPjdEYsYwX2L_ugtK4_pa56RLmnEU6d99Tv9Eh1o.sock: pipe timeout after 5s without recipient acknowledgment: kind=metadata-snapshot
```

The failed watcher PID `64271` exited and removed its socket while the source
remained HTTP 200 and the rail stayed 28 columns. Because that setup also reused
the file URL loaded by prior live heads and introduced a post-ready rename not
present in the required harmless-PaneUpdate proof, it cannot establish either
the current-head handoff or a current-head product rejection. Validation
retired tab `6` and preserved the log at
`/Users/clkao/Library/Application Support/org.Zellij-Contributors.Zellij/zaphod-watch-tab.1289599368.log`

Validation copied the frozen artifacts byte-for-byte into a unique checkout
path. The unique WASM SHA-256 is
`64140a1249c5d53c632abfd9ffe5e1f5feca8062b04fd928a9b547043649ee64`;
the native binary SHA-256 is
`26b0c63e590131255e8499349d8961d7de41355642c5c14af85241b9397739be`.
Fresh stable tab `7`, renamed before watcher startup to
`Task 91 validator unique 7bdb3d7`, now contains exact unique-URL rail
`plugin_47` at `x=0,y=1,28x49` beside selected terminal `38` at
`x=28,y=1,153x49`. Its watcher launcher failed closed before readiness with
`recipient-ready timeout for stable tab 7`, the expected ungranted native
permission boundary for this new URL. No watcher or socket remains. Standing
KDL hashes remain `398ff6d6…be316` and `bb9e8e21…3980e`.

**Captain cue:** open `Task 91 validator unique 7bdb3d7` and only look. Report
`PASS` if one fixed 28-column rail is visible at left with one selected terminal
to its right; otherwise report `FAIL` and what differs. Also report whether a
permission prompt is visible. Do not approve anything, start the watcher or
Codex, or press pane keys yet.

## Unique-URL permission checkpoint — captain approval recorded

The captain reported two human facts: the native permission prompt was visible
in `Task 91 validator unique 7bdb3d7`, and they approved it exactly once. No
28-column visual result, selected-terminal cardinality, prompt-closure result,
or empty AGENTS observation is inferred from that report.

Post-approval native state records stable tab `7`, unique-URL rail `plugin_47`
at `x=0,y=1,28x58`, sole focused terminal `38` at
`x=28,y=1,210x58`, and intact 238-column chrome. No validator watcher or socket
is live, the targeted rail dump is zero bytes, and standing KDL hashes remain
`398ff6d6…be316` and `bb9e8e21…3980e`. These facts support but do not replace the
captain's view.

Validator-owned AgentsView PID `33168` has stopped and port `18092` is no longer
listening. It is not required for this visual-only checkpoint and will be
restored before any watcher startup; no watcher or Codex action is authorized
now.

**Captain cue:** look at the same tab now. Report `PASS` only if one fixed
28-column rail is visibly present at left, exactly one selected terminal is
visible to its right, and no AGENTS heading is visible. Otherwise report `FAIL`
and what differs. Do not start the watcher or Codex, or press pane keys.

## Unique-URL post-approval visual checkpoint — captain PASS

The captain clarified that current `Task 91 validator unique 7bdb3d7` passes
all three requested visual checks: the rail is visibly fixed at 28 columns,
exactly one selected terminal is visible to its right, and no AGENTS heading is
visible. This supersedes their intermediate wording about one stale AGENTS row.

That stale row was in a previous tab, not the current unique-URL tab. Native
inventory records older canonical-URL rails `plugin_38` in tab `4` and
`plugin_41` in tab `5`, both at 28 columns, and no process owns the remaining
older watcher socket. Native inspection cannot recover the stale row's exact
registered pane identity or focus authority, so validation records only the
captain's prior-tab observation; it does not claim proven last-good retention
and does not classify it as current unique-URL leakage.

Current native state still records stable tab `7`, unique rail `plugin_47` at
`x=0,y=1,28x58`, and sole focused terminal `38` at
`x=28,y=1,210x58`. Frozen unique artifacts remain SHA-256
`64140a12…ee64` for WASM and `26b0c63e…739be` for the native binary.

Validator AgentsView v0.38.1 is restored from the same disposable root as PID
`90305` on `127.0.0.1:18092`; its session endpoint is healthy. The executable
short wrapper `.task91-validator-live-7b/start-unique-watcher.sh` passes
`sh -n` and now checks that endpoint before it starts the exact tab-7 watcher.
Validation has not run the wrapper after permission approval.

**Captain cue:** in the selected terminal of the current unique tab, run
exactly:

```bash
/Users/clkao/git/zaphod/.worktrees/spacedock-ensign-nonblocking-pane-metadata-architecture/.task91-validator-live-7b/start-unique-watcher.sh
```

Expected visible result: `watch-tab ready pid=… log=…`, the rail remains fixed
at 28 columns, and no AGENTS heading appears before SessionStart. Report `PASS`
or `FAIL` with the exact command output, then stop. Do not start Codex or press
pane keys.

## Unique-URL watcher checkpoint — captain PASS

The captain ran the short health-checked wrapper and reported exact successful
output:

```text
watch-tab ready pid=18396 log=/Users/clkao/Library/Application Support/org.Zellij-Contributors.Zellij/zaphod-watch-tab.2004140966.log
```

The captain separately reported PASS for the post-watcher view: the current
unique-tab rail remains visibly fixed at 28 columns and AGENTS remains empty
before SessionStart. This human result is not inferred from native geometry.

Independent support confirms PID `18396` owns the byte-verified unique native
binary, holds Unix socket
`/tmp/zaphod-watch-tab-v1-501/w-qigSzMFVOGxOc7KEKsze3iSRYwSHcVCjIn-Cq9zCj0c.sock`,
and has an established connection to validator AgentsView on port `18092`.
The readiness log binds generation `generation-00000000000000000002` to
session `WORK`, tab `7`, terminal `38`, and rail `47`. At the final check it had
remained live for 81 seconds after the readiness log timestamp, with no
post-ready delivery error; this exceeds the former five-second acknowledgment
failure window and supports successful empty projection acknowledgment.

Native inventory still records unique rail `plugin_47` at
`x=0,y=1,28x58`, sole focused terminal `38` at `x=28,y=1,210x58`, and intact
chrome. The targeted rail dump is zero bytes. This supports, but does not
replace, the captain's visible empty-state PASS.

**Captain cue:** in the same selected terminal, run exactly:

```bash
codex
```

Expected visible result: the Codex TUI opens in terminal `38` and the rail shows
an AGENTS heading with exactly one top-level Codex row. If Codex instead shows a
hook-trust prompt, report `TRUST PROMPT` and stop there. Otherwise report
`PASS`, or `FAIL` with what differs. Do not send the task prompt or press pane
keys yet.

## Unique-URL Codex hook-trust checkpoint — expected consent boundary

The captain reported verbatim:

```text
TRRUST PROMPT
```

The spelling is preserved exactly. This is classified as the expected
first-run consent boundary for the checkout-local SessionStart hook, not as
consent. It does not establish that the hook was trusted or ran, that a
SessionStart reached the watcher, or that the rail rendered a session row.

The exact hook source is
`/Users/clkao/git/zaphod/.worktrees/spacedock-ensign-nonblocking-pane-metadata-architecture/.codex/hooks.json`;
its SessionStart command is `scripts/zaphod-codex-session-hook.sh`. Codex records
trust for the exact hook definition. Because this already-started process
skipped the untrusted SessionStart hook, it must be exited after trust and a new
Codex process must be started in the same watched terminal to exercise a fresh
SessionStart.

**Captain cue:** at the current Codex screen, open `/hooks` if the hook browser
is not already open, select only the project-local hook from the exact source
above with command `scripts/zaphod-codex-session-hook.sh`, and choose **Trust**.
Then exit that Codex process and run `codex` again in the same terminal `38`.

Expected visible result: the restarted Codex TUI opens in terminal `38` and the
rail shows AGENTS with exactly one top-level Codex row. Report `PASS`, or `FAIL`
with what differs, and stop. Do not send a task prompt or press pane keys yet.

## Unique-URL pre-prompt row checkpoint — validation ordering corrected

The captain trusted the exact checkout-local hook, exited the first Codex
process, restarted `codex` in terminal `38`, and reported FAIL because no
AGENTS section or row appeared. This human failure is recorded as observed.

Native state confirms that the restarted pane command is `codex` in the exact
unique checkout CWD. Validator-owned AgentsView on `127.0.0.1:18092` is healthy
but has zero sessions for `.task91-validator-entry-7b`, and no new Codex session
file for that CWD is visible. The captain had been explicitly told not to send
a first prompt, so AgentsView has no persisted session to return for the exact
SessionStart ID yet.

Watcher PID `18396` remains live with the exact tab-7 socket
`/tmp/zaphod-watch-tab-v1-501/w-qigSzMFVOGxOc7KEKsze3iSRYwSHcVCjIn-Cq9zCj0c.sock`
and an established connection to port `18092`. Its log still contains only the
generation-2 ready record for `WORK`/tab `7`/pane `38`/rail `47`, with no
post-ready delivery error. Hook acceptance is intentionally in-memory and no
durable SessionStart record can be inferred from these facts.

The concrete first unavailable boundary is AgentsView exact-ID enrichment, not
the native pane route or watcher readiness. This is a validation-script ordering
defect: the original gate sequence sends
`TASK91_CAPTAIN_REAL: inspect README.md without edits, then wait.` before it
expects enrichment and a visible row. The premature no-row expectation is
withdrawn and is not a product rejection.

**Captain cue:** send exactly this as the first prompt in the restarted Codex
TUI in terminal `38`:

```text
TASK91_CAPTAIN_REAL: inspect README.md without edits, then wait.
```

Expected visible result: AgentsView records that exact Codex session and the
same tab-7 rail shows AGENTS with exactly one top-level Codex row for terminal
`38`, carrying the `TASK91_CAPTAIN_REAL` identity/summary. Report `PASS`, or
`FAIL` with what differs, and stop. Do not press pane keys or open Subspace yet.

## Unique-URL marker enrichment checkpoint — AGENTS section observed

After sending the exact marker prompt, the captain reported verbatim:

```text
I see AGENTS now
```

This is a human PASS only for appearance of the AGENTS section. It does not
establish exact row count, visible row identity or summary, focusability, or
terminal binding.

Supporting source state now serves exact session
`codex:019f7ee7-672a-7a72-9726-ecf1e0a4ce35` with CWD
`.task91-validator-entry-7b`, first message
`TASK91_CAPTAIN_REAL: inspect README.md without edits, then wait.`, and agent
`codex`. The local Codex session file carries the same session ID, exact CWD,
and first user message.

Watcher PID `18396` still owns the exact tab-7 socket and has established
connections to validator AgentsView on port `18092`; its log retains the
generation-2 binding to `WORK`/tab `7`/pane `38`/rail `47` with no post-ready
error. Native inventory records unique rail `plugin_47` at `28x58` beside the
sole focused selectable terminal `38` at `210x58`; that terminal runs `codex`
in the exact unique CWD. These facts support the route but do not replace the
remaining visual observation.

**Captain cue:** look only at the AGENTS section in the current unique tab.
Report `PASS` only if it contains exactly one top-level Codex row, no additional
rows, and the visible row text identifies the `TASK91_CAPTAIN_REAL` marker;
otherwise report `FAIL` with the row count and visible text. Do not click the
row, press pane keys, or open Subspace yet.

## Unique-URL exact marker row checkpoint — captain PASS

The captain reports PASS: the current unique tab's AGENTS section contains
exactly one top-level Codex row, contains no additional rows, and the visible
row identifies `TASK91_CAPTAIN_REAL`. This is the required human row-cardinality
and marker-identity observation. Focus behavior and exact terminal binding have
not yet been exercised.

Before the click, native inventory records unique rail `plugin_47` as the
non-selectable 28-column pane in tab `7` and terminal `38` as the sole focused
selectable 210-column pane immediately to its right. Watcher PID `18396` still
owns the exact tab-7 socket and retains established validator AgentsView
connections. This instrumentation is preserved for the post-click comparison.

**Captain cue:** click the visible `TASK91_CAPTAIN_REAL` Codex row exactly once.
Report `PASS` only if focus lands on or remains on the sole terminal immediately
to the rail's right in the same unique tab—the Codex terminal `38`—with no tab
switch and no floating pane; otherwise report `FAIL` with what received focus.
Then stop. Do not click anything else, press pane keys, or open Subspace yet.
