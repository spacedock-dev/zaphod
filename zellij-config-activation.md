---
id: fqjswvmd93vek5y12zemf1k2
title: Activation preserves valid Zellij KDL
status: implementation
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

### Feedback Cycles

- **Cycle 1 — 2026-07-13: REVISED at validation, routed to implementation.**
  The captain requested a chat-guided demo because the Subspace TUI float does
  not support clipboard copy/paste, and clarified that the intended live
  journey includes the AgentsView subscription rather than only fresh-tab and
  standing-KDL evidence. The live direct-entry run created stable tab `8` with
  the exact selected-worktree rail and left standing KDL byte-identical, but
  its sidecar log reported `127.0.0.1:8080: connection refused`; the shell pane
  therefore remained `unknown . unknown` and did not prove a subscribed
  session row. Rework the demo packet so the first officer walks it through in
  chat against a real AgentsView-backed agent session, without pressing Alt
  keys or changing standing config/layout, then return it to the same validator
  for independent review and a new gate.

- **Cycle 2 — 2026-07-13: REVISED during the convergence live drill, routed to implementation.**
  At frozen head `234ed30`, AgentsView health and sessions endpoints returned
  HTTP 200 and the external control-terminal entry created stable tab `8` with
  exact candidate rail pane `156`, terminal pane `117` in the selected checkout,
  and a present per-entry recipient token. No permission prompt appeared and
  standing `config.kdl` / `layouts/zaphod.kdl` hashes remained byte-identical,
  but the entry emitted no success tuple and its sidecar terminated with
  `recipient-ready timeout for stable tab 8`; the shell-only rail correctly
  rendered no empty `AGENTS` section. Reproduce the real attached-WORK boundary
  and fix why the rendered token-matched rail does not return the
  `agent-event-ready` CLI-pipe acknowledgement. Do not work around it with a
  manual sidecar or by starting an agent before recipient readiness.

- **Cycle 3 — 2026-07-14: REJECTED by the authorized convergence panel, captain routed to implementation.**
  Roborev `code_completion` parent `497` reviewed the exact range
  `3b27e202176740ed976aa94aa8aa7cc91ee66118..faacfc11e6f5663ad7c1dfc5600c1f2739c2b7f8`;
  correctness `494`, journey `495`, and proof `496` each ran once and failed.
  Fix the three Medium findings: retry a native empty pane inventory (`[]`)
  before declaring the target lost; publish completed SSE-token activity in
  the same critical section as fragment and pending state; and request
  `ReadCliPipes` only for rails configured with a non-empty recipient token so
  tokenless installed-layout rails preserve their prior permission journey.
  These are bounded implementation defects within the existing task contract,
  so no reframe is required. Return a frozen head and green evidence to the
  convergence gate; do not launch another `code_completion` panel without a
  new captain approval.

- **Cycle 4 — 2026-07-14: REVISED during the fresh-session live drill, captain routed to implementation.**
  In fresh Zellij session `FQ-FRESH`, direct entry at frozen head `9808800`
  completed its permission handshake and delivered the first marked AgentsView
  row (`fq-initial-1783981287`). AgentsView then indexed the second marked
  session (`fq-sse-1783981287`), but the sidebar remained at four total rows and
  sidecar PID `49088` exited at `2026-07-14 06:28:21 +0800`. Its log recorded
  `target-lost: malformed native pane state: invalid character 'l' looking for
  beginning of value`; a subsequent read-only probe returned valid JSON 100/100
  times. Diagnose and preserve the exact malformed bytes or provenance, then
  make the live refresh lifecycle tolerate only evidence-backed transient native
  replies while keeping persistent malformed state and real tuple loss fail
  closed. Prove that a post-readiness `data_changed` refresh survives the bounded
  transient, renders the second session, and leaves the sidecar alive; prove the
  terminal cases still exit and clean up. This remains within the advertised
  subscription end value, so no task reframe is required. Do not launch another
  `code_completion` panel without a new captain approval.

