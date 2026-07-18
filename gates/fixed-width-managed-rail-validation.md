# Validation: Keep the managed rail at one fixed width without blocking native fullscreen

Current cycle: `3`
Current repaired SHA: `c269b31bd9ada398cebb79e839580bb4f2a6b0d8`
Current recommendation: **PASSED.** O1-O4 survive independent offline
reproduction, and the captain reported I1-I2 passing in the supplied
throwaway-session walkthrough. Cycles 1-2 are preserved below; cycle 3 is
appended at the end.

Entity: `fixed-width-managed-rail.md`  
Implementation worktree: `.worktrees/spacedock-ensign-fixed-width-managed-rail`  
Cycle-1 raw implementation SHA: `2f209781c10184a768292555aea0effc8aa77145`  
Cycle-1 validation checkout: detached throwaway clone of that SHA, nested under the assigned worktree

The implementation worktree was clean at the checked SHA before validation.
No implementation file was changed during validation.

## Cycle 1 gate recommendation

**REJECTED.** O1 through O3 pass, and the stored Roborev panel is current and
unanimous, but O4 is refuted: the supposedly isolated Zellij smoke writes its
server log into the host's default `$TMPDIR/zellij-501` root. A separate
responsiveness-mode retry also exposed an intermittent fullscreen proof failure
on terminal cursor state. I1 and I2 remain unrun for the captain until the
offline packet is isolated and deterministic.

## Frozen-review integrity

- Worktree `HEAD`, reviewed head, and current head are all
  `2f209781c10184a768292555aea0effc8aa77145`; `git status --short --branch`
  was clean.
- `main` and `merge-base(main, 2f20978)` are both
  `ac0ae2a5c301052daadc2c5cca89c2bbfbf6fdcc`.
- `roborev show --job 67 --json` names panel `code_completion`, exact range
  `ac0ae2a5c301052daadc2c5cca89c2bbfbf6fdcc..2f209781c10184a768292555aea0effc8aa77145`,
  status `done`, and synthesis verdict `P`.
- Members appeared exactly once: correctness `64`, journey `65`, and proof
  `66`. Each had status `done`, verdict `P`, and `retry_count: 0`.
- No later code commit or worktree change invalidates parent `67`.

## Offline AC verdicts

| AC | Verdict | Independently reproduced evidence |
|---|---|---|
| O1 — stable fixed rail and intact chrome | **PASS** | The pre-granted and permission-upgrade 160x48 tmux smokes both reached `candidate-settled`. The native assertion required a six-pane managed-tab inventory: one exact canonical-WASM rail at `x=0,y=1,28x46`, three terminals, one `160x1` tab bar at `y=0`, and one `160x1` status bar at `y=47`. |
| O2 — former toggle inputs are layout-inert | **PASS** | Both smokes reached `fixed-inputs-inert`. Captured native JSON and normalized KDL were byte-identical across literal `Alt /`, the real column-24 `FIXED` click, and a stale toggle pipe. A same-path mouse row click changed focus to a different exact pane ID and back before the inert header assertion. Focused Rust tests passed for the inert header, granted active-tiled `PipeSource::Keybind` toggle, and permission set without `Reconfigure`. |
| O3 — native fullscreen round-trips both pane kinds | **PASS for the required ordinary packet; proof instability found adversarially** | Both required smokes reached `native-fullscreen-roundtrips-complete`: the terminal and rail each became `is_fullscreen=true` at `160x46`, then restored all managed panes with fullscreen flags false, fixed rail/chrome geometry, and normalized layout. One of five extra responsiveness-mode runs failed the terminal restore comparison because `cursor_coordinates_in_pane` changed `[8,2] -> [8,1]`; four immediate repetitions passed. This is an intermittent exact-snapshot proof defect, not observed layout loss. |
| O4 — entry, rows, permissions, recipient, and standing-root isolation | **REFUTED** | `cargo test` passed 89, `tests/zellij-new-tab-test.sh` passed 14 cases, both permission modes passed the main smoke, and `tests/zellij-two-rail-recipient-smoke-test.sh` proved exact stable-tab delivery. However, the host default `$TMPDIR/zellij-501/zellij-log/zellij.log` contains the detached candidate path and its 22:07:37 server lifecycle. The harness redirects socket/data/config but omits isolated `HOME` and `TMPDIR` from control calls (`tests/zellij-tmux-smoke-test.sh:108-126`; companion smoke `tests/zellij-two-rail-recipient-smoke-test.sh:42-50`). O4 explicitly forbids writes to the standing socket root. |

