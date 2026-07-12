# Validation: Safe managed-tab entry and guarded keybindings

Entity: `docs/agent-rail-dev/.spacedock-state/zellij-managed-tab-controller.md`
Implementation worktree: `.worktrees/zellij-new-tab-entry`
Raw candidate SHA: `d5137e602dcef91a721852bffa099bb21380102c` (clean before
validation; no candidate files were changed by this validator).

## Gate recommendation

**REJECTED — return two confined tmux-smoke proof repairs to implementation.**
The end-value mechanism has successful real runs, but AC-O2 is not
reproducible yet: an unmodified repeated smoke observed a native rail-state
change unrelated to the literal toggle between its before and after snapshots.
AC-O3 also captures its visible foreign-tab evidence without comparing it.
Do not spend CL's normal-consent drill time until the offline proof is stable.

## Offline AC verdicts

| AC | Verdict | Independent evidence |
|---|---|---|
| AC-O1 | PASS in successful runs | From the selected worktree, `./tests/zellij-new-tab-test.sh` passed 8/8. Fresh real tmux/Zellij smoke runs observed literal `Alt Shift z` add exactly one active `zaphod` tab, with the candidate `file:` WASM URL in both native pane state and `dump-layout`. |
| AC-O2 | **REFUTED** | An unmodified repeated `./tests/zellij-tmux-smoke-test.sh` run failed at `tests/zellij-tmux-smoke-test.sh:346-349`: the candidate rail's otherwise-normalized native state changed only from `is_selectable: true` to `false` after literal `Alt /`. This breaks the criterion's unchanged pane/process/focus identity proof. A subsequent unmodified 10-run control passed, so this is an intermittent readiness race, not a deterministic semantic result. |
| AC-O3 | **REFUTED as the full criterion** | Successful real smoke runs sent the literal foreign-tab `Alt /` only after the observed managed route; normalized native pane state and dumped layout were byte-equal, and candidate count remained one. But the harness captures `foreign-before.screen` and `foreign-after.screen` at :362-365 and never compares or otherwise asserts their visible invariant at :366-376, so a terminal-visible-only foreign effect could pass. |
| AC-O4 | PASS for executed paths | Successful smoke preserved standing config/layout hashes and removed its tmux server, Zellij session, and root. Independent forced-build failure left no new `/tmp/zs.*` root; a TERM sent after the dedicated tmux server was live exited 143, removed that server/root, and preserved both standing hashes. A detached no-pregrant probe removed the disposable permission cache before both attaches; the smoke failed rather than falsely passing, then removed session `zs81556`, tmux server `zs81556`, and `/tmp/zs.svFKAR` while retaining its isolated standing sentinel hashes. |

Supporting reruns at the raw SHA: `cargo test --release` 134/134;
`cargo check --tests --release` clean; one fresh `zellij-new-tab` shell suite
8/8; successful tmux smoke runs exercised all four offline paths. `git diff
--check` was clean before validation. A host ENOSPC event aborted one attempted
repeat before `mktemp`; it is reported as environment noise and is not used as
candidate evidence.

## Reproduced failure and root-cause evidence

The failed native diff was:

```diff
-    "is_selectable": true,
+    "is_selectable": false,
FAIL: managed Alt / replaced a pane, process, focus, or candidate identity
```

The smoke accepts the candidate as soon as its URL exists
(`tests/zellij-tmux-smoke-test.sh:310-312`), then snapshots it at :312 and
presses `Alt /` at :336. In the rail, permission is requested on first render
(`src/main.rs:630-646`). `PermissionRequestResult(Granted)` later calls
`set_selectable(false)` and requests the runtime route (`src/main.rs:429-438`,
`:699-720`); literal key delivery is correctly rejected until that grant is
present (`src/main.rs:610-627`, `:1075-1087`). Thus the pre-grant cache avoids
human input but does not establish that the plugin has processed its grant
event before the test's baseline. A delayed-baseline control passed, which is
consistent with this ordering; it is not accepted as the proof.

## Required narrow repair and revalidation

Do not change the managed-tab architecture, restore the optimistic route flag,
or touch 7h/4d/custom PTY/lease code. In the existing worktree only:

1. Make the real smoke wait for an observable, stable post-grant resident
   before the AC-O2 baseline: candidate URL, active tiled 28-column shape,
   no visible permission prompt, and the native `is_selectable: false` state
   produced by the grant handler. Do not replace the literal key with a helper
   call or weaken the after-key identity comparison.
2. Assert AC-O3's retained foreign tmux screen evidence: compare normalized
   before/after captures or assert an equally stable visible invariant that
   rejects a candidate launch or dock/layout transition. Keep the existing
   native byte-equality checks; they are not a substitute for the visible
   proof.
3. Add the smallest regressions around those two smoke conditions, then rerun
   `./tests/zellij-new-tab-test.sh`, `cargo test --release`,
   `cargo check --tests --release`, and at least ten consecutive unmodified
   `./tests/zellij-tmux-smoke-test.sh` runs. Retain the per-run exit status and
   any native diff on failure.
