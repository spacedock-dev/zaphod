---
id: tjj3aqdq4c4at5wrke8cmk0v
title: Keep the managed rail at one fixed width without blocking native fullscreen
status: validation
source: captain direction after live dirty-tab layout corruption investigation, 2026-07-17
sprint: s1-managed-tab-safety
group: layout-stability
sprint-readiness: ready
started: 2026-07-17T10:10:10Z
completed:
verdict:
score: 0.98
worktree: .worktrees/spacedock-ensign-fixed-width-managed-rail
issue:
pr:
mod-block:
---

## Problem

The managed rail currently toggles between fixed widths by replacing or cycling the whole tiled layout. In tabs with additional panes, that operation can flatten or stack the user layout and misplace tab-bar/status-bar chrome. The first safe product step is to keep the rail docked at one fixed expanded width, make toggle input incapable of restructuring the tab, and preserve Zellij native fullscreen for ordinary and rail panes.

## Captain constraint

Prefer layout preservation over reclaiming the rail columns. Assume native pane fullscreen remains supported, and verify that assumption as part of the design and live acceptance proof.

## Proposed approach

Keep one layout-owned, tiled rail at `size=28` for the lifetime of a managed tab. Remove the `docked` and `undocked` swap layouts from `layouts/zaphod.kdl`; the birth layout retains exactly one tab-bar row, the 28-column rail, one `children` content region, and one status-bar row. Adding, splitting, resizing, floating, or fullscreening content panes must use Zellij's ordinary pane operations and must never cause Zaphod to apply or cycle a layout.

Retire the dock toggle rather than assign its keys a surprising new meaning:

- Persistent `Alt /` remains a silent `NoOp`. The rail stops requesting a runtime `MessagePluginId` binding, drops `Reconfigure` from `permissions_for_config`, and treats any stale or directly sent `toggle` pipe as an unknown inert message.
- Replace the header's `⇄` affordance with the static word `FIXED`. Clicking the header does nothing; pane, session, and gate rows keep their existing actions.
- Fullscreen remains Zellij-native. Zaphod adds no fullscreen keybinding: an ordinary pane uses the operator's normal fullscreen action, while the rail can be focused through its existing navigation mode and then fullscreened with the same native action. Exiting fullscreen restores the fixed rail and prior content geometry.

Implementation should extend the existing pure seams `permissions_for_config` and `decide_rail_click`: the former no longer emits `Reconfigure`, and the latter maps the header to `ClickAction::None`. Remove the now-unreachable `decide_toggle`, runtime-route, pending-steer, dump-transform, `override_layout`, and swap-steering machinery instead of retaining a second layout-mutating path. Keep `decide_rail_click`'s row behavior and the agent/session/gate delivery functions unchanged.

## Acceptance criteria

### Offline

- **O1 — A populated managed tab has one stable fixed rail and intact chrome.** In an isolated 160x48 tmux-hosted Zellij 0.44.3 session, a selected-checkout managed tab with two added terminal panes has exactly one tiled, non-suppressed Zaphod rail at `x=0`, `y=1`, `28x46`, one `160x1` tab bar at `y=0`, and one `160x1` status bar at `y=47`.
  **Verified by:** an automated smoke captures native `list-panes --json --all --command --geometry --state --tab`, identifies the rail by exact canonical WASM URL and stable tab ID, and compares the observed six-pane inventory with these independently specified dimensions.
- **O2 — Former toggle inputs cannot mutate pane layout.** Literal `Alt /`, a real click on the `FIXED` header, and a direct stale `toggle` pipe leave pane IDs, plugin URLs, terminal commands, pane geometry, chrome geometry, and normalized `dump-layout` state unchanged; the rail remains 28 columns.
  **Verified by:** the isolated tmux smoke records before/after native inventories and normalized layout dumps for each input, while Rust tests drive `permissions_for_config` and `decide_rail_click` and require `Alt /` route absence plus an inert header result.
- **O3 — Native fullscreen round-trips both pane kinds without layout loss.** With the same two extra panes present, fullscreening an ordinary terminal and the Zaphod rail in turn expands the target to the full `160x46` content rectangle; leaving fullscreen restores every non-focus field from O1, including the rail at 28 columns and both chrome rows.
  **Verified by:** the smoke uses Zellij's pane-ID-targeted native `toggle-fullscreen` action for each target, observes `is_fullscreen=true` and full content geometry, then compares canonical post-round-trip snapshots against the pre-fullscreen baseline.
