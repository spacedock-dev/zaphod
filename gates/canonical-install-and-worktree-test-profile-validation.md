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
