---
id: fqjswvmd93vek5y12zemf1k2
title: Activation preserves valid Zellij KDL
status: validation
source: staff review of fp merge 2026-07-12; captain direction
sprint: s1-managed-tab-safety
group: hardening
sprint-readiness: ready
started: 2026-07-12T14:33:06Z
completed:
verdict:
score: 0.98
worktree: .worktrees/spacedock-ensign-zellij-config-activation
issue:
pr:
mod-block:
---

## Problem

The selected-worktree fresh-tab command currently conflates two jobs: it creates
an inline layout for the selected checkout, then rewrites standing
`config.kdl` and `layouts/zaphod.kdl` through the 437-line
`zellij-config-activate.awk` brace counter. A valid unrelated value such as
`WriteChars "{"` makes that rewrite fail before it creates the tab.

The rewrite is also the wrong ownership boundary. A global native `Alt Shift
z` binding has one already-configured layout path; it cannot discover or choose
an arbitrary worktree artifact. Repointing that binding and its layout every
time a selected-checkout command runs turns a fresh-tab operation into a
lasting global configuration change. Zellij already supports the needed
operation directly: `action new-tab --layout-string` accepts raw KDL for one
new tab.

## Required outcome

An operator can run `scripts/zellij-new-tab.sh --session <name>` from a selected
checkout against a valid supported Zellij profile, including quoted braces and
unrelated bindings. It builds that checkout and creates exactly one fresh
managed tab whose resident URL is that checkout's WASM. On success, ordinary
failure, and interruption, the standing `config.kdl` and
`layouts/zaphod.kdl` are byte-for-byte unchanged. The command may read the
profile and ask Zellij to validate it, but it must not parse, rewrite, stage,
or restore either standing KDL file.

`Alt Shift z` remains honest about its narrower role: a preconfigured global
native binding may open its one fixed installed layout, but it is not the
selected-worktree entry point and must never claim to select an arbitrary
checkout artifact. The direct script is the selected-worktree path. This task
does not add a helper pane, controller, runtime keybinding, or a new key.
The profile's persistent `Alt /` policy likewise remains global setup; the
direct script does not repair, validate, or rewrite a legacy key route.

## Proposed approach

Do not replace one standing-KDL rewriter with another. The selected-worktree
path needs no transformation of operator KDL, so neither a Rust helper nor a
Go parser belongs in this task.

1. Keep the current selected-checkout build and canonical `file:` URL
   derivation. Render the repository-owned `layouts/zaphod.kdl` into a
   disposable file or shell value containing that URL. Rendering substitutes
   the one template placeholder; it is not a parse-and-rewrite of operator
   KDL.
2. Pass that value straight to the existing native command:
   `zellij --session <name> action new-tab --name <name> --layout-string <KDL>`.
   The only persistent layout involved is the new tab's server state. The
   script cleans its disposable rendered-layout directory on every exit.
3. Retain the current native post-create verification of the returned stable
   tab ID and exact selected WASM URL before starting the existing private
   subscriber. That preserves the direct command's current session-row entry
   behavior without redesigning subscriber ownership.
4. Remove `zellij-config-activate.awk` from the selected-worktree execution
   path, together with config/layout candidate files, backups, atomic moves,
   and rollback code. A read-only `zellij setup --check` may reject malformed
   profile KDL before `new-tab`; Zellij, not a shell parser, is the authority
   for the operator's configuration. A failed check or failed `new-tab` leaves
   the standing files untouched.
5. Leave any existing global `Alt Shift z` binding untouched. It continues to
   point at whichever fixed layout its owner configured (normally a layout
   installed from the canonical checkout), and it does not start a
   selected-worktree subscriber. Leave persistent `Alt /` untouched too; the
   supported baseline owns its fail-closed global policy.
   Documentation must direct selected-worktree use to the script and call the
   global shortcut a fixed-layout convenience.

`install.sh` remains a separately invoked global-install operation; this task
does not broaden the selected-worktree command into an installer. Any compact
layout-identity helper retained for the global installer is not used to parse
or preserve operator `config.kdl`; real native pane/layout state is the direct
command's identity oracle.