- **O4 — Entry, row interaction, and permission behavior regress neither functionality nor standing configuration.** Managed entry still creates one tab from the selected checkout, row clicks still focus their exact targets, subscriber delivery still reaches only its stable-tab/token-bound rail, and no test writes standing config, layout, data, socket, or permission roots.
  **Verified by:** `cargo test`, `tests/zellij-new-tab-test.sh`, the existing recipient smoke, and the revised isolated tmux smoke all pass with standing-root hashes unchanged; the permission fixture expects no `Reconfigure` grant.

### Interactive

- **I1 — Extra panes remain arranged after former toggle inputs.** In the captain's `WORK` session, create a fresh managed tab, add one vertical and one horizontal split, note their geometry plus both chrome rows, press literal `Alt /`, and click the `FIXED` header; nothing moves, stacks, disappears, or changes width.
  **Verified by:** captain observation plus before/after `list-panes --json --all --geometry --state --tab` and `dump-layout` captures reviewed at the implementation gate.
- **I2 — The captain's native fullscreen journey works for ordinary and rail panes.** Fullscreen and restore an ordinary pane with the captain's existing Zellij binding; then enter rail navigation, fullscreen and restore the rail with that same native binding, and exit navigation.
  **Verified by:** captain confirms both targets occupy the content area while fullscreen and the original split/chrome geometry returns after each restore, with no permission prompt or new pane.

## Test plan

1. **Riskiest mechanism first — already exercised:** in a disposable 160x48 tmux + Zellij 0.44.3 session, birth a 28-column fixed, layout-owned plugin rail with canonical chrome, add two terminals, and fullscreen/restore one ordinary pane and the rail. The terminal and plugin each reached `160x46`; the normalized six-pane snapshots after both round-trips matched the baseline, with chrome still at `y=0`/`y=47` and the rail restored to `28x46`. A built-in tiled plugin isolated this host-layout assumption from Zaphod permission behavior; O3 repeats it with the shipped WASM. The disposable server, socket, data root, and tmux server were removed.
2. Add failing pure tests for permissions and header click, then remove route/toggle/swap code and reduce the layout to the fixed base.
3. Revise the real-key smoke: add two panes before baseline; prove literal `Alt /`, header click, and stale pipe are no-ops; perform both pane-ID fullscreen round-trips; compare native geometry, identity, layout, and chrome after every operation.
4. Run the unit, layout/entry, recipient, and tmux smoke suites in isolated roots. Only after all offline checks pass, run I1 and I2 in the captain's `WORK` session.

## Required documentation diff

- `README.md`: replace the screenshot's `⇄` with `FIXED`; replace the dock/sliver feature with the always-28-column behavior, silent `Alt /`, inert header, and native-fullscreen escape hatch; remove the runtime-route/Reconfigure instructions; revise live verification and Status accordingly.
- `docs/roadmap.md` and `docs/zaphod-workspace-architecture.md`: change Sprint 1's current managed-tab toggle claims to a fixed layout-owned rail with layout-inert former toggle inputs. Leave future driver-level resizing explicitly dependent on a safe upstream API rather than presenting it as shipped.
- `SPEC.md`: mark the docked/sliver and regenerated-swap passages as historical prototype behavior, record the fixed-rail safety boundary and fullscreen proof, and retain the old failure analysis as evidence.
- `layouts/zaphod.kdl` and smoke-harness documentation: describe the single fixed layout and the new no-op/fullscreen proofs; do not claim that any key cycles a swap layout.

## Out of scope

- Reclaiming rail columns through resize, swap layouts, `override_layout`, hide/show, float/embed, or another automatic reflow.
- Rebinding `Alt /` to fullscreen or another global action; the operator's existing native fullscreen binding remains authoritative.
- Hot-repairing tabs created by an older docked/sliver WASM or layout; create a fresh managed tab after upgrading.
- General retained-pane layout transactions, exact fixed-dimension setters, right-docking APIs, tmux parity, or changes to future hub/adoption architecture.

## Stage Report: ideation