## Reproduction commands and results

All commands ran from the detached clone at `2f20978`:

```bash
env -u CARGO_TARGET_DIR cargo test
bash tests/zellij-new-tab-test.sh
ZAPHOD_SMOKE_EVIDENCE_DIR="$PWD/.validation-evidence/pregranted" \
  bash tests/zellij-tmux-smoke-test.sh
ZAPHOD_PERMISSION_FIXTURE=upgrade \
  ZAPHOD_SMOKE_EVIDENCE_DIR="$PWD/.validation-evidence/upgrade" \
  bash tests/zellij-tmux-smoke-test.sh
bash tests/zellij-two-rail-recipient-smoke-test.sh

# Adversarial scheduling/proof retry: one failure, then four passes.
ZAPHOD_SMOKE_RESPONSIVENESS_CHECK=1 \
  bash tests/zellij-tmux-smoke-test.sh
```

Results: Rust `89/89`; new-tab `14/14`; ordinary smoke `2/2`; recipient smoke
PASS. All isolated sessions, tmux servers, owned processes, sockets, and `/tmp`
smoke roots were removed by their cleanup reports. Standing config, layout,
and application-data hashes stayed byte-identical. The permission file's mtime
predated validation. The whole live cache root changed with active `WORK` and
`Spacedock` session metadata, so it is not a useful isolation oracle; the
candidate-bearing default Zellij log is the direct attribution for O4.

## Refutation audit

- **False-positive mouse coordinate — SURVIVES.** The captured screen puts
  `FIXED` on one-based row 2 and columns 23-27. Native rail `pane_y=1` therefore
  yields SGR row `pane_y + 1 = 2`; the rejected former row 3 visibly contains
  the first pane row. The positive row-click control changed the focused exact
  terminal ID and restored it, proving raw mouse delivery before the negative
  header assertion.
- **Keybind-source authority — SURVIVES.** The focused test constructs an
  active, tiled, permission-granted resident and sends `PipeSource::Keybind`
  named `toggle`; `pipe` returns false and resident state is unchanged.
  `permissions_for_config` grants neither `Reconfigure` nor any runtime route.
- **Identity/cardinality false positive — SURVIVES.** O1 requires exactly one
  non-suppressed rail whose plugin URL equals the canonical candidate WASM and
  whose stable tab ID equals the entry result; it also requires exactly three
  live terminals and both exact chrome plugins. Duplicate, wrong-URL,
  suppressed, or wrong-tab candidates cannot satisfy that assertion.
- **Geometry/chrome false positive — SURVIVES.** The native predicate pins all
  four rail coordinates, both chrome rectangles, client size, pane count, and
  normalized layout. Literal inputs are compared against full before/after
  native inventories, not screen appearance alone.
- **Fullscreen terminal-state false positive — SURVIVES for geometry; proof
  flake remains.** The wait targets exact pane IDs, requires
  `is_fullscreen=true` and `160x46`, then requires every managed pane's flag
  false and compares restored non-focus snapshots. The extra scheduler probe
  exposed nondeterministic cursor coordinates in that broad snapshot.
- **Panic/indexing paths — SURVIVES.** Rust `89/89` passed; shell helpers reject
  non-single rail/focus/fullscreen sets through explicit cardinality checks and
  bounded waits. No panic or unchecked indexing failure appeared.
- **Caller impact and semantic drift — SURVIVES.** The positive pane-row focus
  round trip passed, selected-checkout entry passed 14 black-box cases, and the
  two-rail recipient smoke delivered only to the stable-tab/token target.
