# Validation: Canonical install and isolated worktree test profile

Entity: `docs/agent-rail-dev/.spacedock-state/canonical-install-and-worktree-test-profile.md`

Implementation worktree: `.worktrees/spacedock-ensign-canonical-install-and-worktree-test-profile`
at raw SHA `7cfd3a9b0a9bdf81ecb6c95fa806956799c006f1` (clean before and after validation).

Refutation checkout: fresh `git clone --no-hardlinks`, detached at the same raw
SHA under `/tmp/zaphod-validation-audit.*`; never the implementation worktree.

## Offline AC verdicts

| AC | Verdict | Independent evidence |
|---|---|---|
| AC-1 | PASS | `./tests/zellij-install-profile-test.sh all` reported linked refusal before writes and a successful primary control; destination sentinel bytes survived the linked attempt. |
| AC-2 | PASS | Foreign, mixed, missing-keybind, and missing-rail cases failed without changing their sentinels; coherent and commented-foreign controls installed; `zellij setup --check` passed. |
| AC-3 | PASS | The live lifecycle group validated the candidate URL from `dump-layout`, one explicit-cwd terminal, profile-local config/layout/data, cleanup after normal/TERM/INT, unchanged outside hashes, and absent `zlc-`/`zwp-`/`zpc-` sessions. |
| AC-4 | **REFUTED** | In the fresh detached checkout, the suite passed its first five groups, then failed `timed out waiting for profile value PROFILE_ROOT`. The test waits 10s (`tests/zellij-install-profile-test.sh:28-40`); the profile builds before emitting metadata (`scripts/zellij-worktree-test-profile.sh:75-82`); the clean build took 1m42s. The lifecycle group passed after explicit warming. |
| AC-5 | PASS | The doc diff separates primary-only install from candidate testing, gives setup/inspect/cleanup commands, names `list-panes` plus `dump-layout`, prohibits worktree install, and records the task→j5→eh→7v→yb→hj→pz order and pz gates. |

Independent supporting suites: shell behavior 6/6 in the warm implementation
worktree; Rust 132/132; `cargo check --tests` clean; Go 35/35; `go vet` clean;
shell syntax clean. With Zellij absent from `PATH`, the profile group exited 1
with `zellij 0.44.3 is required`; no silent skip remained.

## Refutation audit

- **Identity false positive:** foreign, mixed, missing-keybind, and missing-rail
  inputs all failed closed and preserved bytes. The attack did not survive.
- **Identity false negative:** a coherent config with a commented foreign
  `MessagePlugin` installed. The parser ignored the disabled example. The
  attack did not survive.
- **Rollback and signal cleanup:** config mutation during postflight, TERM
  during postflight, and TERM immediately after rename all restored the exact
  sentinel bytes. Validation sessions were absent afterward. The attacks did
  not survive.
- **Leaked profile state:** normal, TERM, and INT lifecycle runs removed the
  profile root and unique session; no `zlc-`, `zwp-`, or `zpc-` sessions
  remained. The attack did not survive.
- **Caller impact / indexing:** missing arguments exited 2 with usage;
  nonexistent cwd exited 1; the space-bearing cwd fixture retained its exact
  physical path. No unchecked positional access or unsafe caller mutation
  appeared.
- **Semantic drift:** process-substitution diff showed the shared renderer's
  output is byte-identical to the pre-diff `sed` substitution for the same URL.
  The attack did not survive.
- **SURVIVING attack — cold-checkout readiness:** the fresh suite's fixed
  10-second poll expired during the required build. This makes a warmed tree a
  hidden prerequisite and prevents a fresh validator from reproducing AC-3/4
  with the committed command. Route back to implementation.

## Cheap live-drill proof

The warm process lifecycle test starts a real Zellij 0.44.3 session with the
profile's emitted argv, reads `list-panes --json -a -g -t`, reads a resident
control through `dump-layout`, validates only the candidate URL, and tears down
the session. This proves the CLI, profile-local data/config roots, and live
oracle cheaply. It does not replace CL's real keypress and attached-TUI
observations.

## Exact CL demo script for AC-6

Run only after implementation fixes the cold-checkout timeout and a fresh clone
passes the whole process suite.

