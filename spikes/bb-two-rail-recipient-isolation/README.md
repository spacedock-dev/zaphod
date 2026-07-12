# Two-rail recipient proof

The runner creates a private tmux-hosted Zellij 0.44.3 profile and two actual
Zaphod rail panes in separate tabs. It selects the recipient only from the
native list-panes record: profile/session, tab id, plugin-pane id, and exact
canonical WASM URL. The terminals share one CWD solely to make a later
binding/focus mistake observable; CWD is not used to choose a recipient.

Run:

    ./docs/agent-rail-dev/.spacedock-state/spikes/bb-two-rail-recipient-isolation/run.sh \
      --repo "$(pwd -P)" \
      --out docs/agent-rail-dev/.spacedock-state/spikes/bb-two-rail-recipient-isolation/recorded

A PASS means the bystander did not render the marker. A FAIL is equally useful:
it records that the current rail accepts the server-wide pipe despite the
recipient argument, demonstrating that receiver-side admission must be added
before bb implementation can be approved. The script never uses --plugin.
