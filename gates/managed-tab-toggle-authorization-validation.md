# Validation: Alt-/ only changes a Zaphod-created tab

Candidate: `27f782ea85765433ab4824e9d4ad929ae5085c70` in
`.worktrees/spacedock-ensign-managed-tab-toggle-authorization`.

## Recommendation

**Reject and return v3 to implementation.** The normal managed-tab journey is
green, but a stale active-tab error can still authorize its runtime `Alt /`
pipe. That violates the task's fail-closed receipt boundary; do not spend
captain time on the WORK drill yet.

## Decisive finding

The route receiver uses `current_active_tab()`.

1. It falls back to the cached display position when
   `get_focused_pane_info()` fails or its stable tab ID cannot be mapped from
   the current `TabUpdate` (`src/main.rs:758–768`, `1868–1886`).
2. A former managed rail cached at position 3 therefore receives `Some(3)`.
3. With an otherwise valid marker, exact URL, permission, tiled placement, and
   Keybind source, `should_accept_observed_toggle_pipe(..., own_tab=3,
   active_tab=Some(3))` returns true.

This was reproduced in a detached throwaway checkout with a one-test probe;
the probe failed at its assertion: `a stale cached tab must not authorize
receipt`. The checkout was removed afterwards.

## What independently passed

| Check | Result |
|---|---|
| Rust tests | `cargo test -q`: 137/137 |
| Test compile | `cargo check --tests`: pass |
| Entry suite | 9/9 pass |
| Real Zellij boundary | tmux-hosted fresh entry, managed toggle, same-WASM lookalike inertness, and disposable cleanup: pass |
| Throwaway layout attacks | missing/wrong/duplicate marker; missing/suffix URL; and title/CWD/geometry/rail lookalike: all rejected |

## Narrow repair

Keep the ordinary entry and tmux harness. Add a strict active-tab lookup only
for `Alt /` authorization: it returns no tab if focused-pane lookup fails or
its stable ID is not in the current mapping. Use it for both runtime-route
offer and pipe receipt. Add pure tests for error and unmapped-ID paths, then
replay this same offline packet. No lease, custom PTY, controller, registry,
or Sprint 2 work is needed.

## Deferred captain drill

After the repair validates, pane 68 in `WORK` can run:

```bash
./scripts/zellij-new-tab.sh --session WORK --name 'Zaphod v3 drill'
```

Approve normally if prompted; literal `Alt /` must toggle the fresh tab.
Switch to Chaplin or a legacy tab and press `Alt /`: nothing changes. Return
to the fresh tab and confirm it still toggles.
