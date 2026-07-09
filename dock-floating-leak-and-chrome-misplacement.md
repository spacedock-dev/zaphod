---
id: ehvya2cwpsk28m8thtncbxe2
title: Floating sidebar-instance leak and dirty-tab chrome misplacement — suspected shared root cause
status: ideation
source: finding — spun out of dock-toggle-restructures-panes ideation (AC-2, AC-4), 2026-07-08
started: 2026-07-08T07:50:19Z
completed:
verdict:
score: 0.85
worktree:
issue:
pr:
mod-block:
---

## Problem

Two live, independently-confirmed defects surfaced during ideation of
`dock-toggle-restructures-panes` (AC-2 and AC-4 there), spun out here because
neither is understood well enough yet to fix, and a plausible shared root
cause ties them together:

**1. Floating sidebar-instance leak (AC-2 finding).** Read-only
`dump-layout` of CL's live `WORK` session found 62 stray floating
`zellij-sidebar.wasm` panes accumulated across its tabs — 14 in "Noteplan",
47 in "CEO", 1 in "Tab #6", 0 in "GTM". Each is a config-matched (`rail
"1"`) bootstrap instance that never got promoted to a tiled resident and
never closed itself — `is_stray_floating_bootstrap`
(`src/main.rs:1375-1384`) only fires once its own tab holds a *tiled*
sidebar, which these tabs never got. Zellij's shared log
(`/tmp/zellij-501/zellij-log/zellij.log`) shows repeated `No such file or
directory` bursts for the sidebar wasm path at points across the session's
lifetime, consistent with a toggle firing while `./build.sh` was
mid-rebuild — a plausible contributor, though the log is shared across
other concurrently running sessions/worktrees building the same plugin, so
the correlation is suggestive, not conclusive.

**2. Dirty-tab chrome misplacement (AC-4 finding).** Live in the same
`WORK` session, tab "Tab #6": rail docked, CL closed one of the tab's other
panes (dirtying the swap layout), then toggled. Result, captured via
`dump-layout`: `zellij:tab-bar` landed as a `pane size="50%"
borderless=true { plugin location="zellij:tab-bar" }` **sibling inside the
vertical split** instead of its canonical `pane size=1 borderless=true` top
row — visibly breaking the tab bar (cramped into ~40-50% width, nothing
correctly below it). `status-bar` stayed correctly placed. A static trace
of `extract_chrome_panes` (`src/main.rs:1613-1667`) against the reported
shape found no obvious classification bug — the 3-line multi-line-child
form is exactly the case that function's branch is built to catch. Four
repro attempts in a clean, single-sidebar-instance disposable session (dock
→ add panes → toggle; close-to-2 → toggle; close-to-1 → toggle; a
`command=`/`start_suspended` pane matching CL's `spacedock` pane's shape →
toggle twice) **all toggled cleanly** — none reproduced the defect.

**Suspected shared root cause (unconfirmed).** `WORK`'s corrupted tab lives
in a session already carrying dozens of leaked floating instances (finding
1) in other tabs; the disposable sessions that failed to reproduce finding
2 only ever had one sidebar instance alive. `docs/docking-approach.md`'s
"Tab ids vs positions" section already documents a stale `tab_id →
position` translation race under load, and the plugin's "relaxed election"
lets any instance that perceives a sidebar-less or dirty active tab act on
it. A leaked zombie elsewhere racing a legitimate regenerate on the same
tab — each building its `override_layout` transform from a dump taken at a
slightly different instant, both writing to the same tab — is a plausible
way to corrupt chrome placement that neither actor's transform alone would
produce, and would explain why single-instance sessions can't reproduce it.
Not verified; this is the leading lead, not a conclusion.

### Spike results (2026-07-08, ideation pass) — candidate trigger not reproduced; a different, real concurrency signature found instead

Ran the concurrency spike this entity's Test plan calls for, in a disposable
`zellij --session ztest-leak` (tmux pty, real `Alt /` / `Alt <N>` keypresses
via `tmux send-keys`, `dump-layout` for read-only inspection, never touching
`WORK`), escalating through four rounds rather than stopping at one negative
result:

1. **Candidate trigger as stated.** `touch src/main.rs && ./build.sh` (forces
   a real ~4-6s relink — confirmed distinct from a no-op incremental build,
   which finishes in ~0.5s) looped 15x while a second loop fired `Alt /`
   into a single tab every 150ms for ~30s. A 5ms-resolution poll of the
   output path during an isolated forced rebuild never observed the file
   missing — cargo's replace is atomic on this filesystem, so the "mid-rebuild
   ENOENT window" the candidate trigger assumes may not exist at all here.
   Result: one tiled resident, zero floating panes.
