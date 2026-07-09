# Validation: Floating sidebar-instance leak and dirty-tab chrome misplacement — suspected shared root cause

Entity: `docs/agent-rail-dev/.spacedock-state/dock-floating-leak-and-chrome-misplacement.md`
Worktree at review: `.worktrees/spacedock-ensign-dock-floating-leak-and-chrome-misplacement` @ `2aaba569bdf4563cc17426186b26ee260374ea59`
Refutation checkout (throwaway, never the implementation worktree): a fresh
clone at the same commit under `/private/tmp/claude-501/.../scratchpad/throwaway-audit`,
also reused (checked out to the parent commit) for the attempted control build.

**Verdict recommendation: REJECT — route back to ideation/implementation with concrete findings.**
The fix is applied correctly and passes its own claimed tests, but the entity's
own designated riskiest-remaining check (the population-seeded spike) shows it
does not close the race at population sizes matching `WORK`'s real, current
state. `dock-toggle-restructures-panes`' (j5) AC-1 should stay parked, not close.

## Offline AC verdicts (independently re-run, not re-read from the stage report)

| Item | Verdict | Command | Evidence |
|---|---|---|---|
| Patch matches ideation cycle 3's drafted diff | PASS | `git show 2aaba56` | Byte-for-byte match against the entity's "Patch for implementation to apply verbatim" section; 57 insertions, 2 hunks |
| Recheck call site at the claimed location | PASS | `Read src/main.rs:895-993` | `self.instances.len() > 1` gate at `:940-956`, immediately before `override_layout` at `:964` |
| `cargo test` (fresh) | PASS | `cargo test` | 133 passed, 0 failed, 0 ignored |
| `cargo check --tests` (fresh) | PASS | `cargo check --tests` | Clean, no warnings |
| AC-1 (leak creation/persistence) untouched | PASS | `git show 2aaba56 --stat`, `grep "^@@"` | Diff scope is exactly `main.rs:930-956` + test module; `should_close_self`/`is_stray_floating_bootstrap`/`MessagePlugin` spawn config all outside the diff |
| AC-2/AC-3/j5-AC-1 (race closed under realistic population) | **FAIL** | population-seeded spike, see below | Corruption reproduced on first clean trial, patch applied |

## Refutation audit (throwaway checkout, never the implementation worktree)