- **Cycle 5 — 2026-07-14: REVISED during the captain lifecycle smoke, routed to implementation.**
  At frozen head `25d950c`, the captain ran
  `tests/zellij-subscription-lifecycle-smoke-test.sh` first from a loaded Zellij
  panel and received `FAIL: isolated profile unexpectedly started on the
  selected checkout rail`, then from an ordinary terminal outside Zellij and
  received `FAIL: foreground entry did not return a stable tab ID`. Both fail
  before the promised second-row lifecycle proof, while cleanup deletes the
  captured native reply. Make the smoke isolate inherited Zellij client state
  itself, work from both supported caller environments, and preserve bounded
  stdout/stderr provenance when stable-tab discovery fails. Verify whether the
  missing tab ID is caused by the foreground harness omitting production entry
  context or by an invalid assumption about `new-tab` stdout; use one proven
  stable-ID mechanism across the production and diagnostic paths. Re-run both
  captain-equivalent invocations and the full lifecycle matrix before offering
  another manual test. This is proof-harness repair within the existing end
  value; no task reframe and no new `code_completion` panel are authorized.

## Stage Report: implementation addendum (review-convergence pause)

- DONE: Freeze the revised implementation at
  `234ed30ef8aa916b8d2311486907a57513123b71` without absorbing the unrelated
  main-branch review-policy documentation change. The product worktree is
  clean and no further code_completion panel was launched after the captain's
  convergence pause.
- DONE: Close every exact-tip quick-review finding and preserve the evidence.
  Quick `419` identified an overall recipient-deadline overrun and quick `428`
  identified descendant-held stdout after cancellation; both received focused
  regressions and fixes. Exact-tip quick `430` reviewed frozen head `234ed30`
  and passed with no findings.
- DONE: Re-run the full local and live evidence at the frozen head.
  `go test ./...`, `go vet ./...`, focused readiness/recipient cases repeated
  ten times, Rust `cargo test` (136/136), `cargo check --tests`, the entry suite
  (9/9), shell/doc syntax checks, and the `main...HEAD` diff check all passed.
  The real tmux-hosted fresh-entry smoke and real two-rail stable-recipient
  smoke also passed, with disposable cleanup.
- DONE: Preserve the release-blocking invariants in the revised path.
  Direct inline tab creation does not mutate standing KDL; stable tab plus
  per-entry token prevents cross-rail delivery; readiness requires recipient
  and snapshot acknowledgement, catch-up, and a quiet boundary spanning body
  read, split, scanned, and consumed state. Source EOF/error blocks readiness,
  startup bounds include descendant-held stdout, failure/TERM reaps the managed
  sidecar, and tuple handoff must complete.
- DONE: Record the convergence disposition honestly.
  There are no known unresolved findings from exact-tip quick, local, or live
  evidence. Prior panel `406` findings are closed in code/tests, but panel
  consensus was deliberately not refreshed under the captain's pause.
- FOLLOW-UP: When authorized, rerun code_completion on exact frozen head
  `234ed30`; acceptance is all required members PASS with the reviewed ref
  resolving to that head.
- FOLLOW-UP: Optionally harden descendant lifecycle cleanup with a fixture that
  backgrounds a long-lived process and records its PID; acceptance is bounded
  return plus proof that the recorded descendant is no longer alive.
- FOLLOW-UP: Optionally run an interactive permission drill in pristine Zellij
  config/data directories; acceptance is approving ReadCliPipes within 18
  seconds and observing baseline and post-baseline AgentsView markers without
  a manual sidecar or standing writes.

### Summary

The revised implementation is frozen at `234ed30`, exact-tip quick `430` and
all local/live checks pass, and no unresolved implementation finding is known.
Broader terminal/OS permutations, the optional detached-descendant reap, and
the pristine interactive permission drill remain bounded follow-ups pending
captain disposition; no new completion panel was started during the pause.

## Stage Report: implementation (cycle 2)

- DONE: Reproduce and explain the frozen-head WORK timeout without further live mutation.
  Stable tab `8`, exact recipient token, and the synthetic event reached the
  intended rail, but the ready probe returned status 0 with empty output.
- DONE: Identify the permission mechanism behind the empty acknowledgement.
  Zellij's raw-path permission cache contained the older grants but omitted
  `ReadCliPipes`; event admission ran, while `cli_pipe_output` could not return
  `ready`. The native expanded-permission prompt remained unfocused.