2. **Faster/longer variant.** 8 rebuilds interleaved with `Alt /` every 80ms
   for ~32s, same single tab. Same clean result.
3. **Forced load failure (harder than "mid-rebuild").** Moved
   `zellij-sidebar.wasm` aside entirely for ~3s (not just a narrow relink
   window), fired `Alt /` three times into a fresh sidebar-less tab, restored
   the file. No literal `No such file or directory` was logged this round
   (that exact error class only appears earlier in the shared log, at
   14:18-14:35, well before this test started — likely other activity on the
   machine; timing between a keypress and the plugin loader's actual file
   read isn't tight enough to be certain my presses landed disk-side during
   the window). What the log did show, at the same millisecond as each press:
   bursts of 6-9 consecutive `Plugin with id: N not found`
   (`zellij-server/src/plugins/mod.rs:1320`/`:1326`), and — right as the file
   was restored — a plugin (id 3) that loaded, had `ReadApplicationState`
   denied for both `PaneUpdate` and `TabUpdate`
   (`zellij-server/src/plugins/wasm_bridge.rs:2208`), and exited cleanly
   (`Bye from plugin 3`, logged twice,
   `zellij-server/src/plugins/wasm_bridge.rs:494`). No floating pane was left
   behind; the tab landed tiled on the press that finally succeeded.
4. **Multi-instance race — the entity's actual hypothesis.** With 2 (then 6)
   tabs each already carrying a tiled resident, fired `Alt /` at a fresh
   sidebar-less tab (one press, then rapid `Alt <N>`-then-`Alt /` cycling all
   6 tabs for 24s, concurrent with 6 forced rebuilds). One press logged
   `Action KeybindPipe did not complete within 1s timeout`
   (`zellij-server/src/route.rs:75`) — the toggle pipe itself can time out
   with 2+ live instances present — and the 6-tab round reproduced the same
   `Plugin with id: N not found` bursts, up to 34 consecutive ids in one
   burst. Every round still converged: `dump-layout` afterward showed exactly
   one tiled sidebar per tab, zero `floating_panes` blocks anywhere, and
   chrome correctly placed in all 6 tabs.

**Conclusion.** Across ~600 presses spanning single-tab/6-tab,
single-instance/6-instance, and clean/missing-wasm conditions, neither AC-1's
leak nor AC-2's chrome misplacement reproduced. The candidate trigger
(`Alt /` during a `./build.sh` rebuild window) is not sufficient on its own,
including under materially harder versions of it. What is real and
reproducible on demand instead: a missing/failed plugin load and a
multi-instance toggle both generate internal churn — orphaned
plugin-id registrations (up to dozens per event) and, once, a pipe timeout —
that the existing dedup/cleanup machinery absorbed every time it was
observed here. This weakens, without disproving, the shared-root-cause
hypothesis: the concurrency machinery is demonstrably raceable, but not, in
this pass, in a way that leaves a visible floating zombie or corrupts
chrome. The likeliest explanation for the gap: `WORK` ran this same class of
race continuously over 3+ hours of varied real usage (many tabs,
attach/detach, concurrent builds from other worktrees sharing this log) — a
regime a ~5-minute synthetic burst under-samples by orders of magnitude,
not one it's shown not to trigger under. AC-1 and AC-2 are **not met this
pass** — see revised ACs below.

### Docs check — does "Tab ids vs positions" already document this race?

Read `docs/docking-approach.md:518-544` ("Tab ids vs positions" plus its
immediate siblings in the "Toggle v3" section) and the full "Toggle
v3.7-v3.12" hardening section (`docs/docking-approach.md:567-656`) in full,
per the checklist. **No** — "Tab ids vs positions" itself documents only the
stale `tab_id → position` translation fail-safe (a dump that returns no tab
node falls back safely); it says nothing about floating-instance
accumulation or about the `KeybindPipe`-timeout / orphaned-plugin-id-burst
signature found in this pass's spike — those are not documented anywhere in
this file. What the surrounding v3.7-v3.12 section *does* already document,
and this task must build on rather than re-invent, is that races between
concurrent instances are an accepted, designed-for condition with three
existing dedup/defer layers: the **relaxed election** (any instance may act,
v3.8), the **dump-abort seeder dedup** (a fresh dump aborts a redundant
retrofit, v3.7), and **defer-never-blind-absorb** (an instance with stale
state defers instead of destructively absorbing, v3.9) — plus "the
higher pane-id one is redundant and closes itself" for duplicate tiled
rails. If AC-1/AC-2 are eventually confirmed, the fix is very likely a gap
in one of these three existing layers (or in `KeybindPipe`'s own timeout/
retry behavior, which none of the three cover), not a new locking mechanism
layered on top.