- **Standing-root isolation — REFUTED.** The candidate-bearing server lifecycle
  in the host default Zellij log proves a smoke write outside `ROOT`. Setting
  `ZELLIJ_SOCKET_DIR` alone does not redirect Zellij's default log/cache roots;
  the control helpers need the same isolated `HOME`/`TMPDIR` boundary as the
  attached client, followed by an outside-root hash assertion.

## Captain demo script for I1 and I2

Do not run this until O4 is repaired and the complete offline packet is green
again. CL must drive the keys and mouse in the fresh `WORK` tab; the validator
must not claim those observations.

Cheap preflight from an ordinary pane inside `WORK`:

```bash
set -euo pipefail
WT=/Users/clkao/git/zaphod/.worktrees/spacedock-ensign-fixed-width-managed-rail
cd "$WT"
test "$(git rev-parse HEAD)" = 2f209781c10184a768292555aea0effc8aa77145
test "$(zellij --version)" = 'zellij 0.44.3'
rg -n 'bind "Alt /" \{ NoOp; \}|bind "f" \{ ToggleFocusFullscreen;' \
  /Users/clkao/.config/zellij/config.kdl
cargo test fixed_header_clicks_are_inert_without_changing_row_behavior -- --nocapture
./build.sh
./scripts/zellij-new-tab.sh --session WORK --name 'Fixed rail validation'
```

Copy `TAB_ID` from the final command, then populate and capture I1:

```bash
TAB_ID=<printed-tab-id>
DEMO=/tmp/fixed-rail-demo-$TAB_ID
zellij --session WORK action new-pane --tab-id "$TAB_ID" --direction right \
  --cwd "$WT" --name fixed-demo-right
zellij --session WORK action new-pane --tab-id "$TAB_ID" --direction down \
  --cwd "$WT" --name fixed-demo-down
zellij --session WORK action list-panes --json --all --command --geometry --state --tab \
  > "$DEMO.before.json"
zellij --session WORK action dump-layout > "$DEMO.before.kdl"
```

CL should now confirm one 28-column left rail, three arranged terminals, and
both chrome rows. Press literal `Alt /`; then click directly on the word
`FIXED`. Nothing should move, stack, disappear, change width, or prompt.

```bash
zellij --session WORK action list-panes --json --all --command --geometry --state --tab \
  > "$DEMO.after-inputs.json"
zellij --session WORK action dump-layout > "$DEMO.after-inputs.kdl"
```

For I2, focus an ordinary pane and press the captain's existing native
fullscreen sequence: current standing authority is `Ctrl-p`, then `f`.
Capture the native inventory while fullscreen and after pressing the same
sequence again. The terminal must occupy the whole content rectangle; restore
must return the 28-column rail, all three terminals, and both chrome rows.

Then focus the active rail through its supported navigation message and use
the same native sequence:

```bash
zellij --session WORK pipe --name navigate -- ""
# CL observes rail focus, presses Ctrl-p then f, and inspects fullscreen.
zellij --session WORK action list-panes --json --all --command --geometry --state --tab \
  > "$DEMO.rail-fullscreen.json"
# CL presses Ctrl-p then f again, then Esc to leave rail navigation.
zellij --session WORK action list-panes --json --all --command --geometry --state --tab \
  > "$DEMO.restored.json"
zellij --session WORK action dump-layout > "$DEMO.restored.kdl"
```

While fullscreen, the exact target must have `is_fullscreen=true`, span from
`y=1` to the row immediately above the status bar, and match the chrome width.
After each restore every managed pane must have `is_fullscreen=false`; the
rail, terminal split geometry, and both chrome rows must match the I1 baseline.
No permission prompt or new pane may appear.

## Demo outcome

Not run. Offline O4 is refuted, so I1 and I2 remain pending for CL after a
bounded implementation repair and fresh authoritative review.

## Return-to-implementation finding

Make every Zellij CLI/control invocation in both real smoke harnesses inherit
the disposable `HOME` and `TMPDIR` boundary, then prove host default config,
layout, data, socket/log, and permission/cache paths remain unchanged. Keep the
product code frozen. Also decide whether cursor coordinates belong in O3's
canonical restore snapshot; whichever contract is chosen must be deterministic
under repeated responsiveness-mode runs.

