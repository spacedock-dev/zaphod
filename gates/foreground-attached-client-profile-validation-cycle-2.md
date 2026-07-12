# Validation cycle 2: Foreground attached-client disposable Zellij profile

Entity: `foreground-attached-client-profile.md`  
Implementation worktree: `.worktrees/spacedock-ensign-foreground-attached-client-profile`  
Raw implementation SHA: `b847a3b605eacaeb286cd406d7dfe6512c174f7b`

The worktree was clean at the checked SHA before and after validation. This
review did not run a captain-live drill.

## Gate recommendation

**REJECTED.** The raw-PTY end-value proof is not reproducible across the
required full offline packet. Successful focused and target-free runs show the
intended foreground handoff can work, but they cannot redeem three independent
`all`-packet failures in readiness-related paths.

## Offline AC verdicts

| AC | Verdict | Independently reproduced evidence |
|---|---|---|
| AC-O1 — real keys reach the disposable terminal | **REFUTED** | Three isolated `worktree-profile` runs and one detached cold clone reached the real raw probe/canary. Three `all`-packet runs nevertheless failed: one before raw proof on lease ordering, one with `profile exited before attached lifecycle readiness completed`, and one in the forced-loss setup with `list-panes failed: There is no active session!`. A canary that is unavailable in required fresh paths is not a reproducible end-value proof. |
| AC-O2 — direct client owns the foreground PTY group before a lease | **PASS in successful runs; not gateable** | A direct run recorded `CLIENT_PID=43615` and `PTY_FOREGROUND_PGID=43615`; successful focused/cold runs also completed the driver’s PID/PGID check. But the driver can reject a lease that the publisher writes after its own OS observation yet before the driver records its own result, so the required ordering evidence is nondeterministic. |
| AC-O3 — ProfileLeaseV1 is immutable and bounded | **PASS in successful runs; not gateable** | Successful focused and detached lifecycle runs exercised exact schema/path/identity/PID/PGID/mode/digest checks plus early-file and early-line rejection. The gate cannot endorse the handoff while AC-O1/O2 readiness is unstable. |
| AC-O4 — normal cleanup and standing-file isolation | **PASS in successful runs** | The detached clone and three focused lifecycle runs completed normal `kill-session` cleanup; the direct run ended with `PTY_PROFILE_EXIT_STATUS=0`, removed its profile root, and retained its global sentinel hashes. |
| AC-O5 — HUP, TERM, and INT cleanup | **PASS in successful runs** | Each successful focused lifecycle printed `PASS: disposable profile preserved global bytes across normal, TERM, INT, and HUP cleanup`; the cold clone completed that same packet. The separate forced-loss setup is itself intermittent before its kill assertion. |
| AC-O6 — no TTY fails loudly before state creation | **PASS** | `profile-no-tty` reported `an attached terminal is required to run the disposable Zellij profile` and passed its no-root/no-session/no-lease checks. |

## Reproduction commands and results

```bash
cd /Users/clkao/git/zaphod/.worktrees/spacedock-ensign-foreground-attached-client-profile
python3 tests/zellij-profile-pty-driver-test.py
./tests/zellij-install-profile-test.sh worktree-profile
./tests/zellij-install-profile-test.sh cold-profile-readiness
./tests/zellij-install-profile-test.sh profile-no-tty
./tests/zellij-install-profile-test.sh profile-build-metadata
./tests/zellij-install-profile-test.sh profile-readiness-shared-deadline
./tests/zellij-install-profile-test.sh profile-timeout-cleanup
./tests/zellij-install-profile-test.sh profile-foreground-pgid-validity
./tests/zellij-install-profile-test.sh all
cargo test
cargo check --tests
(cd grout && go test ./... && go vet ./...)
```

- Python driver regressions: `7` passed.
- Isolated `worktree-profile`: `3/3` passed, including normal plus
  HUP/TERM/INT cleanup and standing-file checks.
