# Zaphod Workspace Architecture

**Status:** Evergreen architecture

## Current Sprint 1 boundary

This document describes the later workspace product, not a prerequisite for
the first operator journey. Sprint 1 has a usable narrow onramp in the existing
Zellij plugin: `scripts/zellij-new-tab.sh --session <name>` creates a fresh
layout-owned tab from the selected checkout without rewriting standing KDL.
A separately installed `Alt Shift z` route can open only its one fixed layout;
it is not a selected-checkout entry. The ordinary `Alt /` path changes the
direct-entry tab's docked/sliver layout. Task `v3` is closing the remaining
adversarial proof that a tiled sidebar-shaped unmanaged resident cannot qualify
for the route; no future architecture may treat visual shape or a URL substring
as ownership.
There is no controller, stable binding record, create-or-focus behavior,
adoption, hub, or portable CLI in this slice.

The tmux-hosted isolated smoke proves the real keys, candidate WASM identity,
managed transition, post-route foreign no-op, and cleanup. A future driver may
replace this seam only after it delivers the same operator journey with a
measured additional benefit.

## Outcome

Zaphod becomes a simple workspace launcher with a portable dock for Zellij and
tmux. The dock lists sessions and review items. A session action navigates to
the correct pane; a review action opens the provider's review surface. The dock
does not render provider-specific forms or resolve reviews.

Each workspace has one lightweight Zaphod hub. The hub subscribes to event
sources, keeps an ephemeral workspace projection, and routes dock actions. An
event source may remain global: each workspace hub can subscribe to AgentsView
and filter its global stream without introducing a global Zaphod daemon.

## Context

The current prototype proved the Zellij plugin system and paid down difficult
Zellij layout, focus, and pipe behavior. Its two processes now divide work at
the wrong boundaries:

- The Zellij WASM owns multiplexer mechanics, row state, rendering, navigation,
  and review commands.
- `grout` owns AgentsView ingestion, gate discovery, subspace server startup,
  normalization, and `zellij pipe` transport.
- The event format fixes two row kinds, `session` and `gate`.
- Review actions name `subspace-tui` and `curl` inside the Zellij view.

Those choices served the spike. A multiplexer-independent product needs the
typed attention-item stream as its center, with Zellij, tmux, AgentsView, and
review tools behind replaceable boundaries.

## Goals

The first release must:

1. Offer one public launcher, `zaphod`.
2. Create a Zellij or tmux workspace, adopt the current session, or attach to
   an existing session without changing foreign tabs or windows.
3. Run one hub for each active canonical workspace root.
4. Show sessions and review items in a portable dock TUI.
5. Focus a bound session pane from the dock.
6. Open a review in its provider's own UI.
7. Support watched sources and a safe hook ingress for tools that push events.
8. Keep provider truth in the provider's durable store.
9. Expose failures without erasing the last good projection.
10. Test both multiplexers through the same behavioral contract.
11. Converge on one managed view per workspace/session binding.
12. Move a pane into the managed view only after an explicit user action.

## Non-goals

The first release excludes:

- inline questions, forms, comments, or verdict controls;
- a global Zaphod broker or cross-workspace dashboard;
- persistent item history;
- arbitrary commands supplied by events;
- automatic reassignment between workspaces;
- a provider marketplace or automatic provider discovery;
- a cross-multiplexer dock-toggle abstraction;
- automatic dock insertion into every future tab or window;
- retrofitting a dock into a foreign tab or window;
- automatic pane adoption; and
- a patched or forked Zellij.

## User experience

### Entry paths

The same command supports three entry paths.

#### Start outside a multiplexer

```text
zaphod [PATH]
```

Zaphod resolves the canonical workspace root and chooses Zellij or tmux. A
`--mux zellij|tmux` argument overrides the user's configured default. With no
argument or configured default, Zaphod selects Zellij when available and tmux
otherwise. It attaches to the workspace's managed session when one exists;
otherwise, it creates the session.

#### Start inside Zellij or tmux

```text
zaphod [PATH]
```