## Validation cycle 2

Repaired implementation SHA: `b9568fc8645646c15bb31878827cb86e4f1fff28`  
Validation checkout: detached throwaway clone of that SHA, nested under the
assigned worktree. The implementation worktree was clean before and after the
cycle; no implementation file was changed.

### Cycle 2 recommendation

**REJECTED.** The cycle-1 main-smoke root leak and cursor-oracle flake are
repaired: normal and upgrade smokes passed, and five consecutive
responsiveness smokes preserved every stable fullscreen field while leaving
zero candidate records in host defaults. O4 remains refuted because the
unchanged two-rail recipient smoke writes its exact detached candidate path to
the host default `$TMPDIR/zellij-501/zellij-log/zellij.log`.

### Captain-accepted review evidence

- Clean code `HEAD` and current head are
  `b9568fc8645646c15bb31878827cb86e4f1fff28`; there is no post-panel commit or
  worktree change.
- `main` and `merge-base(main, b9568fc)` are both
  `ac0ae2a5c301052daadc2c5cca89c2bbfbf6fdcc`; the frozen task range contains
  15 commits.
- `roborev show --job 130 --json` names `code_completion`, status `done`,
  synthesis verdict `P`, and zero retries. Correctness `127`, journey `128`,
  and proof `129` each appear exactly once with `done`, PASS, and zero retries.
- Parent `130` stores `c5ecf0c..b9568fc`, a 16-commit inclusive range that
  contains the full frozen task and one extra base commit `ac0ae2a`. The
  captain explicitly accepted that over-wide evidence. Validation carried the
  exception and did not rerun Roborev. Exact-head quick parent `126` also PASSed.

### Cycle 2 offline AC verdicts

| AC | Verdict | Independently reproduced evidence |
|---|---|---|
| O1 — stable fixed rail and intact chrome | **PASS** | Normal, upgrade, and five responsiveness smokes all reached `candidate-settled`. Each required the exact six-pane 160x48 inventory: one canonical candidate rail at `x=0,y=1,28x46`, three terminals, tab bar `160x1@y=0`, and status bar `160x1@y=47`. |
| O2 — former toggle inputs are layout-inert | **PASS** | All seven smokes reached `fixed-inputs-inert`: literal `Alt /`, the delivered column-24 visual-header click, and stale toggle pipe preserved native JSON and normalized KDL. Rust `89/89` includes the granted active-tiled `PipeSource::Keybind`, inert header, permission, and row-action controls. |
| O3 — native fullscreen round-trips both pane kinds | **PASS** | All seven smokes reached `native-fullscreen-roundtrips-complete`; five responsiveness runs also completed refresh deadlines and the six-second stable post-close window. Only `is_focused` and `cursor_coordinates_in_pane` are removed. Synthetic mutations of identity, pane geometry, content geometry, chrome URL, fullscreen, command, suppression, selectability, and exited state all remained detectable after normalization. |
| O4 — entry, rows, permissions, recipient, and standing-root isolation | **REFUTED** | Rust `89/89`, new-tab `14/14`, main smoke `7/7`, and stable-tab recipient delivery passed. Every main-smoke cleanup reported config/layout/candidate records unchanged and resolved log/socket/session-info/permission roots under `/tmp/zs.*`. The recipient smoke then changed the detached-path count in the host default log from `0` to `1`, despite reporting PASS. |

### Cycle 2 commands and results

```bash
env -u CARGO_TARGET_DIR cargo test
bash tests/zellij-new-tab-test.sh
ZAPHOD_SMOKE_EVIDENCE_DIR="$PWD/.validation-evidence/pregranted" \
  bash tests/zellij-tmux-smoke-test.sh
ZAPHOD_PERMISSION_FIXTURE=upgrade \
  ZAPHOD_SMOKE_EVIDENCE_DIR="$PWD/.validation-evidence/upgrade" \
  bash tests/zellij-tmux-smoke-test.sh

for run in 1 2 3 4 5; do
  ZAPHOD_SMOKE_PREBUILT_ARTIFACTS=1 \
  ZAPHOD_SMOKE_RESPONSIVENESS_CHECK=1 \
  ZAPHOD_SMOKE_EVIDENCE_DIR="$PWD/.validation-evidence/responsiveness-$run" \
    bash tests/zellij-tmux-smoke-test.sh
done

bash tests/zellij-two-rail-recipient-smoke-test.sh
```

