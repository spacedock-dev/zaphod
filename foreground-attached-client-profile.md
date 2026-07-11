---
id: 7hm8rw9kzp9m2chdmbe721qr
title: Foreground attached-client disposable Zellij profile
status: implementation
source: managed-view roadmap Sprint 1 entry gate, senior staff review 2026-07-11
started: 2026-07-11T05:09:21Z
completed:
verdict: PASSED
score: 1.0
worktree: .worktrees/spacedock-ensign-foreground-attached-client-profile
issue:
pr:
mod-block:
sprint: s1-trusted-test-profile-onramp
sprint-lane:
group: walking-skeleton
sprint-readiness: ready
---

## Problem

The canonical disposable profile backgrounds its attached Zellij client, so it can render but cannot reliably own or read the controlling terminal. Repair the profile before any real-key acceptance drill consumes captain time.

The current diagnostic metadata is also not a safe handoff to the later CLI
and native-feasibility lanes: a consumer could infer a session or teardown
right from a path, display name, or active client. The foreground lane must
publish one narrow, test-only lease after foreground readiness instead of
leaving those lanes to derive identity or lifecycle ownership.

## Sprint role

This is Sprint 1's mandatory entry task and merges before other live Zellij work. Limit changes to the disposable profile and process-level tests. Prove foreground process-group ownership, raw PTY input reaching a terminal canary, normal and signal cleanup, temporary-root removal, and unchanged standing config/layout hashes.

The foreground task is the sole owner of the base profile root, private
namespace, primary client, exact disposable session, and final teardown. It
also publishes the immutable test-only `ProfileLeaseV1` needed by later lanes.
It does not attach or clean up a second PTY client, prove session incarnation
or a marker, or acquire CLI ownership.

## Riskiest unproven mechanism

The current trailing `&` makes the attached client an asynchronous Bash job.
With monitor mode off, Bash redirects that job's stdin to `/dev/null`; it may
render a session but cannot be the reliable receiver of the operator's PTY
bytes. The first invalidation must therefore exercise the actual transport,
not Zellij's control-plane input commands. It must also prove that a lease is
not exposed before the kernel sees the primary client in the foreground.

Smallest end-to-end check: launch the current profile under a test-owned PTY,
write a nonce-bearing `printf` command to the PTY master, select the live
terminal from `action list-panes --json`, and require the nonce in that
terminal's live `action dump-screen --pane-id` output. The current background
launch is expected to fail that check. After the repair, the same test must
independently read the PTY foreground PGID from the kernel, prove that it is
the attached client's own process group, and only then observe and parse the
published `PROFILE_LEASE` path. No second attachment, marker pane, or captain
drill belongs to that invalidation.

## Proposed approach

Keep `scripts/zellij-worktree-test-profile.sh` a disposable-profile launcher;
do not add a controller, production binding, or standing-file write. Make its
attached client a Bash monitor-mode job and foreground it immediately, then
publish one bounded test interface for the later lanes:

1. Fail loudly before launch unless the profile has a controllable terminal.
   Enable Bash job control (`set -m`), start the one attached Zellij client as
   a job, record its PID, then use `fg` immediately. This gives the client its
   own process group and transfers the profile PTY foreground group to that
   client instead of leaving an asynchronous job with stdin detached.
2. Keep the current temporary config, data, and layout roots, and add
   disposable cache and home roots beneath the same absolute `PROFILE_ROOT`.
   Allocate one opaque private namespace and one exact disposable session name
   before launch; retain both values exactly as selected. They are inputs to
   later attachment, not values to reconstruct from a path, display name,
   active client, or cwd.
3. Start a repository-owned, test-only readiness publisher as part of the
   profile lifecycle. It observes the controlling PTY from the OS, waits until
   the primary PID is live, `getpgid(CLIENT_PID) == tcgetpgrp(pty)`, and the
   exact private session is observable. It neither injects a key nor invokes a
   Zellij action. Before those observations succeed it writes and prints no
   lease. After they succeed it atomically writes the lease with a temporary
   file plus rename, makes the completed file read-only inside the private
   root, and prints exactly `PROFILE_LEASE=<path>` once. The profile cleanup
   reaps this publisher along with the primary-client process group. Existing
   early `PROFILE_ROOT` and `SESSION_NAME` lines remain foreground-test
   diagnostics only; dependent lanes must treat `PROFILE_LEASE` as the sole
   ready-to-share handoff.
