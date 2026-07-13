# Gate review: Alt-/ only changes a Zaphod-created tab

Candidate: `b3b003ab1608d95b475d8034e895fa7a516f044a` in
`.worktrees/spacedock-ensign-managed-tab-toggle-authorization`.

## Decision requested

**Approve v3's offline validation and authorize the captain-owned post-merge
`WORK` drill.** This approval says the stale-focus repair is safe to merge;
the live drill remains the final interactive observation.

## Independent proof

1. The dedicated stale-focus regression passes for both authorization paths:
   unavailable focus and an unmapped stable tab ID leave route offer and
   received keybind pipe inert, even when cached UI state still names the
   former managed rail.
2. The repaired candidate's Rust suite passes 138/138 and
   `cargo check --tests` passes.
3. The fresh-tab entry suite passes 9/9, including exact managed marker and
   candidate-URL validation plus persistent `Alt /` fail-closed behavior.
4. One real isolated tmux/Zellij smoke passed: literal `Alt Shift z` created
   one managed tab, literal `Alt /` moved its rail from 28 to 1 column, and a
   same-WASM tiled lookalike remained unchanged in native inventory, focus,
   loaded-plugin projection, layout, and settled screen. Disposable cleanup
   left standing configuration untouched.

## Refutation evidence

Validation deliberately restored the unsafe fallback in a detached checkout:
when missing/stale focus chose the cached-looking tab position, the committed
regression failed at `src/main.rs:3931` with `Some(3)` instead of `None`.
That proves the test catches the exact cycle-1 defect rather than merely
exercising a happy path. The detached checkout and its temporary build output
were removed.

## Post-merge `WORK` drill

From a terminal pane in `WORK`, after this candidate is merged to `main`:

```bash
cd /Users/clkao/git/zaphod
./scripts/zellij-new-tab.sh --session WORK --name 'Zaphod v3 drill'
```

Expect one fresh active tab with a 28-column rail; grant the ordinary plugin
permission only if prompted. Press `Alt /` to make that same rail a 1-column
sliver, then press it again to restore it. On an existing unmanaged or legacy
tab, `Alt /` must cause no layout, pane, or focus change. Record the created
tab ID plus `list-panes --json --all --command --geometry --state --tab` and
`dump-layout`; the new rail must retain `zaphod_managed_tab "v1"` and its exact
candidate `zaphod_wasm_url`.

## Recommendation

**Approve.** The repair removes the rejected stale-cache authorization source
without broadening the mechanism; no custom PTY, lease, registry, controller,
or further implementation cycle is warranted.