Results: Rust `89/89`; new-tab `14/14`; main smoke normal and upgrade `2/2`;
responsiveness `5/5`; recipient delivery PASS. All seven main cleanup records
had `original_status=0`, `cleanup_status=0`, `root_exists_after=0`, and
`standing_config_unchanged=1`, `standing_layout_unchanged=1`,
`standing_candidate_records_unchanged=1`. The resolved main roots were all
under `/tmp/zs.*`, including log, socket, session-info, and permission cache.

The recipient attack bracketed its run with an exact fixed-string count of the
detached checkout path in the standing log. Count `0` became `1`; the new line
was:

```text
4356:INFO |zellij_server::plugins::p| 2026-07-17 23:37:50.868 ... Loaded plugin '/Users/clkao/git/zaphod/.worktrees/spacedock-ensign-fixed-width-managed-rail/.validation-audit-b9568fc/target/wasm32-wasip1/release/zaphod.wasm'
```

No candidate record appeared in the standing permission file, session-info
cache, data tree, or surviving socket names. One candidate-attributable host
log write is sufficient to refute O4's no-standing-root-write contract.

### Cycle 2 refutation audit

- **Main log/cache/socket/data/permission containment — SURVIVES.** Seven real
  main smokes resolved every writable root below `/tmp/zs.*`; candidate record
  counts remained zero in host defaults, and cleanup removed all owned roots,
  sockets, sessions, and processes.
- **Recipient host-root containment — REFUTED.** The two-rail script's control
  helpers provide only socket/config/data (`tests/zellij-two-rail-recipient-smoke-test.sh:42-50`),
  its version probe precedes containment (`:99`), and its server command sets
  `HOME` but not `TMPDIR` or XDG roots (`:167-170`). Its passing real server
  appended the exact candidate WASM identity to the host default Zellij log.
- **Cursor-only normalization — SURVIVES.** Five responsiveness runs passed.
  An independent synthetic attack proved focus/cursor mutations normalize
  equal while identity, outer/content geometry, chrome, fullscreen, command,
  suppression, selectability, and exited mutations remain unequal.
- **Fullscreen terminal state — SURVIVES.** Each run required the exact target
  ID at `160x46` with `is_fullscreen=true`, then all managed flags false and
  byte-identical stable normalized inventory/layout on restore.
- **Mouse/keybind/identity false positives — SURVIVES.** The same-path positive
  row click changed and restored the exact focused pane before the derived
  visual-header negative assertion; Keybind-source toggle remained inert;
  rail cardinality, URL, stable tab ID, chrome, terminal count, and geometry
  stayed exact.
- **Panic/indexing, caller impact, semantic drift — SURVIVES.** Rust `89/89`,
  new-tab `14/14`, all main journeys, and recipient stable-tab behavior passed;
  helpers retain explicit cardinality errors and bounded waits.

### Cycle 2 captain demo for I1 and I2

Do not run while O4 is red. After the recipient harness is contained and the
replacement offline packet/review passes, CL should drive this exact script
from an ordinary pane inside `WORK`:

```bash
set -euo pipefail
WT=/Users/clkao/git/zaphod/.worktrees/spacedock-ensign-fixed-width-managed-rail
cd "$WT"
test "$(git rev-parse HEAD)" = b9568fc8645646c15bb31878827cb86e4f1fff28
test "$(zellij --version)" = 'zellij 0.44.3'
rg -n 'bind "Alt /" \{ NoOp; \}|bind "f" \{ ToggleFocusFullscreen;' \
  /Users/clkao/.config/zellij/config.kdl
cargo test fixed_header_clicks_are_inert_without_changing_row_behavior -- --nocapture
./build.sh
./scripts/zellij-new-tab.sh --session WORK --name 'Fixed rail validation cycle 2'
```

