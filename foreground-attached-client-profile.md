---
id: 7hm8rw9kzp9m2chdmbe721qr
title: Foreground attached-client disposable Zellij profile
status: ideation
source: managed-view roadmap Sprint 1 entry gate, senior staff review 2026-07-11
started: 2026-07-11T05:09:21Z
completed:
verdict: REJECTED
score: 1.0
worktree:
issue:
pr:
mod-block:
sprint: s1-trusted-test-profile-onramp
sprint-lane:
group: walking-skeleton
sprint-readiness: ready
reopened: 2026-07-12
reopened-reason: Captain requested a new ideation pass after rejected validation; preserve cycle-1 findings and validated boundaries.
---

## Problem

The foreground handoff, immutable lease, cleanup, no-TTY rejection, and
standing-file isolation now pass when the lifecycle reaches readiness.
Validation nevertheless rejected AC-O1: the PTY driver accepts `ECHO=off` as
input readiness, selects terminals only once, and sends its final raw nonce
only once. A Zellij client can own the foreground process group and expose an
exact session before it has created one terminal or before that terminal's
shell can read input.

A fresh target-free clone has a separate ambiguity. The launcher runs its
mandatory build before printing `PROFILE_ROOT`, so the lifecycle metadata clock
can expire during a cold compile. The repair must distinguish that bounded
candidate preparation from attached-session readiness without extending either
deadline or loosening the existing lifecycle boundary.

## Sprint role

This is Sprint 1's mandatory entry task and merges before other live Zellij work. Limit changes to the disposable profile and process-level tests. Prove foreground process-group ownership, raw PTY input reaching a terminal canary, normal and signal cleanup, temporary-root removal, and unchanged standing config/layout hashes.

The foreground task is the sole owner of the base profile root, private
namespace, primary client, exact disposable session, and final teardown. It
also publishes the immutable test-only `ProfileLeaseV1` needed by later lanes.
It does not attach or clean up a second PTY client, prove session incarnation
or a marker, or acquire CLI ownership.

## Riskiest unproven mechanism

The foreground process-group repair is proven. The remaining risk is the gap
between client attachment and a sole terminal shell accepting bytes: kernel
PGID equality, an exact visible session, and `ECHO=off` prove none of those
last two facts. The independent validator observed both zero terminals and a
terminal that never displayed the one-shot nonce.

The first invalidation is therefore a driver-owned raw-input handshake in a
real profile. After the driver independently sees the exact session and
foreground client PGID, it must wait for exactly one non-plugin terminal, write
a unique raw canary command to the PTY master, and find that driver-generated
canary in that same pane's live `dump-screen`. If either terminal availability
or the canary is absent, the driver re-enters the bounded condition loop; the
first observed canary is the end-value proof. `ECHO=off` remains a diagnostic,
never the unlock. A target-free clean build is a prior, separately bounded
phase; the existing attached-lifecycle metadata deadline starts only after the
profile reports that build complete.

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

### Cycle 3 deterministic readiness amendment

Use one test-owned, two-phase readiness design. It adds no profile controller,
second client, managed tab, or lease authority:

1. Keep the ordinary launcher and its mandatory build. Immediately before
   `$REPO_ROOT/build.sh`, it prints exactly `PROFILE_BUILD_STARTED=1`; only
   after a successful build and artifact check, before profile allocation, it
   prints exactly `PROFILE_BUILD_READY=1`. The driver treats those lines as a
   distinct, fixed, reported clean-build phase. Its current 180-second
   lifecycle metadata budget begins at `PROFILE_BUILD_READY`, not at `Popen`,
   and it cannot absorb a missing build marker, build failure, or build timeout.
   The target-free test still removes `target`; it may not prewarm, copy, or
   reuse an artifact.
2. Retain the existing independent OS gate unchanged: the driver must first
   observe the exact private session and
   `CLIENT_PID == getpgid(CLIENT_PID) == tcgetpgrp(test_pty_master)`. It must
   continue rejecting an early lease file or line. The one-shot
   `ProfileLeaseV1` publisher still depends only on that OS/session condition;
   it neither sends a canary nor waits for a test-driver acknowledgement.
3. Replace the one-shot `sole_terminal_id` plus `ECHO=off` decision with a
   bounded driver state machine. A zero-terminal response is pending and is
   reported with bounded diagnostics; any count other than one at the final
   decision fails. Before every raw write, the driver rechecks the same live
   terminal through `list-panes --json` and records its pane ID. It uses only
   the PTY master for writes and live `dump-screen --pane-id` for observations.