### Third possible symptom raised mid-pass (team-lead): `GetPaneRunningCommand`
timeout correlation with the leak — checked, inconclusive from the shared log

Team-lead reported a live `WORK` observation: the FO pane's `self.rows`
classification (`agent.rs`, a separate code path from AC-1/2/3's toggle/
retrofit scope) showed "unknown . unknown" for a stretch, then
self-corrected a few minutes later with no user action — consistent with a
transient `GetPaneRunningCommand` timeout that `enrich_fields` doesn't retry
aggressively. Asked whether timeout frequency correlates with live
floating-instance count (more instances → more host-call contention → more
misclassification).

Checked against this pass's own log window (retrospective, no new live
test): bucketing every `GetPaneRunningCommand`/`GetPaneCwd` timeout in the
shared log by minute across 12:30-16:07 shows continuous timeout activity
throughout, including 20-33/minute bursts at 14:44-14:57 — well before this
pass's spike started (~15:07) — of the same magnitude as the 20-31/minute
bursts seen during this pass's heaviest hammering (15:53-16:01). The five
plugin ids carrying nearly all the volume (4, 16, 73, 82, 85; 302/186/135/
84/58 timeouts respectively) look at first like a stable set of long-lived
residents, but plugin ids are reused across process lifetimes (id 4 logged
its own "Bye" — a close — at 13:00:22, then resumed timing out under the
same id number two hours later), so they cannot be attributed to a specific
session from this log alone — the log has no session tag, only process-local
ids, exactly the shared-log ambiguity this entity's own Problem section
already flagged for the ENOENT correlation. **Inconclusive, not settled**:
the data is consistent with team-lead's hypothesis (this pass's own
heaviest-activity window does show elevated timeouts) but equally
consistent with ambient contention from `WORK`/other concurrent sessions
unrelated to this pass's induced instance count — the pre-spike 14:44-14:57
burst proves elevated timeout rates happen independent of anything this
pass did. Settling it needs either session-tagged log instrumentation (a
code change) or an isolated single-session before/after measurement (no
other zellij sessions competing for the same PTY thread) — bigger than a
retrospective check on the existing shared log can deliver. Recommend
folding this into whichever follow-up (soak test or live tracing, see
Proposed approach) is chosen next: instrument or isolate enough to make the
correlation answerable, rather than re-attempting it from this log.

### Live re-verification (2026-07-09, ideation cycle 2) — Tab #7 confirmed, Tab #8 shows the identical corruption, root cause substantially narrowed

CL reported a live third symptom in `WORK`: Tab #7 (freshly created, single
`Alt /`) showing `plugin_88` (sidebar) plus **two** real terminal panes
(`terminal_41`, `terminal_42`) instead of one. Re-verified independently this
pass via fresh, read-only `list-panes -a` and `dump-layout` against `WORK` —
no keypress or action sent to any tab, no zombie touched, per this pass's
look-don't-touch constraint.

**Independent confirmation, plus a second occurrence CL had not flagged.**
`list-panes -a` confirms Tab #7 exactly as reported: `plugin_88` (sidebar,
tiled) + `terminal_41`/`terminal_42`, both real `/bin/zsh` at cwd
`/Users/clkao`. **Tab #8 — not mentioned in CL's report — shows the
byte-for-byte identical shape**: `plugin_91` (sidebar, tiled) +
`terminal_43`/`terminal_44`, same cwd, same x-offsets (28/126), same 98-col
widths. `dump-layout` confirms both tabs' outer split as the same three flat
siblings: `sidebar (size=28)`, `pane size="50%"`, `pane size="50%"` — direct
siblings, no wrapper container. Two independent, naturally-occurring
fresh-tab-plus-single-toggle events produced the identical corrupted shape,
ruling out one-off randomness and arguing for a mechanism that is
deterministic-under-precondition, not a rare fluke.

**Zombie count re-checked, close to but not exactly CL's figure.** Currently
14 floating sidebar instances in "Noteplan" + 4 in "CEO" = 18 (CL's dispatch
reported 16: 12+4) — the qualitative precondition (dozens of live floating
zombies) holds; the small drift is consistent with a few minutes' elapsed
churn between captures, not a correction to the finding.