## Acceptance criteria

### Offline

**AC-1 — Selected-checkout fresh tab is the end value.**
Verified by: `tests/zellij-new-tab-test.sh` runs the script from one checkout
and observes exactly one `new-tab --layout-string` invocation for the requested
session; its captured raw KDL contains that fixture's canonical WASM URL, not a
stale or global URL. `tests/zellij-tmux-smoke-test.sh` then confirms one tiled,
non-suppressed resident with that URL at the returned stable tab ID via native
`list-panes` and `dump-layout` state.

**AC-2 — Valid unrelated KDL is preserved, not transformed.**
Verified by: `tests/zellij-tmux-smoke-test.sh` starts with a real-profile config
containing `WriteChars "{"`, comments, and an unrelated fixed-layout `Alt Shift
z` binding, then runs `zellij setup --check`. SHA-256 hashes of that config and
a sentinel `layouts/zaphod.kdl` are identical before and after
selected-worktree creation; the profile's original binding still names its
original fixed layout. The test captures the expected hashes before the command,
not from its output.

**AC-3 — Failure and interruption cannot mutate standing KDL.**
Verified by: `tests/zellij-new-tab-test.sh` injects a setup-check failure,
`new-tab` failure, and TERM while the action is blocked. In each case pre-run
config and layout hashes remain identical, no config/layout backup or temporary
file is left in their directories, and a setup-check failure issues no
`new-tab`.

**AC-4 — The direct path does not depend on the AWK transformer or a pre-existing Zaphod key route.**
Verified by: `tests/zellij-new-tab-test.sh` omits
`zellij-config-activate.awk` from the selected-entry fixture and uses a valid
profile with no Zaphod `MessagePlugin` route. It still creates its inline
selected-WASM tab; a separate pre-existing global shortcut remains
byte-identical. This proves the command no longer treats a global route as its
worktree-selection mechanism.

### Interactive

No new live interaction is introduced. After the offline packet passes, the
captain may run the direct script from a selected checkout in a disposable
profile or ordinary session and observe one fresh selected-WASM tab. The
captain-facing instructions distinguish that action from pressing `Alt Shift
z`: the latter opens only its already-configured fixed layout. Existing
permission and sidecar-target checks remain unchanged; this task adds no
prompt, provider flow, or hotkey behavior.

## Test plan

The riskiest unproven point is that the selected-worktree command can use a
raw inline layout against a valid profile without needing to persist a
worktree-specific keybinding or layout. The first check is therefore a focused
tmux-hosted isolated profile: start with quoted-brace config and a sentinel
layout, run the direct script, then prove selected resident state and unchanged
file hashes. It uses literal tmux keys/native Zellij actions where keys are
needed; it does not introduce a custom PTY, lease, polling daemon, or process
controller.

No separate parser spike is needed. Zellij 0.44.3 documents
`action new-tab --layout-string` as raw KDL, and the existing tmux smoke
already exercises that native action. On 2026-07-13, native
`zellij setup --check` accepted a config containing `WriteChars "{"` directly;
the failing behavior is the current AWK rewrite, not Zellij's parser.

1. Rewrite the existing fake-Zellij entry suite around the no-write contract.
   Use a valid quoted-brace/comment fixture and a sentinel fixed-layout
   shortcut; remove the transformer from the copied checkout. Assert the
   original config/layout hashes, exactly one inline `new-tab`, selected WASM
   URL, selected stable tab ID, existing sidecar target tuple, and disposable
   temp-root cleanup.
2. Add setup-check failure, new-tab failure, and TERM cases. Assert
   byte-identical config/layout hashes and no `new-tab` for the preflight
   failure. These are behavioral preservation checks, not substring checks of
   files written by the implementation.
3. Extend the tmux-hosted isolated smoke with a valid quoted-brace config and
   a pre-existing fixed-layout shortcut. Invoke the direct script from the
   candidate checkout, inspect native panes/layout at the returned tab ID, and
   compare both isolated-profile and standing-root config/layout hashes before
   cleanup. Keep the real-key `Alt Shift z` check separate: it exercises only
   the preconfigured fixed layout and is not evidence of worktree selection.