- DONE: Define the fixed-width rail behavior, including the exact inert/replacement behavior for Alt / and the header toggle, without any whole-tab override or swap-layout mutation; include the required user-facing documentation diff.
  The proposed approach fixes the rail at 28 columns, leaves `Alt /` as silent `NoOp`, replaces `⇄` with inert `FIXED`, removes all toggle layout APIs, and names the README/roadmap/architecture/SPEC diff.
- DONE: Specify independently reproducible offline and captain-driven interactive acceptance proofs that extra panes retain geometry/chrome and that native fullscreen still works for ordinary and rail panes.
  O1-O4 define isolated native-state proofs; I1-I2 require captain observation and before/after native inventories in `WORK`.
- DONE: Exercise the riskiest assumption first in a throwaway Zellij session: a managed fixed-width rail remains fullscreen-compatible after the tab gains additional panes, or record the concrete invalidating result.
  Zellij 0.44.3 at 160x48 passed ordinary and fixed-plugin fullscreen round-trips after two added terminals; normalized six-pane geometry/chrome snapshots returned exactly to baseline.

### Summary

The design removes layout mutation from the rail's steady state: one 28-column birth-layout rail, inert former toggle inputs, and native fullscreen as the temporary space-reclamation mechanism. A disposable host-level probe validated the riskiest fullscreen assumption, while the implementation acceptance proof intentionally repeats it with the real Zaphod WASM and captain interaction.

## Stage Report: implementation

- DONE: Ship the approved fixed-28-column managed layout with former toggle inputs inert, remove the whole-tab override/swap/runtime-route machinery, preserve row behavior, and update every approved user-facing document through red-then-green behavior slices.
  Commits `fdd6c2b`, `67fced9`, `a259e40`, `1acd034`, `b6322de`, `c55713c`, and `d9872d4` deliver the fixed rail, documentation, exact-byte header, literal fullscreen journey, and corrected FIXED click coordinate.
  `src/main.rs` no longer requests `Reconfigure`, installs a runtime `MessagePluginId` toggle, or retains toggle/swap/override/pending-steer machinery; header clicks return `None`, stale toggle pipes are unknown and inert, and row actions are unchanged.
  `layouts/zaphod.kdl` owns exactly one 28-column rail, one children region, and native tab/status bars; no swap layouts remain.
  README, SPEC, roadmap, workspace architecture, and smoke-harness documentation now describe the fixed safety boundary and gate future resizing on a safe upstream exact-width/retained API.
  RED: `cargo test cli_pipe_permission_is_reserved_for_token_bound_entry -- --nocapture` observed `Reconfigure` in the permission set; green after removal.
  RED: `cargo test fixed_header_clicks_are_inert_without_changing_row_behavior -- --nocapture` observed `ToggleDock` instead of `None`; green after the click change.
  RED: `bash tests/zellij-new-tab-test.sh` reported that the tokenless installed layout did not contain exactly one rail; green after removing swap layouts.
  RED: the focused header-byte test failed to compile because `fixed_header_line` did not exist; green with exact 28-, 13-, and 8-column Unicode/ANSI byte assertions.
  Native test count changed from 143 before retired-toggle removal to 88 after removal, then 89 after adding the fixed-header byte seam.
- DONE: Make the offline acceptance proof green: exact populated-tab rail/chrome geometry, byte-stable layout under Alt /, FIXED-header click and stale toggle pipe, native fullscreen round-trips for terminal and rail, entry/recipient isolation, and unchanged standing roots.
  Resolved shared native target: `/Users/clkao/git/zaphod/target`; native commands used `RUSTC_WRAPPER=sccache` and `SCCACHE_DIR=/Users/clkao/git/zaphod/target/sccache`.
  `cargo test` passed 89 tests and `cargo check --tests` passed with the shared target; `env -u CARGO_TARGET_DIR ./build.sh` produced the isolated candidate WASM.
  `bash tests/zellij-new-tab-test.sh`, `bash tests/zellij-layout-capture-test.sh`, `bash tests/zellij-install-profile-test.sh`, and the docs scrollback test passed.
  `bash tests/zellij-tmux-smoke-test.sh` passed at 160x48 with a six-pane inventory, exact rail `x=0,y=1,28x46`, exact chrome, literal Alt `/`, column-24 FIXED click, stale CLI toggle, and literal terminal/rail fullscreen restores.
  `ZAPHOD_PERMISSION_FIXTURE=upgrade bash tests/zellij-tmux-smoke-test.sh` and `bash tests/zellij-two-rail-recipient-smoke-test.sh` passed; disposable-root hashes proved standing config/layout/data/socket/permission roots unchanged.
  The semantic adversarial pass pinned exact header bytes and identity/cardinality/geometry/fullscreen terminal state, exercised every former input path available in the shipped profile, and made restore exhaustion fail instead of normalize away `is_fullscreen`.