4. The published file is exactly
   `$PROFILE_ROOT/profile-lease-v1.json`; its stable `ProfileLeaseV1` shape is:

   ```json
   {
     "schema": "zaphod.profile.v1",
     "profile_root": "/absolute/disposable/root",
     "namespace": "opaque-exact-private-zellij-namespace",
     "native_session_id": "exact-disposable-session-name",
     "primary_client": {"pid": 1234, "pgid": 1234},
     "attach": {
       "zellij_bin": "/exact/path/to/zellij",
       "config_dir": "/absolute/disposable/root/config",
       "data_dir": "/absolute/disposable/root/data",
       "cache_dir": "/absolute/disposable/root/cache",
       "home_dir": "/absolute/disposable/root/home",
       "namespace": "opaque-exact-private-zellij-namespace",
       "native_session_id": "exact-disposable-session-name"
     },
     "teardown_owner": "foreground-attached-client-profile"
   }
   ```

   All fields are immutable once the file is published: `schema` is exactly
   `zaphod.profile.v1`, `profile_root` and every filesystem attach input are
   absolute, and the duplicated `namespace` and `native_session_id` values
   equal their top-level values byte-for-byte. A consumer passes every attach
   field back verbatim; it may not derive or normalize namespace/session data.
   The lease proves neither a session incarnation nor marker-pane ownership,
   and it carries no permission, controller, or candidate-binary authority.
5. Ownership is deliberately asymmetric. This task owns the base root,
   namespace, session, primary client, lease publisher, and final teardown.
   `zellij-managed-identity-feasibility` owns any second PTY client, its
   PID/PGID, temporary controller, permission cache, evidence, and teardown;
   it must release that client before the profile's final cleanup. The CLI
   task owns only `$PROFILE_ROOT/bin/zaphod`; it creates no client and cannot
   alter or tear down the lease. Every independent run receives a fresh lease.
6. Add a test-local, repository-owned PTY driver using only the Python 3
   standard-library `pty`, `os`, and `subprocess` modules. It must preflight
   `python3`, allocate the slave PTY itself, retain the master for byte writes,
   and independently observe the primary process-group facts before consuming
   the lease. It writes an externally generated alphanumeric nonce to the
   master and uses Zellij's live `list-panes` plus `dump-screen` only to
   observe the terminal result. It must not use `zellij action write` or
   `write-chars`, because those bypass the attached-client input path being
   proved.
7. Preserve the current `file_state` snapshot discipline, unique session
   naming, Zellij 0.44.3 gate, and existing identity/layout validators. The
   canary is the ordinary sole terminal shell executing the raw nonce command;
   it introduces no new profile keybinding, persistent layout, or production
   artifact.

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

**AC-O2 — the attached client owns the foreground PTY process group before a lease is exposed (mechanism serving AC-O1).** While AC-O1's session is alive,
`CLIENT_PID == getpgid(CLIENT_PID) == tcgetpgrp(test_pty_master)`. A detached,
background, or merely rendered client cannot satisfy this equality. No
`PROFILE_LEASE` line or `$PROFILE_ROOT/profile-lease-v1.json` may appear
before the independent OS observation and exact private-session readiness.

Verified by: `tests/zellij-install-profile-test.sh worktree-profile` invokes
the PTY driver, records both process-group values from the OS, and fails before
any raw-input assertion if they differ. Only after it records the equality and
the live session may it consume `PROFILE_LEASE`; the current `&` launch is the
predicted red baseline because Bash gives its asynchronous client `/dev/null`
as stdin.