- DONE: Identify the mixed-version broadcast mechanism.
  A legacy rail on another WORK tab still accepted the session-wide
  `agent-event` name, so sender arguments could not isolate old receivers.
- DONE: Preserve the live boundary after diagnosis.
  No further WORK tab was created, focused, or switched during implementation;
  all green evidence used disposable isolated tmux and Zellij sessions.
- DONE: Add private versioned per-entry agent pipes with test-first evidence.
  Red `0debecf` failed three Rust admission/render cases and two Go private-name
  cases; green `45bd13d` derives ready, snapshot, and event names from the
  fresh recipient token and leaves legacy names inert.
- DONE: Add the real cached-permission upgrade boundary with test-first evidence.
  Red `15db369` timed out at the owned sidecar because the old cache could not
  expose an actionable prompt; green `8014ddd` retains the verified plugin ID
  and focuses that exact pane before the bounded startup handshake.
- DONE: Cover the Zellij 0.44 redraw race exposed after literal approval.
  Red `93c3c70` made one successful `list-panes` call return empty output and
  failed as malformed state; green `06e0bb0` retries only blank successful
  replies three times and still fails closed on `[]`, malformed data, or a
  tuple mismatch.
- DONE: Synchronize the adversarial two-rail stable-tab proof.
  Both rails deliberately share one private token; a bystander-addressed
  barrier proves it processed the shared pipe before the target marker is
  asserted absent. Commits `9c73547` and `faacfc1` close quick findings.
- DONE: Verify the complete exact-head product packet.
  Rust passed 137/137 plus `cargo check --tests`; Go passed `go test ./...` and
  `go vet ./...`; the entry shell suite passed 9/9.
- DONE: Verify all disposable real-Zellij boundaries.
  Pre-granted entry, old-cache literal-`y` upgrade, and shared-token two-rail
  smokes passed with native state, visible rows, persisted `ReadCliPipes`, and
  complete cleanup.
- DONE: Preserve operator-owned KDL.
  Standing config hash remains `8ce2a42dcd962a7f069e3c66a76ed5b6e0296e4300db3024e8abbe69bc81a196`;
  standing layout hash remains `f100474193d9c914eb59c01cc1252bf1ee6d53ece156d3d654b64eab13ca8d6e`.
- DONE: Clear the exact-tip quick gate and stop at the convergence boundary.
  Quick job `480` reviewed `faacfc11e6f5663ad7c1dfc5600c1f2739c2b7f8`
  and passed with no findings. No `code_completion` panel was launched.

### Summary

Direct entry now isolates agent traffic, exposes native permission expansion
on the verified rail, survives redraw races, and leaves standing KDL unchanged.

## Stage Report: implementation (cycle 3)

- DONE: Retry blank and native-empty pane inventories without masking terminal target loss.
  Red `0065526`: `transient native [] list-panes reply was terminal:
  target-lost: expected one resident rail in stable tab 73, found 0`.
- DONE: Bound the native-empty retry and preserve terminal semantics.
  Green `7ae4c50` retries blank or decoded `[]` at most three times; persistent
  `[]`, malformed JSON, command failure, cancellation, and tuple loss remain terminal.
- DONE: Publish SSE fragment, pending, and completed-token activity atomically at readiness.
  Red `88c3a0b`: `token publication gap allowed readiness: "ready\n"`.
- DONE: Close the deterministic ScanLines-token publication gap.
  Green `c42b4d3` publishes fragment state, clears transport pending, increments
  completed-token activity, and records EOF under one `streamBoundary` lock.
- DONE: Preserve tokenless rail permissions while token-bound entry still obtains ReadCliPipes.
  Red `acda8df`: Rust `E0425` reported `cannot find function
  permissions_for_config in this scope` at all three permission assertions.
- DONE: Make the permission vector depend on a non-empty recipient token.
  Green `da26a89` keeps the five installed-layout grants for absent/empty tokens
  and adds only `ReadCliPipes` for token-bound direct entry.
- DONE: Complete the semantic adversarial pass over all three findings.
  Blank→valid, `[]`→valid, persistent target loss, fragmented reads, queued
  tokens, and the exact publication gap passed ten repetitions.