- DONE: Complete the semantic adversarial pass and exact-head Roborev quick plus code_completion review, resolving every material finding and recording reproducible commands, counts, commit SHAs, and panel evidence in the implementation report.
  Exact-head quick parent `16`, panel `quick`, reviewed `d9872d4b849f40716622871d7e9e28f2763cd26d`; required member `15` PASS and parent verdict PASS with no issues.
  Authoritative round 1 parent `6`, range `ac0ae2a5c301052daadc2c5cca89c2bbfbf6fdcc..b6322de051098a2432eed0863924b6fa6e507b6c`, failed: literal-key/focus fullscreen was bypassed and restore timeout/fullscreen terminal state could falsely pass.
  Disposition: both were O3 evidence defects; `c55713c` added literal Alt-f terminal/rail journeys through supported rail navigation, explicit restore deadlines, and final `is_fullscreen=false` assertions.
  Authoritative round 2 parent `14`, range `ac0ae2a5c301052daadc2c5cca89c2bbfbf6fdcc..c55713c720198acb0feb5941be96830cf7af3aa9`, failed because column 5 clicked PANES rather than FIXED.
  Disposition: O2 evidence defect; `d9872d4` moved the raw SGR click to column 24 and the complete smoke passed.
  A non-authoritative over-wide range audit parent `21` passed all three required members; it is not exit evidence because its stored range begins at the merge-base parent.
  Authoritative round 3 parent `25`, panel `code_completion`, exact required range `ac0ae2a5c301052daadc2c5cca89c2bbfbf6fdcc..d9872d4b849f40716622871d7e9e28f2763cd26d`, current head `d9872d4`, returned FAIL.
  Required members executed exactly once: correctness `22` FAIL, journey `23` PASS, proof `24` FAIL; synthesis verdict FAIL.
  Surviving Medium findings are O2 evidence defects: the inert FIXED click lacks a positive raw-mouse delivery control, and stale toggle coverage does not exercise `PipeSource::Keybind` against a permission-granted active tiled resident.
  Release-scope triage: the released managed-tab behavior is green, but O2 explicitly promises literal header and former-keybind inertness; negative-only or wrong-source evidence can falsely certify that safety boundary.
  Frozen head is `d9872d4`; the least-invasive proposed disposition for both findings is `must fix now` with tests only: add a same-path positive row-click/focus control and an authorized active-resident keybind-source unit assertion.
  Repair cost is small and isolated to proof code; product behavior need not change. Risk is smoke focus restoration/flakiness, mitigated by exact inventory baselines and deadlines.
  The three-round convergence budget was exhausted at this gate; subsequent bounded test-only work and replacement reviews were performed only after explicit captain authorization, as recorded below.

### Summary

The fixed 28-column product and offline runtime journey reached a green frozen head at `d9872d4`, including native terminal/rail fullscreen and layout-inert former inputs. The exhausted-budget convergence gate identified two bounded O2 proof repairs and correctly required captain authorization before the continuation below.

## Implementation Convergence Continuation