4. Repeat the failure/TERM cleanup probes and preserve the standing-file hash,
   absent tmux server, absent Zellij session, and absent temporary-root result.

## Detached refutation audit

A throwaway detached checkout at the same raw SHA,
`/tmp/zaphod-fp-refutation-d5137e6`, was used rather than the implementation
worktree. Static attacks found that persistent `Alt /` is `NoOp`, the runtime
route targets an already-running `MessagePluginId`, and a received pipe still
requires granted permission, `Keybind` source, a tiled rail, and matching live
tab (`src/main.rs:607-627`, `:699-720`, `:1047-1087`). The successful real
smoke separately exercises that route before the foreign-tab no-op
(`tests/zellij-tmux-smoke-test.sh:329-376`). No broader routing or foreign-tab
semantic hole was found. The surviving proof attacks are the AC-O2 baseline
ordering and the absent AC-O3 screen assertion above; together they reject the
offline gate.

The detached audit also removed its disposable HOME permission cache immediately
before both Zellij attaches. The smoke correctly failed instead of treating the
ungranted rail as ready, and its cleanup removed the temporary root, tmux
server, and Zellij session while the audit's standing config/layout sentinels
remained byte-identical. This is cleanup evidence only; it does not substitute
for CL's ordinary permission decision in AC-I1.

## Captain-live drill

**Held.** AC-I1 deliberately remains unrun. After the repeated offline packet
is green, the drill must use a disposable config/data/socket root and a real
tmux-hosted attached Zellij client: run the selected worktree's entry command,
restart so the native binding is loaded, press literal `Alt Shift z`, approve
the ordinary prompt without injected consent, press literal `Alt /`, save
native before/after pane+layout snapshots, switch to a foreign tab, and prove
its literal `Alt /` snapshot remains unchanged. The pre-granted smoke does not
settle this AC.

### Exact captain drill, after the repair

Run this in Terminal A from the selected candidate worktree. It uses tmux only
as the real terminal boundary; it does not create a custom PTY, lease, or
permission-cache grant.

```bash
repo=/Users/clkao/git/zaphod/.worktrees/zellij-new-tab-entry
ROOT="$(mktemp -d /tmp/zaphod-cl-drill.XXXXXX)"
CONFIG_DIR="$ROOT/config"
CONFIG_FILE="$CONFIG_DIR/config.kdl"
DATA_DIR="$ROOT/data"
SOCKET_DIR="$ROOT/socket"
HOME_DIR="$ROOT/home"
SESSION="zaphod-cl-$$"
TMUX_SERVER="zaphod-cl-$$"
TMUX_SESSION=zaphod-cl-drill
WASM="$repo/target/wasm32-wasip1/release/zellij-sidebar.wasm"

state() {
  if [ -e "$1" ]; then shasum -a 256 "$1" | awk '{print $1}'; else printf missing; fi
}
STANDING_ROOT="${ZELLIJ_CONFIG_DIR:-$HOME/.config/zellij}"
STANDING_CONFIG="${ZELLIJ_CONFIG_FILE:-$STANDING_ROOT/config.kdl}"
STANDING_LAYOUT="$STANDING_ROOT/layouts/zaphod.kdl"
CONFIG_HASH_BEFORE="$(state "$STANDING_CONFIG")"
LAYOUT_HASH_BEFORE="$(state "$STANDING_LAYOUT")"

mkdir -p "$CONFIG_DIR/layouts" "$DATA_DIR" "$SOCKET_DIR" "$HOME_DIR"
cp "$repo/tests/fixtures/zellij-tmux-smoke-config.kdl" "$CONFIG_FILE"
"$repo/build.sh" >/dev/null
source "$repo/scripts/zellij-layout-lib.sh"
WASM_URL="$(zaphod_canonical_file_url "$WASM")"

ctl() {
  env ZELLIJ_SOCKET_DIR="$SOCKET_DIR" zellij --session "$SESSION" \
    --config-dir "$CONFIG_DIR" --config "$CONFIG_FILE" --data-dir "$DATA_DIR" "$@"
}
start_client() {
  tmux -L "$TMUX_SERVER" new-session -d -x 160 -y 45 -s "$TMUX_SESSION" \
    "env HOME='$HOME_DIR' ZELLIJ_SOCKET_DIR='$SOCKET_DIR' zellij --config-dir '$CONFIG_DIR' --config '$CONFIG_FILE' --data-dir '$DATA_DIR' attach --create '$SESSION'"
  until ctl action list-panes --json --all --command --geometry --state --tab >/dev/null 2>&1; do sleep 0.1; done
}
capture() {
  ctl action list-panes --json --all --command --geometry --state --tab >"$ROOT/$1.json"
  jq -S . "$ROOT/$1.json" >"$ROOT/$1.json.sorted"
  ctl action dump-layout >"$ROOT/$1.kdl"
  tmux -L "$TMUX_SERVER" capture-pane -p -t "$TMUX_SESSION:0.0" >"$ROOT/$1.screen"
}

# Bootstrap the selected root, then restart so Zellij reads the new native binding.
start_client
env ZELLIJ_CONFIG_DIR="$CONFIG_DIR" ZELLIJ_CONFIG_FILE="$CONFIG_FILE" \
  ZELLIJ_DATA_DIR="$DATA_DIR" ZELLIJ_SOCKET_DIR="$SOCKET_DIR" \
  "$repo/scripts/zellij-new-tab.sh" --session "$SESSION" --name 'Zaphod CL bootstrap'
ctl delete-session --force "$SESSION" || true
tmux -L "$TMUX_SERVER" kill-server || true
start_client
printf "Attach in Terminal B: tmux -L %q attach -t %q\\n" "$TMUX_SERVER" "$TMUX_SESSION"
```