- **Attack A — REFUTED at `main.rs:512` + `main.rs:940`.** The population
  gate (`self.instances.len() > 1`) that decides whether the recheck even
  runs is itself populated only from `Event::PaneUpdate` (`:512`), the same
  manifest-derived state this codebase's own comment at `:914-917` already
  names as lagged relative to a fresh spawn ("immune to the PaneUpdate
  instance-list lag" — said of the *first* dump-based check, which is why
  that one is dump-based and not manifest-based). The new gate does not
  extend that same discipline to itself. Concrete scenario: instance B (an
  existing zombie) receives the identical broadcast "toggle" pipe
  concurrently with a fresh `MessagePlugin` spawn A. If B's most recent
  `PaneUpdate` predates A's registration — entirely plausible, since pipe
  delivery and `PaneUpdate` delivery are independent, unordered event
  streams (confirmed: no ordering guarantee documented in the vendored
  `zellij-tile-0.44.3` crate, and this codebase's own docs, v3.8, already
  describe instances disagreeing on state "at a different rate") — B reads
  `self.instances.len()==1` and skips the recheck entirely, reopening the
  *full* pre-patch race window (up to the first dump's ~1s timeout), not the
  narrowed one. This is exactly the two-instance minimum the fix's own
  design comment (`:937-939`, "a single instance cannot race itself")
  claims to cover — the blind spot is largest exactly where the fix claims
  coverage begins.
- **Attack B — SURVIVES-AS-DOCUMENTED, not a new hole.** Can two rechecks
  both land before either's `override_layout` is applied? Confirmed
  structurally from the vendored host shim
  (`zellij-tile-0.44.3/src/shim.rs:1667`: `dump_session_layout_for_tab` is
  documented "synchronously"; `:2753`, `override_layout`, carries no such
  note and is a one-way `object_to_stdout` dispatch with no wait for
  server-side application). This matches ideation's own honest-limit framing
  verbatim ("the residual window left is only the async dispatch latency of
  `override_layout` itself") — confirmed true from primary source, not
  refuted, not new.
- **Empirical confirmation, not just analysis**: the population-seeded spike
  below reproduces the exact corruption shape Attack A predicts, on the
  first clean trial, with the patch applied. See next section.

## Population-seeded spike (this entity's own Test-plan-designated riskiest-remaining check)

**Setup.** `./build.sh` from the worktree (not the main repo's already-installed
wasm — confirmed via `~/.config/zellij/config.kdl:61` that the global `Alt /`
keybind points at `/Users/clkao/git/zaphod/target/...`, not this worktree's
path; a prior session in this workflow hit exactly this mixup). Disposable
tmux-hosted zellij sessions only, never `WORK` (confirmed read-only afterward:
`WORK` still shows 22 sidebar instances, untouched). A scratch zellij config
repoints `Alt /`/`Alt .` at the worktree wasm, otherwise identical to
`~/.config/zellij/config.kdl`.

**Seeding.** 18 floating, config-matched (`rail=1`) sidebar instances via
`zellij action launch-plugin -f -c rail=1 "file:<worktree wasm>"`, into a tab
that never gets promoted (promoting the seed tab first triggers
`is_stray_floating_bootstrap`'s self-close convention and silently removes
seeded instances — hit this once, corrected the methodology). Organic
per-tab-press seeding was ruled out: this entity's own prior spike found 0
floating zombies left behind across ~600 organic presses, so direct seeding
replicates the *precondition* (many live floating rail=1 instances) without
depending on the still-unreproduced organic trigger — consistent with the
entity's own AC-1 status (creation trigger still not reproduced on demand).

**Trial 1.** Fresh tab, explicitly verified clean immediately before the
press (tab-bar + status-bar + exactly 1 terminal, no sidebar). One `Alt /`
press. Result: sidebar tiled + **two** terminal panes.
`dump-layout`:
```
pane split_direction="vertical" {
    pane name="sidebar" size=28 borderless=true { plugin location="...zellij-sidebar.wasm" { rail "1" } }
    pane focus=true size="50%"
    pane size="50%"
}
```
This is structurally identical to `WORK`'s own Tab #7/#8 finding cited in
this entity's Problem section (sidebar + two real terminals in a 50/50
split, not the canonical sidebar + one full-width pane).

**Trial 2.** Second freshly-created, freshly-verified-clean tab, same
population. Same corruption.

**Trial 3 (traced).** Same population, `debug "1"` enabled, cross-checked
against `zellij.log`. Different failure mode: both the retrofitting
instance's first dump *and* its recheck dump timed out
(`Timeout waiting for session layout`), deferring the retrofit entirely (no
corruption, but no working toggle either). Consistent with this entity's own
prior finding (Spike results, 2026-07-08: `KeybindPipe` can time out with
2+ live instances present) — now shown to also afflict the recheck's own
added round trip under an 18-instance population.

**Methodology correction on the record.** An initial run was contaminated by
`zellij action toggle-floating-panes` (called only to reveal a hidden
first-run permission prompt) itself auto-spawning an empty floating terminal
when none existed — confirmed via an isolated probe (`toggle-floating-panes`
alone, zero panes, zero interaction with the sidebar plugin, still produces a
new floating terminal). Excluded from the reported results above; all
reported trials used a clean methodology with an explicit pre-press state
check.

**Control run (unpatched) — attempted, not completed.** Built the parent
commit (`2aaba56~1`) in the throwaway checkout for a direct comparison. Hit
unrelated environment friction (a fresh-wasm-path permission grant that never
resolved, ending in the tmux client being logged out by the server's own
"1000 consecutive unknown messages" loop guard) before a comparable trial
could run. Not repeated given this was explicitly optional ("if time
allows") in the checklist, and the patched-build finding stands on its own
via the clean before/after methodology (verified-empty tab → one press →
verified-corrupted tab) without needing a control baseline.

## Demo script (optional — for CL's own independent live confirmation)

The finding above was reproduced by this validation pass itself (headless,
via tmux + `zellij action` CLI, not a live human press) — this already
satisfies the "riskiest-remaining check" the entity's Test plan calls for.
The following lets CL reproduce the same result live, if desired, using a
fresh disposable session (never `WORK`):

1. Build from the worktree: `cd .worktrees/spacedock-ensign-dock-floating-leak-and-chrome-misplacement && ./build.sh`
2. Start a disposable session with a scratch config that repoints `Alt /` at
   that build's wasm path (do **not** use your normal `WORK`-attached
   terminal — its global keybind loads the main repo's already-installed
   wasm, not this worktree's).
3. Seed ~18 floating instances:
   `for i in $(seq 1 18); do zellij --session <name> action launch-plugin -f -c "rail=1" "file:<worktree wasm path>"; done`
   (into a tab that is never itself toggled/promoted).
4. Create one more fresh tab, confirm it shows only a bare terminal
   (`zellij --session <name> action list-panes -a`).
5. Press `Alt /` once in that fresh tab.
6. **What you should see:** frequently, two terminal panes appear where one
   should — the same corrupted shape as `WORK`'s Tab #7/#8. Occasionally
   instead: the toggle silently does nothing (a host-timeout deferral,
   distinct failure mode, same underlying population contention).

This is settled by the trial results already run and recorded above, not
solely by CL's own re-run — the demo is offered for independent confirmation,
not because a fresh agent categorically cannot drive this particular
interactive check (it did, this pass).

## Note on scope

AC-1 (the leak's own creation/persistence mechanism) is confirmed still
correctly OPEN and untouched by this diff — not silently implied resolved.
This gate's REJECT recommendation is specifically about AC-2/AC-3/j5-AC-1
(does the fix prevent corruption from a concurrent race), not about AC-1.