- Captain approved exactly the two test-only O2 repairs and one replacement exact-head review; product behavior and scope remained frozen.
- `73a6852` strengthens `stale_keybind_toggle_is_inert_for_granted_active_tiled_resident`: the source is `PipeSource::Keybind`, permissions are granted, and the resident is active, tiled, and state-stable after the stale pipe.
- The RED for that evidence slice is authoritative parent `25`'s proof finding: the prior unauthorized/default-sidebar unit plus CLI smoke would not fail against the retired dangerous keybind route.
- `cargo test stale_keybind_toggle_is_inert_for_granted_active_tiled_resident -- --nocapture` passed 1 test with 88 filtered out; `cargo check --tests` passed.
- `d1d84b5` adds a same-path raw-SGR positive control: choose an exact non-focused native pane ID, map it to the sorted rendered rail row, observe focus move, click the original row, and require the full native/layout baseline to return before the negative header assertion.
- RED: the first positive-control run of `bash tests/zellij-tmux-smoke-test.sh` sent the row click to SGR row 3 and failed `pane 1 did not become the focused target through the supported path`.
- GREEN: mapping the first pane row to SGR row 4 made the positive focus/restore control pass, and both pregranted and `ZAPHOD_PERMISSION_FIXTURE=upgrade` complete smoke journeys passed.
- Verification at `d1d84b5`: shared-target `cargo test` passed 89; `cargo check --tests`, new-tab, layout-capture, docs scrollback, and two-rail recipient tests passed.
- Exact-head quick parent `29`, panel `quick`, reviewed `d1d84b5fc1f0ce79c6beaf45da3ed1b7c3ea22e5`; required member `28` PASS and parent verdict PASS with no issues.
- Captain-authorized replacement parent `38`, panel `code_completion`, reviewed exact range `ac0ae2a5c301052daadc2c5cca89c2bbfbf6fdcc..d1d84b5fc1f0ce79c6beaf45da3ed1b7c3ea22e5` and returned FAIL.
- Required members executed exactly once: correctness `35` PASS, journey `36` FAIL, proof `37` PASS.
- Remaining Medium is an O2 evidence defect: the negative FIXED assertion uses SGR row 3, while the visually rendered header at zero-based `pane_y=1` occupies one-based screen row 2; row 3 is the first rendered pane row.
- Proposed `must fix now` disposition remains test-only: derive the visual header row as `rail.pane_y + 1`, retain the positive row-click round trip and exact header before/after comparison, then run a newly authorized exact-head quick and exact-range completion panel.
- Captain sent the finding back for the bounded correction and one new exact-head quick plus exact-range completion panel; product behavior and scope remained unchanged.
- `2f20978` adds `rail_header_screen_y`, derives the sole native rail's visual header as zero-based `pane_y + 1`, and sends the column-24 SGR press/release to that row while preserving the positive row-focus/restore control and exact before/after assertion.
- RED: authoritative replacement parent `38` proved the row-3 assertion targeted the first rendered pane row rather than visual FIXED; this was classified as an O2 evidence defect.
- GREEN: `bash tests/zellij-tmux-smoke-test.sh` and `ZAPHOD_PERMISSION_FIXTURE=upgrade bash tests/zellij-tmux-smoke-test.sh` passed with the derived visual header row; `bash tests/zellij-two-rail-recipient-smoke-test.sh` passed.
- Final verification at `2f20978`: shared-target `cargo test` passed 89, `cargo check --tests` passed, and new-tab, layout-capture, and docs scrollback tests passed.
- Exact-head quick parent `63`, panel `quick`, reviewed `2f209781c10184a768292555aea0effc8aa77145`; required member `62` PASS and parent verdict PASS with no issues.
- Authoritative parent `67`, panel `code_completion`, reviewed exact range `ac0ae2a5c301052daadc2c5cca89c2bbfbf6fdcc..2f209781c10184a768292555aea0effc8aa77145`; current head and reviewed head match.
- Required members executed exactly once: correctness `64` PASS, journey `65` PASS, proof `66` PASS; synthesis parent verdict PASS with no issues.
- Every material finding is fixed; no rebuttal, deferred risk, decision point, or post-panel code change remains. Final clean head is `2f20978`.

### Continuation Summary

All captain-approved proof gaps were addressed without product changes. Focused and runtime checks are green at `2f20978`, exact-head quick passed, and authoritative exact-range `code_completion` parent `67` passed unanimously; implementation is complete and ready for validation.

## Stage Report: validation

- DONE: Verify that authoritative code_completion parent 67 covers the frozen merge-base(main, 2f20978)..2f20978 range, that every required member executed exactly once and passed, and that no later code change invalidates it.
  Parent `67` names `code_completion`, exact range `ac0ae2a..2f20978`, and PASS; correctness `64`, journey `65`, and proof `66` each appear once with `done`, PASS, and zero retries; clean `HEAD` remains `2f20978`.