4. Re-run `cargo test`, the entry shell suite, and the tmux smoke. Inspect that
   the dedicated Zellij session, tmux server, and temporary root are removed.

Doc diff proposed: update the README's fresh-tab section and
`docs/roadmap.md` Sprint 1 wording to say the direct script is the
selected-worktree entry and never rewrites standing config/layout. Update
`docs/zellij-tmux-smoke-harness.md` to describe `Alt Shift z` as a separately
preconfigured fixed-layout native shortcut, never a mechanism for selecting a
worktree. These docs must be reviewed with the implementation rather than
claiming a changed hotkey behavior.

## Out of scope

Changing or automatically installing the global `Alt Shift z` binding; a
controller, helper pane, create-or-focus record, pane adoption, or a public Go
or `zaphod` CLI; global-install policy; multi-client behavior; managed-tab
route authorization; Sprint 2 bb/session work; gate pooling; 7h; 4d; leases;
and custom PTYs. This task does not redesign the private subscriber, the
global persistent `Alt /` policy, or the global installer. It specifically
rejects a Rust/Go replacement parser because the selected-worktree command no
longer needs to rewrite operator KDL.

## Stage Report: ideation

- DONE: Prove whether a Go/KDL parser replacement is available and round-trips fixtures.
  Main, all retained worktrees, and reachable refs contain only `scripts/zellij-config-activate.awk`; the Go `sblinch/kdl-go` spike parsed quoted braces but normalized the document, while Rust `kdl` 4.7.1 losslessly round-tripped the real smoke fixture and a quoted-brace fixture.
- DONE: Specify the smallest atomic activation contract without AWK brace parsing.
  The selected-checkout internal Rust helper is stdin-to-stdout only; the existing shell script remains the sole writer and retains candidate validation, atomic replace, and rollback.
- DONE: Keep the existing entry command and no Sprint 2 dependency.
  `scripts/zellij-new-tab.sh`, `Alt Shift z`, and the Sprint 2 entry gate are unchanged; no controller, lease, custom PTY, 7h, 4d, or public CLI is introduced.

### Summary

The AWK transformer is a real operator-blocking compatibility defect, but it
is not a Sprint 2 dependency: a valid user config can make the current entry
fail closed before any mutation. No existing Go implementation was found. The
smallest proven replacement is the already-resolved lossless Rust KDL parser,
kept behind the existing shell entry and its atomic write boundary.

## Stage Report: ideation (cycle 2)

- DONE: Preserve valid KDL without the AWK brace parser.
  Captain direction replaces the parser-rewrite premise: the selected-worktree
  path leaves standing KDL unparsed and unwritten; native Zellij 0.44.3
  `setup --check` accepted `WriteChars "{"` directly on 2026-07-13.
- DONE: Define selected-worktree CLI creation with no standing config/layout mutation.
  The chosen seam is the existing raw `action new-tab --layout-string` command
  with a disposable selected-WASM layout; global `Alt Shift z` is documented
  only as its owner's fixed-layout shortcut.
- DONE: Prove the chosen boundary with focused offline and isolated-profile evidence.
  The revised plan begins with a quoted-brace, hash-checked tmux-hosted
  profile drill, then covers offline success/failure/TERM behavior and native
  selected-tab state without custom PTY machinery.

### Summary

The Rust-helper direction is superseded, not implemented. Fq now removes the
unnecessary global-write step from the selected-worktree journey: Zellij owns
parsing the operator's read-only profile, while the script passes only its
own rendered layout inline and proves the resulting tab through native state.
The preceding ideation report remains historical evidence of the rejected
parser-replacement route; the sections above and this cycle are controlling.

## Stage Report: implementation

- DONE: Selected-checkout entry creates exactly one verified stable-ID tab with the selected checkout's canonical WASM URL while standing config/layout hashes remain unchanged on success, failure, and TERM.
  Commits `4a506d8` and `95cba63`; focused shell 6/6 passed, and the isolated Zellij-in-tmux journey passed with native pane/tab/layout state plus isolated and standing hashes.
