# Validation: Activation preserves valid Zellij KDL

Entity: `zellij-config-activation` (`fq`)

Implementation worktree: `.worktrees/spacedock-ensign-zellij-config-activation`
at raw SHA `e238f7a90a62b068772742ccd5ac4a0fc6071727` (clean before and after
validation). Frozen range: `3b27e202176740ed976aa94aa8aa7cc91ee66118..e238f7a90a62b068772742ccd5ac4a0fc6071727`.

## Recommendation

**PENDING CAPTAIN DEMO; offline packet recommends APPROVE.** The selected
checkout command creates one inline selected-WASM tab and leaves standing KDL
byte-identical. The demo uses only the direct script. Do not press `Alt /` or
`Alt Shift z`; global key policy and the deferred complex-layout chrome issue
are outside this gate.

## Stored review evidence

Roborev synthesis parent `189` is `code_completion` for the exact frozen range
and current head. Required members `correctness` (`186`), `journey` (`187`),
and `proof` (`188`) each appear exactly once, finished successfully, and
returned PASS; the parent verdict is PASS. Validation did not rerun this
unchanged panel.

Earlier findings are fully disposed: TERM cleanup was fixed by `ea61e22` and
`947582c`; stable foreign-tab identity by `699b6cb`; the stale hash-checkpoint
description by `0c729cf`. Persistent `Alt /` and `Alt .` retargeting requests
were rejected as forbidden global-policy scope and accepted by quick parents
`164` and `185`, both PASS with their sole required member successful.

## Offline AC verdicts

| AC | Verdict | Fresh independent evidence |
|---|---|---|
| AC-1 | PASS | `./tests/zellij-new-tab-test.sh` observed one `new-tab --layout-string` with the selected checkout's canonical WASM URL. The real tmux/Zellij smoke then found exactly one tiled, non-suppressed resident with that URL at the returned stable tab ID in `list-panes`, active in `list-tabs`, and present in `dump-layout`. |
| AC-2 | PASS | The isolated real profile passed native `zellij setup --check` with `WriteChars "{"`, comments, and a fixed unrelated `Alt Shift z` layout. Isolated config/layout hashes and standing config/layout states were identical before and after direct entry. |
| AC-3 | PASS | Focused setup-check failure, `new-tab` failure, and blocked-action TERM all preserved pre-run config/layout hashes; setup failure issued no `new-tab`; in-flight TERM hashes matched; temporary rendered-layout and config-directory backup/temp probes were empty. |
| AC-4 | PASS | The focused fixture omitted `zellij-config-activate.awk` and supplied no Zaphod `MessagePlugin` route, yet created the selected inline tab. Its unrelated fixed shortcut stayed byte-identical. |

Fresh command results: focused shell `6/6`; tmux-hosted Zellij smoke PASS with
literal keys and native state; Rust `135/135`. The broader installer/profile
suite was started only as a non-required caller check and was manually stopped
while building its primary-checkout control; no result from that interrupted
extra run is used as acceptance evidence.

## Refutation audit

The audit used three fresh `git clone --no-hardlinks` throwaway checkouts at
the exact head, all nested under the assigned validation worktree and removed
afterward. The implementation checkout was never edited.

- **URL-only false positive:** a pane with the exact selected URL but stable tab
  `74` while `new-tab` returned `73` failed with `sidecar-target-unready`.
- **Cross-tab false negative:** one exact target at tab `73` plus a same-URL
  foreign pane at tab `74` passed all focused cases; foreign duplication does
  not hide the unique target.
- **Same-target ambiguity / indexing:** two matching residents at tab `73`
  failed closed with `sidecar-target-unready`; no first-element guess or panic
  path survived.
- **Cleanup:** success, setup failure, action failure, and TERM removed rendered
  temporary roots; the real smoke removed its session, dedicated tmux server,
  socket/config/data root, and rechecked standing hashes.
- **Caller impact:** repository search found the script invoked only by its
  focused/native tests and documented operator entry. `install.sh` and the
  disposable profile share layout helpers but do not call this script; Rust
  `135/135` passed.
- **Semantic drift:** CLI flags, default name/session handling, stable
  `TAB_ID`/`WASM_URL`/`SIDECAR_LOG` outputs, exact sidecar tuple, and
  selected-checkout build remain exercised. The intentional change is only
  removal of standing config/layout activation, now replaced by one inline
  layout action.

No attack survived and no implementation defect was found.

## Captain demo (direct script only)

This is a short confirmation, not a keybinding or complex-layout drill. In the
ordinary session where the selected checkout should open a fresh tab:

```bash
cd /Users/clkao/git/zaphod/.worktrees/spacedock-ensign-zellij-config-activation
CONFIG_ROOT="${ZELLIJ_CONFIG_DIR:-$HOME/.config/zellij}"
CONFIG_FILE="${ZELLIJ_CONFIG_FILE:-$CONFIG_ROOT/config.kdl}"
LAYOUT_FILE="$CONFIG_ROOT/layouts/zaphod.kdl"
shasum -a 256 "$CONFIG_FILE" "$LAYOUT_FILE" > /tmp/fq-kdl-before.sha256
./scripts/zellij-new-tab.sh --session WORK --name 'Zaphod fq drill' | tee /tmp/fq-entry.out
TAB_ID="$(sed -n 's/^TAB_ID=//p' /tmp/fq-entry.out)"
WASM_URL="$(sed -n 's/^WASM_URL=//p' /tmp/fq-entry.out)"
zellij --session WORK action list-panes --json --all --command --geometry --state --tab > /tmp/fq-panes.json
zellij --session WORK action list-tabs --json --all --state --layout > /tmp/fq-tabs.json
zellij --session WORK action dump-layout > /tmp/fq-layout.kdl
jq --arg id "$TAB_ID" --arg url "$WASM_URL" \
  '[.[] | select((.tab_id|tostring)==$id and .is_plugin and .plugin_url==$url and (.is_floating|not) and (.is_suppressed|not))] | length' \
  /tmp/fq-panes.json
jq --arg id "$TAB_ID" '[.[] | select((.tab_id|tostring)==$id and .active)] | length' /tmp/fq-tabs.json
shasum -a 256 "$CONFIG_FILE" "$LAYOUT_FILE" > /tmp/fq-kdl-after.sha256
cmp /tmp/fq-kdl-before.sha256 /tmp/fq-kdl-after.sha256
```

CL should see one fresh active tab, both `jq` commands print `1`, `WASM_URL`
names this worktree, `cmp` exits zero, and the existing fixed global shortcut
is untouched. Do not press any `Alt` key for this gate. Reject only with the
command output, native-state mismatch, or changed hash; otherwise approve.

## Demo outcome

Pending CL. Validation did not mutate or exercise the operator's standing
`WORK` profile and claims no live captain observation.
