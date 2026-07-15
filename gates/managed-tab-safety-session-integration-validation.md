# Validation: exact tab-bound AgentsView session delivery, cycle 2

Entity: `managed-tab-safety-session-integration`

Frozen candidate: `bd181658e06b917964e5697580cd0d198266ac99`

Required review range:
`9343129b721b7f6a966d14b38fd845492c71297e..bd181658e06b917964e5697580cd0d198266ac99`

## Recommendation

**REJECTED — return to implementation.** AC-O5 failed twice at its required
native congestion proof. The stored `code_completion` parent also covers only
the final commit, not the required merge-base range. Do not spend captain time
on AC-I1 until both blockers are repaired and validation reruns.

## Blocking findings

### 1. AC-O5 native responsiveness did not reproduce

From a clean detached checkout at the frozen SHA, this command failed twice:

```bash
./tests/zellij-sidebar-congestion-test.sh
```

Both runs reached `responsive-fixture-refresh-complete`, completed the three
literal `Alt p` pane actions, then failed the literal `Alt n` observation at
`tests/zellij-tmux-smoke-test.sh:854`:

```text
FAIL: new-tab missed the 1s complete-tab deadline
FAIL: responsive-action smoke failed
```

The first run ended with `terminals=7 tabs=3 new=2 active=2
active_terminals=1`; the second ended with `terminals=6 tabs=3 new=2 active=1
active_terminals=0`. The wrapper at
`tests/zellij-sidebar-congestion-test.sh:17` therefore never reached its
positive responsiveness verdict or its injected-timeout cleanup phase.

The pure responsive-proof test passed, and the zero-post-ready-inventory Go
tests passed. Those results do not replace the required real tmux/Zellij
caller-impact proof. AC-O5 remains refuted at its supported proof path.

### 2. Stored Roborev parent does not cover the required range

`roborev show --job 1653 --json` confirms a structurally complete PASS packet:

- panel `code_completion`;
- correctness 1650, journey 1651, and proof 1652, each exactly once, `done`,
  and `P`;
- synthesis parent 1653, `done`, `P`, with `No issues found.`;
- branch and right endpoint equal the frozen candidate.

The coverage check fails. Every member and the parent record
`git_ref=bd181658...` and `patch_id=364e099...`. Independent patch hashes are:

```text
bd18165 commit only: 364e09905d82ab6448f7031f76d3b88ac8adc65e
9343129..bd18165:    6aff3af9dcc54e4fc9f03377ab730471f69f30cc
```

The stored parent covers only `bd18165`, not
`9343129b..bd181658`. This is an evidence defect. A replacement
`code_completion` panel must cover the full frozen range after the AC-O5
repair reaches its final SHA.

## Offline acceptance results

| Criterion | Verdict | Independent evidence |
|---|---|---|
| AC-O1 | PASS | The native two-rail smoke resolved two same-CWD watchers to distinct exact tab, terminal, and original-rail tuples and produced `1/1/0`; the focused startup test rejected duplicate, missing, and foreign rail state. |
| AC-O2 | PASS | Go tests and the hook shell test passed atomic envelope validation, exact session/pane admission, bounded input, missing-socket rejection, one in-memory replacement, and no durable registration. |
| AC-O3 | PASS | The native two-rail smoke projected `1/1/0`, used only exact session endpoints, excluded the child and global list, and focused the exact watched pane. |
| AC-O4 | PASS | Rust 147/147 plus the two-rail smoke passed manifest clearing, exact original-rail admission, lease expiry, inert stale focus, bystander isolation, restart-empty, and explicit watcher cleanup. |
| AC-O5 | **REFUTED** | Startup-only inventory tests passed, but `zellij-sidebar-congestion-test.sh` failed twice at literal `Alt n`'s one-second complete-tab deadline. |
| AC-O6 | PASS | The four-case stress-evidence packet passed forced lifecycle failure, hung native command, vanished startup, and explicitly inconclusive cleanup probes; cleanup preserved standing state and removed owned runtime state. |
| AC-I1 | NOT RUN | The captain-live gate is blocked because AC-O5 and the required review coverage are not green. |

## Independent command packet

All commands below ran in detached clone
`/tmp/kj-validation-bd18165.xvmHcc/repo` at the frozen SHA. The implementation
worktree remained read-only.

```bash
cd grout
go test ./... -count=1
go vet ./...

cd ..
cargo test -q
cargo check --tests
./build.sh
./tests/codex-session-hook-test.sh
./tests/zellij-new-tab-test.sh
./tests/build-artifact-test.sh
./tests/zellij-two-rail-recipient-smoke-test.sh
./tests/zellij-watcher-lifecycle-smoke-test.sh
./tests/zellij-responsive-proof-test.sh
./tests/zellij-layout-capture-test.sh
./tests/sidebar-scrollback-docs-test.sh
./tests/zellij-stress-evidence-test.sh
git diff --check
```

Results: Go tests and vet passed; Rust passed 147/147; cargo check and release
build passed; hook, entry, build-artifact, two-rail, outside/inside lifecycle,
pure responsive proof, layout capture, scrollback docs, and all four stress
evidence cases passed. The first lifecycle invocation used an invalid
`ZAPHOD_SMOKE_PREBUILT_ARTIFACTS=1` setup before the host validator existed;
the supported unmodified command then passed outside and inside. This setup
mistake is not product evidence.

`zellij-sidebar-congestion-test.sh` failed twice as recorded above. No test
root or disposable session from either run survived cleanup. The detached
checkout returned to a clean source state at the frozen SHA after the mutation
audit.

## Adversarial refutation audit

The audit changed only the detached throwaway checkout, ran the narrow proof,
then restored the exact frozen source before the next attack.