In Terminal B, run the printed attach command. Press literal `Alt Shift z` in
the foreign tab. The new `zaphod` tab must appear, and its ordinary permission
prompt must be approved by CL—not by an injected key or a cache. Back in
Terminal A, save the pre-toggle observations:

```bash
capture managed-before
jq -e --arg url "$WASM_URL" \
  'any(.[]; .is_plugin and .plugin_url == $url and .tab_name == "zaphod")' \
  "$ROOT/managed-before.json"
before_width="$(jq -er --arg url "$WASM_URL" \
  '[.[] | select(.is_plugin and .plugin_url == $url) | .pane_columns] | if length == 1 then .[0] else error("one candidate rail required") end' \
  "$ROOT/managed-before.json")"
test "$before_width" = 28
jq -e --arg url "$WASM_URL" \
  'any(.[]; .is_plugin and .plugin_url == $url and .is_selectable == false)' \
  "$ROOT/managed-before.json"
```

CL now presses literal `Alt /` once in Terminal B. It must visibly collapse
the rail from 28 columns to its one-column sliver without creating a pane or
changing the candidate URL. Terminal A then records and checks it:

```bash
capture managed-after
after_width="$(jq -er --arg url "$WASM_URL" \
  '[.[] | select(.is_plugin and .plugin_url == $url) | .pane_columns] | if length == 1 then .[0] else error("one candidate rail required") end' \
  "$ROOT/managed-after.json")"
test "$after_width" = 1
jq -S 'map(del(.pane_x,.pane_content_x,.pane_y,.pane_content_y,.pane_rows,.pane_content_rows,.pane_columns,.pane_content_columns))' \
  "$ROOT/managed-before.json" >"$ROOT/managed-before.identity.json"
jq -S 'map(del(.pane_x,.pane_content_x,.pane_y,.pane_content_y,.pane_rows,.pane_content_rows,.pane_content_columns,.pane_columns))' \
  "$ROOT/managed-after.json" >"$ROOT/managed-after.identity.json"
cmp "$ROOT/managed-before.identity.json" "$ROOT/managed-after.identity.json"
cmp -s "$ROOT/managed-before.kdl" "$ROOT/managed-after.kdl" && exit 1
cmp -s "$ROOT/managed-before.screen" "$ROOT/managed-after.screen" && exit 1
```

Finally, use the same attached client for the foreign-tab check. Terminal A
switches it with the native action, records its baseline, then CL presses
literal `Alt /` in Terminal B. All three retained foreign observations must be
unchanged:

```bash
ctl action go-to-previous-tab
until ctl action list-tabs --json --all --state --layout | jq -e 'any(.[]; .active and .name != "zaphod")' >/dev/null; do sleep 0.1; done
capture foreign-before
# CL presses Alt / once in Terminal B while that foreign tab is visible.
capture foreign-after
cmp "$ROOT/foreign-before.json.sorted" "$ROOT/foreign-after.json.sorted"
cmp "$ROOT/foreign-before.kdl" "$ROOT/foreign-after.kdl"
cmp "$ROOT/foreign-before.screen" "$ROOT/foreign-after.screen"
jq -e --arg url "$WASM_URL" \
  '([.[] | select(.is_plugin and .plugin_url == $url)] | length) == 1' \
  "$ROOT/foreign-after.json"

ctl delete-session --force "$SESSION" || true
tmux -L "$TMUX_SERVER" kill-server || true
test "$(state "$STANDING_CONFIG")" = "$CONFIG_HASH_BEFORE"
test "$(state "$STANDING_LAYOUT")" = "$LAYOUT_HASH_BEFORE"
rm -rf "$ROOT"
```

## Subspace review instruction

When implementation returns with the repeated offline packet, review this
single artifact in `subspace-tui --gate-review` and persist its feedback log as
`gates/zellij-managed-tab-controller-validation.decisions.jsonl`. The human
fold—not chat prose—selects approval, revision, or rejection.
