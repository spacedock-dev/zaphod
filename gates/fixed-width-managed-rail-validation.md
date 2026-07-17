# Validation: Keep the managed rail at one fixed width without blocking native fullscreen

Entity: `fixed-width-managed-rail.md`  
Implementation worktree: `.worktrees/spacedock-ensign-fixed-width-managed-rail`  
Raw implementation SHA: `2f209781c10184a768292555aea0effc8aa77145`  
Validation checkout: detached throwaway clone of that SHA, nested under the assigned worktree

The implementation worktree was clean at the checked SHA before validation.
No implementation file was changed during validation.

## Gate recommendation

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
