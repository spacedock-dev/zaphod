# Zaphod roadmap

This file is authoritative for product delivery order. Every sprint starts with
an operator trigger, a visible result, and reproducible proof. No sprint puts
its first real action behind a controller, hub, driver, or other component
chain.

The [workspace architecture](zaphod-workspace-architecture.md) remains the
long-term design. It does not authorize replacing useful behavior before an
operator journey proves the replacement better.

The [archived agent-rail prototype plan](archive/plan-agent-rail-prototype-2026-07-07.md),
root prototype documents, and their old sprint numbers are evidence only. They
do not control delivery order or dispatch.

## Delivery rules

- Start with one complete operator journey. State its trigger, visible result,
  and repeatable proof before naming components.
- Preserve working behavior until a replacement proves equal or better value in
  that same journey.
- Name the operator failure before replacing a working surface. Architectural
  neatness alone is not a failure.
- Keep review truth and review resolution in the provider. Zaphod may surface a
  review and open provider UI; it does not render a provider form or issue a
  verdict.
- Run live captain drills only after reproducible offline checks pass.
- Never mutate standing Zellij or tmux configuration during development tests.

## Product baseline and evidence

The shipped rail lists terminal panes and agent state for its current tab.
Grout can surface session and review rows. Session actions focus a bound pane,
and review actions open the provider's UI. Later work must preserve or improve
that behavior.

The old `yb`, `7v`, and `pz` work remains useful evidence: session ingestion,
stale-data handling, focus behavior, and provider-owned resolution. Their
implementation boundaries are not product architecture. In particular, the
rail must not inherit `pz`'s rail-issued `approve` action.

## Sprint 1 — safe managed-tab onramp (shipped)

### Operator journey

**Trigger:** In the selected checkout, the operator presses `Alt Shift z` or
runs the fresh-tab entry command.

**Visible result:** Zellij opens one fresh managed Zaphod tab built from that
checkout's WASM. `Alt /` toggles only the shared rail in an initialized managed
tab; it is inert in foreign, unmanaged, floating, absent, or unpermitted
contexts. Clients viewing the same initialized managed tab operate that tab's
shared rail.

**Reproducible proof:** The tmux-hosted smoke uses isolated, short Zellij
config/data/socket roots; sends literal keys; verifies the candidate WASM in
native pane and layout state; proves `Alt /` changes only the initialized
managed tab's known rail state; proves foreign-tab `Alt /` is a no-op; and
checks cleanup plus standing-root hashes.

### Scope

- `Alt Shift z` stays a native `NewTab` action using the selected checkout's
  absolute rendered layout path.
- Persistent `Alt /` stays `NoOp`; the managed tab owns any safe runtime
  behavior. No key path creates, retrofits, or restructures a foreign tab.
- The entry script and native action must use the invoking checkout's artifact,
  never a stale global layout.

### Evidence and deferrals

The `7h` foreground-PTY/lease experiment is rejected release-path evidence. Its
task record remains untouched; this roadmap neither changes its status nor uses
it as a Sprint 1 prerequisite. `bc`, `qb`, and `6v` remain deferred and do not
auto-dispatch.

`fp` passed the isolated smoke packet and the captain's ordinary-consent drill.
The supported upgrade path is a fresh tab through `scripts/zellij-new-tab.sh`;
an already-running rail is not hot-reloaded in place.

## Sprint 2 — one dependable attention loop

### Entry gate

Sprint 1's managed-tab smoke and captain drill have passed. Sprint 2 may start
its approved walking-skeleton work; it does not wait for `7h`.

### Operator journey

**Trigger:** During normal Zellij work, a live session needs attention and a
pending gate exists.

**Visible result:** The session appears in the rail and focuses its bound pane;
the gate appears and opens its provider UI; after provider resolution, the rail
reflects the new provider state without tab hunting.

**Reproducible proof:** One end-to-end drill proves session appearance and
focus, gate appearance and provider UI opening, and truthful post-resolution
state. Offline checks precede the live drill.

### Scope and deferrals

Sprint 2 delivers the whole attention loop, not a sequence of component
allocations. The re-scoped `fp` walking skeleton owns Sprint 1's narrow entry
bridge; it does not promote a broad managed-tab controller. A hub and pane
adoption remain deferred until the completed loop exposes a measured failure
that requires them. Inline verdicts and rail-owned review routing stay out of
scope because the provider owns those semantics.

## Sprint 3 — evidence-led walking skeleton

Sprint 3 starts only after Sprint 2 records a remaining operator failure. Its
first task must again state a trigger, visible result, and reproducible proof
for one complete journey—for example, recovery after a proven stale-state
failure or a second supported workspace. Do not preallocate Sprint 3 to a
controller, hub, adoption mechanism, or other component.

## Operational boundary

This roadmap does not mutate task records, `7h`, `4d`, or workflow state. Any
prefiled deferred work still needs an explicit captain decision before dispatch.