- **False-positive tab/original-rail identity — caught.** Removing the
  `pane.TabID == tabID` rail filter made
  `TestWatcherStartupResolvesExactTerminalTabAndRailOnce` fail: the foreign
  same-WASM rail raised the count from one to two. The proof does not accept a
  session-wide URL match as original-rail authority.
- **Semantic drift to post-ready native inventory — caught.** Adding
  `watchNativePanes` to each heartbeat made
  `TestWatchTabUsesNativeInventoryOnlyBeforeReady` report three calls instead
  of one and `TestTwoWatchersUseNativeInventoryOnlyForStartup` report six
  instead of two.
- **False-negative lease expiry — caught.** Preventing
  `expire_session_lease` from clearing sessions made
  `watcher_snapshot_lease_expires_atomically_and_blocks_stale_focus` fail on
  the retained row.
- **Watcher lifecycle cleanup — caught.** Disabling listener close and socket
  removal made `TestWatchTabUsesNativeInventoryOnlyBeforeReady` fail with
  `watch socket survived explicit cleanup`.
- **Malformed and indexing paths — no panic survived.** The clean Go and Rust
  suites passed malformed, trailing, duplicate, over-limit, absent-pane,
  duplicate-pane, empty-snapshot, sole-pane, lease, and focus-handback cases.
- **Caller impact — survived.** The required real congestion packet failed
  twice at literal `Alt n`. This surviving attack refutes AC-O5 even though the
  pure deadline predicates pass.

The audit found no surviving exact-identity, post-ready-inventory,
lease-retention, cleanup, or panic attack. It did find the caller-impact
failure above.

## Captain-live demo: prepared, but held

Do not run this demo at the rejected SHA. After implementation repairs AC-O5,
records a full-range PASS parent, and validation clears AC-O1 through AC-O6,
the captain may run this smallest live packet.

From an ordinary control terminal outside the review float:

```bash
set -euo pipefail
WT=/Users/clkao/git/zaphod/.worktrees/spacedock-ensign-managed-tab-safety-session-integration
cd "$WT"
EXPECTED_SHA=<replacement-frozen-sha>
test "$(git rev-parse HEAD)" = "$EXPECTED_SHA"
AGENTSVIEW_URL=http://127.0.0.1:8080
agentsview serve status
until curl -fsS "$AGENTSVIEW_URL/api/v1/sessions?limit=1" >/dev/null; do sleep 2; done

CONFIG_ROOT="${ZELLIJ_CONFIG_DIR:-$HOME/.config/zellij}"
CONFIG_FILE="${ZELLIJ_CONFIG_FILE:-$CONFIG_ROOT/config.kdl}"
LAYOUT_FILE="$CONFIG_ROOT/layouts/zaphod.kdl"
shasum -a 256 "$CONFIG_FILE" "$LAYOUT_FILE" > /tmp/kj-live-before.sha256

SESSION="kj-live-$(date +%s)"
./scripts/zellij-new-tab.sh --session "$SESSION" --name 'KJ exact A' \
  --agentsview-url "$AGENTSVIEW_URL" | tee /tmp/kj-entry-a.out
./scripts/zellij-new-tab.sh --session "$SESSION" --name 'KJ exact B' \
  --agentsview-url "$AGENTSVIEW_URL" | tee /tmp/kj-entry-b.out
printf 'SESSION=%s\n' "$SESSION"
zellij attach "$SESSION"
```

In tab A, run `./target/zaphod watch-tab`; run the same command in tab B.
Record both ready tuples, watcher PIDs, and log paths. Before Codex starts,
both rails must contain zero agent rows.

Start a fresh Codex process in each watched terminal. If project-hook trust is
requested, approve `scripts/zaphod-codex-session-hook.sh`, exit, and start a
new measured Codex process. Send:

```text
KJ_TAB_A_REAL: inspect README.md without edits, then wait.
```

```text
KJ_TAB_B_REAL: inspect README.md without edits, spawn one subagent to inspect the README title without edits, then wait.
```

The captain must observe `1/1/0`: one A row only in A, one B row only in B,
and no child, historical, or `unbound` row. Click each row and report that
focus lands on its exact watched terminal.

Kill A's recorded watcher PID. A must clear within 2.5 seconds while B remains.
Restart `./target/zaphod watch-tab` in A; A must remain empty. Exit and start a
fresh Codex process in A; only its new `SessionStart` may restore A's one row.
Then close A's watched terminal. A must clear on the next manifest update while
B remains. The orphaned A watcher may remain alive but grants no visible or
focus authority.

From the control terminal, finish cleanup and prove standing-state equality:

```bash
kill <final-A-watcher-pid> <B-watcher-pid> 2>/dev/null || true
zellij delete-session --force "$SESSION"
shasum -a 256 "$CONFIG_FILE" "$LAYOUT_FILE" > /tmp/kj-live-after.sha256
cmp /tmp/kj-live-before.sha256 /tmp/kj-live-after.sha256
```

The captain reports the ready tuples, initial row cardinalities, both focus
results, A's watcher-loss time, restart-empty result, fresh-start result,
terminal-close result, B's isolation, cleanup, and hash comparison. Automated
validation proves native inventory counts and failure injection; captain
observation proves only the visible live journey.

## Route back to implementation

1. Reproduce and repair the literal `Alt n` congestion failure without
   weakening its native-state deadline, overlap proof, or cleanup assertions.
2. Run `zellij-sidebar-congestion-test.sh` repeatedly from a clean detached
   checkout at the final SHA.
3. Run a new `code_completion` panel over the exact
   `9343129b..final-head` range. Any repair changes the frozen head and
   invalidates parent 1653.
4. Return to validation. Do not run AC-I1 before all six offline criteria pass.