Copy the printed tab ID and create the I1 baseline:

```bash
TAB_ID=<printed-tab-id>
DEMO=/tmp/fixed-rail-cycle-2-$TAB_ID
zellij --session WORK action new-pane --tab-id "$TAB_ID" --direction right \
  --cwd "$WT" --name fixed-demo-right
zellij --session WORK action new-pane --tab-id "$TAB_ID" --direction down \
  --cwd "$WT" --name fixed-demo-down
zellij --session WORK action list-panes --json --all --command --geometry --state --tab \
  > "$DEMO.before.json"
zellij --session WORK action dump-layout > "$DEMO.before.kdl"
```

CL confirms one 28-column rail, three arranged terminals, and both chrome rows;
presses literal `Alt /`; and clicks the visible word `FIXED`. Nothing moves,
stacks, disappears, changes width, or prompts. Capture the result:

```bash
zellij --session WORK action list-panes --json --all --command --geometry --state --tab \
  > "$DEMO.after-inputs.json"
zellij --session WORK action dump-layout > "$DEMO.after-inputs.kdl"
```

For I2, CL focuses an ordinary pane and uses the current native sequence
`Ctrl-p`, then `f`; the terminal must fill the content rectangle. CL presses
the same sequence to restore and confirms the I1 split/chrome baseline. Then:

```bash
zellij --session WORK pipe --name navigate -- ""
# CL observes rail focus and presses Ctrl-p then f.
zellij --session WORK action list-panes --json --all --command --geometry --state --tab \
  > "$DEMO.rail-fullscreen.json"
# CL presses Ctrl-p then f again, then Esc to exit rail navigation.
zellij --session WORK action list-panes --json --all --command --geometry --state --tab \
  > "$DEMO.restored.json"
zellij --session WORK action dump-layout > "$DEMO.restored.kdl"
```

The rail must fill the same content rectangle with `is_fullscreen=true` and
restore with every managed fullscreen flag false, the 28-column rail, original
terminal split, and both chrome rows. No permission prompt or new pane appears.

### Cycle 2 demo outcome

Not run. O4 remains refuted, so I1 and I2 stay pending for CL.

### Cycle 2 return-to-implementation finding

Apply the main smoke's complete disposable environment and candidate-scoped
standing-root attribution to `tests/zellij-two-rail-recipient-smoke-test.sh`,
including its version probe, control/session helpers, attached server, and
cleanup. Keep product code and the repaired cursor oracle frozen. Re-run the
recipient behavior plus host-root attack, obtain a replacement exact-head and
completion review, then return to validation.

## Validation cycle 3

Repaired implementation SHA: `c269b31bd9ada398cebb79e839580bb4f2a6b0d8`
Validation checkout: detached throwaway clone of that SHA, nested under the
assigned worktree. The implementation worktree remained clean and no
implementation file was changed.

### Cycle 3 recommendation

**PASSED.** O1-O4 independently pass. The main smoke is green in normal and
permission-upgrade modes; the recipient smoke preserves exact stable-tab/token
delivery in both normal and hostile inherited-environment runs; no
candidate-attributable record appears in host log, cache, socket, data,
session-info, or permission roots. The captain reported I1 and I2 passing in
the supplied throwaway-session walkthrough.

### Accepted review disposition

- Clean worktree `HEAD` and current head are
  `c269b31bd9ada398cebb79e839580bb4f2a6b0d8`; no later code change exists.
- The captain explicitly waived another authoritative `code_completion` panel
  for this bounded recipient-harness-only commit. Validation did not launch
  Roborev.
- Stored exact-head quick parent `132`, panel `quick`, is `done`, PASS, zero
  retries, and names `c269b31`. Its sole required member `131` appears exactly
  once with `done`, PASS, zero retries, and no issues.
- Product code, the main-smoke containment, and the cursor-only fullscreen
  oracle are unchanged from the previously accepted evidence.

### Cycle 3 offline AC verdicts