Zaphod detects the current multiplexer and session. It adopts that session,
starts or reuses the workspace hub, and creates or focuses the binding's
managed tab or window. It leaves the invoking foreign view unchanged and never
starts a nested multiplexer.

Inside Zellij, the current Sprint 1 entry is intentionally simpler: the direct
script creates one fresh layout-owned tab from the selected checkout, and
`Alt /` works only in that initialized tab. It does not create-or-focus, adopt
the invoking tab, rewrite global key policy, or record a stable binding. The
future driver described below may add idempotent create-or-focus only when an
observed operator failure requires it.

#### Attach to an existing session

```text
zaphod attach SESSION [PATH]
```

Outside a multiplexer, Zaphod attaches to the named session and initializes its
workspace runtime when needed. Inside the same multiplexer, the driver uses the
native session-switch or adoption operation instead of nesting another client.
An existing binding identifies the driver. An unbound session name requires
`--mux zellij|tmux` when both multiplexers are available.

The future launcher paths are idempotent: repeated entry reuses the hub and
repairs or focuses one managed view without duplicating panes or tabs. One
canonical root may have only one active session binding, and one session may
bind only one root. Zaphod reports either conflict and requires `--rebind`; it
never guesses which binding to replace. That convergence behavior is expressly
deferred from the current fresh-tab onramp.

### Managed view and pane adoption

Each binding owns one Zaphod-managed Zellij tab or tmux window. Zaphod creates
that view from its own layout, so it owns the dock, terminal slots, and swap
layouts from birth. Other tabs and windows remain foreign, even when they
belong to the same multiplexer session.

A user may explicitly adopt an existing terminal into the managed view. The
driver moves the native pane; it does not reconstruct or respawn it. A
successful move preserves the pane ID and process PID, leaves unrelated panes
untouched, and may close an emptied source view according to multiplexer
behavior. Zaphod never automates the consent or permission response.

### Dock

The portable TUI opens as a 32-column dock in the managed tab or window. It has
two sections:

```text
ZAPHOD · zaphod             ● live
zellij · 3 sources · 1 stale

SESSIONS
● codex · test infra
  awaiting input · 12s
● claude · gate plumbing
  working · 41s
○ pi · reviewer
  unbound · 3m

REVIEWS
◆ Spacedock gate
  canonical install · ready
◇ Plan review
  workspace hub · pending
? AskUserQuestion
  choose storage boundary

j/k select · enter focus/open · ? help
```

Keyboard and mouse selection invoke the same action. `Enter` on a session
focuses its bound pane. `Enter` on a review opens a provider-owned UI in a
Zellij floating command pane or tmux popup, with a normal split as the tmux
fallback.

The first release has one dock per workspace/session binding. Running `zaphod`
from another tab or window focuses the same managed view. It never inserts a
dock into the invoking foreign view.

## Architecture

```text
AgentsView SSE -----\
gate artifacts ------> provider adapters -- upsert/remove --> workspace hub
tool hooks ----------/                                      filter · model · route
                                                                    |       |
                                                     snapshot/delta |       | action
                                                                    v       v
                                                              portable   action
                                                                dock      router
                                                                            |
                                                           provider adapter + mux driver
                                                                            |
                                                                            v
                                                    managed view · review surface · pane focus
```

### Launcher

`zaphod` owns workspace resolution, multiplexer selection, create/adopt/attach
behavior, and runtime convergence. It presents a small public command surface;
internal hub and dock commands remain implementation details.

Workspace identity starts with the canonical root. Its active binding records
the multiplexer kind and native session identifier. Small records in Zaphod's
platform runtime directory support attach and adoption without a global broker
process. A binding admits one hub and exactly one managed view. The binding
records the view's native stable ID, not only its display name. A reserved name
helps discovery but never overrides an inconsistent or duplicate binding.

### Workspace hub

The hub owns four concerns:

1. Start provider adapters and receive their events.
2. Filter events to the workspace without guessing across roots.
3. Hold the current in-memory item projection and source health.
4. Route `focus` and `open` actions.

