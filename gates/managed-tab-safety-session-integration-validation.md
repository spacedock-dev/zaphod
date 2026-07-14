# Validation: exact tab-bound AgentsView session delivery

Entity: `managed-tab-safety-session-integration`

Frozen candidate: `dec685e8cf6efbc84ef2b26f60a81e4eda9aacf5`

Exact review range:
`9343129b721b7f6a966d14b38fd845492c71297e..dec685e8cf6efbc84ef2b26f60a81e4eda9aacf5`

## Recommendation

**PENDING CAPTAIN DEMO.** AC-O1 through AC-O6 pass independent validation.
AC-I1 remains captain-live: approval is appropriate only after the captain
observes the two-tab manual watcher journey below.

## Stored review packet

The implementation's isolated Roborev daemon was intentionally removed after
review, so its live endpoint cannot be queried. That is an **evidence defect**
at the ephemeral-daemon retention boundary, not an outcome defect. Validation
did not add a controller or rerun an unchanged green panel.

Instead, validation read the immutable raw `roborev show --job 8 --json` tool
result from implementation session
`codex:019f5fc2-a2f9-7da1-be9c-971047acf512`, message ordinal 1274. It records:

- run UUID `ecc48d63-e2cc-4074-9479-33d3122821b1`;
- panel `code_completion`;
- the exact range above, whose right endpoint is the frozen head;
- correctness job 5, journey job 6, and proof job 7, each present exactly once,
  `done`, and verdict `P`;
- synthesis parent 8, verdict `P`, output `No issues found.`

Message ordinal 1283 records deletion of the isolated database and temporary
profile. The archived raw result validly establishes the frozen review packet;
the absent live daemon does not justify a replacement panel.

## Offline acceptance results

| Criterion | Verdict | Independent evidence |
|---|---|---|
| AC-O1 | PASS | `tests/zellij-two-rail-recipient-smoke-test.sh` resolved two same-CWD watched terminals to distinct exact tabs/rails and projected `1,1,0`; `tests/zellij-watcher-lifecycle-smoke-test.sh` passed outside and inside callers. |
| AC-O2 | PASS | Go envelope/socket tests rejected wrong session, wrong pane, child, unknown/duplicate fields, trailing bytes, missing watcher, and over-limit input; the private socket retained one in-memory registration and no registry file. |
| AC-O3 | PASS | The two-rail native test rendered only each exact top-level session, never fetched the child or global list, and literal mouse input focused the exact watched pane. |
| AC-O4 | PASS | The two-rail and lifecycle tests proved watcher kill, original-terminal loss, bystander isolation, lease clearing, disabled stale focus, restart empty, explicit daemon cleanup, and socket absence. |
| AC-O5 | PASS | The responsive proof, congestion test, and native smoke proved no idle native watcher query, one-second literal pane/tab action deadlines, same-WASM lookalike isolation, and conclusive injected-timeout cleanup. |
| AC-O6 | PASS | Four stress cases passed: lifecycle failure, hung native command, vanished startup, and explicitly inconclusive cleanup probes. Candidate status stayed clean; no test-owned Zellij or tmux session remained. |

The supporting packet also passed `go test -count=1 -timeout 120s ./...`,
`go vet ./...`, 145 Rust tests, `cargo check --tests`, build, hook, entry,
responsive-proof, scrollback-doc, and `git diff --check` checks.

## Adversarial refutation audit

The audit used detached throwaway worktrees at the frozen head and removed
them afterward. It did not modify the implementation worktree.

- **False-positive exact binding.** Mutating `registered_session_pane` to
  accept any row made `absent_or_stale_registered_pane_renders_unbound` fail:
  stale pane 99 became `Some(99)` instead of `None`. The proof distinguishes
  exact pane identity from mere row presence.
- **Atomic record validation.** Bypassing the complete JSON uniqueness pass
  made `TestWatchEnvelopeIsValidatedAtomically/duplicate_field` fail because
  the duplicate-field record was admitted. The proof checks the whole record,
  not a sequence of independent field-presence assertions.