1. In terminal A, start the implementation profile and keep it attached:

       cd /Users/clkao/git/zaphod/.worktrees/spacedock-ensign-canonical-install-and-worktree-test-profile
       ./scripts/zellij-worktree-test-profile.sh --cwd "$PWD"

   Record the printed `PROFILE_ROOT`, `SESSION_NAME`, `CANDIDATE_COMMIT`, and
   `CANDIDATE_URL` in terminal B. Do not substitute a global path.

2. In terminal B, start the resident control from that same profile:

       CONTROL="zpc-ac6-$(date +%s)"
       zellij --config-dir "$PROFILE_ROOT/config" --data-dir "$PROFILE_ROOT/data" \
         --session "$CONTROL" --new-session-with-layout zaphod

   Approve the profile-local permission prompt if Zellij shows it.

3. In terminal C, capture live state before the keypress:

       ZELLIJ_SESSION_NAME="$CONTROL" zellij --config-dir "$PROFILE_ROOT/config" \
         --data-dir "$PROFILE_ROOT/data" action list-panes --json -a -g -t > /tmp/ac6-before.json
       ZELLIJ_SESSION_NAME="$CONTROL" zellij --config-dir "$PROFILE_ROOT/config" \
         --data-dir "$PROFILE_ROOT/data" action dump-layout > /tmp/ac6-before.kdl
       jq '[.[] | select(.is_plugin and (.plugin_url | endswith("/zellij-sidebar.wasm")))] | {count:length, ids:map(.id), urls:map(.plugin_url)}' /tmp/ac6-before.json

   CL should see `count: 1`, one plugin ID, and only `CANDIDATE_URL`.

4. CL presses the profile's real `Alt /` once in the resident control. Capture
   the same files as `/tmp/ac6-after.json` and `/tmp/ac6-after.kdl`, then run the
   same `jq` command on the after file. PASS requires one sidebar before and
   after, the same ID, and only `CANDIDATE_URL` in both live dumps. A second ID
   or foreign URL fails AC-6.

5. Clean the control and attached profile:

       zellij delete-session --force "$CONTROL"
       zellij delete-session --force "$SESSION_NAME"

   Terminal A must exit, report no global-byte change, and remove
   `PROFILE_ROOT`.

## Exact CL demo script for AC-7

Precondition: land this profile on current main, then rebase the j5 candidate so
both checkouts contain the identical profile script. Run this protocol first
on current main (expected red), then on the rebased j5 candidate (expected
green). For each checkout:

1. Snapshot standing files and launch the profile from terminal A:

       CHECKOUT=/absolute/path/to/checkout
       shasum -a 256 ~/.config/zellij/config.kdl ~/.config/zellij/layouts/zaphod.kdl
       cd "$CHECKOUT"
       ./scripts/zellij-worktree-test-profile.sh --cwd "$PWD"

2. In terminal B, use the printed inspection commands to save the baseline:

       ZELLIJ_SESSION_NAME="$SESSION_NAME" zellij --config-dir "$PROFILE_ROOT/config" \
         --data-dir "$PROFILE_ROOT/data" action list-panes --json -a -g -t > /tmp/ac7-before.json
       ZELLIJ_SESSION_NAME="$SESSION_NAME" zellij --config-dir "$PROFILE_ROOT/config" \
         --data-dir "$PROFILE_ROOT/data" action dump-layout > /tmp/ac7-before.kdl
       jq '{terminals:[.[]|select(.is_plugin|not)|.id], rails:[.[]|select(.is_plugin and (.plugin_url|endswith("/zellij-sidebar.wasm")))|{id,plugin_url}]}' /tmp/ac7-before.json

   Before the keypress, CL should see one terminal ID, zero rails, and one
   top `zellij:tab-bar` plus one bottom `zellij:status-bar` in the dump.

3. CL presses real `Alt /` once in terminal A. Repeat step 2 into
   `/tmp/ac7-after.json` and `/tmp/ac7-after.kdl`.

   - Current main must record the known red baseline: an extra terminal.
   - The j5 candidate passes only if the original terminal ID remains, no second
     terminal exists, exactly one rail uses `CANDIDATE_URL`, and the canonical
     top/bottom chrome remains.

4. Delete the printed session. After terminal A exits, repeat the `shasum`.
   PASS also requires identical hashes, an absent profile root, and an absent
   session for both baseline and candidate runs.

## Demo outcome