- `cold-profile-readiness`: passed from a detached target-free clone after
  deleting its candidate `target/` tree; it copies and byte-compares the
  launcher and PTY driver under review before running the lifecycle.
- No-TTY, cold-build marker isolation, shared deadline, timeout cleanup, and
  foreground-PGID input-validation probes passed.
- Rust: `132` passed; `cargo check --tests` passed. Go checks passed from the
  repository’s `grout/` module.
- The required `all` packet failed on all three independent attempts:
  1. `PTY_DRIVER_ERROR=profile lease file appeared before independent readiness observation`, followed by `FAIL: timed out waiting for profile value PROFILE_PTY_READY`.
  2. `PTY_DRIVER_ERROR=profile exited before attached lifecycle readiness completed`, followed by `FAIL: profile exited before printing PROFILE_PTY_READY`.
  3. The normal lifecycle completed, then `profile-foreground-cleanup` failed before its forced-kill assertion with `PTY_DRIVER_ERROR=timed out completing raw PTY readiness state loop: timed out waiting for exactly one live terminal: list-panes failed: There is no active session!`.

## Refutation audit

The detached clone was a throwaway target-free checkout, never the
implementation worktree. Named probes and results follow.

- **Early lease file / early lease line false positives — survived.** The two
  committed fixtures reject a pre-readiness file and a pre-readiness
  `PROFILE_LEASE=` line before any raw input assertion; they passed in every
  packet that reached them.
- **Lease ordering semantic drift — REFUTED.** The publisher writes only after
  its own foreground-PGID and private-session checks
  (`scripts/zellij-profile-lease-publisher.py:104-135`). The driver then
  observes the same conditions, but rejects an already-existing lease before
  setting its local observation flag (`tests/zellij-profile-pty-driver.py:811-820`).
  A successful transcript ordered `PTY_FOREGROUND_PGID` before `PROFILE_LEASE`;
  the failing packet observed the reverse. There is no cross-process handshake
  establishing which independent observation the AC’s ordering language means.
- **Lost terminal/session false negative — REFUTED.** The raw state loop does
  re-query a sole terminal and uses fresh probe/canary nonces
  (`tests/zellij-profile-pty-driver.py:453-535`), but it cannot recover once
  the private session has disappeared. The retained required-suite trace had a
  valid foreground client group then only `There is no active session!` until
  the lifecycle deadline.
- **Pane-change/stale-marker and bounded-query paths — survived at unit level.**
  The seven driver tests cover zero terminals, same-pane rechecks, fresh
  probe/canary retry, bounded five-second queries, and failure diagnostics.
  They do not redeem the failed real lifecycle path.
- **Candidate/source drift — survived.** The cold clone explicitly copies and
  compares the candidate launcher and PTY driver, removes `target/`, and
  passed a mandatory clean build/lifecycle.
- **Panic/indexing and caller-impact audit — survived in successful paths.**
  The driver rejects malformed/non-single terminal results and validates JSON
  structure before use; successful callers retained the exact lease and
  cleanup boundaries. The unanswered caller-impact issue is the intermittent
  profile/session loss before raw readiness.
- **Disk pressure — surfaced, not attributed.** At audit time `/tmp` had
  `921Mi` available and reported `100%` capacity. No retained trace contained
  `ENOSPC`, a panic, or a Zellij error; successful runs occurred at the same
  measured pressure. It is only a possible environmental contributor.

## Captain-live demo script for AC-I1

Do not run this until the full offline packet is repeatable. When the gate is
green, CL should run the foreground profile from its primary checkout, type a
short marker in its sole terminal, exit it, and verify the recorded profile
root is gone. That live ergonomics drill does not replace AC-O1 through AC-O6.

## Demo outcome

Not run. AC-I1 remains for CL after a deterministic offline packet; this
validator did not claim or simulate captain evidence.

## Return-to-implementation finding

Reconcile the lease publisher/driver ordering contract and diagnose why a
foreground client group can be observed while its private session vanishes.
Prove the selected interpretation through repeated complete `all` packets,
including the forced-loss setup, before returning to validation.