**Code trace: no bug in the transform itself — third independent static
trace to reach that conclusion.** Walked `install_split_preserving_swaps`
(`src/main.rs:895-969`) and `split_preserving_layout_kdl`
(`src/main.rs:1467-1530`, plus `extract_chrome_panes`,
`src/main.rs:1619-1673`) end to end against both tabs' reported shape. The
transform logic is a pure, single-dump-in/single-KDL-out function with no
shared mutable state; for a genuinely fresh tab (`new_tab_template`'s single
`pane cwd="/Users/clkao"`, confirmed in this pass's own `dump-layout`,
`region.len() == 1`), it produces exactly the 2-way rail+1-pane split the
`region.len() == 1` branch is built for (`main.rs:1494-1499`). No
classification or counting bug found — the same negative result the parent
entity's independent AC-4 trace reached for `extract_chrome_panes`
(`dock-toggle-restructures-panes.md:142-151`). Two static traces, by two
different passes, against two different symptom shapes, both clean: the bug
is not in the KDL-generation logic.

**The gap: `install_split_preserving_swaps` has no mutual exclusion across
concurrent invocations targeting the same tab.** `decide_toggle`'s final
branch (`main.rs:1120-1138`) intentionally lets *every* instance that
perceives the active tab as sidebar-less independently decide
`ToggleAction::Retrofit` for one broadcast press — the "relaxed election," by
design, because a single-actor election deadlocks under `PaneUpdate`
staleness (comment at `main.rs:1053-1056`, `1123-1136`). The only dedup is
`dump_contains_sidebar` (`main.rs:918-925`): a fresh dump is taken, and if it
already shows a sidebar, the rebuild aborts. This is a **check-then-act race
with no lock between the check and the act**: if two (or more) instances each
call `dump_session_layout_for_tab` (server round trip, up to a 1s timeout per
the code's own comment at `main.rs:905-906`) *before either's*
`override_layout` has landed, both dumps show "no sidebar, 1 pane," both
pass the abort check, and both proceed to build and install a layout —
concurrently, against the same tab. Nothing after the dump check re-checks
before the install. The corrupted geometry (a real second terminal process,
not a rendering glitch — each zellij pane is definitionally one PTY) is
consistent with two such overlapping installs each independently retaining
"the" one existing terminal into their own single-slot template, and
zellij's own relayout (documented as advancing to "the first fitting entry,"
`main.rs:947-949`) falling through the docked/undocked/BASE templates —
none of which declare a 2-terminal slot — to a flat, evenly-split
arrangement for the pane that doesn't fit any declared template. This part
(the exact zellij-internal reconciliation of two overlapping
`override_layout` calls) is inferred, not traced in zellij's own source —
out of this pass's scope (the checklist scopes tracing to our two functions)
and no zellij source checkout is available locally to verify further.