AC-6 and AC-7 were not run. CL supplied no attached-session observation in this
validation round, and validation claims none. The offline cold-checkout failure
blocks the gate before CL's time should be spent.

## Cycle 2 revalidation at 8c2fe5e

The new detached audit clone began without `target/`. Its fixture build took
2m05s, after which `./tests/zellij-install-profile-test.sh all` passed all
eight groups, including its own second target-free detached lifecycle. Rust
passed 132/132 and `cargo check --tests`; Go passed 35/35 and vet; removing
Zellij from `PATH` exited 1 with `zellij 0.44.3 is required`.

### Boundary attacks

- **Slow but live:** the 2m05s target-free build completed before readiness
  expired. The attack did not survive.
- **Dead launcher:** a fixture that exited 23 failed in 1s with
  `profile exited before printing PROFILE_ROOT`. The attack did not survive.
- **Interrupted cleanup and outside bytes:** normal, TERM, and INT lifecycle
  groups removed profile/session state and preserved both sentinel hashes.
  The attack did not survive.
- **Full default expiry and process-group teardown — REFUTED:** a live fixture
  withheld metadata, started a Zellij session, and spawned a child that ignored
  TERM. Expiry returned the exact timeout after 210s wall clock. Parent, profile,
  and session were absent; config SHA stayed `1f7fa0d4…` and layout SHA stayed
  `c487cae2…`. The child remained live (`CHILD_LIVE=1`, PID 10165) until the
  validator killed it manually.
- **Root cause:** `cleanup_profile_process` sends TERM to the recorded group at
  `tests/zellij-install-profile-test.sh:18-23`, waits only for
  `PROFILE_LAUNCHER_PID` at :24-27, and enters group KILL at :28-33 only if
  that parent still lives. The committed timeout fixture (:607-624) has no child,
  and its assertions (:654-659) check only parent/profile/session.
- **Nominal duration:** 1,800 polls with per-poll transcript parsing measured
  210s, not 180s. The wait remains bounded, but the implementation report's
  “180 seconds” is an attempt-count approximation rather than observed wall
  time.

### Interactive reconciliation

Every agent-reproducible prerequisite is green: candidate URL generation,
profile-local config/layout/data, explicit-cwd one-terminal baseline, live
`list-panes`/`dump-layout`, signal cleanup, and unchanged outside hashes.
CL has not pressed real `Alt /`; AC-6 remains unrun.

AC-7 cannot honestly run before this task lands. The documented order is
v9-before-j5: land the canonical/profile change, rebase j5 so both current main
and candidate contain the identical profile, then run the existing two-checkout
script at j5's gate. Required evidence remains before/after terminal IDs and
counts, rail count/URL, chrome dumps, global hashes, and absent roots/sessions.
No baseline or candidate observation is claimed here.

### Cycle 2 verdict

**REJECTED.** The cold-readiness fix closes cycle 1, but process-group cleanup
does not remove a surviving descendant. Add a TERM-ignoring descendant to the
timeout regression, verify the group is empty after escalation, and report the
readiness bound as attempts or enforce a wall-clock deadline. AC-6 remains for
CL; AC-7 remains at the post-v9 j5 gate.

## Cycle 3 revalidation at 4957f6c

### Reproduced fixes and required verification

- The preserved cycle-2 RED was “timed-out readiness left disposable state:
  descendant” at 8c2fe5e. At 4957f6c, the focused timeout test passes: the
  TERM-ignoring child fails kill -0, and launcher/session/profile state is absent.
- A separate 60-second canary outside the profile group remained live through
  cleanup, proving escalation did not target an unrelated process.
- The dead/slow control passes: dead launcher failure is prompt, and a live
  launcher that emits metadata after 2 seconds succeeds inside 3.
- The complete shell run passes 10/10, including its own fresh detached checkout
  with target removed before the profile's mandatory build.
- Rust passes 132/132 and cargo check --tests; Go passes 35/35 and vet.
  Missing Zellij exits 1 with “zellij 0.44.3 is required”. Normal, TERM, and INT
  cleanup preserves global sentinels; no disposable session remains.

### Deadline refutation

The required per-poll-delay attack survives. A throwaway PATH shim delayed one
tr invocation by 2 seconds, while readiness used a 1-second timeout and a live
10-second launcher. Exact output:

    POLL_DELAY_STATUS=1 POLL_DELAY_ELAPSED=3
    REFUTED: one delayed poll overran the 1-second deadline by 2s