- DONE: Verify native and real operator boundaries at exact head.
  Go passed uncached full tests plus vet; Rust passed 138/138 plus check; entry
  passed 9/9; artifact, pre-granted, permission-upgrade, and two-rail smokes passed.
- DONE: Prove tokenless and token-bound permission behavior.
  The Rust vector test covers absent, empty, and non-empty tokens; the isolated
  upgrade smoke persisted `ReadCliPipes` after one literal `y` and rendered rows.
- DONE: Preserve standing KDL and disposable cleanup.
  Config/layout hashes remain `8ce2a42d...a196` and `f1004741...d6e`; every
  isolated Zellij session, tmux server, sidecar, and temporary root was removed.
- DONE: Clear exact-tip quick review and honor the convergence gate.
  Quick job `521` passed at `980880099d59c57a154fe611cfe465f9dce51139`;
  dispatch prohibited and no new `code_completion` panel was launched.

### Summary

The three panel-497 findings are fixed with independent red/green commits.
Readiness and target identity remain fail closed, while tokenless installed
rails avoid the direct-entry-only CLI-pipe permission.

## Stage Report: implementation (cycle 4)

- DONE: Preserve bounded provenance for malformed native pane replies.
  The failed live sidecar log contained only `invalid character 'l'`; its raw
  reply was not retained. Zellij logged a concurrent dump-layout timeout, so a
  delayed `layout {…}` reply is evidence-backed but remains an inference.
- DONE: Add command, attempt, byte length, and escaped stdout/stderr prefixes.
  Red `55a8cc4`: `malformed error = "target-lost: malformed native pane state:
  invalid character 'l' looking for beginning of value", want provenance
  "command=list-panes"`; green `02d04ae` bounds each prefix to 256 bytes.
- DONE: Reproduce the post-readiness refresh exit across the full Go lifecycle.
  Red `928093a`: `subscriber exited before second marked session: target-lost:
  malformed native pane state: invalid character 'l' looking for beginning of
  value; command=list-panes attempt=1/3 stdout_len=17
  stdout_prefix="layout { pane; }\n" stderr_len=0 stderr_prefix=""`.
- DONE: Retry only the observed complete wrong-action record.
  Green `5926931` recognizes bounded UTF-8 output beginning `layout {` and
  ending `}`, retries at most three times, and never accepts it as pane JSON.
- DONE: Keep persistent and unrelated malformed replies fail closed.
  The persistent-layout regression requires `attempt=3/3`; the existing
  oversized layout-prefix-plus-garbage case still fails on attempt one, while
  command failure, cancellation, persistent empty inventory, and tuple loss
  retain their terminal paths.
- DONE: Make successful recovery observable rather than swallowing evidence.
  Red `f33e67a`: `surviving transient diagnostic = "", want
  "transient-native-pane-reply"`; green `976faba` writes the same bounded
  provenance to the sidecar diagnostic stream before retrying.
- DONE: Prove `data_changed -> refresh -> plugin delivery` with a second row.
  The integrated test injects one layout reply, then delivers session 2 with
  `fq-second-marker`, asserts the exact private stable-tab/token pipe tuple,
  and proves the subscriber remains alive until owned cancellation.
- DONE: Validate a foreground diagnostic before automatic handoff.
  The first disposable run exposed a missing isolated socket environment with
  bounded ANSI/session-list provenance; after correcting the harness, direct
  `target/zaphod subscribe` rendered `SMOKE_INITIAL_ROW`, then
  `SMOKE_SECOND_ROW`, and stayed alive.
- DONE: Validate the production direct-entry handoff on the same lifecycle.
  `tests/zellij-subscription-lifecycle-smoke-test.sh` runs foreground then
  automatic modes; both passed with distinct rows, live PIDs, routing checks,
  standing-KDL preservation, and complete disposable cleanup.
- DONE: Complete exact-head native and entry verification at `25d950c`.
  Go test/vet passed; Rust passed 138/138 plus `cargo check --tests`; entry
  passed 9/9; artifact, permission-upgrade, and two-rail smokes all passed.
- DONE: Preserve operator-owned KDL.
  Config hash is `8ce2a42d...a196`; layout hash is `f1004741...d6e`.
