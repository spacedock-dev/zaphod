# Chat-guided exact AgentsView binding demo

This drill proves that each managed tab projects only the Codex session
started in its watched terminal. It uses two tabs with the same checkout.

## 1. Prepare AgentsView

Run in a control terminal:

```bash
set -euo pipefail
FQ="$(git rev-parse --show-toplevel)"
AGENTSVIEW_URL=http://127.0.0.1:8080
agentsview serve status
until curl -fsS "$AGENTSVIEW_URL/api/v1/sessions?limit=1" >/dev/null; do
  sleep 2
done
```

Wait until AgentsView reports a running server rather than a resync.

## 2. Create two managed tabs

```bash
./scripts/zellij-new-tab.sh --session WORK --name 'KJ exact A' \
  --agentsview-url "$AGENTSVIEW_URL" | tee /tmp/kj-entry-a.out
./scripts/zellij-new-tab.sh --session WORK --name 'KJ exact B' \
  --agentsview-url "$AGENTSVIEW_URL" | tee /tmp/kj-entry-b.out
```

Each command reports a distinct `TAB_ID`, the selected `WASM_URL`, a
`WATCH_DIR`, a recipient token, and a `WATCH_COMMAND`. It starts no watcher.

## 3. Start one watcher in each selected terminal

In tab A, run:

```bash
./target/zaphod watch-tab
```

Run the same command in tab B. Each command returns only after its background
watcher has proved the terminal, stable tab, original rail, AgentsView stream,
recipient, private socket, and initial empty lease. It prints the watcher PID
and log path.

An empty `AGENTS` section is correct before SessionStart. If the command
fails, inspect its reported log. Do not launch a watcher from a control pane;
terminal identity is inherited from the pane that runs the command.

## 4. Trust the project hook

Start Codex in tab A and inspect `/hooks`. Trust
`scripts/zaphod-codex-session-hook.sh`. Repeat in tab B if Codex asks.

The startup that showed the trust prompt may have skipped the hook. Start a
new Codex process for the measured run.

## 5. Start two measured sessions

Send this prompt in tab A:

```text
KJ_TAB_A_REAL: inspect README.md without edits, then wait.
```

Send this prompt in tab B:

```text
KJ_TAB_B_REAL: inspect README.md without edits, then wait.
```

Ask tab B's Codex process to spawn one subagent. The rails must settle to:

- tab A: one `KJ_TAB_A_REAL` row;
- tab B: one `KJ_TAB_B_REAL` row;
- both tabs: zero child rows and zero older same-CWD rows.

A row in both tabs, an `unbound` row, or a child row fails the drill. Click
each row and confirm that Zellij focuses its exact terminal.

## 6. Prove replacement and restart-empty behavior

Start another top-level Codex session in tab A's watched terminal. Its
SessionStart replaces the old in-memory registration, so tab A still shows
one row.

Terminate tab A's watcher with the PID printed by `watch-tab`. Its row must
expire. Run `./target/zaphod watch-tab` again in the same terminal. The new
watcher starts empty because registrations are never durable. Start a new
Codex session; only that fresh SessionStart may restore the row.

## 7. Run the isolated proof

```bash
./tests/zellij-two-rail-recipient-smoke-test.sh
```

The native test proves `1/1/0` projection, exact-ID requests, direct pane
focus, restart-empty behavior, lease expiry, and immediate row fail-close when
the watched terminal disappears. The rail clears the row from its exact
manifest; the harness then explicitly terminates the silently orphaned daemon
and proves socket cleanup. The watcher writes no durable authority record.
