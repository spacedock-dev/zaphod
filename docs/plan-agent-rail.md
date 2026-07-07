# Agent rail — development plan

> Plan date: 2026-07-07 · executes `docs/prd-agent-rail.md`
> agentsview v0.36.1 · subspace recon at HEAD 9be5fbc · zellij CLI 0.44.1
> Dock container shipped at v3.12 (`docs/docking-approach.md`, adopted architecture)
> Scope: the grout daemon, the row protocol, and the rail's rows section — M1
> through the M2 seam. Walking skeleton first; each sprint exits on a demoed
> criterion, not a checklist.

## Decisions (grill, CL 2026-07-07)

1. **Identity binding lives in the plugin.** The grout tags session rows with
   `cwd`; the plugin matches against its `PaneManifest`; no match renders as
   *unbound*, never guessed. The grout never knows zellij exists beyond
   invoking `zellij pipe`.
2. **Grout in Go**, new `grout/` directory in this repo. It sits beside
   subspace and agentsview (both Go), the SSE client is trivial, and it ships
   as a single static binary.
3. **Row protocol: two typed row kinds over one pipe name, `agent-event`**,
   JSON per line:

   ```
   {kind:"session", id, cwd, agent, state, summary, ts}
   {kind:"gate",    log_path, workflow, entity, entity_title, stage, round, recommendation, ts}
   ```

   The plugin understands the semantics — actions are per-kind.
4. **Gate-log discovery: a grout config of globs for M1** (playground +
   workflow dirs). A canonical gate-log directory is deferred until real
   emitters adopt one.

## Sprint 0 — walking skeleton

Every joint exercised end to end, nothing polished.

- **(a) Plugin pipe-unblock precondition — first, TDD offline.** `ReadCliPipes`
  grant + explicit `unblock_cli_pipe_input` + a `GetPaneRunningCommand` wedge
  mitigation. The spike's landmine: the `zellij pipe` CLI never exits against a
  wedged instance, so the grout piping into a wedged rail hangs — this is the
  one change that protects CL's real sessions, which is why it leads the
  sprint. It reinstates the `ReadCliPipes` grant + explicit unblock the dock
  rework deleted: auto-unblock happens only when `pipe()` returns, which is
  exactly what a wedged instance never does.
- **(b) Go grout skeleton.** Hardcoded config; one agentsview session via a
  one-shot `session get`; one gate log (the subspace playground fixture); emit
  both row kinds via `zellij pipe`, fire-and-forget with a kill timer.
- **(c) Plugin rows.** `agent-event` handler parses both kinds; a minimal rows
  section; click a session row → focus the cwd-bound pane; click a gate row →
  float `subspace-tui` on the artifact.
- Verify row payload size limits (untested) while the skeleton is up.

**Exit:** live demo in CL's fresh zellij session — both row kinds rendered,
both clicks work.

## Sprint 1 — sessions for real

SSE `data_changed` consumption plus `session get --json` enrichment — the
events carry no content. State mapping from agentsview's signals; many
sessions; unbound handling; daily dogfood. The grout absorbs the daemon
quirks the spike found: agentsview auto-spawns its daemon on :8080, and `list`
hides one-shot/automated sessions without flags.

**Exit:** CL stops alt-tabbing to find blocked agents.

## Sprint 2 — gates for real + review dogfood

Glob watching per the config; fold via the subspace binary — never reimplement
fold. Parked/defer rows: defer is rail-local — the agent keeps polling its
decision log for resolution (the log is the channel), so parking costs
nothing. It is *not* subspace `hold`, which resolves and unblocks the waiter.
This project's own review gates run through subspace review in the fresh
session — the rail's first real tenant is zaphod development itself.

**Exit:** a real zaphod dev gate resolved via a rail-surfaced review.

## Sprint 3 — M2 seam

The subspace gate server writes `<log>.addr` beside the log (additive, no
model change); rail verdict actions — approve / revise / reject / hold — POST
to it. Direct log append stays forbidden (single-writer flock).

**Exit:** a verdict issued from the rail lands on the record and unblocks the
waiting agent.

## Dogfood posture

The rail fails visible-not-blocking: dead grout means stale rows, never a
wedged session — the reason the pipe fix leads sprint 0. `install.sh` points
the layout at the repo wasm in place, so `./build.sh` hot-swaps what the next
fresh session loads. Artifact review moves into the fresh zellij session
starting now: pre-sprint-2 gates reviewed there manually via `subspace-tui`;
sprint 2 automates the discovery.

## References & constraints

- PRD this plan executes: `docs/prd-agent-rail.md`. Dock container:
  `docs/docking-approach.md`.
- Landmine and seam facts above are from the 2026-07-07 agentsview spike
  (v0.36.1) and subspace recon (HEAD 9be5fbc), cited inline where they bear.
- Constraints: single-user, single-machine. Row payload size limits are
  untested — sprint 0 verifies.