- DONE: Clear the authorized exact-tip quick gate and stop at the boundary.
  Quick parent `715` reviewed exact head `25d950ca08ccff935b214933291fa9ecf569761e`,
  panel `quick`, member `714` ran once and passed, parent verdict PASS, no
  findings. Per captain dispatch, no new `code_completion` panel was launched.

### Summary

The sidecar now survives only the evidenced complete layout-reply misroute,
records bounded diagnostics even when recovery succeeds, refreshes and renders
the second indexed session, and remains fail closed for persistent malformed
state and real target loss. Foreground diagnosis and automatic handoff both
pass in isolated real Zellij/tmux sessions without changing standing KDL.

## Stage Report: implementation (cycle 5)

- DONE: The lifecycle smoke self-isolates inherited Zellij client/session state and starts the same disposable foreign profile when invoked both inside and outside Zellij.
  Captain red at `25d950c` was `FAIL: isolated profile unexpectedly started on
  the selected checkout rail`; deterministic red `c28ab6b` was `FAIL:
  inherited Zellij client identity reached isolated smoke: ZELLIJ`.
- DONE: Clear client identity before disposable server/control setup.
  Green `1b1d972` captures then unsets `ZELLIJ`, `ZELLIJ_SESSION_NAME`, and
  `ZELLIJ_PANE_ID`; every tmux/server/control child also uses explicit `env -u`.
- DONE: Foreground and production entry use a proven stable-tab identity mechanism, and any discovery failure reports bounded captured stdout/stderr before cleanup.
  Captain red outside Zellij was `FAIL: foreground entry did not return a
  stable tab ID`; focused red `bf9d9b6` ended `sidecar-target-unready` when
  successful `new-tab` stdout was empty.
- DONE: Replace the stdout assumption with exact native inventory identity.
  Green `3c74006` shares one validator across production and foreground: the
  before inventory must remain present and exactly one nonnegative integer tab
  ID must be added in the complete after inventory.
- DONE: Preserve bounded discovery evidence before temporary cleanup.
  Red `f4242e5` was `FAIL: ambiguous discovery omitted new-tab status
  provenance`; green `53d0708` reports status, byte lengths, and JSON-escaped
  256-byte stdout/stderr prefixes for new-tab and latest list-tabs replies.
- DONE: Exercise the complete adjacent identity matrix.
  Exact-one addition returned `73`; no addition, lost-and-added, two additions,
  duplicate IDs, malformed JSON, and fractional IDs all failed closed.
- DONE: Make the inside-caller journey meaningful at the production boundary.
  Quick `741` correctly found that the inside label had been sanitized before
  entry. Red `a56e666` was `FAIL: version probe inherited loaded Zellij client
  identity`; green `d659c36` injects the captured loaded-client tuple into the
  entry call, then production resolves `--session` and clears it before native
  version/setup/inventory/action and sidecar child calls.
- DONE: Captain-equivalent inside/outside invocations complete initial-row and distinct post-readiness second-row delivery with live subscriber, cleanup, unchanged standing KDL, and exact-head quick PASS; no code_completion panel is launched.
  `tests/zellij-subscription-lifecycle-smoke-test.sh` passed outside-terminal
  foreground and loaded-panel automatic entry; each rendered
  `SMOKE_INITIAL_ROW`, then `SMOKE_SECOND_ROW`, and kept its subscriber alive.
- DONE: Re-run the complete exact-head verification packet.
  Entry passed 12/12; Go test/vet passed; Rust passed 138/138 plus check;
  artifact, permission-upgrade, lifecycle, and two-rail smokes passed.
- DONE: Preserve owned cleanup and operator KDL.
  The interrupted four-server stress probe cleaned its active disposable root;
  the exact two-environment packet cleaned all sessions, processes, sockets,
  and roots. Standing hashes remain `8ce2a42d...a196` and `f1004741...d6e`.
- DONE: Clear the replacement exact-tip quick gate.
  Quick `747` reviewed `d659c36dbbbc716b2b1e158f108df470eaa2a639`;
  member `746` and parent passed, no findings. No completion panel was launched.

### Summary

Both captain failures have durable regressions. Disposable setup cannot inherit
client identity; validated inventory owns tab identity; both captain journeys
deliver the second SSE row alive through the sanitized production boundary.