wait_for_profile_value computes the deadline at
tests/zellij-install-profile-test.sh:73-75, checks it at line 77, then performs
an unbounded external tr/awk read at line 78. The next time check cannot run
until that poll returns. The normal wall-clock regression at lines 696-725
exercises only fast local polls and therefore passes.

The exact-expiry control wrote PROFILE_ROOT at 1 second with timeout=1. It
returned timeout at elapsed=1, which matches an exclusive deadline and is not
the defect. The defect is blocking work admitted after the deadline check.

### Interactive gate evidence

All agent-reproducible prerequisites remain green: candidate identity,
profile-local config/layout/data, explicit-cwd single-terminal baseline, live
list-panes/dump-layout, signal cleanup, and unchanged outside bytes. AC-6 still
requires CL to press real Alt-/ in the resident control and capture one
unchanged sidebar ID with only CANDIDATE_URL.

AC-7 remains deferred by delivery order. After v9 lands, j5 must rebase, then CL
runs the existing two-checkout script: current-main red and j5 green, with
terminal IDs/counts, one rail at the candidate URL, canonical chrome, unchanged
hashes, and absent roots/sessions. No human result is claimed.

### Cycle 3 verdict

**REJECTED — ESCALATE.** Descendant cleanup is fixed, but the wall-clock
deadline excludes the duration of its own transcript poll. Because this is
feedback cycle 3, return the architectural choice to the captain: either bound
each transcript read by the remaining deadline or state and test a bounded-poll
contract instead of a hard wall-clock deadline.

## Cycle 4 revalidation at 16b4faa

### Targeted repair and parser audit

- The prior one-second/two-second-tr attack now returns within one second
  because readiness uses Bash built-in reads over the regular transcript.
- The exact integration wrapper returned in 6 seconds: one second of readiness
  plus the bounded five-second TERM grace. Parent, TERM-ignoring descendant,
  session, and profile root were absent; an unrelated canary remained live.
- A metadata line without a newline stayed invisible until the newline arrived,
  then returned the exact value. Replacing/truncating the file between polls
  returned the new complete value. A 1 MiB single line timed out in one second.
- A 32 MiB single-line stress probe remained inside one built-in read for more
  than 40 seconds and was interrupted by the validator. This is an extreme
  transcript shape, but it shows the time check remains between lines, not
  inside a line.

### Diagnostic-path refutation

The timeout diagnostic still performs up to 160 synchronous printf writes to
stderr. A FIFO reader held stderr open without consuming it; 160 lines of 2048
bytes exceeded the pipe buffer. Under a one-second readiness timeout:

    LARGE_1M status=1 elapsed=1
    DIAGNOSTIC status=141 elapsed=5

The diagnostic returned only when the reader exited after five seconds and the
writer received SIGPIPE. print_transcript_excerpt therefore violates the hard
return budget even though read_profile_value_until no longer invokes tr/awk.
The committed delayed-poll regression uses an empty transcript and regular-file
stderr, so it cannot expose this path.

### Required matrix

The complete shell suite passed 11/11, including its own fresh detached
target-free lifecycle. Rust passed 132/132 and cargo check --tests; Go passed
35/35 and vet. Missing Zellij exited 1 with the required message. Normal, TERM,
and INT lifecycle groups preserved outside hashes and removed sessions/roots;
no disposable session remained and the code worktree stayed clean.

### Interactive evidence

AC-6 remains an explicit captain-driven resident-control keypress: one sidebar
ID before and after real Alt-/, every live URL equal to CANDIDATE_URL. AC-7
remains deferred to j5 after v9 lands and j5 rebases; its evidence remains the
current-main red/j5-green terminal IDs, rail URL/count, chrome, hashes, and
absent roots/sessions. Neither human observation ran in cycle 4.

### Cycle 4 verdict

**REJECTED.** The targeted poll repair works, but timeout diagnostics can still
block beyond the same hard wall-clock budget. Bound or suppress diagnostic
writes after deadline (while preserving useful failure evidence), and add a
blocked-stderr regression. The 32 MiB line result is a secondary extreme-input
risk; the blocked diagnostic is the concrete acceptance blocker.
