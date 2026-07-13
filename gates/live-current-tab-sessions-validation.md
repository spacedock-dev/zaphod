# Gate: a fresh managed tab receives a real session and returns to its pane

Entity: `live-current-tab-sessions` (`bb`)

Implementation: `.worktrees/spacedock-ensign-live-current-tab-sessions` at
`10a78ba492be52e01506a6889acc7ba35e7ac2f0` (clean at review end).

## Decision requested

Approve this slice if the drill shows one real row and a same-tab click-back.
The direct fresh-tab script starts one private subscriber; a source session
appears in that rail; clicking it returns to its terminal in the same tab.

`Alt Shift z` remains a **tab-only** shortcut. It is not part of this drill and
does not start a subscriber.

## Offline result

All reproducible acceptance criteria passed independently at the committed
head, without changing standing Zellij configuration.

| Proof | Independent result |
|---|---|
| Stable-tab receiver admission | Rust `cargo test --quiet`: 135 passed; `cargo check --tests` passed. Stable tab ID `0`, malformed, stale, ambiguous, mismatched, and same-CWD bystander events are inert. |
| Real source to addressed row | `cd grout && GOPROXY=off go test -count=1 ./...`: passed. Loopback AgentsView empty → `data_changed` → one `recipient-tab-id=73` session payload. |
| Direct entry and failed sidecar exec | `./tests/zellij-new-tab-test.sh`: 10/10 passed. Exact target discovery precedes one private sidecar; an unready target or failed exec is visible and leaves the new tab intact. |
| Build boundary | `./tests/build-artifact-test.sh`: passed. The checkout builds the WASM plus private `target/zaphod`; no public `grout subscribe` entry. |
| Native two-rail recipient boundary | `./tests/zellij-two-rail-recipient-smoke-test.sh`: passed. A real broadcast reached only its stable-tab target while a same-CWD bystander remained active and showed no marker. |
| Existing managed-tab boundary | `./tests/zellij-tmux-smoke-test.sh`: passed. Isolated tmux/Zellij profile, literal keys, native state, and cleanup all passed. |

## Adversarial audit

- A throwaway clone with receiver admission replaced by unconditional accept
  failed the focused stable-tab test at the same-CWD bystander assertion. The
  receiver guard is not decorative.
- A second throwaway clone removed the target rail after the SSE stream had
  opened, then sent `data_changed`. The subscriber returned `target-lost`, did
  not re-list or pipe, and did not retarget a bystander.
- The direct-entry fake forced the private executable to fail before its FIFO
  readiness signal. The script reported `sidecar-start-failed`, created no
  sidecar command, and retained the new tab.
- Diff and runtime audit found no `--plugin` delivery, helper pane, lease,
  controller, custom PTY, supervisor, or standing-config mutation in this
  slice. The old worktree-profile script remains documented parked evidence,
  not a test path.

One small evidence boundary is deliberate: the native two-rail smoke proves
screen isolation; the exact click decision is covered in the Rust receiver
fixture. The captain drill below is the first real visible row-click proof.

## Captain drill (about two minutes)

Precondition: AgentsView serves the real local source. Use an actual agent
session created **in the fresh tab's terminal**, not a synthetic pipe or a
manual `zaphod subscribe` command.

1. In the checkout, create the managed tab:

   ```bash
   cd /Users/clkao/git/zaphod/.worktrees/spacedock-ensign-live-current-tab-sessions
   AGENTSVIEW_URL=http://127.0.0.1:8080 # replace only if your source differs
   ./scripts/zellij-new-tab.sh --session WORK --name 'Zaphod session drill' \
     --agentsview-url "$AGENTSVIEW_URL"
   ```

   Record the printed `TAB_ID`, `WASM_URL`, and `SIDECAR_LOG`. A permission
   prompt in the new rail is ordinary Zellij consent; approve it once.

2. In the new tab's terminal pane, start or advance one real agent session for
   this checkout. Confirm that AgentsView sees it, for example:

   ```bash
   agentsview session list --active --include-one-shot --json | jq '.sessions[] | {id,cwd}'
   ```

   Its `cwd` must exactly equal that terminal's `pwd`.

3. Wait for one source change/list cycle. The rail should show the session
   row. Do not run `grout`, `zaphod subscribe`, or a pipe command yourself.
   If no row appears, inspect `SIDECAR_LOG` and reject with its terminal
   reason.

4. Click the session row. PASS: focus returns to the unique terminal pane in
   this same fresh tab. Zero or multiple exact-CWD terminal matches must stay
   unbound rather than guessing.

5. Optional capture for the record:

   ```bash
   zellij --session WORK action list-panes --json --all --command --geometry --state --tab > /tmp/bb-live-panes.json
   ```

   The tab stays intact; the sidecar owns only its own source stream and pipe
   children. It does not stop AgentsView or alter existing tabs, panes, or
   plugins.

## Recommendation

Approve if the drill shows one real row and same-tab click-back. Reject only
with the observed `SIDECAR_LOG` reason or a mismatched focus target.