**AC-O3 — ProfileLeaseV1 is an immutable, bounded test handoff.** While the
base profile is live, the one published path is exactly
`$PROFILE_ROOT/profile-lease-v1.json` and its parsed object has the fixed
schema, absolute root/attach paths, opaque exact namespace, exact native
session ID, OS-observed primary PID/PGID, and teardown owner specified above.
Its two attach identity fields equal their top-level values byte-for-byte;
the completed bytes do not change before cleanup. It contains no incarnation,
marker, permission, controller, or candidate-binary ownership claim.

Verified by: the focused process test parses the emitted file with an external
JSON parser, compares the primary fields with the PTY driver's OS observation
and launch inputs, captures its completed digest during the live session, and
requires the same digest immediately before normal cleanup. The test starts no
secondary client: later feasibility evidence must consume the fields verbatim
and own client B separately.

**AC-O4 — normal non-signal completion is contained.** After the live
profile session ends through the normal Zellij session-termination path, the
profile process returns, its named session is absent, its emitted temporary
root no longer exists, and the pre-launch `file_state` values for both
`$ZELLIJ_CONFIG_DIR/config.kdl` and
`$ZELLIJ_CONFIG_DIR/layouts/zaphod.kdl` are identical afterward.

Verified by: `tests/zellij-install-profile-test.sh worktree-profile` captures SHA-256 sentinels before launch, terminates the disposable session without sending a POSIX signal, waits for the profile, and compares the saved hashes and root/session existence. A companion missing-file fixture verifies the profile creates neither standing file nor standing directory.

**AC-O5 — foreground INT, TERM, and HUP clean up.** For three separate fresh
profiles, delivery of `INT`, `TERM`, or `HUP` to the proven foreground client
PGID leaves no client/job, named Zellij session, or profile root and preserves
the same two pre-launch global-file states. Each profile exits nonzero rather
than reporting a successful normal run.

