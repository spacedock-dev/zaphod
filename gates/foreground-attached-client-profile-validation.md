# Validation: Foreground attached-client disposable Zellij profile

Entity: `foreground-attached-client-profile.md`

Implementation worktree: `.worktrees/spacedock-ensign-foreground-attached-client-profile`
at raw SHA `4320b2f9fadb27d500097b31dc85c76a083918fa` (clean before and after
validation).

Refutation checkout: a detached throwaway clone at the same SHA. It was never
the implementation worktree and was discarded after the audit.

## Offline AC verdicts

| AC | Verdict | Independent evidence |
|---|---|---|
| AC-O1 — raw PTY keys reach one live terminal | **REFUTED** | A focused lifecycle run passed once, but repeated real runs failed both before terminal discovery (`PTY_DRIVER_ERROR=expected exactly one terminal, found 0`) and after a foreground handoff (`canary was absent from live dump-screen for pane 0; completed dumps=243`). The one-shot canary is not reproducible. |
| AC-O2 — direct client owns the foreground PTY group before lease | PASS in successful runs | The driver observed `CLIENT_PID == getpgid(CLIENT_PID) == tcgetpgrp(master)` before lease consumption. One audit trace recorded `CLIENT_PID=70930` and `PTY_FOREGROUND_PGID=70930`; the focused lifecycle test passed this assertion before its normal/HUP/TERM/INT cleanup checks. |
| AC-O3 — ProfileLeaseV1 is immutable and bounded | PASS in successful runs | The focused lifecycle parsed the exact schema, paths, duplicated identities, primary PID/PGID, mode, and live digest. A throwaway writable-lease mutation was rejected with `completed profile lease retained a write bit`; its private session and root were then absent. |
| AC-O4 — normal cleanup and standing-file isolation | PASS in successful runs | `worktree-profile` passed normal `kill-session` cleanup: root, lease, and private session disappeared and standing config/layout hashes matched. A separate missing-standing-root probe reached `PROFILE_LEASE`, ended through private `kill-session`, and left `$root/absent` nonexistent. |
| AC-O5 — HUP, TERM, and INT clean up the foreground group | PASS in successful runs | The focused lifecycle passed all three signals, exact nonzero statuses, root/lease/session removal, and standing-file hashes. `profile-foreground-cleanup` also passed its forced launcher-loss group-reap path once. |
| AC-O6 — no TTY fails loudly before state creation | PASS | `./tests/zellij-install-profile-test.sh profile-no-tty` exited nonzero with `an attached terminal is required` and passed its no-root/no-session/no-lease checks. |

Commands independently run against the frozen worktree included:

```bash
./tests/zellij-install-profile-test.sh worktree-profile
./tests/zellij-install-profile-test.sh profile-foreground-cleanup
./tests/zellij-install-profile-test.sh profile-foreground-pgid-validity
./tests/zellij-install-profile-test.sh profile-no-tty
```

The first focused lifecycle run printed all three expected PASS lines. In a
three-run repeat, run 1 passed and run 2 instead ended with
`FAIL: timed out waiting for profile value PROFILE_PTY_READY`. The audit then
captured the more specific driver failures above. That retry failure is the
gate blocker: AC-O1 must be reproducible, not merely observable once.

## Fresh-checkout reproduction

A detached target-free clone at the same SHA ran the focused lifecycle command.
Its early-file and early-line lease fixtures passed, then the profile failed to
print `PROFILE_ROOT` within the driver's 180-second readiness limit while the
mandatory release build was still running. A later direct clean-build timing
attempt hit this validator's low disk-space ceiling, so this record does not
claim a compiler defect. It does show that the committed fresh-checkout command
is not reproducible in this validation environment without a warmed target.

## Refutation audit

- **Raw-input readiness — REFUTED.** The driver regards only `ECHO=off` as
  input readiness (`tests/zellij-profile-pty-driver.py:232-257`), writes the
  nonce once (`:303-309`), then retries only `dump-screen` (`:315-332`). A
  Zellij client can enter raw mode before its terminal pane/shell is available;
  the audit observed both zero terminals and a live pane that never displayed
  the nonce. This explains the AC-O1 flake.
- **False client identity — survived.** The publisher checks that the client is
  its own process-group leader and owns the controlling TTY
  (`scripts/zellij-profile-lease-publisher.py:49-56`); the driver independently
  repeats the PID/PGID/TTY check (`tests/zellij-profile-pty-driver.py:351-394`).
- **Early lease file or line — survived.** Both committed early-publication
  fixtures passed on every lifecycle attempt. A widened local bookkeeping
  window was investigated and withdrawn as a finding: its lease was written
  after the driver's independent session and kernel-PGID observation, even
  though before a later flag assignment.
- **Mutable lease — survived.** In a throwaway copy, changing the completed
  lease to retain a write bit produced the expected driver error. Its root and
  private session were absent afterward.
- **Cleanup and global state — survived in successful runs.** Normal,
  HUP/TERM/INT, forced-loss, and the missing-standing-root probe removed their
  disposable state and preserved standing config/layout bytes.

## Captain-live demo script for AC-I1

Do not run this demo until AC-O1 is stable in repeated focused and target-free
runs. CL, not this validator, drives the attached terminal.

1. In terminal A, run:

   ```bash
   cd /Users/clkao/git/zaphod/.worktrees/spacedock-ensign-foreground-attached-client-profile
   zellij --version
   ./scripts/zellij-worktree-test-profile.sh --cwd "$PWD"
   ```

   Record the printed `PROFILE_ROOT` and `SESSION_NAME`. The command should
   replace terminal A with one foreground-attached Zellij terminal.

2. In that attached terminal, type:

   ```bash
   printf 'ZAPHOD_CAPTAIN_MARKER:%s\n' "$(date +%s)"
   ```

   CL should see the marker in the same terminal. This is the human ergonomics
   check; it does not replace the offline kernel-PGID or raw-PTY evidence.

3. Type `exit` in the sole terminal. Terminal A should return to its original
   shell. In terminal B, set the copied path, then verify:

   ```bash
   PROFILE_ROOT=/recorded/profile-root
   test ! -e "$PROFILE_ROOT"
   ```

   The root and its disposable session should be gone. `PROFILE_LEASE` remains
   a test-harness handoff, not a command for the captain to use.

## Demo outcome

Not run. The workflow reserves AC-I1 for CL, and AC-O1 is currently refuted.
No captain time should be spent on a live drill until the offline proof is
stable.

## Recommendation

**REJECTED.** Stabilize the raw-input readiness contract before the one-shot
nonce is sent, add deterministic coverage for the zero-terminal and delayed
terminal-input states, and make the target-free lifecycle reproducible without
timing out before profile metadata. Then rerun the complete offline packet and
return this gate to CL for AC-I1.
