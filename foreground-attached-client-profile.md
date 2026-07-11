---
id: 7hm8rw9kzp9m2chdmbe721qr
title: Foreground attached-client disposable Zellij profile
status: ideation
source: managed-view roadmap Sprint 1 entry gate, senior staff review 2026-07-11
started: 2026-07-11T05:09:21Z
completed:
verdict:
score: 1.0
worktree:
issue:
pr:
mod-block:
---

## Problem

The canonical disposable profile backgrounds its attached Zellij client, so it can render but cannot reliably own or read the controlling terminal. Repair the profile before any real-key acceptance drill consumes captain time.

## Sprint role

This is Sprint 1's mandatory entry task and merges before other live Zellij work. Limit changes to the disposable profile and process-level tests. Prove foreground process-group ownership, raw PTY input reaching a terminal canary, normal and signal cleanup, temporary-root removal, and unchanged standing config/layout hashes.

## Riskiest unproven mechanism

The current trailing `&` makes the attached client an asynchronous Bash job.
With monitor mode off, Bash redirects that job's stdin to `/dev/null`; it may
render a session but cannot be the reliable receiver of the operator's PTY
bytes. The first invalidation must therefore exercise the actual transport,
not Zellij's control-plane input commands.

Smallest end-to-end check: launch the current profile under a test-owned PTY,
write a nonce-bearing `printf` command to the PTY master, select the live
terminal from `action list-panes --json`, and require the nonce in that
terminal's live `action dump-screen --pane-id` output. The current background
launch is expected to fail that check. After the repair, the same test must
also read the PTY foreground PGID from the kernel and prove that it is the
attached client's own process group.

## Proposed approach

Keep `scripts/zellij-worktree-test-profile.sh` a disposable-profile launcher;
do not add a controller, production binding, or standing-file write. Make its
attached client a Bash monitor-mode job and foreground it immediately:

1. Fail loudly before launch unless the profile has a controllable terminal.
   Enable Bash job control (`set -m`), start the one attached Zellij client as
   a job, record its PID, then use `fg` immediately. This gives the client its
   own process group and transfers the profile PTY foreground group to that
   client instead of leaving an asynchronous job with stdin detached.
2. Print the actual `CLIENT_PID` with the existing disposable metadata. The
   test may use it as a kernel identity, never as a claim by the script.
   Once `fg` returns, retain the existing EXIT/signal cleanup and global-file
   comparison; when cleanup interrupts an active job, terminate and reap that
   client process group before removing the temporary root.
3. Add a test-local, repository-owned PTY driver using only the Python 3
   standard-library `pty`, `os`, and `subprocess` modules. It must preflight
   `python3`, allocate the slave PTY itself, retain the master for byte writes,
   and expose the profile's metadata to the shell test. No `expect`, terminal
   emulator, or hidden human terminal is allowed.
4. Extend `tests/zellij-install-profile-test.sh`'s profile lifecycle helpers
   to run the profile through that driver. The driver obtains
   `os.getpgid(CLIENT_PID)` and `os.tcgetpgrp(master_fd)` independently of the
   profile, writes an externally generated alphanumeric nonce to the master,
   and uses Zellij's live `list-panes` plus `dump-screen` only to observe the
   terminal result. It must not use `zellij action write` or `write-chars`,
   because those bypass the attached-client input path being proved.
5. Preserve the current temp config/layout/data roots, `file_state` snapshot
   discipline, unique session naming, Zellij 0.44.3 gate, and existing
   identity/layout validators. The canary is the ordinary sole terminal shell
   executing the raw nonce command; it introduces no new profile keybinding,
   persistent layout, or production artifact.

The design extends the test-only `start_profile_process` and
`run_profile_signal_foreground` lifecycle helpers. It deliberately does not
change the shared pure layout/identity helpers (`zaphod_render_layout`,
`zaphod_validate_message_plugin_identity`, or
`zaphod_validate_layout_identity`); there is no existing pure PTY helper to
reuse.

## Acceptance criteria

### Offline (agent-reproducible)

**AC-O1 — real keys reach the disposable terminal (end value).** A fresh,
test-owned PTY writes a nonce chosen by the test, and exactly one live terminal
in the profile displays `ZAPHOD_PTY_CANARY:<nonce>` in its actual
`zellij action dump-screen --pane-id` result. The expected nonce originates in
the PTY driver, not in a file written by the profile or test fixture.

Verified by: the focused profile lifecycle test launches the real profile,
discovers its real terminal ID from `list-panes --json`, writes raw bytes to
the PTY master, and polls the live dump until that nonce appears. It contains
no authored dump fixture; should one ever be needed, it must record Zellij's
real single-line dump shape rather than synthetic multiline KDL.

**AC-O2 — the attached client owns the foreground PTY process group (mechanism serving AC-O1).** While AC-O1's session is alive,
`CLIENT_PID == getpgid(CLIENT_PID) == tcgetpgrp(test_pty_master)`. A detached,
background, or merely rendered client cannot satisfy this equality.

Verified by: `tests/zellij-install-profile-test.sh worktree-profile` invokes the PTY driver, reads both process-group values from the OS after metadata is emitted, and fails before any raw-input assertion if they differ. The current `&` launch is the predicted red baseline: it cannot pass the raw canary because Bash gives its asynchronous client `/dev/null` as stdin.