| AC | Verdict | Independently reproduced evidence |
|---|---|---|
| O1 — stable fixed rail and intact chrome | **PASS** | Normal and upgrade 160x48 main smokes each reached `candidate-settled` with the exact six-pane inventory: one canonical candidate rail at `x=0,y=1,28x46`, three terminals, tab bar `160x1@y=0`, and status bar `160x1@y=47`. |
| O2 — former toggle inputs are layout-inert | **PASS** | Both main smokes reached `fixed-inputs-inert`; literal `Alt /`, the delivered visual `FIXED` click, and stale toggle pipe preserved native inventory and normalized KDL. Rust `89/89` includes the granted active-tiled Keybind-source, inert-header, permission, and row-action controls. |
| O3 — native fullscreen round-trips both pane kinds | **PASS** | Both main smokes reached `native-fullscreen-roundtrips-complete`. Exact terminal and rail IDs became `is_fullscreen=true` at `160x46`; restore required all managed flags false plus identical stable identity, command, geometry, content geometry, chrome, floating/suppressed/selectable/exited state, and normalized layout. |
| O4 — entry, rows, permissions, recipient, and standing-root isolation | **PASS** | Rust `89/89`, new-tab `14/14`, normal and upgrade main smokes, and two recipient smokes passed. Recipient stable-tab/token targeting remained exact. Candidate records stayed absent in host log, permission, cache, and data; a hostile inherited Zellij identity and socket directory also remained untouched. Internal positive controls required the candidate lifecycle log, socket, permission cache, and optional session-info only beneath the disposable root. |

### Cycle 3 commands and results

```bash
env -u CARGO_TARGET_DIR cargo test
bash tests/zellij-new-tab-test.sh

ZAPHOD_SMOKE_EVIDENCE_DIR="$PWD/.validation-evidence/main" \
  bash tests/zellij-tmux-smoke-test.sh
ZAPHOD_SMOKE_PREBUILT_ARTIFACTS=1 ZAPHOD_PERMISSION_FIXTURE=upgrade \
  ZAPHOD_SMOKE_EVIDENCE_DIR="$PWD/.validation-evidence/upgrade" \
  bash tests/zellij-tmux-smoke-test.sh

bash tests/zellij-two-rail-recipient-smoke-test.sh
ZELLIJ='host-client' ZELLIJ_SESSION_NAME='host-session' ZELLIJ_PANE_ID='999' \
  ZELLIJ_SOCKET_DIR="$PWD/.validation-host-socket-sentinel" \
  bash tests/zellij-two-rail-recipient-smoke-test.sh
```

Results: Rust `89/89`; new-tab `14/14`; main normal/upgrade `2/2`; recipient
normal/hostile-inherited `2/2`. Main cleanup reported all owned processes,
session, tmux server, socket, and disposable root absent, with standing config,
layout, and candidate records unchanged. Main resolved paths placed config,
layout, data, socket, lifecycle log, permission cache, and session-info under
`/tmp/zs.*`.

An external candidate-state digest searched the detached checkout path through
the host default Zellij log, standing permission file, cache tree, and data
tree before and after each recipient run. It remained the SHA-256 of empty
input, `e3b0c442...b855`; host log candidate count stayed `0 -> 0`. The hostile
inherited socket sentinel contained zero entries before and after.

### Cycle 3 refutation audit

- **Exact stable-tab/token false positive — SURVIVES.** Two real rails shared
  CWD and token. The target-tab broadcast rendered only in its stable-tab
  recipient; a separate accepted bystander barrier proved the bystander was
  live without ever rendering the target marker.
- **Inherited-client identity and socket escape — SURVIVES.** The hostile run
  supplied fake loaded-client variables plus an inherited socket directory.
  The script cleared identity before its first Zellij call, both rails passed,
  and the inherited socket directory remained empty.
- **Host log/cache/data/session-info/permission leakage — SURVIVES.** Both
  recipient runs preserved the internal candidate-scoped digest. The external
  detached-path digest and exact log count also stayed empty/zero. No standing
  permission record or cache/data file contained the candidate path.
- **Disposable-root false negative — SURVIVES.** The recipient harness requires
  every resolved HOME/TMPDIR/XDG/config/layout/data/socket path under `/tmp/zr.*`,
  positively observes its candidate-bearing lifecycle log and live socket
  there, requires the disposable permission cache, records session-info if
  materialized, and deletes the root on cleanup.