**New, concrete evidence pinning down why the race needs many instances:
plugin-id ordering proves a fresh spawn happens on *every* first toggle,
regardless of existing population.** Tab #7's resident (`plugin_88`) and Tab
#8's resident (`plugin_91`) both carry ids strictly higher than every
currently-live zombie (`plugin_73` is the highest pre-existing id, in
"CEO"). zellij assigns plugin ids monotonically at spawn time, so both
residents are **freshly spawned instances, not promoted pre-existing
zombies**. Cross-checked against `~/.config/zellij/config.kdl:59-67`: `Alt /`
binds `MessagePlugin ... { floating true skip_cache true rail "1" }`, zellij's
launch-if-missing-then-message action — confirming
`docs/docking-approach.md`'s "Bootstrap gap" behavior (a floating
launch-if-missing spawn on the active tab) fires **per sidebar-less tab**,
independent of how many config-matched instances already exist elsewhere in
the session. This means every fresh-tab first toggle is *inherently* a race
between (1) the instance the keybind just spawned locally and (2) every
other config-matched instance that also receives the same broadcast `toggle`
pipe message and, per its own (possibly stale) view, also perceives that tab
as active and sidebar-less. With 0 pre-existing instances (every prior
disposable-session repro's starting condition), only actor (1) exists —
clean, deterministic, matches the validated N→N+1 case every time. With 18
pre-existing zombies (`WORK`'s real state), the odds that *at least one*
other instance's dump-and-install window overlaps the freshly-spawned
bootstrap's own window rise sharply with the racer count — a population
effect, not a duration effect, which is why the prior ~600-press/6-instance
synthetic spike (Spike results above) never hit it while `WORK`'s much larger,
organically-accumulated population hit it twice, unprompted, on ordinary use.

**Shared root cause across all three symptoms — assessed, with one
distinction drawn.** The unguarded TOCTOU race in
`install_split_preserving_swaps` is a **single, common mechanism** that
plausibly explains both the terminal-duplication symptom (this pass, via the
`Retrofit` branch on a fresh tab) and the chrome-misplacement symptom (AC-4
in `dock-toggle-restructures-panes`, via the `RegenerateSwaps` branch on a
dirty already-docked tab) — both branches call the identical
`install_split_preserving_swaps` → `split_preserving_layout_kdl` machinery
(`main.rs:848-850` and `867-884`), just from different `decide_toggle` entry
points, so the same race window corrupts whichever shape the racing dumps
happen to disagree about. The floating-instance leak (symptom 1) is more
precisely two sub-mechanisms: **the leak's *creation*** — a new floating
bootstrap spawning on every sidebar-less tab's first toggle regardless of
existing population (confirmed this pass via the id-ordering evidence
above) — is a distinct trigger from the race itself, tied instead to
`MessagePlugin`'s per-tab (not session-wide) "already running" check
(consistent with, though not the same finding as, the Problem section's
already-noted mid-rebuild `ENOENT` correlation). But **the leak's
*persistence*** — why a spawned instance, once it exists, never gets
promoted or self-closes — plausibly *is* downstream of the same race: a
bootstrap instance whose own retrofit attempt loses or corrupts a concurrent
race neither wins a tiled slot (never promoted) nor hits the
existing "redundant tiled rail closes itself" convention (which is keyed to
detecting a *duplicate tiled* rail, `main.rs:150-155` doc citation — not an
*unpromoted floating* one that was never seated at all), so it sits forever
as a floating zombie, feeding the population that makes the *next* tab's
race worse. **Conclusion: one shared mechanism (the unguarded race) plus one
distinct-but-feeding mechanism (per-tab spawn-without-session-wide-dedup) —
not three unrelated bugs, and not fully one bug either.**

**What remains unconfirmed.** No live trace or debug capture of the actual
race timing exists (no `debug "1"` gate was on, and this machine's zellij
server log has since rotated/cleared — `/tmp/zellij-501/zellij-log/`
no longer exists, so the ENOENT-correlation-style retrospective check this
entity ran on 2026-07-08 cannot be repeated for this incident). The exact
zellij-internal reconciliation of two overlapping `override_layout` calls
producing this specific flat-3-way-split shape is inferred from the code's
own documented "advance to first fitting entry" behavior, not observed
directly. This pass did not attempt a new synthetic repro (per its
checklist); AC-1/AC-2/AC-3 below are updated to reflect confirmed live
evidence and a substantially narrowed hypothesis, not an on-demand repro.

## Proposed approach

Run a concurrency-focused spike as the first step, since it's the one
variable finding 2's four failed repro attempts didn't vary:

1. Reproduce finding 1 deliberately in a disposable session first (a known
   trigger, even approximate, for the leak) — candidate: fire `Alt /`
   repeatedly during a `./build.sh` rebuild window (`skip_cache true`
   forces a fresh compile per launch) and confirm stray floating instances
   accumulate the way they did in `WORK`.
2. With multiple live floating instances confirmed present in the same
   tab/session (not a clean single-instance session), reproduce finding 2's
   trigger sequence (dock → close a pane → toggle) and check whether the
   corruption now appears only when 2+ instances are live, confirming or
   refuting the race hypothesis directly rather than by inference.
3. Read `docs/docking-approach.md`'s "Tab ids vs positions" section in full
   — it may already document the exact race window rather than this being
   new territory.
4. Depending on what the spike finds: either (a) close the race at its
   source (idempotent/locked `override_layout`, or a stricter election that
   rules out a zombie acting on a tab it doesn't own), or (b) if the leak
   and the corruption turn out to be unrelated, root-cause each
   independently — but exhaust the shared-cause hypothesis first since it's
   the cheapest single spike that could resolve both findings at once.

**Revised (2026-07-08, this pass).** Step 1 and step 2 were run (see Spike
results above) and did not reproduce either finding, so step 4 cannot be
decided yet. Two candidates for the next spike, in place of further short
synthetic bursts: (a) a long-duration soak — an unattended background
rebuild+toggle loop held open for the better part of an hour across many
tabs, approximating `WORK`'s 3+ hour accumulation window rather than a
~5-minute burst; or (b) turn on the plugin's existing `debug "1"` trace gate
(`docs/docking-approach.md`'s "Trace behind a debug gate" entry, v3.6) in a
real long-lived session and capture the next organic occurrence with full
`zaphod-trace[<id>]:` context, since two independent isolated-repro attempts
(the parent entity's four tries, and this pass's four escalating rounds)
have now both failed to force it synthetically. Either is a bigger
investment than this ideation pass should make; recommend the FO/captain
choose one before further design work here.

**Revised further (2026-07-09, cycle 2).** This pass's finding changes which
of the two candidates above is worth running: duration was never the
missing variable — **racer population** is. A targeted spike that recreates
the specific precondition now confirmed live (many pre-existing
config-matched floating instances, then one clean press on a fresh tab) is
cheaper than either a soak or blind live tracing and directly tests this
pass's hypothesis: seed a disposable session with ~15-20 floating
config-matched instances (e.g. by forcing the launch-if-missing spawn
repeatedly across that many sidebar-less tabs, matching how `WORK`'s
population actually accumulated, not a bulk/synthetic spawn), then fire a
**single** `Alt /` on one additional fresh tab and dump-layout the result.
This is a single clean press, not the "not synthetic load" press-storm this
entity's checklist ruled out for this pass — it targets the one precondition
this pass's evidence newly identifies as load-bearing (population size, not
press rate or duration) rather than repeating the prior spike's approach.
Recommend this as the next step over the soak/tracing pair proposed
2026-07-08, which targeted duration rather than population. If it reproduces
the corruption in a controlled, disposable session, this entity gets its
long-sought on-demand repro and AC-1/AC-2/AC-3 can close on a design; if it
does not, the population-size theory is itself falsified and the
mid-rebuild-timing / other explanations proposed above (the entity's own
prior leads) move back to the front.

## Acceptance criteria

**AC-1 — The floating-instance leak's trigger is reproduced on demand and
either fixed or bounded. OPEN — trigger named and evidenced live, not yet
reproduced on demand in a controlled setting.**
Verified by: a disposable-session repro that reliably produces at least one
stray floating instance (not a hypothesis), plus a fix (preventing the leak)
or a bound (a cleanup sweep, a cap, or a self-terminating timeout for an
unpromoted floating bootstrap) — whichever the spike's root cause supports.
Attempted (2026-07-08): four escalating rounds (candidate rebuild-window
trigger, a faster variant, a full missing-wasm forced failure, and a
2-then-6-instance concurrent race) across ~600 presses — see Spike results.
None left a stray floating pane. **Cycle 2 (2026-07-09):** live re-verification
plus code/config tracing names a concrete trigger — `Alt /`'s `MessagePlugin`
launch-if-missing (`~/.config/zellij/config.kdl:59-67`) spawns a fresh
floating instance on *every* sidebar-less tab's first toggle regardless of
existing population (proven via plugin-id ordering, see Live re-verification
above), and a spawned instance that loses a concurrent retrofit race has no
promotion or self-close path — a plausible persistence mechanism, not yet a
disposable-session repro. Not satisfied yet: needs the population-seeded
spike proposed in Proposed approach (cycle 2) to turn this from a named
mechanism into an on-demand repro before a fix or bound can be designed.

**AC-2 — The dirty-tab chrome-misplacement defect is reproduced on demand.
OPEN — not reproduced this pass; precondition (multiple live floating
instances) was never established synthetically, but a second, structurally
related live corruption (terminal duplication, not chrome relocation) was
independently confirmed twice under the exact precondition this AC needs.**
Verified by: a disposable-session repro using the concurrency setup from
the Proposed approach that triggers the tab-bar relocation into the content
region at will, not just the single live sighting in `WORK`. Attempted
(2026-07-08): since AC-1's floating-instance precondition never held, ran
the closest analog instead — multiple *tiled* residents (2, then 6) racing
to retrofit a fresh sidebar-less tab under concurrent rebuilds. Chrome
landed correctly placed in every tab every round (see Spike results).
**Cycle 2 (2026-07-09):** AC-4's chrome-misplacement path
(`RegenerateSwaps`, dirty tab) and this pass's terminal-duplication finding
(`Retrofit`, fresh tab) share the identical unguarded
`install_split_preserving_swaps` machinery (see Live re-verification,
"Shared root cause" above) — the same race window, different `decide_toggle`
entry points. Not satisfied yet: still needs a repro of *this* AC's specific
shape (chrome relocated, not pane count changed) on demand; the population-
seeded spike proposed above targets the shared precondition and may surface
either shape.

**AC-3 — Chrome placement (and pane count) cannot be corrupted by a
concurrent regenerate/retrofit race, regardless of how many stray instances
exist. Unverified — blocked on AC-1/AC-2, but the gap is now named.**
Verified by: a test or live repro showing that with AC-1's leak trigger
reproduced (multiple live instances) and a legitimate regenerate firing
concurrently, `extract_chrome_panes`/`override_layout`'s output still places
`tab-bar`/`status-bar` in their canonical rows and pane count stays N+1 every
time — not just when the leak happens to be absent. Cannot be exercised
until AC-1 establishes a live leak scenario to regenerate against.
**Cycle 2 (2026-07-09):** the gap is no longer a blank slate among the three
existing dedup layers — code tracing narrows it to a specific one:
`install_split_preserving_swaps`'s `dump_contains_sidebar` check
(`main.rs:918-925`, the "dump-abort seeder dedup" layer) is a
check-then-act race with no lock between the dump and the
`override_layout` call, so it only dedupes a second retrofit whose dump is
taken *after* the first's install lands — not two dumps taken before either
lands. The other two layers (relaxed election, defer-never-blind-absorb)
are not implicated by this pass's trace. A fix candidate (not designed this
pass): serialize `install_split_preserving_swaps` per tab_id, or have the
dump-abort re-check immediately before `override_layout` fires rather than
only at dump time.

## Test plan

Riskiest first, and it's the same step for both findings: the concurrency
repro (AC-1's leak trigger, then AC-2's corruption trigger on top of it).
Nothing else in this entity can be verified until that repro exists — four
single-instance attempts already ruled out the non-concurrent case in the
parent entity's ideation pass, so re-attempting without concurrency would
waste the spike.

**Run this pass (2026-07-08): refuted, did not confirm.** The concurrency
repro was executed exactly as specified above (see Spike results) — four
escalating rounds, up to 6 live instances, ~600 presses — and did not
reproduce either AC-1 or AC-2. This is the pass/fail signal the "riskiest
first" ordering exists to surface: short synthetic bursts are not sufficient
to force this race, so the next test must change the axis that's different
between this spike and `WORK`'s real occurrence — duration and organic
variety, not press count or instance count, which this pass already pushed
hard on. Recommended next test (pick one, do not run both — see Proposed
approach): (a) an unattended soak — same rebuild+toggle harness used here,
run for 30-60+ minutes across many tabs; or (b) `debug "1"` tracing left on
in a real long-lived session, captured on the next natural occurrence. Both
are bigger investments than this ideation pass should make.

**Cycle 2 (2026-07-09): riskiest-first re-targeted — population, not
duration.** CL's live Tab #7 repro plus this pass's independent Tab #8
finding are new evidence that changes the riskiest-first ordering: the axis
that mattered was never duration (finding 1's soak candidate) or blind
tracing (finding 2's candidate) — it was **the number of pre-existing
config-matched instances racing the same broadcast press**, confirmed via
this pass's plugin-id-ordering evidence (a fresh spawn happens on every
first toggle regardless of population, so a larger population means more
*other* racers, not a different trigger mechanism). The riskiest untested
assumption is now specifically: "does seeding ~15-20 floating instances
before one clean press reproduce the corruption on demand?" — see the
population-seeded spike in Proposed approach (cycle 2). This test was not
run this pass (checklist scoped this pass to read-only snapshots and code
tracing, not a new spike); it is the recommended next step.

## Out of scope

The first-toggle pane-count invariant (AC-1/AC-2/AC-3 of
`dock-toggle-restructures-panes`) — already ideated and documented there,
unaffected by this entity's findings. Do not re-litigate that decision here.

## Stage Report: ideation

- DONE: Reproduce the floating-instance leak's trigger on demand in a disposable session (candidate: Alt-/ during a ./build.sh rebuild window)
  Not reproduced. Four escalating rounds in `zellij --session ztest-leak` (candidate trigger, faster variant, forced full wasm removal, 2-then-6-instance concurrent race), ~600 presses total — zero floating panes left behind in any round. See "Spike results" in Problem section.
- DONE: With multiple live floating instances confirmed present, reproduce the dirty-tab chrome-misplacement defect on demand and confirm or refute the concurrent regenerate/retrofit race hypothesis directly
  Precondition (live floating instances) never established, so the exact test couldn't run; ran the closest analog (2-6 tiled residents racing a fresh tab) instead — chrome landed correctly placed every time. Race hypothesis neither confirmed nor refuted outright: a real, reproducible adjacent signature was found instead (`Action KeybindPipe did not complete within 1s timeout`, bursts of up to 34 consecutive `Plugin with id: N not found`, one permission-denied plugin self-exit), but it self-healed in every observed instance rather than corrupting final state.
  Read docs/docking-approach.md's "Tab ids vs positions" section in full and report whether it already documents this race window before treating it as new territory
  Read in full (docs/docking-approach.md:518-544) plus the surrounding "Toggle v3.7-v3.12" hardening section (:567-656). Answer: no — that section documents only the stale tab_id-to-position translation fail-safe, not this pass's KeybindPipe-timeout/orphaned-plugin-id signature or floating-instance accumulation. The section does already document three existing dedup layers (relaxed election, dump-abort dedup, defer-never-blind-absorb) that any eventual fix should extend rather than duplicate. See "Docs check" subsection.

### Summary

Ran the concurrency spike this entity's own Test plan called for (riskiest-first), across four escalating rounds in a disposable session, and it refuted the specific candidate trigger without confirming or fully refuting the shared-root-cause hypothesis: neither the floating-instance leak nor the chrome misplacement reproduced under up to 6 live instances and ~600 presses, but a real, reproducible, adjacent concurrency signature (pipe timeout, orphaned plugin-id bursts, one permission-denied self-exit) surfaced instead, and it self-heals every time rather than leaving visible corruption. AC-1/AC-2/AC-3 are revised to OPEN with the attempt recorded rather than closed out; the entity now recommends a longer-duration soak or live `debug "1"` tracing as the next step, since two independent short-repro attempts (the parent entity's four tries plus this pass's four) have both failed to force the defect synthetically. This is exactly the "riskiest assumption invalidated by direct testing" outcome the ideation stage exists to catch before implementation work is designed around an unconfirmed mechanism.

## Stage Report: ideation (cycle 2)

- DONE: Investigate the live repro just captured in the WORK session -- Tab #7 (fresh tab, single Alt-/) shows sidebar + TWO real terminal panes instead of sidebar + one, alongside 16 zombie floating sidebar instances currently sitting in WORK. Trace whether a zombie racing the legitimate retrofit produced the corrupted split, via read-only dump-layout/list-panes snapshots and code tracing of install_split_preserving_swaps/split_preserving_layout_kdl -- not synthetic load.
  Independently confirmed via fresh `list-panes -a`/`dump-layout` against WORK (read-only): Tab #7 exactly as reported, plus a second, previously-unflagged occurrence in Tab #8 with the byte-for-byte identical corrupted shape. Code trace of both functions found no bug in the transform logic itself (third independent static trace to reach that conclusion, alongside this entity's own 2026-07-08 pass and the parent entity's AC-4 trace). Found the gap instead: `dump_contains_sidebar`'s dedup (`main.rs:918-925`) is a check-then-act race with no lock before `override_layout` fires. New evidence (plugin-id ordering: `plugin_88`/`plugin_91` both higher than every existing zombie id) proves a fresh floating instance spawns on every sidebar-less tab's first toggle regardless of existing population, confirmed against `~/.config/zellij/config.kdl:59-67`'s `MessagePlugin` keybind — meaning every first toggle inherently races the fresh spawn against however many other instances also perceive that tab as active. See "Live re-verification (2026-07-09, ideation cycle 2)" in Problem section.
- DONE: Determine whether this terminal-duplication symptom, the chrome-misplacement symptom (AC-4 in dock-toggle-restructures-panes), and the floating-instance leak share one root cause, or are distinct mechanisms.
  Nuanced answer, not a flat yes/no: the terminal-duplication and chrome-misplacement symptoms share one confirmed mechanism (both flow through the identical `install_split_preserving_swaps`/`split_preserving_layout_kdl` machinery from different `decide_toggle` branches — `Retrofit` vs `RegenerateSwaps` — so the same unguarded race window corrupts whichever shape the racing dumps disagree about). The leak is two sub-mechanisms: its *creation* (a fresh per-tab spawn, tied to `MessagePlugin`'s per-tab rather than session-wide dedup) is a distinct trigger from the race; its *persistence* (why a spawned instance never gets promoted or self-closes) plausibly is downstream of the same race — a losing racer has no promotion or self-close path. See "Shared root cause across all three symptoms" in Problem section.
- DONE: Read-only only -- do NOT send any zellij action/keypress to Tab #7 or close/touch the zombie instances in WORK. Pull your own fresh dump-layout/list-panes rather than trusting secondhand paraphrase, but treat the live session as look-don't-touch.
  No action/keypress sent to WORK; no zombie touched. All evidence gathered via `zellij --session WORK action list-panes -a` and `action dump-layout`, both read-only. Zombie count re-checked independently: 14 Noteplan + 4 CEO = 18 (vs. the dispatch's reported 16), consistent with a few minutes' elapsed churn, not a correction.

### Summary

CL's live Tab #7 repro was independently re-confirmed, and this pass found a second, previously-unflagged occurrence (Tab #8) with the byte-for-byte identical corruption shape — ruling out one-off randomness. A third independent static trace of the transform code (this entity's own second pass, following its own 2026-07-08 trace and the parent entity's AC-4 trace) again found no logic bug, narrowing the gap to a specific, named check-then-act race in `install_split_preserving_swaps`'s dump-abort dedup. New plugin-id-ordering evidence proves the leak's creation mechanism (a fresh spawn on every first toggle, regardless of existing population) and ties it to why population size, not press rate or duration, is the variable that made this reproducible live but not in the prior synthetic spikes. AC-1/AC-2/AC-3 move from "unverified hypothesis" to "confirmed live twice, mechanism substantially narrowed, still not reproduced on demand" — the recommended next step is a population-seeded disposable-session spike (proposed in Proposed approach, not run this pass per the checklist's read-only/no-new-load constraint), replacing the prior duration-focused soak/tracing recommendation.