The hub is not a durable database. AgentsView, Spacedock, subspace, or the
originating tool remains authoritative. A replaceable cache may speed startup,
but a fresh provider snapshot supersedes it.

The workspace-local Unix socket and an exclusive lock enforce the singleton.
The hub survives client detach and exits after its multiplexer session
disappears. Zaphod never deletes or renames an adopted session; it cleans up
only its socket, hub process, dock panes, and ephemeral binding. Hubs consult
the runtime binding records when global sources contain nested roots: the
longest active canonical-root match owns the item.

### Portable dock TUI

The dock owns sections, selection, keyboard and mouse input, status rendering,
and transient error messages. It consumes the local item protocol and emits
logical action intents. It contains no AgentsView, Spacedock, subspace, Zellij,
or tmux integration code.

### Provider adapters

A provider adapter maps native data to canonical items. It owns native parsing,
freshness rules, durable references, and trusted review-tool arguments.

The first release includes:

- an AgentsView adapter for sessions;
- a Spacedock/subspace adapter for gate artifacts and `subspace-tui`; and
- a registered hook ingress for plan and question providers.

Two ingestion forms share one item contract:

- **Watch:** a long-running adapter emits snapshots and changes from a durable
  source.
- **Notify:** a tool writes one scoped item to `zaphod notify` on standard
  input. The notification names a registered provider and logical action; it
  cannot name an executable or supply shell arguments.

Inside an adopted session, `zaphod notify` resolves the hub from the inherited
workspace socket. Outside a session, the caller must pass `--workspace PATH`.
The command fails when it cannot resolve exactly one active hub.

When the user invokes `open`, the hub asks the registered provider adapter to
prepare the review surface. The adapter returns a trusted launch request to the
multiplexer driver. Event data remains inert.

### Multiplexer drivers

Zellij and tmux implement the same driver contract:

```text
ensure_workspace
ensure_managed_view
current_view
list_panes
focus_pane
adopt_pane
toggle_managed_layout
open_surface
session_exists
```

`ensure_managed_view(binding)` creates the managed tab or window from a Zaphod
layout when absent and otherwise focuses its recorded stable ID.
`toggle_managed_layout` succeeds only when the invoking pane resolves to that
ID. `adopt_pane` requires an explicit pane ID and user action. Drivers fail
closed on stale bindings, duplicate reserved names, missing controllers, or
permission refusal.

The drivers translate these operations into native commands and layouts. A
provider never calls Zellij or tmux; the dock never calls either multiplexer.

#### Zellij controller boundary

This is a future driver boundary, not a Sprint 1 implementation dependency.
Sprint 1 deliberately avoids a controller, `Run` pane, stable managed-tab
record, and portable key-request CLI: its real entry is the layout-owned tab
and guarded `MessagePluginId` route described above.

The option-2 spike used CLI helpers launched through Zellij's `Run` action to
prove create-or-focus and guarded toggle behavior. That harness is not the
product keybinding: a tiled `Run` pane briefly took focus and reduced an
80×24 foreign pane to 80×12 before restoring it. `current-tab-info` also failed
for the transient CLI client, while mapping `ZELLIJ_PANE_ID` through
`list-panes --json --all` identified the invoking tab reliably.

The durable Zellij driver therefore includes a thin native controller. Both
keybindings target it. For `Alt Shift z`, it executes the native half of the
launcher's validated create-or-focus request. For `Alt /`, it resolves the
invoking pane and stable tab ID before calling the managed swap-layout action.
It also moves explicitly selected panes with `break_panes_to_tab_with_id`.

The portable launcher still derives bindings, serializes convergence,
validates names and IDs, and preflights the controller artifact. External
entry and tests may use the CLI's create-or-focus path directly. Providers,
hub routing, item state, and dock rendering never enter the controller.

The spike moved terminal pane `0` into managed tab `2` without changing its PID
and without replacing an unrelated terminal. It also proved that stale IDs,
unbound reserved names, missing source panes, and permission denial can leave
foreign layouts unchanged. A missing controller must fail before launch;
Zellij otherwise opens an error float. The driver waits for
`PermissionRequestResult` and never sends an automated consent keystroke.