- FAILED: Independently reproduce every offline O1-O4 proof and perform the required adversarial refutation audit from a throwaway checkout, including false-positive mouse coordinates, keybind-source authority, fullscreen terminal state, identity, geometry, chrome, and standing-root isolation.
  O1-O3 passed in both permission modes, Rust passed `89/89`, entry `14/14`, and recipient isolation passed; O4 is REFUTED because the detached candidate's server lifecycle was written to host `$TMPDIR/zellij-501/zellij-log/zellij.log`.
  Mouse row/column, positive delivery control, granted active-tiled Keybind source, exact identity/cardinality/geometry/chrome, and terminal/rail fullscreen attacks survived; one of five extra responsiveness runs also failed on cursor `[8,2] -> [8,1]`, exposing an intermittent restore-oracle defect.
- DONE: Prepare the exact captain demo for interactive I1-I2 and the required subspace gate artifact plus decisions log, reporting per-AC verdicts and any surviving attack rather than modifying the implementation.
  `gates/fixed-width-managed-rail-validation.md` contains the post-fix I1-I2 script and per-AC evidence; sibling brief and `.decisions.jsonl` recommend REJECTED, and the captain demo was correctly not run while O4 is red.

### Summary

The fixed-width behavior and native fullscreen journey passed independent O1-O3 reproduction at clean `2f20978`, and authoritative panel `67` is current. Validation rejects the gate on a concrete O4 standing-root write plus an intermittent cursor-snapshot proof failure; implementation code was not changed, and I1-I2 remain for the captain only after repair and replacement review.

### Feedback Cycles

#### Cycle 1 — validation → implementation

- Verdict: `REJECTED`; feedback target: `implementation`.
- Outcome defect, O4: the isolated tmux smoke sets `ZELLIJ_SOCKET_DIR` but leaves Zellij logging/cache resolution on host-default roots. A detached candidate wrote its lifecycle and exact candidate WASM path to `$TMPDIR/zellij-501/zellij-log/zellij.log` and attempted the standing session-info cache. Repair the supported harness so every Zellij process resolves logs, cache, socket, data, config, layout, and permission state only inside disposable roots, and prove standing-root hashes/records remain unchanged.
- Evidence defect, fullscreen restore oracle: one of five responsiveness-enabled reproductions changed only `cursor_coordinates_in_pane` from `[8,2]` to `[8,1]` while geometry, chrome, identity, and fullscreen terminal state remained stable. Classify volatile cursor state explicitly and replace the flaky comparison with an oracle that still fails on layout/fullscreen drift without treating benign cursor motion as product failure.
- Keep the fixed-width product behavior and O1-O3 contract unchanged. Implement the two findings as separate red/green slices, obtain replacement exact-head review evidence, then re-run the same validation reviewer against the frozen repaired head.

#### Cycle 2 — validation → implementation

- Verdict: `REJECTED`; feedback target: `implementation`.
- Outcome defect, O4: the repaired main tmux smoke is fully contained, but the separate passing two-rail recipient smoke still launches the detached candidate without disposable `TMPDIR`/XDG coverage. Validation observed the candidate-path count in host `$TMPDIR/zellij-501/zellij-log/zellij.log` change from `0` to `1`.
- Repair only `tests/zellij-two-rail-recipient-smoke-test.sh` and its direct helpers so every server, client, version/control call, and sidecar inherits the complete disposable environment. Prove the recipient journey still passes and produces zero candidate-attributable host log/cache/socket/data/permission records.
- Keep product behavior, the main-smoke repair, and the cursor oracle unchanged. The implementation review budget remains exhausted: obtain the exact-head quick result, then stop at a convergence gate before launching any new authoritative `code_completion` panel.
- Captain disposition after the convergence report: no additional authoritative review is required for the bounded recipient-harness-only correction at `c269b31`; exact-head quick parent `132` is accepted as sufficient review evidence. Proceed directly to same-reviewer O1-O4 revalidation without running Roborev.

## Validation Feedback Cycle 1 Repair