4. Generate a new alphanumeric canary outside the profile for each bounded
   handshake attempt. The driver writes its harmless `printf` command, polls
   the recorded pane's real dump, and, if the canary is absent or the pane is
   no longer the sole terminal, returns to the condition loop with a distinct
   nonce. The first canary observed in a fresh dump of the same still-sole pane
   proves the terminal shell has completed a raw-input round trip and emits
   `PROFILE_PTY_READY=1`. Terminal mode alone does not. Profile-created files,
   shell transcripts, synthetic dumps, `zellij action write`, and
   `write-chars` remain invalid evidence. The existing lease parser,
   immutable-byte digest check, ownership split, normal cleanup, signal
   cleanup, and no-TTY path run unchanged after this proof.

## Acceptance criteria

### Offline (agent-reproducible)

**AC-O1 — real keys reach the disposable terminal (end value).** A fresh,
test-owned PTY writes a driver-generated `ZAPHOD_PTY_CANARY:<nonce>` command
only when one real non-plugin terminal is live, then observes that exact nonce
in the same pane's `zellij action dump-screen --pane-id` result. If a terminal
is absent or the canary is absent, the driver re-enters its bounded condition
loop with a new nonce; the first observed canary proves the endpoint. `ECHO=off`,
a visible session, or a prior terminal count alone cannot satisfy this AC.

Verified by: the focused lifecycle test launches the real profile, repeatedly
observes real `list-panes --json` state until one non-plugin terminal is live,
and performs raw-master canary handshakes against its live dump. A test-only
delay may hold real session or shell startup, but it may not fake `list-panes`
or `dump-screen`; the old one-shot path is the red baseline.

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

**AC-O7 — cold candidate build is bounded without consuming attached
readiness.** In a target-free clone with no `target` directory, the launcher
prints `PROFILE_BUILD_STARTED=1` before its mandatory clean build and
`PROFILE_BUILD_READY=1` only after that build succeeds, before a profile root,
session, or lease exists. The driver gives this phase its own reported build
budget. Its unchanged lifecycle deadline begins only at `PROFILE_BUILD_READY`;
it may not borrow build time, raise that deadline, or use a warmed artifact as
clean-build evidence.

Verified by: `cold-profile-readiness` removes the clone's `target`, records
both build markers and the separate elapsed result, then runs the real PTY
lifecycle. The test rejects a missing/late ready marker or build failure,
asserts no disposable Zellij state exists during the build, and retains the
committed metadata/readiness budget for the attached launch.

### Captain-live (only after AC-O1 through AC-O7 pass)

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

1. **First invalidating offline proof.** Add the driver handshake state before
   changing the launcher. Exercise a real profile through a test-owned PTY
   with test-only delayed session and delayed terminal-shell fixtures. The
   fixtures may delay real process startup, but may not forge Zellij list or
   dump output. Record the red result: the old path either fails at zero
   terminals or sends its one-shot canary before the terminal shell can return
   it.
2. Implement the bounded real-state loop: exact session plus foreground PGID,
   repeatedly one terminal, a raw canary observed in its fresh live dump, then
   a same-pane/sole-terminal recheck. Re-run the delayed fixtures green; a
   zero-terminal state must wait, a multiple-terminal final state must fail,
   and an absent canary must name its phase and deadline.
3. Add `PROFILE_BUILD_STARTED=1` immediately before the mandatory build and
   `PROFILE_BUILD_READY=1` only after it succeeds. Add the target-free
   clean-build test first: remove `target`, require the bounded markers, and
   assert no temporary profile/session/lease exists. Start the unchanged
   lifecycle clock only at the ready marker; record a red result if build time
   still leaks into metadata readiness.
4. Re-run the existing early-file, early-line, false-client, and mutable-lease
   attacks. Make each driver fixture pass the build markers before it exercises
   its original lease fault, so a missing build phase cannot mask an early-lease
   failure. Verify that the publisher still emits only one immutable lease after
   independent PGID/session readiness, with the existing exact schema and
   without accepting or publishing the PTY canary.
5. Re-run normal termination, forced launcher loss, and three fresh
   foreground-PGID signal cases (`INT`, `TERM`, and `HUP`). Retain root, lease,
   session, process-reap, present/missing standing-file, and exact hash checks.
6. Re-run the no-TTY case and all bounded timeout/readiness regressions. Keep
   the Zellij 0.44.3 and Python 3 preflights; missing prerequisites must fail
   clearly, not produce a detached fallback.
7. Assemble the revalidation packet: three independent focused lifecycle runs
   with fresh roots/sessions/nonces, one target-free clean-build lifecycle,
   the full shell suite, and the relevant Rust and Go suites. Return the same
   validator to the packet only after every offline AC passes. Do not run
   AC-I1 during this rework.

## Documentation change

Update the README's disposable-profile instructions to say that the command
is foreground-attached, is the supported place to exercise real candidate
keys, and removes its disposable session/root on normal exit or interruption.
Keep the existing isolation warning explicit: it never installs, rewrites, or
restores standing Zellij configuration or layouts. Explain that
`PROFILE_LEASE` is a test-harness handoff, not a user-facing identity or
configuration interface. No user-facing production keybinding documentation
changes in this task. The build markers, raw canary handshake, and timing
phases are process-test internals; the documented ordinary command remains one
foreground-attached invocation that builds before opening its disposable
session.

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
- A profile-side input reader, readiness file, new keybinding, or second client
  to acknowledge the canary. The canary belongs only to the test-owned PTY
  driver and proves the ordinary sole terminal shell.