- **False-negative stale authority.** Mutating lease expiry to retain sessions
  made `watcher_snapshot_lease_expires_atomically_and_blocks_stale_focus` fail
  at the expiry boundary. The test cannot pass while a stale row remains
  actionable.
- **Panic and indexing paths.** Clean-head malformed/trailing/duplicate and
  over-limit envelope tests passed, as did the sole-pane session-birth and
  focus-handback tests that protect prior missing-target panic paths.
- **Caller impact.** The native congestion packet passed literal `Alt p`,
  `Alt n`, `Alt 1`, and `Alt 2` deadlines, then deliberately injected one
  missed deadline and proved the failure was retained while cleanup completed.
- **Semantic drift.** Automatic subscription is intentionally replaced by the
  approved manual watcher. Retained entry and managed-tab tests prove direct
  entry starts no subscriber, `Alt /` remains managed-only, and the documented
  watcher-before-Codex journey matches the shipped CLI.

No adversarial attack survived against AC-O1 through AC-O6. The archived
Roborev endpoint gap is confined to proof retention and was satisfied from the
immutable raw result without changing the deliverable or its lifecycle.

## Captain-live demo: AC-I1

Run from the candidate worktree in a control terminal. This uses a new Zellij
session; do not use `WORK`.

```bash
set -euo pipefail
WT=/Users/clkao/git/zaphod/.worktrees/spacedock-ensign-managed-tab-safety-session-integration
cd "$WT"
test "$(git rev-parse HEAD)" = dec685e8cf6efbc84ef2b26f60a81e4eda9aacf5
AGENTSVIEW_URL=http://127.0.0.1:8080
agentsview serve status
curl -fsS "$AGENTSVIEW_URL/api/v1/sessions?limit=1" >/dev/null
SESSION="kj-live-$(date +%s)"
zellij attach --create-background "$SESSION"
./scripts/zellij-new-tab.sh --session "$SESSION" --name 'KJ exact A' \
  --agentsview-url "$AGENTSVIEW_URL" | tee /tmp/kj-entry-a.out
./scripts/zellij-new-tab.sh --session "$SESSION" --name 'KJ exact B' \
  --agentsview-url "$AGENTSVIEW_URL" | tee /tmp/kj-entry-b.out
printf 'session=%s\n' "$SESSION"
zellij attach "$SESSION"
```

In tab A, run `./target/zaphod watch-tab`. Run the same command in tab B.
Record both printed watcher PIDs. Before Codex starts, each rail may show an
empty `AGENTS` section; it must not show historical or unbound rows.

Start a fresh Codex process in each watched terminal. If project-hook trust is
requested, approve `scripts/zaphod-codex-session-hook.sh`, then restart that
Codex process because the prompting startup may have skipped `SessionStart`.

Send in A:

```text
KJ_TAB_A_REAL: inspect README.md without edits, then wait.
```

Send in B:

```text
KJ_TAB_B_REAL: inspect README.md without edits, spawn one subagent to inspect the README title without edits, then wait.
```

The captain should observe exactly one A row in tab A and one B row in tab B.
The spawned child, older same-CWD sessions, and `unbound` rows appear nowhere.
Click each row; focus must land on its originating watched terminal.

Kill A's recorded watcher PID. Only A's row must disappear within 2.5 seconds;
B remains. Restart `./target/zaphod watch-tab` in A: A must stay empty. Exit and
start a fresh Codex process in A; only that new `SessionStart` may restore one
A row.

For cleanup, kill the final A and B watcher PIDs, then run from the control
terminal:

```bash
zellij delete-session --force "$SESSION"
```

Report the two initial watcher PIDs, the observed row cardinalities, both
focus results, A's removal time, restart-empty result, fresh-start result, and
cleanup result. Validation records only those observations; it does not infer
screenshots or native state the captain did not report.