- DONE: Contain every Zellij process and every writable Zellij root in the disposable tmux-smoke tree, while detecting only candidate-attributable writes in mutable standing roots.
  RED: `ZAPHOD_SMOKE_RESPONSIVENESS_CHECK=1 bash tests/zellij-tmux-smoke-test.sh` observed the candidate lifecycle in the host Zellij log/tmp root and a standing socket-root change, reproducing validation's O4 outcome defect.
  `f0eaf05` supplied disposable `HOME`, `TMPDIR`, XDG cache/config/data/runtime, config/layout/data/socket roots to every server, client, cleanup, and sidecar-launched Zellij process and added resolved-root/lifecycle evidence.
  `8d82f60` moved the pinned-version probe behind that same complete disposable environment so no Zellij invocation precedes containment.
  `cb68911` replaced false-positive-prone whole mutable-root comparisons with exact config/layout hashes plus candidate-specific log, session-info, socket, and data attribution.
  `b9568fc` additionally snapshots only the candidate WASM's standing permission record, handles absent standing socket/data directories without failing under `set -euo pipefail`, and verifies the managed tab is active before using compatible unscoped pane creation.
  GREEN: normal, upgrade-permission, and responsiveness smoke journeys passed with resolved config, layout, data, socket, log, session-info, and permission paths under the disposable root; cleanup evidence reported candidate standing records unchanged and all owned processes/session/socket/root absent.
- DONE: Normalize only volatile native cursor coordinates in the fullscreen restore oracle while retaining every stable O1-O3 field.
  RED: independent validation observed one of five responsiveness runs change only `cursor_coordinates_in_pane` from `[8,2]` to `[8,1]` after an otherwise exact native fullscreen round trip.
  `a71ee09` removes only `is_focused` and `cursor_coordinates_in_pane` from the non-focus comparison. Its contract assertion requires identity, tab/plugin/command state, geometry, content geometry, chrome, floating/suppressed/selectable/fullscreen/exited state, `pane_command`, and `pane_cwd` to remain present.
  GREEN: five consecutive responsiveness-enabled smoke runs passed after the normalization; the final-head responsiveness run also passed literal Alt p/Alt n/Alt 1/Alt 2 one-second deadlines and the six-second stable post-close inventory.
- DONE: Re-run the repaired offline checks and resolve every material replacement-review finding without changing fixed-width product behavior.
  Shared-target `cargo test` passed `89/89`; `cargo check --tests`, new-tab, layout-capture, docs scrollback, normal and upgrade tmux smoke, and the isolated standing-root evidence passed during the repair sequence. The unchanged recipient harness had passed before this feedback cycle; its final rerun entered the live journey but was interrupted after an environment-dependent hang rather than recorded as a new pass.
  Exact-head quick reviews passed after each accepted repair slice; final quick parent `126`, panel `quick`, reviewed `b9568fc8645646c15bb31878827cb86e4f1fff28`, with required member `125` PASS and no issues.
  Replacement parent `124`, panel `code_completion`, failed with three Medium harness findings: mutable whole-permission-cache comparison, absent-directory pipeline failure, and pane creation compatibility. `b9568fc` resolved all three as test-only changes; O1-O3 product behavior remained frozen.
  Captain-accepted authoritative parent `130`, panel `code_completion`, returned PASS with no issues. Correctness `127`, journey `128`, and proof `129` each executed exactly once and passed.
  The positional inclusive invocation stored `c5ecf0ce1d81060ce79a97502e449499c7f62681..b9568fc8645646c15bb31878827cb86e4f1fff28`, one commit wider than `merge-base(main, HEAD)=ac0ae2a5c301052daadc2c5cca89c2bbfbf6fdcc`; the captain explicitly accepted this evidence because it reviewed one extra base commit and omitted none of the fixed-width task.
  Final implementation head is clean at `b9568fc8645646c15bb31878827cb86e4f1fff28`; no post-panel code change or unresolved material review finding remains.

### Repair Summary

Validation's O4 leak is contained at process launch and proved with candidate-scoped standing-state attribution. The fullscreen oracle ignores only volatile cursor position while preserving all layout, identity, chrome, command, suppression, and fullscreen terminal state. Final quick and captain-accepted completion reviews pass at clean head `b9568fc`; the repaired head is ready for validation to repeat O1-O4 and then present I1-I2 to the captain.

## Stage Report: validation (cycle 2)

- DONE: Verify clean repaired head b9568fc and the captain-accepted parent 130/member 127-129 PASS record, confirming no post-panel code change and explicitly carrying the accepted inclusive-range exception rather than rerunning Roborev.
  Clean `HEAD` is `b9568fc`; parent `130` and correctness `127`, journey `128`, proof `129` are `done`, PASS, zero-retry, exactly once. Its accepted `c5ecf0c..b9568fc` range includes all 15 task commits plus base `ac0ae2a`; no rerun was made.