The tmux driver offers the analogous managed-window contract. Its native
implementation may differ, but the observable guards and identity guarantees
remain the same.

The Zellij driver should preserve the prototype's verified layout and focus
knowledge when that knowledge serves this contract. It should not preserve
WASM-owned polling, normalization, rows, or provider commands. The tmux driver
implements the same observable behaviors with tmux panes and popups.

## Canonical item model

The hub and dock exchange a small presentation model:

```text
Item {
  id:          provider ID + stable native key
  workspace:   canonical root or explicit workspace scope
  group:       session | review
  subtype:     agent | gate | plan | question | provider-defined
  title:       string
  summary:     string
  updated_at:  timestamp
  status:      { label, tone }
  location?:   { cwd, provider_session_key }
  artifact?:   { provider_reference, media_type }
  actions:     [focus | open]
}
```

`tone` is one of `neutral`, `info`, `attention`, `success`, or `error`. The
provider supplies the label and maps its native state to a tone. The dock does
not interpret provider-specific states.

The hub derives pane bindings from item locations and the multiplexer pane
snapshot. It enables `focus` only for an unambiguous match. An item without a
match may remain visible only when the provider explicitly scoped it to the
workspace.

## Local protocol

The dock connects to a workspace-local Unix socket. NDJSON frames evolve the
prototype's existing JSON-line event stream:

```text
hello    { protocol, client_id }
resync   { after_seq? }
snapshot { seq, items: [...] }
upsert   { seq, item }
remove   { seq, id }
invoke   { request_id, id, action: "focus" | "open" }
result   { request_id, ok, message? }
```

On connection or reconnection, the dock requests a full snapshot. Subsequent
upserts and removals are idempotent. A sequence gap forces another snapshot.
The hub coalesces queued changes by item ID; a slow dock cannot backpressure a
provider indefinitely.

The protocol never carries executable paths, shell fragments, or untrusted
argument vectors.

## Data flows

### Session update and focus

1. The AgentsView adapter consumes its global SSE stream.
2. The adapter emits session items with stable IDs and locations.
3. The hub filters the items to its canonical workspace root.
4. The hub associates each item with at most one pane from the driver snapshot.
5. The dock receives an upsert and renders the session.
6. The user invokes `focus`.
7. The hub revalidates the binding and asks the driver to focus the tab or
   window and pane.

### Review discovery and open

1. A watch adapter discovers an artifact, or a registered tool calls
   `zaphod notify`.
2. The hub validates the provider, workspace scope, item shape, and action.
3. The dock renders the item under `REVIEWS`.
4. The user invokes `open`.
5. The registered provider prepares a trusted launch request.
6. The driver opens the provider UI in a floating pane, popup, or fallback
   split.
7. The provider UI writes the durable response. The adapter later reflects
   the resulting state through a normal item update or removal.

Zaphod never infers that opening a surface approved or resolved a review.

## Failure handling

| Failure | Required behavior |
| --- | --- |
| Provider unavailable | Keep the last projection, mark it stale, reconnect with bounded backoff, and show one provider-health line. |
| Hub unavailable | Show `reconnecting` in the dock and retry. A singleton restart must acquire the socket lock. |
| Malformed event | Reject that event, retain the previous good item, and attribute the error to its provider. |
| Action failure | Show the error and preserve provider truth. Do not claim navigation or launch succeeded. |
| Ambiguous pane binding | Disable `focus`; never guess. |
| Slow dock | Coalesce updates by item ID and replace deltas with a fresh snapshot when needed. |
| Sequence gap | Request a full snapshot before applying more deltas. |
| Conflicting workspace binding | Report the bound root and require explicit rebind. |
| Stale managed-view ID | Report the stale binding; do not use a matching display name or create another view. |
| Duplicate reserved view name | Fail visibly and require repair; do not choose either view. |
| Toggle from a foreign view | Make no layout, pane, focus, or process change. |
| Pane adoption denied or unavailable | Preserve the source pane and PID; report the refusal or missing controller. |