- **Main fixed-width/fullscreen semantic drift — SURVIVES.** Normal and upgrade
  journeys independently re-proved exact rail/chrome geometry, all inert former
  inputs, terminal and rail fullscreen terminal state, restored stable fields,
  entry/subscriber behavior, and complete cleanup with zero host records.
- **Panic/indexing and caller impact — SURVIVES.** Rust `89/89`, new-tab
  `14/14`, both main journeys, and both recipient journeys passed; explicit
  cardinality checks and bounded waits remained intact.

### Cycle 3 captain demo for I1 and I2

CL should drive these steps from an ordinary pane inside `WORK`. First run the
cheap identity/toolchain/binding/unit preflight:

```bash
set -euo pipefail
WT=/Users/clkao/git/zaphod/.worktrees/spacedock-ensign-fixed-width-managed-rail
cd "$WT"
test "$(git rev-parse HEAD)" = c269b31bd9ada398cebb79e839580bb4f2a6b0d8
test "$(zellij --version)" = 'zellij 0.44.3'
rg -n 'bind "Alt /" \{ NoOp; \}|bind "f" \{ ToggleFocusFullscreen;' \
  /Users/clkao/.config/zellij/config.kdl
cargo test fixed_header_clicks_are_inert_without_changing_row_behavior -- --nocapture
./build.sh
./scripts/zellij-new-tab.sh --session WORK --name 'Fixed rail validation cycle 3'
```

Copy the printed tab ID and prepare I1:

```bash
TAB_ID=<printed-tab-id>
DEMO=/tmp/fixed-rail-cycle-3-$TAB_ID
zellij --session WORK action new-pane --tab-id "$TAB_ID" --direction right \
  --cwd "$WT" --name fixed-demo-right
zellij --session WORK action new-pane --tab-id "$TAB_ID" --direction down \
  --cwd "$WT" --name fixed-demo-down
zellij --session WORK action list-panes --json --all --command --geometry --state --tab \
  > "$DEMO.before.json"
zellij --session WORK action dump-layout > "$DEMO.before.kdl"
```

CL confirms one 28-column rail, three arranged terminals, and both chrome rows;
presses literal `Alt /`; then clicks the visible word `FIXED`. Nothing moves,
stacks, disappears, changes width, or prompts. Capture the result:

```bash
zellij --session WORK action list-panes --json --all --command --geometry --state --tab \
  > "$DEMO.after-inputs.json"
zellij --session WORK action dump-layout > "$DEMO.after-inputs.kdl"
```

For I2, CL focuses an ordinary pane and presses the current native sequence
`Ctrl-p`, then `f`. The terminal must occupy the whole content rectangle. CL
presses the same sequence to restore and confirms the original rail, split,
and chrome geometry. Then:

```bash
zellij --session WORK pipe --name navigate -- ""
# CL observes rail focus and presses Ctrl-p then f.
zellij --session WORK action list-panes --json --all --command --geometry --state --tab \
  > "$DEMO.rail-fullscreen.json"
# CL presses Ctrl-p then f again, then Esc to exit rail navigation.
zellij --session WORK action list-panes --json --all --command --geometry --state --tab \
  > "$DEMO.restored.json"
zellij --session WORK action dump-layout > "$DEMO.restored.kdl"
```

The rail must fill the same content rectangle with `is_fullscreen=true` and
restore with every managed fullscreen flag false, the 28-column rail, original
terminal split, and both chrome rows. No permission prompt or new pane appears.

### Cycle 3 demo outcome

**I1 PASS — captain observed.** In the supplied throwaway-session walkthrough,
the former toggle inputs left the 28-column rail, pane arrangement, and chrome
unchanged.

**I2 PASS — captain observed.** In the same walkthrough, native fullscreen for
an ordinary pane and for the rail each restored the original geometry, with no
pane left fullscreen.

Together with independently reproduced O1-O4 at clean head `c269b31` under the
recorded review waiver, the final validation recommendation is **PASSED**.
