# Native hotkey helper-pane refutation

This is a state-owned, repeatable Zellij 0.44.3 spike. It tests the tempting
single-key proposal: one `Alt Shift z` binding runs both `NewTab` and a
floating `Run` helper. It intentionally uses no product script or standing
Zellij configuration.

Run it from the Zaphod checkout:

```bash
./docs/agent-rail-dev/.spacedock-state/spikes/bb-hotkey-helper-pane/run.sh \
  --repo "$(pwd -P)" \
  --out docs/agent-rail-dev/.spacedock-state/spikes/bb-hotkey-helper-pane/recorded
```

The runner starts Zellij inside a dedicated tmux server with temporary config,
data, socket, and HOME roots. It sends literal `ESC Z`, records the native pane
inventory, tab inventory, and visible tmux screen, then deletes the Zellij
session and tmux server. A passing run requires exactly one visible,
non-suppressed floating pane after the key.

`recorded/` is one raw successful run. The conclusion is narrow: Zellij may
create the tab, but `Run` materializes a visible helper pane. Therefore the
native hotkey remains a managed-tab shortcut; it is not a no-helper sidecar
handoff.