- FAILED: Independently rerun O1-O4 from a detached throwaway checkout, with special attacks against host log/cache/socket/data/permission leakage and at least five responsiveness fullscreen round trips proving the cursor-only normalization cannot mask geometry, chrome, identity, or fullscreen terminal-state drift.
  Rust passed `89/89`, new-tab `14/14`, normal/upgrade main smoke `2/2`, and responsiveness `5/5`; synthetic stable-field mutations all remained detectable and every main cleanup proved disposable roots plus zero host candidate records.
  O4 is REFUTED: the passing two-rail recipient smoke changed the detached candidate-path count in host `$TMPDIR/zellij-501/zellij-log/zellij.log` from `0` to `1`; `tests/zellij-two-rail-recipient-smoke-test.sh:42-50,99,167-170` lack the complete disposable environment.
- DONE: Update the validation gate artifact, brief, decisions log, and exact I1-I2 captain demo from reproduced evidence; recommend PASSED only if every offline AC survives, otherwise report the concrete surviving attack without modifying implementation.
  Gate round 2 remains REJECTED; the detailed artifact records per-AC evidence, the exact leaked log line, and a post-repair captain script. I1-I2 were correctly not run, and implementation code stayed untouched.

### Summary

Cycle 2 validates the main harness repair and cursor-only fullscreen oracle at clean `b9568fc`, including five consecutive responsiveness attacks. The unchanged recipient harness still writes its candidate lifecycle to the standing Zellij log, so O4 remains red and the gate returns to implementation without spending captain time on I1-I2.

## Validation Feedback Cycle 2 Repair and Convergence Gate

- DONE: Contain the two-rail recipient harness without changing product behavior, the repaired main smoke, or the cursor-only fullscreen oracle.
  RED: the pre-repair `bash tests/zellij-two-rail-recipient-smoke-test.sh` journey passed while the candidate-WASM count in host `$TMPDIR/zellij-501/zellij-log/zellij.log` changed from `4` to `5`, independently reproducing validation's remaining O4 outcome defect.
  `c269b31bd9ada398cebb79e839580bb4f2a6b0d8` gives the version probe, setup/control calls, tmux-hosted server, session clients, cleanup probes, and rail-launched children one complete disposable `HOME`, `TMPDIR`, XDG cache/config/data/runtime, config/layout, data, socket, lifecycle-log, session-info, and permission environment. Inherited Zellij client identity is removed before the first probe.
  The harness snapshots candidate-specific standing log lines, permission records, session-info, socket, and data paths before launch and requires the same digest both before cleanup and after the disposable root is removed. It also verifies that the live lifecycle log, socket, permission cache, and any materialized session-info path resolve only beneath the disposable root.
- DONE: Preserve the exact two-rail stable-tab/token isolation journey and establish no regression in the already-contained main harness.
  GREEN: two post-repair recipient runs passed `stable-tab recipient delivers only to the target rail with zero host-default candidate records`; the independently counted host candidate path stayed `5 → 5` on both final checks.
  `bash -n tests/zellij-two-rail-recipient-smoke-test.sh` and `git diff --check` passed. The final-head normal `bash tests/zellij-tmux-smoke-test.sh` also passed its fixed-input, terminal/rail fullscreen, entry, subscriber, routing, and cleanup journey unchanged.
  Exact-head quick parent `132`, panel `quick`, reviewed `c269b31bd9ada398cebb79e839580bb4f2a6b0d8`; required member `131` PASS and synthesis PASS with no issues.
- DECISION REQUIRED: Authoritative implementation-review convergence remains exhausted.
  Clean implementation head is `c269b31bd9ada398cebb79e839580bb4f2a6b0d8`. The only cycle-2 change is the bounded recipient-harness O4 containment slice; no product file or other proof path changed.
  Per the dispatch gate, no new `code_completion` panel was launched. Captain authorization is required before spending another authoritative replacement review and returning the frozen head to validation for O1-O4 and I1-I2.

### Repair Summary

The remaining recipient-smoke leak is reproduced red and contained green while exact stable-tab/token routing remains intact. The main harness remains green, exact-head quick review passes, and clean head `c269b31` is stopped at the required convergence gate awaiting the captain's review-budget decision.