**AC-O3 — normal non-signal completion is contained.** After the live
profile session ends through the normal Zellij session-termination path, the
profile process returns, its named session is absent, its emitted temporary
root no longer exists, and the pre-launch `file_state` values for both
`$ZELLIJ_CONFIG_DIR/config.kdl` and
`$ZELLIJ_CONFIG_DIR/layouts/zaphod.kdl` are identical afterward.

Verified by: `tests/zellij-install-profile-test.sh worktree-profile` captures SHA-256 sentinels before launch, terminates the disposable session without sending a POSIX signal, waits for the profile, and compares the saved hashes and root/session existence. A companion missing-file fixture verifies the profile creates neither standing file nor standing directory.

**AC-O4 — foreground INT, TERM, and HUP clean up.** For three separate fresh
profiles, delivery of `INT`, `TERM`, or `HUP` to the proven foreground client
PGID leaves no client/job, named Zellij session, or profile root and preserves
the same two pre-launch global-file states. Each profile exits nonzero rather
than reporting a successful normal run.

Verified by: `tests/zellij-install-profile-test.sh worktree-profile` runs three fresh signal cases; its PTY driver signals the PGID measured for AC-O2 (not the test runner's group), waits with a bounded deadline, then asserts session absence, root removal, process reaping, and exact before/after hashes.

**AC-O5 — failure is visible rather than a silent fallback.** Invoking the
attached profile without a controllable terminal fails before creating a
session or profile root and tells the operator that an attached terminal is
required.

Verified by: `tests/zellij-install-profile-test.sh profile-no-tty` checks
nonzero exit, no session, and no temporary-root metadata. This prevents a
future refactor from reintroducing a background/detached fallback just to make
CI appear green.

### Captain-live (only after AC-O1 through AC-O5 pass)

**AC-I1 — a human can safely exercise real keys.** From an ordinary terminal,
the captain runs the documented disposable-profile command, sees the attached
Zellij terminal, types a short marker command, and sees its result in that
same terminal. Exiting the attached session returns to the original terminal;
the disposable session/root disappear and standing Zellij configuration was
not used as a test target.

Verified by: captain demonstration after the offline packet is green. This
demo assesses typing and recovery ergonomics only; it is not evidence for
the kernel process-group or raw-input infrastructure claims.

## Test plan

1. Add the PTY-driver-focused lifecycle case first and run it against the
   current background launcher. Its smallest red result is absence of the
   nonce from live `dump-screen` after a raw master write; do not substitute
   `action write-chars`, a transcript match, or a hand-authored dump.
2. Make the smallest launcher change: terminal preflight, `set -m`, one
   attached client job, emitted client PID, immediate `fg`, and bounded
   job-group cleanup/reap. Re-run the same test green, requiring both the
   kernel PGID equality and dump-screen nonce.
3. Rework the existing normal lifecycle case around the PTY driver. Verify
   the existing layout/identity checks still use the disposable files, then
   verify normal session termination, root deletion, session deletion, and
   present/missing standing-file snapshots.
4. Run three isolated signal cases (`INT`, `TERM`, `HUP`) against the actual
   foreground client PGID. Do not reuse a possibly contaminated profile root
   or session name between cases; retain bounded liveness diagnostics.
5. Run the focused shell suite and its existing timeout/readiness regressions,
   then the relevant full shell suite. Zellij-dependent cases retain the exact
   0.44.3 preflight and the PTY driver retains an explicit `python3`
   preflight, so a missing prerequisite fails loudly.
6. Only then prepare the captain-live script: start the profile, type the
   marker, observe it, exit, and confirm that no standing configuration was
   repointed. Record it for validation rather than using the captain to
   diagnose an offline failure.

## Documentation change

Update the README's disposable-profile instructions to say that the command
is foreground-attached, is the supported place to exercise real candidate
keys, and removes its disposable session/root on normal exit or interruption.
Keep the existing isolation warning explicit: it never installs, rewrites, or
restores standing Zellij configuration or layouts. No user-facing production
keybinding documentation changes in this task.

## Out of scope

- Production managed-tab behavior, `Alt Shift z`, or any new production
  keybinding.
- Pane adoption, pane movement, focus policy, identity markers, or session
  incarnation/reuse work.
- Any write, restore, or migration of standing Zellij configuration, layout,
  cache, or data directories.
- Zellij controller architecture, native CLI behavior, and the Sprint 1
  driver-contract or identity-feasibility tasks.

## Stage Report: ideation

- DONE: Define the smallest foreground attached-client repair with an independent PTY foreground-process-group and raw-input canary proof.
  The packet specifies monitor-mode job control plus immediate `fg`, kernel PGID evidence, and raw-master-to-live-dump nonce proof; baseline `rtk tests/zellij-install-profile-test.sh worktree-profile` passed but lacks those assertions.
- DONE: Specify offline versus captain-live acceptance evidence for normal and INT/TERM/HUP cleanup, temporary-root removal, and unchanged standing-file hashes.
  AC-O1 through AC-O5 separate reproducible PTY/session/hash evidence from AC-I1's post-green captain ergonomics drill.
- DONE: Keep the design limited to the disposable profile and process-level tests; exclude production keybindings, pane adoption, and global configuration changes.
  Proposed files are the profile, its process test, a test-local PTY driver, and README guidance; the out-of-scope boundary excludes Sprint 2/3 behavior.

### Summary

The entry repair is a foreground job-control change, not a Zellij controller
feature: a disposable attached client must become the test PTY's foreground
process group and carry an independently generated raw nonce to a terminal
dump. The offline suite owns all infrastructure and cleanup evidence before a
captain types real keys; the live drill is reserved for the resulting safe
operator experience.