- Changing the `ProfileLeaseV1` schema or making its publisher wait for the
  canary; its existing foreground-PGID and exact-session gate remains the only
  publication authority.
- Extending the existing attached-lifecycle metadata deadline or treating a
  warmed build artifact as evidence for the target-free clean-build case.

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

## Stage Report: validation

- DONE: Independently re-run every offline AC from the final commit, including raw PTY, PGID, lease, cleanup, and isolation evidence.
  Clean SHA `4320b2f9fadb27d500097b31dc85c76a083918fa`: focused lifecycle, no-TTY, forced-cleanup, PGID validation, and a missing-standing-root normal-cleanup probe exercised the packet.
- FAILED: AC-O1 — real keys reach the disposable terminal (end value).
  Repeated live probes produced both `expected exactly one terminal, found 0` and a raw nonce absent from 243 completed `dump-screen` calls; a second focused run timed out awaiting `PROFILE_PTY_READY`.
- DONE: Attack the packet in a throwaway checkout: early or mutable lease, false client identity, cleanup leaks, and global-state regression.
  Early file/line, false-client, mutable-lease, cleanup, and isolation attacks survived; the apparent lease bookkeeping race was checked and withdrawn because its write followed the independent OS observation.
- DONE: Produce per-AC evidence, the captain-live demo script, and the required gate brief and decision log without self-driving the live demo.
  Wrote `gates/foreground-attached-client-profile-validation.md`, its subspace brief, and its decision log; AC-I1 remains unrun for CL after offline stabilization.

### Summary

Validation rejects the gate pending a deterministic raw-PTY readiness proof. The
profile's foreground ownership, lease shape, cleanup, and isolation evidence
passed when the lifecycle reached readiness, but AC-O1 cannot be claimed after
the observed zero-terminal and absent-canary failures. A target-free clone also
failed to emit profile metadata within the committed readiness window on this
validator; no captain-live drill was run.

### Feedback Cycles

**Cycle 1 (2026-07-12) — REJECTED at validation, routed to implementation.**

1. **AC-O1 readiness is not deterministic.** Repair the raw-PTY lifecycle so
   repeated fresh runs always discover exactly one live terminal and observe
   the nonce in its real `dump-screen`; do not accept an ECHO-off condition as
   the only input-readiness proof.
2. **Cold-run metadata timing must be bounded and reproducible.** Diagnose the
   target-free clone's failure to emit `PROFILE_ROOT` inside the committed
   readiness window without weakening the timeout or masking build/startup
   delay.
3. **Preserve the validated boundaries.** Keep the passing foreground-PGID,
   immutable-lease, cleanup, no-TTY, and global-isolation behavior intact while
   repairing the readiness path; re-run the full offline packet before asking
   the same validation reviewer to recheck it.

**Captain reroute (2026-07-12) — reopen to ideation.** The captain requested a
new design pass before another implementation attempt. Cycle 1's three findings
remain binding, and the passing isolation and cleanup boundaries are preserved.

**Cycle 2 (2026-07-12) — deterministic-readiness design.** The repair is a
driver-owned condition loop that accepts a real raw canary only after it
appears in a still-sole terminal's live dump. Explicit build start/ready
markers give a target-free clean build its own bounded phase and leave the
existing post-build lifecycle deadline intact. The lease publisher and all
passing lifecycle/isolation boundaries remain unchanged.

## Stage Report: ideation (cycle 3)

- DONE: Reframe the rejected raw-PTY and cold-run metadata findings into one smallest deterministic readiness design.
  The body replaces ECHO-only, one-shot input readiness with a bounded real-pane/raw-canary loop and separates build timing with `PROFILE_BUILD_STARTED`/`PROFILE_BUILD_READY`; workflow state validates.
- DONE: Preserve the passing foreground-PGID, immutable-lease, cleanup, no-TTY, and standing-configuration boundaries.
  The amendment keeps the publisher's OS/session gate, `ProfileLeaseV1`, group cleanup, terminal preflight, and file-state snapshots unchanged; validation evidence remains at `gates/foreground-attached-client-profile-validation.md`.
- DONE: Specify the first invalidating offline proof and a revalidation packet without adding controller or managed-tab scope.
  The first proof uses real delayed session/shell states and live Zellij inspection only; the packet requires three fresh focused runs, one target-free cold run, lifecycle/isolation regressions, and the existing full suites.

### Summary

The redesign treats a canary observed in a real terminal dump as readiness and
treats terminal mode only as a diagnostic. It also makes cold compilation a
separate, bounded pre-profile phase without changing the published lease or
claiming that the offline packet is green.