## Security boundaries

Provider identity, not event content, authorizes behavior. Zaphod loads trusted
provider registrations from its XDG user configuration, normally
`~/.config/zaphod/config.toml`. Workspace files may enable a registered
provider but cannot define executables. `zaphod notify` rejects unknown
providers, unsupported actions, ambiguous workspace scope, and executable or
argument fields.

The hub passes only provider-prepared launch requests to a multiplexer driver.
It logs the provider, item ID, action, and result without recording private
artifact contents.

## Verification

Verification proceeds from pure contracts to live multiplexers:

1. **Pure model tests:** normalization, workspace filtering, status mapping,
   expiry, pane binding, and idempotent upsert/remove.
2. **Protocol tests:** reconnect snapshots, malformed frames, sequence gaps,
   slow consumers, action results, and command-injection refusal.
3. **Provider contract tests:** recorded AgentsView and gate fixtures, source
   outage and recovery, notify validation, and trusted launch preparation.
4. **Driver tests:** the same managed-view convergence, guard, adoption,
   list/focus/open, and failure scenarios against fake Zellij and tmux binaries.
5. **Disposable live profiles:** extend the isolated Zellij profile introduced
   by the canonical install and worktree test work, and add an equivalent tmux
   fixture. Inspect live pane state rather than generated configuration.
6. **End-to-end acceptance:** a real session and gate appear; session focus
   navigates to the correct pane; review open launches the provider UI without
   resolving it.

The live acceptance suite must prove:

- `zaphod` creates a workspace from an ordinary shell;
- bare `zaphod` inside Zellij or tmux adopts the current session without
  nesting;
- `zaphod attach` initializes or reuses an existing session;
- repeated entry leaves one hub and exactly one managed view for the binding;
- entry from a foreign view focuses the managed view without changing the
  foreign pane inventory or geometry;
- `Alt /` advances one known swap-layout state in the managed view and has no
  layout effect elsewhere;
- explicit pane adoption preserves the pane ID and process PID;
- stale IDs, duplicate reserved names, missing controllers, and denied
  adoption fail visibly without creating a second managed view;
- detach and reattach preserve the runtime while the session exists;
- an ambiguous session stays unfocused;
- a provider failure marks stale data without clearing unrelated items;
- gate open launches `subspace-tui` but records no verdict by itself;
- registered notify events appear and unknown providers fail closed;
- no path mutates standing global multiplexer configuration; and
- a missing live dependency fails the required drill instead of reporting a
  skipped green result.

## Migration from the prototype

Migration should preserve proved behavior and move ownership in five cuts:

1. Define managed-view bindings and the shared driver contract.
2. Build the thin Zellij controller, idempotent create-or-focus entry, guarded
   toggle, and explicit identity-preserving pane adoption.
3. Define the canonical item and local socket contracts around the existing
   `grout` normalization code, then build the portable dock.
4. Add the tmux managed-window driver and run the shared driver contract
   against both multiplexers.
5. Move AgentsView and review ingestion behind provider contracts, then add
   the registered notify ingress.

The old foreign-tab retrofit work (`j5`, `eh`, `fw`, and `4d`) no longer blocks
this path. Its findings remain evidence for the foreign-view boundary. The
parked upstream transactional-retained-pane request is optional research, not
a release dependency.

The implementation language is not an architectural requirement. The plan
should favor reuse of the existing Go `grout` code for the hub and provider
adapters, but it may select any portable TUI stack that satisfies the socket
and driver contracts.

## Release boundary

The minimum release ships:

- `zaphod` create, adopt, attach, and idempotent managed-view behavior;
- one workspace hub and local socket protocol;
- one portable dock TUI;
- Zellij and tmux drivers with managed-only toggle and explicit pane adoption;
- AgentsView and Spacedock/subspace adapters;
- registered `zaphod notify` ingress;
- session focus and review open actions; and
- source health, stale state, and actionable errors.

This boundary turns the Zellij spike into a small workspace product without
discarding its useful evidence or carrying its multiplexer-specific structure
into every future integration.