- DONE: The direct path no longer depends on the AWK transformer or a pre-existing Zaphod route; global Alt Shift z and persistent Alt / policy remain untouched and honestly documented.
  `scripts/zellij-new-tab.sh` now uses native `setup --check` and one inline `new-tab`; commits `583abe5`, `010171e`, and `e238f7a` document fixed global key-policy ownership without adding a runtime route.
- DONE: Red/green unit and isolated Zellij-in-tmux journey evidence is complete, followed by a cleared exact-tip quick review and passing exact-head code_completion synthesis evidence.
  Red at `4a506d8`: `Zaphod config transformer not found` then `FAIL: selected-checkout fresh-tab entry failed`; green was 6/6 shell, 135/135 `cargo test`, `cargo check --tests`, and the tmux journey.
- DONE: TERM evidence proves standing KDL stays read-only before cleanup and always reaps the blocked action.
  Commits `7419848`, `ea61e22`, and `947582c` capture in-flight hashes, convert unreadable files to a sentinel, then TERM, release, reap, and assert; final focused suite passed 6/6.
- DONE: Exact-tip quick cost gate is clear.
  Quick parent `185` reviewed head `e238f7a90a62b068772742ccd5ac4a0fc6071727` with panel `quick`, one successful required member, PASS, and no findings.
- DONE: Exact-head code_completion synthesis is authoritative and passing.
  Parent `189` reviewed `3b27e202176740ed976aa94aa8aa7cc91ee66118..e238f7a90a62b068772742ccd5ac4a0fc6071727`; `correctness` job `186`, `journey` job `187`, and `proof` job `188` each ran once and passed; parent verdict PASS.
- DONE: All review findings have dispositions.
  TERM cleanup, stable foreign-tab identity, and stale docs were fixed; persistent `Alt /` and `Alt .` retargeting requests were rebutted as forbidden scope and explicitly accepted by quick parents `164` and `185`.

### Summary

The selected-checkout command now renders only its repository layout into a
disposable value, creates one inline tab, verifies the returned stable tab ID
and exact WASM URL, and starts the existing private subscriber. Operator KDL
remains byte-identical, global key policy remains global, and all focused,
native, journey, quick-review, and completion-panel evidence passes.

## Stage Report: validation

- DONE: Verify stored Roborev parent 189 against the frozen merge-base..e238f7a range, current head, code_completion panel, required-member execution, PASS verdict, and recorded finding dispositions without rerunning the unchanged panel.
  Parent `189` covers `3b27e202176740ed976aa94aa8aa7cc91ee66118..e238f7a90a62b068772742ccd5ac4a0fc6071727`; correctness `186`, journey `187`, and proof `188` each ran once and passed; parent PASS; quick dispositions `164`/`185` PASS.
- DONE: Independently reproduce AC-1 through AC-4, including exact selected-WASM stable-tab state and byte-identical standing KDL across success, injected failures, and TERM using the isolated Zellij-in-tmux harness.
  Focused shell `6/6` and the real tmux/Zellij smoke passed; native panes/tabs/layout showed exactly one tiled selected-WASM resident at the returned stable ID, and all isolated/standing hashes matched.
- DONE: Run a throwaway-checkout refutation audit against false positives/negatives, cleanup, caller impact, and semantic drift; prepare the captain demo/gate artifact with explicit per-AC verdicts.
  Wrong-tab same-URL and duplicate-target attacks failed closed, a correct target plus foreign duplicate passed, cleanup removed all disposable state, Rust passed `135/135`, and `gates/zellij-config-activation-validation.md` contains the direct-script-only demo.
- SKIPPED: Exercise the operator's standing WORK profile or press Alt keys there.
  The dispatch forbids both; the captain's deferred complex-layout Alt/chrome issue is outside fq's approved no-write entry scope.

### Summary

Fresh validation found no surviving defect at exact head `e238f7a`. Offline
AC-1 through AC-4 pass independently, stored Roborev evidence is current and
complete, and the gate is ready for the captain's direct-script-only demo.