Verified by: `tests/zellij-install-profile-test.sh worktree-profile` runs three fresh signal cases; its PTY driver signals the PGID measured for AC-O2 (not the test runner's group), waits with a bounded deadline, then asserts session absence, root removal, process reaping, lease disappearance with the root, and exact before/after hashes.

**AC-O6 — failure is visible rather than a silent fallback.** Invoking the
attached profile without a controllable terminal fails before creating a
session or profile root and tells the operator that an attached terminal is
required.

Verified by: `tests/zellij-install-profile-test.sh profile-no-tty` checks
nonzero exit, no session, no temporary-root metadata, and no lease path. This
prevents a future refactor from reintroducing a background/detached fallback
just to make CI appear green.

### Captain-live (only after AC-O1 through AC-O6 pass)

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
   attached client job, immediate `fg`, and bounded job-group cleanup/reap.
   Add the one-shot readiness publisher, but require it to remain silent until
   the OS foreground-PGID observation and exact session readiness succeed.
   Re-run the same test green, requiring both the kernel equality and
   dump-screen nonce.
3. Extend that focused process test to wait for `PROFILE_LEASE`, parse its JSON
   externally, compare every field with the launch and OS observations, and
   compare its completed digest before normal cleanup. It must reject an early,
   duplicate, mutable, path-derived, or identity-overclaiming lease. It must
   not attach a second client or use a marker pane.
4. Rework the existing normal lifecycle case around the PTY driver. Verify
   the existing layout/identity checks still use the disposable files, then
   verify normal session termination, lease/root deletion, session deletion,
   and present/missing standing-file snapshots.
5. Run three isolated signal cases (`INT`, `TERM`, `HUP`) against the actual
   foreground client PGID, plus the no-TTY failure case. Do not reuse a
   possibly contaminated profile root or session name between cases; retain
   bounded liveness diagnostics and prove that no lease survives either path.
6. Run the focused shell suite and its existing timeout/readiness regressions,
   then the relevant full shell suite. Zellij-dependent cases retain the exact
   0.44.3 preflight and the PTY driver retains an explicit `python3`
   preflight, so a missing prerequisite fails loudly.
7. Do not run a captain-live drill as part of this ideation rework. After the
   offline packet is green, AC-I1 remains a separate captain validation of the
   resulting operator experience; it is never used to diagnose PTY, lease, or
   cleanup infrastructure.

## Documentation change

Update the README's disposable-profile instructions to say that the command
is foreground-attached, is the supported place to exercise real candidate
keys, and removes its disposable session/root on normal exit or interruption.
Keep the existing isolation warning explicit: it never installs, rewrites, or
restores standing Zellij configuration or layouts. Explain that
`PROFILE_LEASE` is a test-harness handoff, not a user-facing identity or
configuration interface. No user-facing production keybinding documentation
changes in this task.

## Out of scope

- Production managed-tab behavior, `Alt Shift z`, or any new production
  keybinding.
- A second PTY client, its process group/teardown, temporary controller,
  permissions, marker pane, native identity query, or session
  incarnation/reuse evidence; those belong to the feasibility lane.
- Any write, restore, or migration of standing Zellij configuration, layout,
  cache, or data directories.
- Zellij controller architecture, native CLI behavior, or ownership beyond
  `$PROFILE_ROOT/bin/zaphod`; the CLI may not create clients or tear down this
  lease.
- Executing a captain-live drill during this rework.

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

## Stage Report: ideation (cycle 2)

- DONE: Publish the immutable ProfileLeaseV1 test-only interface only after the primary foreground client is ready, with exact lease fields and non-derivation rules.
  `Proposed approach` defines the one-shot `$PROFILE_ROOT/profile-lease-v1.json` publication condition, fixed JSON shape, byte-exact duplicated identity fields, and AC-O2/AC-O3 process evidence.
- DONE: Specify base-profile versus secondary-client lifecycle/PGID/teardown ownership so CLI and feasibility lanes can share the profile safely.
  `Sprint role` and `Proposed approach` assign the base root/session/client A/final cleanup here, client B and its PGID/teardown to feasibility, and only `$PROFILE_ROOT/bin/zaphod` to the CLI.
- DONE: Preserve the foreground PTY proof and global-isolation boundary without claiming session incarnation or native marker identity.
  AC-O1 and AC-O4, AC-O5, and AC-O6 retain the raw PTY, cleanup, and standing-file-hash proof; the lease and out-of-scope rules explicitly deny incarnation and marker ownership, while AC-I1 remains deliberately deferred to captain validation.

### Summary

The task now exposes a single immutable, test-only lease only after the OS has
observed the actual primary foreground client, allowing later lanes to reuse
the disposable profile without inheriting lifecycle or identity authority.
The raw-key canary, signal cleanup, root removal, and standing-file isolation
remain the foreground lane's proof; no captain-live drill was run or claimed.

## Stage Report: implementation

- DONE: Prove a raw PTY nonce reaches exactly one real disposable terminal before lease exposure.
  `./tests/zellij-install-profile-test.sh all` passed the independent PTY-driver raw-master canary, sole-pane `list-panes`/`dump-screen` proof, and both early-file and early-`PROFILE_LEASE` rejection fixtures; see `a82bdcd`, `983460a`, and `0be78f5`.
- DONE: Prove normal, INT, TERM, and HUP cleanup reaps processes and removes session/root without global config changes.
  The same full suite passed normal, INT, TERM, HUP, forced-cleanup, no-TTY, and standing-config hash cases; the launcher reaps its foreground client group and removes only the private session/root (`c79551b`, `f0b5ea7`, `f799c90`).
- DONE: Prove ProfileLeaseV1 is one-shot, immutable, exact, and bounded; record red/green test evidence.
  Red evidence was `FAIL: timed out waiting for profile value PROFILE_LEASE`; green evidence is the full suite's exact schema/path/PID/PGID/digest/mode checks plus bounded private-session queries and early-publication rejection (`9069381`, `0be78f5`).

### Summary

The foreground launcher now owns one direct Zellij client, a private disposable profile, and an immutable test-only lease published only after the independent PTY observation and raw-terminal proof. Full shell, Rust, and Go verification passed (`132` Rust tests); README guidance makes the real-key path and isolation boundary explicit. No captain-live drill was run or claimed.
