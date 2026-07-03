// ABOUTME: Clickable pane-switcher sidebar for zellij — lists panes in its own tab with their
// ABOUTME: last terminal line; Alt-/ flips it between a docked rail and a 1-col sliver.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};
use zellij_tile::prelude::*;

mod agent;

const STATUS_POLL_SECS: f64 = 2.0;
// Sidebar widths in the two swap-layout states (mirrors layouts/zaphod.kdl).
const DOCKED_COLS: usize = 28;
const UNDOCKED_COLS: usize = 1;
// How long after a deferred steer fires that repeat presses for its tab are
// still swallowed: the steered collapse becomes visible a beat after the
// steer itself, and a press inside that gap would instantly undo the toggle
// the user is still watching land.
const TOGGLE_COOLDOWN: Duration = Duration::from_millis(600);

#[derive(Default)]
struct Sidebar {
    rows: Vec<Row>,
    plugin_id: u32,
    rendered_once: bool,
    permissions_requested: bool,
    permissions_granted: bool,
    own_tab: Option<usize>,
    own_url: Option<String>,
    config: BTreeMap<String, String>,
    nav_mode: bool,
    nav_selected: usize,
    return_focus: Option<u32>,
    active_tab: Option<usize>,
    // Per-tab swap-layout state keyed by display position. Caveat: zellij
    // reports (None, false) for tabs with at most one selectable tiled pane,
    // so swap damage there is invisible.
    tab_states: BTreeMap<usize, TabState>,
    own_floating: bool,
    instances: Vec<SidebarInstance>,
    pending_steer: Option<PendingSteer>,
    close_requested: bool,
    // The tab whose deferred steer last fired, and when: repeat presses for
    // it inside TOGGLE_COOLDOWN are swallowed as bounce.
    toggle_cooldown: Option<(usize, Instant)>,
    // A toggle pipe that reached this instance before any manifest named it
    // (the keybind's launch-if-missing pipes the just-launched instance
    // immediately): the press waits for the first PaneUpdate that fills
    // own_url/own_tab and fires exactly once.
    pending_bootstrap_toggle: bool,
}

// The dock state a swap-set override is driving its tab toward, deferred
// until TabUpdate reports the overridden set installed: the server
// dispatches the override on its own thread, so a step fired immediately
// after could still cycle the old swap set. The direction of the step is
// decided at fire time from the entry the override actually landed on.
#[derive(Debug, Clone, Copy, PartialEq)]
struct PendingSteer {
    tab: usize,
    target: DockState,
}

// One tab's swap-layout state from TabUpdate. `id` is the server's stable
// tab id — the key dump_session_layout_for_tab and get_focused_pane_info
// speak — which diverges from the display position once any tab is closed
// or reordered.
#[derive(Debug, Clone, PartialEq, Default)]
struct TabState {
    id: usize,
    swap_name: Option<String>,
    swap_dirty: bool,
    // While a tab's floating panes are visible, swap_name speaks for the
    // FLOATING layer (tab/mod.rs swap_layout_info) — every tab's floating
    // list carries a birth "BASE" — so the tiled layer's state is unreadable.
    floating_visible: bool,
}

// One sidebar plugin pane somewhere in the session, as seen in the manifest.
#[derive(Debug, Clone, Copy, PartialEq)]
struct SidebarInstance {
    pane_id: u32,
    tab: usize,
    floating: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
struct Row {
    pane_id: u32,
    title: String,
    focused: bool,
    agent: agent::AgentFields,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum LineTarget {
    Header,
    Row(usize),
    None,
}

// The two sidebar presentation states a tab's swap set encodes: a 28-col
// rail or a 1-col sliver.
#[derive(Debug, Clone, Copy, PartialEq)]
enum DockState {
    Docked,
    Undocked,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ToggleAction {
    // One deliberate swap step: forwards (next) or backwards (previous)
    // through the installed [BASE, docked, undocked] order.
    SteerSwap { backwards: bool },
    // Rebuild the tab's swap set around its current pane arrangement, then
    // steer to the target state.
    RegenerateSwaps { target: DockState },
    Retrofit,
    Ignore,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ClickAction {
    ToggleDock,
    FocusPane(u32),
    None,
}

register_plugin!(Sidebar);

impl ZellijPlugin for Sidebar {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.plugin_id = get_plugin_ids().plugin_id;
        self.config = configuration;
        subscribe(&[
            EventType::PaneUpdate,
            EventType::TabUpdate,
            EventType::Mouse,
            EventType::Key,
            EventType::Timer,
            EventType::PermissionRequestResult,
        ]);
        // Permissions are requested on first render, not here: a request made
        // during tab construction races pane registration and gets parked in
        // the server's waiting-for-client queue, so the prompt never shows.
        set_timeout(STATUS_POLL_SECS);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PermissionRequestResult(status) => {
                self.permissions_granted = status == PermissionStatus::Granted;
                // Never take focus: clicks are delivered to the plugin without
                // focusing it (same mechanism as the built-in tab-bar). Only
                // after the grant — the permission prompt needs a focusable pane.
                if status == PermissionStatus::Granted && !self.nav_mode {
                    set_selectable(false);
                }
                true
            }
            Event::Key(key) if self.nav_mode => {
                if key.has_no_modifiers() {
                    match key.bare_key {
                        BareKey::Up | BareKey::Char('k') => {
                            self.nav_selected =
                                move_selection(self.nav_selected, -1, self.rows.len());
                        }
                        BareKey::Down | BareKey::Char('j') => {
                            self.nav_selected =
                                move_selection(self.nav_selected, 1, self.rows.len());
                        }
                        BareKey::Enter => {
                            let target = self.rows.get(self.nav_selected).map(|r| r.pane_id);
                            self.exit_nav(false);
                            if let Some(id) = target {
                                focus_terminal_pane(id, false, false);
                            }
                        }
                        BareKey::Esc => {
                            self.exit_nav(true);
                        }
                        _ => {}
                    }
                }
                true
            }
            Event::TabUpdate(tabs) => {
                self.active_tab = tabs.iter().find(|t| t.active).map(|t| t.position);
                self.tab_states = tabs
                    .iter()
                    .map(|t| {
                        (
                            t.position,
                            TabState {
                                id: t.tab_id,
                                swap_name: t.active_swap_layout_name.clone(),
                                swap_dirty: t.is_swap_layout_dirty,
                                floating_visible: t.are_floating_panes_visible,
                            },
                        )
                    })
                    .collect();
                if let Some(pending) = self.pending_steer {
                    match pending_steer_disposition(pending, self.active_tab, &self.tab_states) {
                        SteerDisposition::Fire { backwards } => {
                            self.pending_steer = None;
                            self.toggle_cooldown = Some((pending.tab, Instant::now()));
                            steer_swap(backwards);
                        }
                        SteerDisposition::Drop => self.pending_steer = None,
                        SteerDisposition::Keep => {}
                    }
                }
                false
            }
            Event::PaneUpdate(manifest) => {
                self.own_tab = own_tab_position(&manifest, self.plugin_id);
                self.instances = sidebar_instances(&manifest);
                let old = std::mem::take(&mut self.rows);
                self.rows = rows_for_own_tab(&manifest, self.plugin_id);
                preserve_agent_fields(&mut self.rows, &old);
                if let Some(own) = manifest
                    .panes
                    .values()
                    .flatten()
                    .find(|p| p.is_plugin && p.id == self.plugin_id)
                {
                    self.own_url = own.plugin_url.clone();
                    self.own_floating = own.is_floating;
                    // If focus ever lands on us outside nav mode (launch,
                    // layout focus), hand it back — but only when the tab has
                    // another selectable pane to receive it: bouncing in the
                    // session-birth window panics the whole server.
                    if should_hand_back_focus(own.is_focused, self.nav_mode, !self.rows.is_empty())
                    {
                        focus_previous_pane();
                    }
                }
                // A retrofit that installed a tiled rail leaves this
                // floating bootstrap instance behind as an invisible
                // pipe-eating zombie; it closes itself (fire-and-forget,
                // no permission gate on CloseSelf). close_self does not
                // remove the pane from the very next manifest snapshot, so
                // the flag makes the call one-shot rather than re-firing on
                // every PaneUpdate until the manifest catches up.
                if should_close_self(self.close_requested, self.own_floating, self.own_tab, &self.instances) {
                    self.close_requested = true;
                    close_self();
                    return false;
                }
                if should_fire_bootstrap_toggle(
                    self.pending_bootstrap_toggle,
                    &self.own_url,
                    self.own_tab,
                    self.active_tab,
                ) {
                    // Consumed exactly once; perform_toggle never re-arms it.
                    self.pending_bootstrap_toggle = false;
                    self.perform_toggle();
                }
                // PaneUpdate fires constantly in agent-heavy tabs; re-rendering
                // a pinned overlay on every one makes the underlying panes
                // flicker. Only render when the derived view changed.
                self.rows != old || !self.rendered_once
            }
            Event::Timer(_) => {
                let changed = self.refresh_statuses();
                set_timeout(STATUS_POLL_SECS);
                changed
            }
            Event::Mouse(Mouse::LeftClick(line, _col)) => {
                self.handle_click(line);
                false
            }
            _ => false,
        }
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        // CLI pipe callers terminate via the server's auto-unblock once this
        // returns; an explicit unblock would need the ReadCliPipes grant.
        if pipe_message.name == "navigate" {
            // Nav belongs to the active tab's resident instance; every tab's
            // layout carries one.
            let active_tab = self.current_active_tab();
            if self.own_tab.is_some() && self.own_tab == active_tab {
                self.enter_nav();
                return true;
            }
            return false;
        }
        if pipe_message.name != "toggle" {
            return false;
        }
        // The keybind's launch-if-missing races its own pipe: the toggle
        // that launched this instance can arrive before the first PaneUpdate
        // names it, when every decision input is still unknown. Park the
        // press; the manifest fires it.
        if !own_pane_known(&self.own_url, self.own_tab) {
            self.pending_bootstrap_toggle = true;
            return false;
        }
        self.perform_toggle();
        false
    }

    fn render(&mut self, _rows: usize, cols: usize) {
        self.rendered_once = true;
        if !self.permissions_requested {
            self.permissions_requested = true;
            request_permission(&[
                PermissionType::ReadApplicationState,
                PermissionType::ChangeApplicationState,
                PermissionType::ReadPaneContents,
            ]);
        }
        // Header: any click runs the same dock toggle as Alt-/ (the ⇄ marks
        // it).
        println!(
            "\u{1b}[7m▾ PANES{}⇄ \u{1b}[0m",
            " ".repeat(cols.saturating_sub(10))
        );
        for (idx, row) in self.rows.iter().enumerate() {
            let title: String = row.title.chars().take(cols.saturating_sub(2)).collect();
            let mark = row_marker(row);
            if self.nav_mode && idx == self.nav_selected {
                println!("{}\u{1b}[7m{}\u{1b}[0m", mark, title); // nav selection
            } else {
                println!("{}{}{}\u{1b}[0m", mark, title_style(row), title);
            }
            let status = agent::status_line(&row.agent, cols.saturating_sub(4));
            println!("    \u{1b}[2m{}\u{1b}[0m", status);
        }
    }
}

impl Sidebar {
    fn current_active_tab(&mut self) -> Option<usize> {
        // get_focused_pane_info blocks on a response the host only writes
        // once ReadApplicationState is granted; pre-grant it panics inside
        // the shim, so fall back to the cached TabUpdate value.
        if !self.permissions_granted {
            return self.active_tab;
        }
        let active_tab =
            active_tab_for_decision(self.active_tab, get_focused_pane_info(), &self.tab_states);
        self.active_tab = active_tab;
        active_tab
    }

    fn enter_nav(&mut self) {
        self.nav_mode = true;
        self.return_focus = self.rows.iter().find(|r| r.focused).map(|r| r.pane_id);
        self.nav_selected = self
            .rows
            .iter()
            .position(|r| r.focused)
            .unwrap_or(0)
            .min(self.rows.len().saturating_sub(1));
        set_selectable(true);
        focus_plugin_pane(self.plugin_id, false, false);
    }

    fn exit_nav(&mut self, restore_focus: bool) {
        self.nav_mode = false;
        set_selectable(false);
        if restore_focus {
            if let Some(id) = self.return_focus.take() {
                focus_terminal_pane(id, false, false);
            }
        }
    }

    fn handle_click(&mut self, line: isize) {
        match decide_click(line, &self.rows) {
            ClickAction::ToggleDock => self.perform_toggle(),
            ClickAction::FocusPane(id) => focus_terminal_pane(id, false, false),
            ClickAction::None => {}
        }
    }

    fn perform_toggle(&mut self) {
        let active_tab = self.current_active_tab();
        if should_swallow_toggle(
            active_tab,
            self.pending_steer,
            self.toggle_cooldown,
            Instant::now(),
        ) {
            return;
        }
        // A press that is not the parked steer's own bounce supersedes it.
        self.pending_steer = None;
        let active_state = active_tab.and_then(|tab| self.tab_states.get(&tab));
        let active_swap_name = active_state.and_then(|state| state.swap_name.clone());
        let active_swap_dirty = active_state.is_some_and(|state| state.swap_dirty);
        match decide_toggle(
            self.own_tab,
            self.own_floating,
            active_tab,
            active_swap_name.as_deref(),
            active_swap_dirty,
            self.plugin_id,
            &self.instances,
        ) {
            ToggleAction::SteerSwap { backwards } => {
                // Flip docked <-> sliver by rearranging the tab's existing
                // panes; the plugin pane itself never hides or moves.
                steer_swap(backwards);
            }
            ToggleAction::RegenerateSwaps { target } => self.regenerate_swaps(target),
            ToggleAction::Retrofit => self.retrofit(active_tab),
            ToggleAction::Ignore => {}
        }
    }

    // Rebuild the tab's swap set around its current pane arrangement so the
    // toggle preserves the user's manual splits, then steer to the target
    // state. Any missing input degrades to plain cycling: on a damaged tab
    // zellij's first call re-applies the current template and the second
    // advances — the arrangement snap-folds, but the toggle still lands.
    fn regenerate_swaps(&mut self, target: DockState) {
        let rebuilt = rebuild_target(self.permissions_granted, self.own_tab, &self.tab_states)
            .and_then(|(tab, tab_id)| self.install_split_preserving_swaps(tab, tab_id, target));
        if rebuilt.is_none() {
            fallback_swap_cycle();
        }
    }

    // Docks the sidebar into the active tab, which has none: rebuild the
    // tab's swap set around its dumped arrangement — the override's base
    // spawns the rail — and steer to docked once the override reports in,
    // so the tab arrives with its splits intact. When the rebuild cannot
    // run, the absorb override docks the rail alone: its base stacks the
    // tab's panes, which is already the docked geometry, so no steer is
    // needed. Every later toggle on the tab is a pure swap cycle.
    fn retrofit(&mut self, active_tab: Option<usize>) {
        let installed = rebuild_target(self.permissions_granted, active_tab, &self.tab_states)
            .and_then(|(tab, tab_id)| {
                self.install_split_preserving_swaps(tab, tab_id, DockState::Docked)
            });
        if installed.is_none() {
            self.absorb_retrofit(active_tab);
        }
    }

    fn absorb_retrofit(&mut self, active_tab: Option<usize>) {
        if let Some(url) = self.own_url.clone() {
            override_layout(
                LayoutInfo::Stringified(retrofit_layout_kdl(&url, &self.config)),
                true, // retain existing terminal panes
                true, // retain existing plugin panes
                true, // apply only to the active tab
                BTreeMap::new(),
            );
            // This override records no PendingSteer (its base is already the
            // docked geometry), so it arms the repeat-press cooldown itself:
            // its visible docking lags the issue like the JIT pipeline's
            // steer does.
            if let Some(tab) = active_tab {
                self.toggle_cooldown = Some((tab, Instant::now()));
            }
        }
    }

    // The shared dump → transform → override machinery behind regenerate
    // and retrofit: rebuild the tab's swap set around its dumped
    // arrangement and record the steer that completes the toggle once
    // TabUpdate reports the new set installed. None means nothing was
    // overridden and the caller runs its own degraded path.
    fn install_split_preserving_swaps(
        &mut self,
        tab: usize,
        tab_id: usize,
        target: DockState,
    ) -> Option<()> {
        let url = self.own_url.clone()?;
        // In-band errors and a 1s server-side timeout; a stale tab id dumps
        // no tab node and fails the rebuild below.
        let (dump, _metadata) = dump_session_layout_for_tab(tab_id).ok()?;
        let layout = split_preserving_layout_kdl(&dump, &url, &self.config).ok()?;
        override_layout(
            LayoutInfo::Stringified(layout),
            true, // retain existing terminal panes
            true, // retain existing plugin panes
            true, // apply only to the active tab
            BTreeMap::new(),
        );
        // Where the override lands is zellij's call, not ours: the server
        // relayouts the tab right after installing the set, advancing it
        // to the first fitting entry — BASE only when the absorb base's
        // exact pane count matches (single-shell tabs), "docked"
        // otherwise. The recorded target lets the TabUpdate handler finish
        // the toggle from whichever entry is reported.
        self.pending_steer = Some(PendingSteer { tab, target });
        Some(())
    }

    // Returns whether any row's agent fields changed (i.e. a render is due).
    fn refresh_statuses(&mut self) -> bool {
        let mut changed = false;
        for row in self.rows.iter_mut() {
            let pane_id = PaneId::Terminal(row.pane_id);
            let command = get_pane_running_command(pane_id);
            let viewport = get_pane_scrollback(pane_id, false).map(|contents| contents.viewport);
            let enriched = agent::enrich_fields(&row.agent, &row.title, command, viewport);
            if enriched != row.agent {
                row.agent = enriched;
                changed = true;
            }
        }
        changed
    }
}

// The pipe broadcasts to every config-matched instance; exactly one may act.
// - The active tab's tiled resident steers the tab's swap layout one step
//   toward the other dock state, by name — never by blind cycling, whose
//   position semantics around BASE and the list end are unreliable.
// - When the tab is damaged (manual split/resize), a swap step would
//   snap-fold the user's arrangement into a stale template; the resident
//   instead rebuilds the swap set around the current arrangement.
// - A floating resident (keybind bootstrap) has no swap set to cycle: it
//   retrofits its own tab, docking itself and installing the swap set.
// - A tab with no sidebar gets a one-time override_layout retrofit; the
//   lowest-pane-id instance is the single actor so the override runs once.
fn decide_toggle(
    own_tab: Option<usize>,
    own_floating: bool,
    active_tab: Option<usize>,
    active_swap_name: Option<&str>,
    active_swap_dirty: bool,
    own_pane_id: u32,
    instances: &[SidebarInstance],
) -> ToggleAction {
    let Some(active) = active_tab else {
        return ToggleAction::Ignore;
    };
    if own_tab == Some(active) && !own_floating {
        // Only "docked" (position 1 of [BASE, docked, undocked]) steers
        // forward, to undocked. Everything else steers back: undocked one
        // step to docked, and BASE — geometrically docked on template-born
        // tabs, so a forward step there is a dead press — wraps from
        // position 0 to the list end (undocked, a visible collapse);
        // zellij's previous-swap wrap is deterministic while next past the
        // end is not (swap_layouts.rs progress_layout!).
        if active_swap_dirty {
            ToggleAction::RegenerateSwaps {
                target: if active_swap_name == Some("undocked") {
                    DockState::Docked
                } else {
                    DockState::Undocked
                },
            }
        } else {
            ToggleAction::SteerSwap {
                backwards: active_swap_name != Some("docked"),
            }
        }
    } else if instances.iter().any(|i| i.tab == active && !i.floating) {
        // The tab's tiled sidebar owns the toggle; a floating instance there
        // (e.g. left behind by a retrofit) must not fire another retrofit.
        ToggleAction::Ignore
    } else if own_tab == Some(active) {
        // Own pane is floating here; the lowest-id floating resident is the
        // single retrofit actor.
        if instances
            .iter()
            .filter(|i| i.tab == active)
            .all(|i| i.pane_id >= own_pane_id)
        {
            ToggleAction::Retrofit
        } else {
            ToggleAction::Ignore
        }
    } else if instances.iter().any(|i| i.tab == active) {
        ToggleAction::Ignore
    } else if instances.iter().all(|i| i.pane_id >= own_pane_id) {
        // The manifest shows no instance in the active tab at all, but
        // PaneUpdate can lag TabUpdate — a swap name already ours means the
        // tab has, or very recently had, an installed rail (an earlier
        // retrofit, or an instance that crashed/closed there). Retrofit is
        // a full-tab override; firing it on a tab that already carries our
        // swap set is the corruption seeder, so the election steers the
        // existing set instead.
        match active_swap_name {
            Some("docked") | Some("undocked") => ToggleAction::SteerSwap {
                backwards: active_swap_name != Some("docked"),
            },
            _ => ToggleAction::Retrofit,
        }
    } else {
        ToggleAction::Ignore
    }
}

// Whether a split-preserving swap rebuild can run against a tab, and the
// server tab id to dump from if so. The dump host call blocks on a
// response that only exists post-grant (it panics in the shim pre-grant),
// and it speaks the server's stable tab id, which only the
// TabUpdate-derived states can translate a display position into. None:
// the rebuild cannot run and the caller degrades.
fn rebuild_target(
    permissions_granted: bool,
    tab: Option<usize>,
    tab_states: &BTreeMap<usize, TabState>,
) -> Option<(usize, usize)> {
    if !permissions_granted {
        return None;
    }
    let tab = tab?;
    let id = tab_states.get(&tab)?.id;
    Some((tab, id))
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SteerDisposition {
    // One swap step in the given direction through the installed
    // [BASE, docked, undocked] order; previous wraps position 0 to the
    // list end deterministically, only forward past the end is unreliable
    // (swap_layouts.rs progress_layout!).
    Fire { backwards: bool },
    Keep,
    Drop,
}

// A steered press recorded alongside a swap-set override waits for the
// server to report where the override landed. zellij relayouts the tab
// right after installing the set, so the report is "BASE" only when the
// absorb base's exact pane count matches (single-shell tabs); multi-pane
// tabs arrive already at "docked". The steer converts the reported entry
// into the one deliberate step that reaches the target — or stands down
// when the relayout already landed there, since one more step would
// overshoot it (the live one-step-off arrival). While floating panes are
// visible the reported name speaks for the floating layer, whose birth
// entry is also "BASE"; the tiled outcome is unreadable, so the press
// stays armed. The dirty flag is no part of the signature — a landed
// override was observed live still flagged dirty. Swap presses act on the
// client's active tab, so the press is abandoned when the tab is gone or
// no longer active — the regenerated set stays installed and a later
// press on the tab steers by name. A foreign name means the override
// failed or raced, and the press is abandoned rather than kept armed
// forever; only a nameless report (the one-selectable-pane blind spot)
// keeps it waiting.
fn pending_steer_disposition(
    steer: PendingSteer,
    active_tab: Option<usize>,
    tab_states: &BTreeMap<usize, TabState>,
) -> SteerDisposition {
    let Some(state) = tab_states.get(&steer.tab) else {
        return SteerDisposition::Drop;
    };
    if active_tab != Some(steer.tab) {
        return SteerDisposition::Drop;
    }
    if state.floating_visible {
        return SteerDisposition::Keep;
    }
    match (state.swap_name.as_deref(), steer.target) {
        (Some("BASE"), DockState::Docked) => SteerDisposition::Fire { backwards: false },
        (Some("BASE"), DockState::Undocked) => SteerDisposition::Fire { backwards: true },
        (Some("docked"), DockState::Undocked) => SteerDisposition::Fire { backwards: false },
        (Some("undocked"), DockState::Docked) => SteerDisposition::Fire { backwards: true },
        (Some(_), _) => SteerDisposition::Drop,
        (None, _) => SteerDisposition::Keep,
    }
}

// Whether any manifest has named this instance yet: pre-manifest, own_url
// and own_tab are unknown and no toggle decision can be made.
fn own_pane_known(own_url: &Option<String>, own_tab: Option<usize>) -> bool {
    own_url.is_some() && own_tab.is_some()
}

// A parked bootstrap press fires on the first manifest that names this
// instance — but only once the active tab is also known, since
// decide_toggle ignores a press without one and the parked press would die
// the same way the piped one did. The caller consumes the press on fire.
fn should_fire_bootstrap_toggle(
    parked: bool,
    own_url: &Option<String>,
    own_tab: Option<usize>,
    active_tab: Option<usize>,
) -> bool {
    parked && own_pane_known(own_url, own_tab) && active_tab.is_some()
}

// A toggle press bounces when it repeats into the same tab's JIT window:
// the pipeline (dump → override → deferred steer) makes its collapse
// visible only a beat after the press, so a quick second press reads the
// tab as needing another toggle and undoes the one still landing. Swallow
// a press for the tab whose steer is still parked, and for the tab whose
// steer fired less than TOGGLE_COOLDOWN ago; a press for any other tab is
// a fresh intent.
fn should_swallow_toggle(
    active_tab: Option<usize>,
    pending_steer: Option<PendingSteer>,
    cooldown: Option<(usize, Instant)>,
    now: Instant,
) -> bool {
    let Some(tab) = active_tab else {
        return false;
    };
    pending_steer.is_some_and(|steer| steer.tab == tab)
        || cooldown.is_some_and(|(cooled, fired)| {
            cooled == tab && now.duration_since(fired) < TOGGLE_COOLDOWN
        })
}

fn steer_swap(backwards: bool) {
    if backwards {
        previous_swap_layout();
    } else {
        next_swap_layout();
    }
}

// Dirty-tab cycling without a regenerated swap set: zellij's first call
// only re-applies the current template (consuming the damage), the second
// advances.
fn fallback_swap_cycle() {
    next_swap_layout();
    next_swap_layout();
}

// Any click on the header — the ⇄ control or the title text — runs the same
// dock toggle as Alt-/; row clicks focus their pane.
fn decide_click(line: isize, rows: &[Row]) -> ClickAction {
    match target_for_line(line, rows.len()) {
        LineTarget::Header => ClickAction::ToggleDock,
        LineTarget::Row(idx) => rows
            .get(idx)
            .map(|row| ClickAction::FocusPane(row.pane_id))
            .unwrap_or(ClickAction::None),
        LineTarget::None => ClickAction::None,
    }
}

// A floating bootstrap instance is superseded once its own tab holds a
// tiled sidebar (a retrofit installed the rail without seating the
// floater): it would linger as an invisible config-matched zombie that
// keeps receiving pipes. Both facts come from the same manifest snapshot,
// so a transient mid-retrofit state cannot half-match.
fn is_stray_floating_bootstrap(
    own_floating: bool,
    own_tab: Option<usize>,
    instances: &[SidebarInstance],
) -> bool {
    let Some(tab) = own_tab else {
        return false;
    };
    own_floating && instances.iter().any(|i| i.tab == tab && !i.floating)
}

// Gates close_self to one call per instance lifetime: the manifest can still
// show the stray-floater shape on the PaneUpdate right after the call, since
// the host has not yet dropped the pane, and a second close_self would fire
// on every such snapshot until it does.
fn should_close_self(
    already_requested: bool,
    own_floating: bool,
    own_tab: Option<usize>,
    instances: &[SidebarInstance],
) -> bool {
    !already_requested && is_stray_floating_bootstrap(own_floating, own_tab, instances)
}

// Handing focus back with nowhere to send it panics the server in the
// session-birth window (get_active_pane_id unwraps None); only bounce when
// the manifest shows another selectable pane in our tab.
fn should_hand_back_focus(own_focused: bool, nav_mode: bool, has_focus_target: bool) -> bool {
    own_focused && !nav_mode && has_focus_target
}

// The absorb override for a tab without a sidebar, used when the
// split-preserving rebuild cannot run: a full replacement layout that docks
// the sidebar and installs the docked/undocked swap set, stacking the tab's
// panes. It must carry the tab-bar/status-bar chrome and a stacked main
// that absorbs the existing panes — override_layout replaces the whole tab,
// and anything the KDL omits is dropped. The tab node stays unnamed so the
// user's tab name survives the override.
fn retrofit_layout_kdl(plugin_url: &str, config: &BTreeMap<String, String>) -> String {
    let tab_body = |sidebar_cols: usize| {
        format!(
            r#"  tab {{
    pane size=1 borderless=true {{
        plugin location="zellij:tab-bar"
    }}
    pane split_direction="vertical" {{
{rail}        pane stacked=true {{
            children
        }}
    }}
    pane size=1 borderless=true {{
        plugin location="zellij:status-bar"
    }}
  }}
"#,
            rail = rail_pane_kdl(plugin_url, config, sidebar_cols)
        )
    };
    format!(
        "layout {{\n swap_tiled_layout name=\"docked\" {{\n{docked} }}\n swap_tiled_layout name=\"undocked\" {{\n{undocked} }}\n{base}}}\n",
        docked = tab_body(DOCKED_COLS),
        undocked = tab_body(UNDOCKED_COLS),
        base = tab_body(DOCKED_COLS),
    )
}

// The sidebar's own slot in a generated layout. The plugin block repeats
// the live instance's URL and configuration exactly, so layout application
// re-seats the existing pane instead of spawning a second instance.
fn rail_pane_kdl(plugin_url: &str, config: &BTreeMap<String, String>, cols: usize) -> String {
    let config_lines: String = config
        .iter()
        .map(|(key, value)| format!("                {key} \"{value}\"\n"))
        .collect();
    format!(
        r#"        pane size={cols} borderless=true name="sidebar" {{
            plugin location="{plugin_url}" {{
{config_lines}            }}
        }}
"#
    )
}

// Builds the layout regenerate_swaps overrides a damaged tab with: the base
// absorbs every pane into a rail + stacked children tab (the only
// retained-pane-correct shape for an override), while the docked/undocked
// swaps carry the tab's dumped arrangement so the follow-up steer re-seats
// the user's splits with only the rail width changed. The dump arrives with
// the requesting plugin's own rail already removed by the server. Chrome
// panes are extracted from wherever the dump seats them — zellij's absorb
// can bake the tab-bar into the user region, and a baked copy would ride
// into the swaps as a user pane and mangle the tab permanently — and
// re-emitted as canonical rows, exactly one node per chrome pane the tab
// actually has: a node without a matching pane wedges the tab (swaps never
// spawn), a pane without a node gets absorbed again.
fn split_preserving_layout_kdl(
    dump: &str,
    plugin_url: &str,
    config: &BTreeMap<String, String>,
) -> Result<String, String> {
    let body = extract_tab_body(dump)?;
    let mut chrome = Vec::new();
    let region: Vec<Vec<String>> = top_level_blocks(&body)
        .into_iter()
        .filter(|block| !is_floating_panes_block(block))
        .map(|block| {
            block
                .iter()
                .map(|line| {
                    remove_prop(
                        &remove_prop(&remove_prop(line, "focus"), "name"),
                        "borderless",
                    )
                })
                .collect()
        })
        .filter_map(|block| extract_chrome_panes(block, plugin_url, &mut chrome))
        .collect();
    if region.is_empty() {
        return Err("dumped tab has no panes besides chrome".to_owned());
    }

    let region_kdl = if region.len() == 1 {
        // A single container fills whatever remains next to the rail; its
        // dumped size described the tab before the rail was removed.
        let mut lines = region[0].clone();
        lines[0] = remove_prop(&lines[0], "size");
        lines.join("\n") + "\n"
    } else {
        // Sibling rows keep their relative sizes inside one flexible
        // container (default split direction: top-to-bottom, as in the tab).
        format!(
            "pane {{\n{}\n}}\n",
            region.iter().flatten().cloned().collect::<Vec<_>>().join("\n")
        )
    };
    let chrome_row = |location: &String| {
        format!("pane size=1 borderless=true {{\nplugin location=\"{location}\"\n}}\n")
    };
    let is_top_bar = |location: &&String| location.as_str() == "zellij:tab-bar";
    let chrome_top: String = chrome.iter().filter(is_top_bar).map(chrome_row).collect();
    let chrome_bottom: String = chrome
        .iter()
        .filter(|location| !is_top_bar(location))
        .map(chrome_row)
        .collect();
    let tab_kdl = |cols: usize, main: &str| {
        format!(
            "tab {{\n{chrome_top}pane split_direction=\"vertical\" {{\n{rail}{main}}}\n{chrome_bottom}}}\n",
            rail = rail_pane_kdl(plugin_url, config, cols),
        )
    };
    Ok(format!(
        "layout {{\nswap_tiled_layout name=\"docked\" {{\n{docked}}}\nswap_tiled_layout name=\"undocked\" {{\n{undocked}}}\n{base}}}\n",
        docked = tab_kdl(DOCKED_COLS, &region_kdl),
        undocked = tab_kdl(UNDOCKED_COLS, &region_kdl),
        base = tab_kdl(DOCKED_COLS, "pane stacked=true {\nchildren\n}\n"),
    ))
}

// Lines between the first depth-1 `tab` node's braces in a dumped layout.
// The dump is machine-generated multi-line KDL, so tracking brace depth per
// line (ignoring braces inside quoted strings) is structure-exact.
fn extract_tab_body(dump: &str) -> Result<Vec<String>, String> {
    let mut depth = 0isize;
    let mut capturing = false;
    let mut body = Vec::new();
    for line in dump.lines() {
        let trimmed = line.trim();
        let delta = brace_delta(line);
        if capturing {
            if depth + delta <= 1 {
                return Ok(body);
            }
            body.push(line.to_owned());
        } else if depth == 1 && (trimmed == "tab" || trimmed.starts_with("tab ")) {
            if delta != 1 {
                return Err("unsupported tab node shape in dump".to_owned());
            }
            capturing = true;
        }
        depth += delta;
    }
    Err("no tab node in dump".to_owned())
}

// Splits a node body into its immediate child nodes, each a run of lines.
fn top_level_blocks(body: &[String]) -> Vec<Vec<String>> {
    let mut blocks = Vec::new();
    let mut current = Vec::new();
    let mut depth = 0isize;
    for line in body {
        current.push(line.clone());
        depth += brace_delta(line);
        if depth <= 0 {
            blocks.push(std::mem::take(&mut current));
            depth = 0;
        }
    }
    if !current.is_empty() {
        blocks.push(current);
    }
    blocks
}

// Removes every built-in `zellij:*` plugin pane and every copy of the
// sidebar's own rail from a dumped pane block, wherever zellij's absorb
// seated them, recording each removed chrome location (rail copies are
// dropped without recording: rail_pane_kdl always contributes the
// canonical slot, so a dumped copy is never re-emitted). A container
// emptied by the removal is dropped with it; a container left with a
// single child is replaced by that child, which takes over the
// container's slot. A pane wrapping a third-party plugin is left intact.
fn extract_chrome_panes(
    block: Vec<String>,
    own_plugin_url: &str,
    removed: &mut Vec<String>,
) -> Option<Vec<String>> {
    if block.len() < 2 {
        return Some(block);
    }
    let children = top_level_blocks(&block[1..block.len() - 1]);
    if let Some(location) = children.iter().find_map(|child| plugin_location(child)) {
        return match classify_plugin_location(&location, own_plugin_url) {
            PluginRole::Chrome => {
                removed.push(location);
                None
            }
            PluginRole::Rail => None,
            PluginRole::ThirdParty => Some(block),
        };
    }
    if children.is_empty() {
        return Some(block);
    }
    let child_count = children.len();
    let kept: Vec<Vec<String>> = children
        .into_iter()
        .filter_map(|child| extract_chrome_panes(child, own_plugin_url, removed))
        .collect();
    if kept.is_empty() {
        return None;
    }
    if kept.len() == 1 && child_count > 1 {
        let mut child = kept.into_iter().next().unwrap();
        child[0] = fill_container_slot(&child[0], &block[0]);
        return Some(child);
    }
    let mut rebuilt = vec![block[0].clone()];
    rebuilt.extend(kept.into_iter().flatten());
    rebuilt.push(block[block.len() - 1].clone());
    Some(rebuilt)
}

// The location of a plugin node, when the block is one directly (its first
// line is a bare `plugin location="..."` node, not a pane wrapping one).
fn plugin_location(block: &[String]) -> Option<String> {
    let first = block.first()?;
    let trimmed = first.trim_start();
    if trimmed != "plugin" && !trimmed.starts_with("plugin ") {
        return None;
    }
    let token = prop_token(first, "location")?;
    let location = token.strip_prefix("location=\"")?.strip_suffix('"')?;
    Some(location.to_owned())
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PluginRole {
    // A zellij built-in (tab-bar/status-bar): extracted and re-emitted as a
    // canonical chrome row.
    Chrome,
    // A copy of the sidebar's own rail, wherever it landed in the region:
    // dropped outright, since rail_pane_kdl always supplies the canonical
    // slot.
    Rail,
    // Some other plugin the user placed: not ours to remove.
    ThirdParty,
}

// A defensive substring match backs up the exact-URL check: a rail built
// from a different path or an older/newer configuration (same plugin,
// different identity string) must still be recognized.
fn classify_plugin_location(location: &str, own_plugin_url: &str) -> PluginRole {
    if location.starts_with("zellij:") {
        PluginRole::Chrome
    } else if location == own_plugin_url || location.contains("zellij-sidebar") {
        PluginRole::Rail
    } else {
        PluginRole::ThirdParty
    }
}

// Rewrites a hoisted child's header to occupy its old container's slot:
// the container's size replaces the child's own, and stack-only flags die
// with the stack.
fn fill_container_slot(child_header: &str, container_header: &str) -> String {
    let stripped = remove_prop(&remove_prop(child_header, "size"), "expanded");
    let Some(size) = prop_token(container_header, "size") else {
        return stripped;
    };
    let trimmed = stripped.trim_end();
    match trimmed.strip_suffix('{') {
        Some(head) => format!("{} {size} {{", head.trim_end()),
        None => format!("{trimmed} {size}"),
    }
}

fn is_floating_panes_block(block: &[String]) -> bool {
    block
        .first()
        .is_some_and(|l| l.trim().split_whitespace().next() == Some("floating_panes"))
}

// Net brace-depth change of one KDL line; braces inside quoted strings
// (pane titles, cwds) do not count.
fn brace_delta(line: &str) -> isize {
    let mut delta = 0;
    let mut in_string = false;
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' if in_string => {
                chars.next();
            }
            '"' => in_string = !in_string,
            '{' if !in_string => delta += 1,
            '}' if !in_string => delta -= 1,
            _ => {}
        }
    }
    delta
}

// Char span of the first `key=value` property in a KDL line — quoted or
// bare values — matching only outside quoted strings, so a cwd or title
// that happens to contain `key=` is left alone.
fn prop_span(chars: &[char], key: &str) -> Option<(usize, usize)> {
    let pattern: Vec<char> = format!("{key}=").chars().collect();
    let mut in_string = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if in_string {
            if c == '\\' {
                i += 2;
                continue;
            }
            if c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        let at_token_start = i > 0 && chars[i - 1].is_whitespace();
        if at_token_start && chars[i..].starts_with(&pattern[..]) {
            let mut j = i + pattern.len();
            if chars.get(j) == Some(&'"') {
                j += 1;
                while j < chars.len() {
                    match chars[j] {
                        '\\' => j += 2,
                        '"' => {
                            j += 1;
                            break;
                        }
                        _ => j += 1,
                    }
                }
            } else {
                while j < chars.len() && !chars[j].is_whitespace() && chars[j] != '{' && chars[j] != '}' {
                    j += 1;
                }
            }
            return Some((i, j));
        }
        if c == '"' {
            in_string = true;
        }
        i += 1;
    }
    None
}

// Removes every `key=value` property from a KDL line.
fn remove_prop(line: &str, key: &str) -> String {
    let mut chars: Vec<char> = line.chars().collect();
    while let Some((start, end)) = prop_span(&chars, key) {
        let start = if chars[start - 1] == ' ' { start - 1 } else { start };
        chars.drain(start..end);
    }
    chars.into_iter().collect()
}

// The full `key=value` token of a property, verbatim.
fn prop_token(line: &str, key: &str) -> Option<String> {
    let chars: Vec<char> = line.chars().collect();
    prop_span(&chars, key).map(|(start, end)| chars[start..end].iter().collect())
}

// get_focused_pane_info reports the server's stable tab id; everything else
// in the plugin (own_tab, instance tabs) speaks display positions, so the
// id is translated through the TabUpdate-derived states. An id with no
// position yet (stale TabUpdate) falls back to the cached position.
fn active_tab_for_decision(
    cached_active_tab: Option<usize>,
    focused_pane_info: Result<(usize, PaneId), String>,
    tab_states: &BTreeMap<usize, TabState>,
) -> Option<usize> {
    focused_pane_info
        .ok()
        .and_then(|(tab_id, _)| {
            tab_states
                .iter()
                .find(|(_, state)| state.id == tab_id)
                .map(|(position, _)| *position)
        })
        .or(cached_active_tab)
}

fn sidebar_instances(manifest: &PaneManifest) -> Vec<SidebarInstance> {
    let mut instances: Vec<SidebarInstance> = manifest
        .panes
        .iter()
        .flat_map(|(tab, panes)| {
            panes
                .iter()
                .filter(|p| {
                    p.is_plugin
                        && p.plugin_url
                            .as_deref()
                            .is_some_and(|u| u.contains("zellij-sidebar"))
                })
                .map(move |p| SidebarInstance {
                    pane_id: p.id,
                    tab: *tab,
                    floating: p.is_floating,
                })
        })
        .collect();
    instances.sort_unstable_by_key(|i| i.pane_id);
    instances
}

fn move_selection(current: usize, delta: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (current as isize + delta).clamp(0, len as isize - 1) as usize
}

// Zellij mouse positions are pane-local and one-based. Each pane occupies two
// display lines (title + status) below the header.
fn target_for_line(line: isize, row_count: usize) -> LineTarget {
    if line <= 0 {
        return LineTarget::None;
    }
    if line == 1 {
        return LineTarget::Header;
    }
    let idx = (line as usize - 2) / 2;
    if idx < row_count {
        LineTarget::Row(idx)
    } else {
        LineTarget::None
    }
}

fn state_marker(fields: &agent::AgentFields) -> &'static str {
    match fields.state {
        agent::AgentState::Blocked => "\u{1b}[31m\u{25cf}\u{1b}[0m ",
        agent::AgentState::Working => "\u{1b}[33m\u{25cf}\u{1b}[0m ",
        agent::AgentState::Done => "\u{1b}[36m\u{25cf}\u{1b}[0m ",
        agent::AgentState::Idle => "\u{1b}[32m\u{2713}\u{1b}[0m ",
        agent::AgentState::Unknown => "  ",
    }
}

fn row_marker(row: &Row) -> &'static str {
    state_marker(&row.agent)
}

fn title_style(row: &Row) -> &'static str {
    match (row.focused, row.agent.kind == agent::AgentKind::Unknown) {
        (true, false) => "\u{1b}[1;36m",
        (true, true) => "\u{1b}[1m",
        (false, false) => "\u{1b}[36m",
        (false, true) => "",
    }
}

fn preserve_agent_fields(rows: &mut [Row], old: &[Row]) {
    for row in rows.iter_mut() {
        if let Some(prev) = old.iter().find(|r| r.pane_id == row.pane_id) {
            row.agent = prev.agent.clone();
        }
    }
}

fn own_tab_position(manifest: &PaneManifest, own_plugin_id: u32) -> Option<usize> {
    manifest.panes.iter().find_map(|(tab, panes)| {
        panes
            .iter()
            .any(|p| p.is_plugin && p.id == own_plugin_id)
            .then_some(*tab)
    })
}

// Rows for the tab this plugin lives in: terminal panes only, top-to-bottom,
// excluding suppressed/unselectable panes. Agent identity is enriched by
// timer polling, after the manifest has created the base rows.
fn rows_for_own_tab(manifest: &PaneManifest, own_plugin_id: u32) -> Vec<Row> {
    let Some(tab) = own_tab_position(manifest, own_plugin_id) else {
        return Vec::new();
    };
    let mut panes: Vec<&PaneInfo> = manifest
        .panes
        .get(&tab)
        .map(|v| v.iter().collect())
        .unwrap_or_default();
    panes.retain(|p| !p.is_plugin && p.is_selectable && !p.is_suppressed);
    panes.sort_by_key(|p| (p.pane_y, p.pane_x));
    panes
        .iter()
        .map(|p| Row {
            pane_id: p.id,
            title: p.title.clone(),
            focused: p.is_focused,
            agent: agent::AgentFields::default(),
        })
        .collect()
}

// Host test builds cannot link the wasm host import; tests only exercise pure
// functions, so a no-op stub satisfies the linker.
#[cfg(all(test, not(target_family = "wasm")))]
#[no_mangle]
extern "C" fn host_run_plugin_command() {}

#[cfg(test)]
mod tests {
    use super::*;

    fn pane(id: u32, is_plugin: bool, title: &str, y: usize, focused: bool) -> PaneInfo {
        PaneInfo {
            id,
            is_plugin,
            title: title.to_owned(),
            pane_y: y,
            is_focused: focused,
            is_selectable: true,
            ..Default::default()
        }
    }

    fn sidebar_pane(id: u32, floating: bool) -> PaneInfo {
        let mut p = pane(id, true, "sidebar", 0, false);
        p.plugin_url = Some("file:/tmp/zellij-sidebar.wasm".to_owned());
        p.is_floating = floating;
        p
    }

    fn inst(pane_id: u32, tab: usize, floating: bool) -> SidebarInstance {
        SidebarInstance {
            pane_id,
            tab,
            floating,
        }
    }

    fn tab_info(
        position: usize,
        tab_id: usize,
        active: bool,
        swap_name: Option<&str>,
        swap_dirty: bool,
    ) -> TabInfo {
        TabInfo {
            position,
            tab_id,
            active,
            active_swap_layout_name: swap_name.map(str::to_owned),
            is_swap_layout_dirty: swap_dirty,
            ..Default::default()
        }
    }

    fn manifest(tabs: Vec<(usize, Vec<PaneInfo>)>) -> PaneManifest {
        let mut panes = std::collections::HashMap::new();
        for (tab, list) in tabs {
            panes.insert(tab, list);
        }
        PaneManifest { panes }
    }

    #[test]
    fn lists_only_terminal_panes_of_own_tab_sorted_top_to_bottom() {
        let m = manifest(vec![
            (0, vec![pane(9, false, "other-tab", 0, false)]),
            (
                1,
                vec![
                    pane(7, true, "sidebar", 0, false), // self (plugin id 7)
                    pane(3, false, "lower", 10, false),
                    pane(2, false, "upper", 2, true),
                ],
            ),
        ]);
        let rows = rows_for_own_tab(&m, 7);
        assert_eq!(
            rows.iter().map(|r| r.pane_id).collect::<Vec<_>>(),
            vec![2, 3]
        );
        assert!(rows[0].focused);
        assert!(!rows[1].focused);
        assert_eq!(rows[0].agent, agent::AgentFields::default());
    }

    #[test]
    fn initial_rows_start_with_unknown_agent_fields() {
        let m = manifest(vec![(
            0,
            vec![
                pane(7, true, "sidebar", 0, false),
                pane(1, false, "shell", 1, false),
            ],
        )]);
        let rows = rows_for_own_tab(&m, 7);
        assert_eq!(rows[0].agent, agent::AgentFields::default());
    }

    #[test]
    fn preserves_agent_fields_by_pane_id_after_manifest_rebuild() {
        let enriched = agent::AgentFields {
            kind: agent::AgentKind::Codex,
            state: agent::AgentState::Blocked,
            status: "allow command?".to_owned(),
            running_command: Some(vec!["codex".to_owned()]),
        };
        let old = vec![Row {
            pane_id: 1,
            title: "old".to_owned(),
            focused: false,
            agent: enriched.clone(),
        }];
        let mut rebuilt = vec![Row {
            pane_id: 1,
            title: "new".to_owned(),
            focused: true,
            agent: agent::AgentFields::default(),
        }];

        preserve_agent_fields(&mut rebuilt, &old);

        assert_eq!(rebuilt[0].agent, enriched);
        assert_eq!(rebuilt[0].title, "new");
        assert!(rebuilt[0].focused);
    }

    #[test]
    fn focused_rows_still_use_agent_state_marker() {
        let row = Row {
            pane_id: 1,
            title: "codex".to_owned(),
            focused: true,
            agent: agent::AgentFields {
                kind: agent::AgentKind::Codex,
                state: agent::AgentState::Blocked,
                status: "press enter to confirm".to_owned(),
                running_command: Some(vec!["codex".to_owned()]),
            },
        };

        assert_eq!(row_marker(&row), state_marker(&row.agent));
    }

    #[test]
    fn focused_known_agent_rows_have_distinct_title_style() {
        let focused = Row {
            pane_id: 1,
            title: "codex".to_owned(),
            focused: true,
            agent: agent::AgentFields {
                kind: agent::AgentKind::Codex,
                state: agent::AgentState::Working,
                status: "Esc to cancel".to_owned(),
                running_command: Some(vec!["codex".to_owned()]),
            },
        };
        let unfocused = Row {
            focused: false,
            ..focused.clone()
        };

        assert_ne!(title_style(&focused), title_style(&unfocused));
        assert_eq!(title_style(&focused), "\u{1b}[1;36m");
        assert_eq!(title_style(&unfocused), "\u{1b}[36m");
    }

    #[test]
    fn skips_suppressed_and_unselectable_panes() {
        let mut hidden = pane(4, false, "hidden", 3, false);
        hidden.is_suppressed = true;
        let mut unselectable = pane(5, false, "nope", 4, false);
        unselectable.is_selectable = false;
        let m = manifest(vec![(
            0,
            vec![pane(7, true, "sidebar", 0, false), hidden, unselectable],
        )]);
        assert!(rows_for_own_tab(&m, 7).is_empty());
    }

    #[test]
    fn empty_when_own_tab_not_found() {
        let m = manifest(vec![(0, vec![pane(1, false, "shell", 0, false)])]);
        assert!(rows_for_own_tab(&m, 99).is_empty());
    }

    #[test]
    fn maps_display_lines_to_header_and_two_line_rows() {
        assert_eq!(target_for_line(0, 2), LineTarget::None);
        assert_eq!(target_for_line(1, 2), LineTarget::Header);
        assert_eq!(target_for_line(2, 2), LineTarget::Row(0));
        assert_eq!(target_for_line(3, 2), LineTarget::Row(0));
        assert_eq!(target_for_line(4, 2), LineTarget::Row(1));
        assert_eq!(target_for_line(5, 2), LineTarget::Row(1));
        assert_eq!(target_for_line(6, 2), LineTarget::None);
        assert_eq!(target_for_line(-3, 2), LineTarget::None);
    }

    #[test]
    fn selection_moves_within_bounds() {
        assert_eq!(move_selection(0, -1, 5), 0);
        assert_eq!(move_selection(0, 1, 5), 1);
        assert_eq!(move_selection(4, 1, 5), 4);
        assert_eq!(move_selection(2, -1, 5), 1);
        assert_eq!(move_selection(0, 1, 0), 0);
    }

    #[test]
    fn clean_tab_steers_between_docked_and_undocked_by_swap_name() {
        // Installed order is [BASE, docked, undocked]; steering by the
        // reported name avoids the position quirks of blind cycling around
        // BASE and the end of the list.
        let instances = [inst(7, 1, false)];
        let steer = |name: Option<&str>| decide_toggle(Some(1), false, Some(1), name, false, 7, &instances);
        assert_eq!(
            steer(Some("docked")),
            ToggleAction::SteerSwap { backwards: false }
        );
        assert_eq!(
            steer(Some("undocked")),
            ToggleAction::SteerSwap { backwards: true }
        );
        // BASE is geometrically identical to docked on template-born tabs, so
        // forwards (BASE -> docked) is a dead press; backwards from position 0
        // wraps to the list end (undocked) — a visible collapse.
        assert_eq!(
            steer(Some("BASE")),
            ToggleAction::SteerSwap { backwards: true }
        );
        assert_eq!(steer(None), ToggleAction::SteerSwap { backwards: true });
        assert_eq!(
            steer(Some("vertical")),
            ToggleAction::SteerSwap { backwards: true }
        );
    }

    #[test]
    fn dirty_tab_regenerates_swaps_toward_the_other_state() {
        // A damaged tab (manual split/resize) would snap-fold to a stale
        // template on the next swap; instead the swap set is rebuilt around
        // the current arrangement and steered to the opposite state.
        let instances = [inst(7, 1, false)];
        let regen = |name: Option<&str>| decide_toggle(Some(1), false, Some(1), name, true, 7, &instances);
        assert_eq!(
            regen(Some("docked")),
            ToggleAction::RegenerateSwaps {
                target: DockState::Undocked
            }
        );
        assert_eq!(
            regen(Some("undocked")),
            ToggleAction::RegenerateSwaps {
                target: DockState::Docked
            }
        );
        assert_eq!(
            regen(Some("BASE")),
            ToggleAction::RegenerateSwaps {
                target: DockState::Undocked
            }
        );
    }

    #[test]
    fn defers_to_the_resident_instance_of_the_active_tab() {
        let instances = [inst(7, 1, false), inst(9, 3, false)];
        assert_eq!(
            decide_toggle(Some(1), false, Some(3), None, false, 7, &instances),
            ToggleAction::Ignore
        );
    }

    #[test]
    fn lowest_id_instance_retrofits_a_tab_without_a_sidebar() {
        let instances = [inst(7, 1, false), inst(9, 2, false)];
        assert_eq!(
            decide_toggle(Some(1), false, Some(5), None, false, 7, &instances),
            ToggleAction::Retrofit
        );
        assert_eq!(
            decide_toggle(Some(2), false, Some(5), None, false, 9, &instances),
            ToggleAction::Ignore
        );
    }

    #[test]
    fn ignores_toggle_when_active_tab_is_unknown() {
        assert_eq!(
            decide_toggle(Some(1), false, None, None, false, 7, &[inst(7, 1, false)]),
            ToggleAction::Ignore
        );
    }

    #[test]
    fn floating_resident_in_active_tab_retrofits_its_own_tab() {
        // Keybind launch-if-missing bootstraps a floating sidebar into the
        // active tab; that tab has no swap set to cycle, so the floating
        // resident retrofits its own tab instead of dead-cycling.
        let instances = [inst(7, 1, true)];
        assert_eq!(
            decide_toggle(Some(1), true, Some(1), None, false, 7, &instances),
            ToggleAction::Retrofit
        );
    }

    #[test]
    fn floating_resident_defers_to_the_tabs_tiled_sidebar() {
        // If a retrofit docks a fresh rail instead of seating the floating
        // actor, the tab holds both; only the tiled one may act, otherwise
        // every toggle would fire another retrofit.
        let instances = [inst(7, 1, true), inst(9, 1, false)];
        assert_eq!(
            decide_toggle(Some(1), true, Some(1), None, false, 7, &instances),
            ToggleAction::Ignore
        );
        assert_eq!(
            decide_toggle(Some(1), false, Some(1), Some("docked"), false, 9, &instances),
            ToggleAction::SteerSwap { backwards: false }
        );
    }

    #[test]
    fn lowest_id_floating_resident_is_the_single_retrofit_actor() {
        // Two floating sidebars in one tab (stray pipe launches) must not
        // both fire the override.
        let instances = [inst(7, 1, true), inst(9, 1, true)];
        assert_eq!(
            decide_toggle(Some(1), true, Some(1), None, false, 7, &instances),
            ToggleAction::Retrofit
        );
        assert_eq!(
            decide_toggle(Some(1), true, Some(1), None, false, 9, &instances),
            ToggleAction::Ignore
        );
    }

    #[test]
    fn floating_bystander_defers_to_any_active_tab_resident() {
        // Floating bootstrap instance parked in tab 2; the active tab's own
        // resident acts, whether tiled or floating.
        let instances = [inst(7, 2, true), inst(9, 1, false)];
        assert_eq!(
            decide_toggle(Some(2), true, Some(1), None, false, 7, &instances),
            ToggleAction::Ignore
        );
        let instances = [inst(7, 2, true), inst(9, 1, true)];
        assert_eq!(
            decide_toggle(Some(2), true, Some(1), None, false, 7, &instances),
            ToggleAction::Ignore
        );
    }

    #[test]
    fn floating_bystander_joins_the_election_for_a_sidebarless_tab() {
        // No sidebar in the active tab: the lowest-id instance session-wide
        // retrofits it, floating or not.
        let instances = [inst(7, 2, true), inst(9, 3, false)];
        assert_eq!(
            decide_toggle(Some(2), true, Some(5), None, false, 7, &instances),
            ToggleAction::Retrofit
        );
        assert_eq!(
            decide_toggle(Some(3), false, Some(5), None, false, 9, &instances),
            ToggleAction::Ignore
        );
    }

    #[test]
    fn remote_election_steers_a_tab_that_already_carries_our_swap_names() {
        // Manifest lag: TabUpdate can report a tab's swap name (docked or
        // undocked) before PaneUpdate reflects an instance living there
        // again (or after one crashed/closed there) — the session-wide
        // election then sees no instance in that tab and would elect a
        // Retrofit. A tab already flying one of our swap names has, or very
        // recently had, an installed rail; overriding it fresh is the
        // corruption seeder (a full replacement of the tab's arrangement).
        // Steering is always safe once a swap name is confirmed ours.
        let instances = [inst(7, 2, false)];
        assert_eq!(
            decide_toggle(Some(2), false, Some(5), Some("docked"), false, 7, &instances),
            ToggleAction::SteerSwap { backwards: false }
        );
        assert_eq!(
            decide_toggle(Some(2), false, Some(5), Some("undocked"), false, 7, &instances),
            ToggleAction::SteerSwap { backwards: true }
        );
        // BASE, a foreign name, or no name at all: no rail is confirmed
        // installed there, so the election still retrofits.
        assert_eq!(
            decide_toggle(Some(2), false, Some(5), Some("BASE"), false, 7, &instances),
            ToggleAction::Retrofit
        );
        assert_eq!(
            decide_toggle(Some(2), false, Some(5), None, false, 7, &instances),
            ToggleAction::Retrofit
        );
    }

    #[test]
    fn tab_update_records_floating_pane_visibility() {
        // While a tab's floating panes are visible, TabInfo's swap name
        // reports the FLOATING layer (tab/mod.rs swap_layout_info), so the
        // tiled override outcome is unreadable; the state must carry the
        // visibility bit for the steer to know whose name it is reading.
        let mut sidebar = Sidebar::default();
        let mut with_floats = tab_info(1, 7, true, Some("BASE"), false);
        with_floats.are_floating_panes_visible = true;
        sidebar.update(Event::TabUpdate(vec![
            tab_info(0, 5, false, Some("BASE"), false),
            with_floats,
        ]));
        assert!(!sidebar.tab_states.get(&0).unwrap().floating_visible);
        assert!(sidebar.tab_states.get(&1).unwrap().floating_visible);
    }

    #[test]
    fn tab_update_records_swap_state_per_tab() {
        let mut sidebar = Sidebar::default();
        sidebar.update(Event::TabUpdate(vec![
            tab_info(0, 5, false, Some("BASE"), false),
            tab_info(1, 7, true, Some("docked"), true),
        ]));
        assert_eq!(sidebar.active_tab, Some(1));
        assert_eq!(
            sidebar.tab_states.get(&0),
            Some(&TabState {
                id: 5,
                swap_name: Some("BASE".to_owned()),
                swap_dirty: false,
                floating_visible: false,
            })
        );
        assert_eq!(
            sidebar.tab_states.get(&1),
            Some(&TabState {
                id: 7,
                swap_name: Some("docked".to_owned()),
                swap_dirty: true,
                floating_visible: false,
            })
        );
    }

    fn steer_states(name: Option<&str>, floating_visible: bool) -> BTreeMap<usize, TabState> {
        let mut map = BTreeMap::new();
        map.insert(
            1,
            TabState {
                id: 4,
                swap_name: name.map(str::to_owned),
                // A landed override was observed live still flagged dirty;
                // the damage flag is no part of the signature.
                swap_dirty: true,
                floating_visible,
            },
        );
        map
    }

    fn steer_to(target: DockState) -> PendingSteer {
        PendingSteer { tab: 1, target }
    }

    #[test]
    fn pending_steer_completes_the_toggle_from_wherever_the_override_landed() {
        // A landed override does not leave the tab at BASE unconditionally:
        // zellij relayouts the tab right after installing the set, so the
        // report is BASE only when the absorb base's exact pane count
        // matches (single-shell tabs); multi-pane tabs arrive already at
        // "docked". The steer turns the reported entry into the one
        // deliberate step that reaches the target.
        assert_eq!(
            pending_steer_disposition(steer_to(DockState::Docked), Some(1), &steer_states(Some("BASE"), false)),
            SteerDisposition::Fire { backwards: false }
        );
        assert_eq!(
            pending_steer_disposition(steer_to(DockState::Undocked), Some(1), &steer_states(Some("BASE"), false)),
            SteerDisposition::Fire { backwards: true }
        );
        // The relayout stopped one entry short of the target: step onward.
        assert_eq!(
            pending_steer_disposition(steer_to(DockState::Undocked), Some(1), &steer_states(Some("docked"), false)),
            SteerDisposition::Fire { backwards: false }
        );
        assert_eq!(
            pending_steer_disposition(steer_to(DockState::Docked), Some(1), &steer_states(Some("undocked"), false)),
            SteerDisposition::Fire { backwards: true }
        );
        // The relayout already landed on the target: the toggle is done and
        // one more step would overshoot it (the live one-step-off arrival).
        assert_eq!(
            pending_steer_disposition(steer_to(DockState::Docked), Some(1), &steer_states(Some("docked"), false)),
            SteerDisposition::Drop
        );
        assert_eq!(
            pending_steer_disposition(steer_to(DockState::Undocked), Some(1), &steer_states(Some("undocked"), false)),
            SteerDisposition::Drop
        );
        // A foreign name means the override failed or raced; the press is
        // abandoned rather than kept armed forever.
        assert_eq!(
            pending_steer_disposition(steer_to(DockState::Docked), Some(1), &steer_states(Some("stacked"), false)),
            SteerDisposition::Drop
        );
        // No name at all is the one-selectable-pane blind spot; the
        // override's outcome is still unreadable, so the press stays armed.
        assert_eq!(
            pending_steer_disposition(steer_to(DockState::Docked), Some(1), &steer_states(None, false)),
            SteerDisposition::Keep
        );
        // The tab vanished (closed) — the press has nowhere to go.
        assert_eq!(
            pending_steer_disposition(
                PendingSteer { tab: 2, target: DockState::Docked },
                Some(1),
                &steer_states(Some("BASE"), false)
            ),
            SteerDisposition::Drop
        );
        // The user switched away: swap presses act on the client's active
        // tab, so firing now would steer the wrong tab. The regenerated
        // set stays installed; a later press on the tab steers by name.
        assert_eq!(
            pending_steer_disposition(steer_to(DockState::Docked), Some(3), &steer_states(Some("BASE"), false)),
            SteerDisposition::Drop
        );
    }

    #[test]
    fn steer_waits_while_floating_panes_obscure_the_swap_report() {
        // While floating panes are visible the reported name speaks for the
        // floating layer, whose birth entry is also "BASE" — during a
        // bootstrap retrofit the floater itself keeps the tab in that
        // state, and firing on it steers before the override lands.
        assert_eq!(
            pending_steer_disposition(steer_to(DockState::Docked), Some(1), &steer_states(Some("BASE"), true)),
            SteerDisposition::Keep
        );
        assert_eq!(
            pending_steer_disposition(steer_to(DockState::Docked), Some(1), &steer_states(Some("docked"), true)),
            SteerDisposition::Keep
        );
    }

    #[test]
    fn bootstrap_arrival_stays_docked_despite_floating_base_reports() {
        // The live E2 failure: retrofit override in flight, the floating
        // bootstrap pane still visible. Gap TabUpdates report the floating
        // layer's "BASE"; firing there adds a stray forward step on top of
        // the relayout's own docked arrival and the tab lands one entry
        // past it, on the undocked sliver.
        let mut sidebar = Sidebar::default();
        sidebar.pending_steer = Some(PendingSteer {
            tab: 1,
            target: DockState::Docked,
        });
        let mut gap = tab_info(1, 7, true, Some("BASE"), true);
        gap.are_floating_panes_visible = true;
        sidebar.update(Event::TabUpdate(vec![gap]));
        assert!(sidebar.pending_steer.is_some());
        assert!(sidebar.toggle_cooldown.is_none());
        // The override landed and hid the floater; the relayout already
        // reached docked, so the steer stands down without a stray step.
        sidebar.update(Event::TabUpdate(vec![tab_info(1, 7, true, Some("docked"), false)]));
        assert!(sidebar.pending_steer.is_none());
        assert!(sidebar.toggle_cooldown.is_none());
    }

    #[test]
    fn rebuild_targets_a_tab_by_server_id_only_when_dumpable() {
        // The dump host call is only safe post-grant, and it speaks the
        // server's stable tab id — not the display position the rest of
        // the plugin uses. A position TabUpdate has not reported has no
        // id to dump.
        let mut tab_states = BTreeMap::new();
        tab_states.insert(
            5,
            TabState {
                id: 42,
                ..Default::default()
            },
        );
        assert_eq!(rebuild_target(true, Some(5), &tab_states), Some((5, 42)));
        assert_eq!(rebuild_target(false, Some(5), &tab_states), None);
        assert_eq!(rebuild_target(true, Some(6), &tab_states), None);
        assert_eq!(rebuild_target(true, None, &tab_states), None);
    }

    #[test]
    fn tab_update_fires_the_pending_steer_once_the_override_lands() {
        let mut sidebar = Sidebar::default();
        sidebar.pending_steer = Some(PendingSteer {
            tab: 1,
            target: DockState::Undocked,
        });
        // The tab reports no swap name (one-selectable-pane blind spot):
        // the override's outcome is unreadable, the press stays armed.
        sidebar.update(Event::TabUpdate(vec![tab_info(1, 7, true, None, false)]));
        assert!(sidebar.pending_steer.is_some());
        // Override landed (BASE — still flagged dirty, as observed live):
        // the steer fires and clears.
        sidebar.update(Event::TabUpdate(vec![tab_info(
            1,
            7,
            true,
            Some("BASE"),
            true,
        )]));
        assert!(sidebar.pending_steer.is_none());
        // A press for a tab that no longer exists is abandoned.
        sidebar.pending_steer = Some(PendingSteer {
            tab: 9,
            target: DockState::Docked,
        });
        sidebar.update(Event::TabUpdate(vec![tab_info(
            1,
            7,
            true,
            Some("BASE"),
            false,
        )]));
        assert!(sidebar.pending_steer.is_none());
    }

    #[test]
    fn press_for_the_in_flight_tab_is_swallowed_others_supersede() {
        let now = Instant::now();
        let in_flight = Some(PendingSteer {
            tab: 1,
            target: DockState::Docked,
        });
        // Tab 1's pipeline is still waiting on its override: a repeat press
        // there is the bounce, not a new intent.
        assert!(should_swallow_toggle(Some(1), in_flight, None, now));
        // A press for another tab is a fresh intent and must act.
        assert!(!should_swallow_toggle(Some(2), in_flight, None, now));
        // No known active tab: nothing to debounce against.
        assert!(!should_swallow_toggle(None, in_flight, None, now));
        // Nothing armed at all.
        assert!(!should_swallow_toggle(Some(1), None, None, now));
    }

    #[test]
    fn press_within_the_cooldown_window_is_swallowed() {
        let fired = Instant::now();
        let cooldown = Some((1, fired));
        assert!(should_swallow_toggle(Some(1), None, cooldown, fired));
        assert!(should_swallow_toggle(
            Some(1),
            None,
            cooldown,
            fired + TOGGLE_COOLDOWN / 2
        ));
        // The window closes exactly at the cooldown bound.
        assert!(!should_swallow_toggle(
            Some(1),
            None,
            cooldown,
            fired + TOGGLE_COOLDOWN
        ));
        // Another tab is never held back by tab 1's cooldown.
        assert!(!should_swallow_toggle(
            Some(2),
            None,
            cooldown,
            fired + TOGGLE_COOLDOWN / 2
        ));
    }

    #[test]
    fn steer_fire_arms_the_cooldown_for_its_tab() {
        let mut sidebar = Sidebar::default();
        sidebar.pending_steer = Some(PendingSteer {
            tab: 1,
            target: DockState::Undocked,
        });
        sidebar.update(Event::TabUpdate(vec![tab_info(1, 7, true, Some("BASE"), false)]));
        assert!(sidebar.pending_steer.is_none());
        assert_eq!(sidebar.toggle_cooldown.map(|(tab, _)| tab), Some(1));

        // A dropped press arms nothing: no collapse is about to happen.
        let mut sidebar = Sidebar::default();
        sidebar.pending_steer = Some(PendingSteer {
            tab: 9,
            target: DockState::Docked,
        });
        sidebar.update(Event::TabUpdate(vec![tab_info(1, 7, true, Some("BASE"), false)]));
        assert!(sidebar.pending_steer.is_none());
        assert!(sidebar.toggle_cooldown.is_none());
    }

    #[test]
    fn absorb_retrofit_arms_the_cooldown_for_the_active_tab() {
        // The absorb override records no PendingSteer, so without its own
        // cooldown a repeat press inside its visible lag would fire a second
        // override at the same tab.
        let mut sidebar = Sidebar::default();
        sidebar.plugin_id = 7;
        sidebar.active_tab = Some(1);
        sidebar.own_tab = Some(1);
        sidebar.own_floating = true;
        sidebar.own_url = Some("file:/tmp/zellij-sidebar.wasm".to_owned());
        sidebar.instances = vec![inst(7, 1, true)];
        sidebar.perform_toggle();
        assert_eq!(sidebar.toggle_cooldown.map(|(tab, _)| tab), Some(1));

        // Without a URL nothing is overridden: no cooldown to arm.
        let mut sidebar = Sidebar::default();
        sidebar.plugin_id = 7;
        sidebar.active_tab = Some(1);
        sidebar.own_tab = Some(1);
        sidebar.own_floating = true;
        sidebar.instances = vec![inst(7, 1, true)];
        sidebar.perform_toggle();
        assert!(sidebar.toggle_cooldown.is_none());
    }

    #[test]
    fn repeat_press_keeps_the_in_flight_steer_armed() {
        let mut sidebar = Sidebar::default();
        sidebar.active_tab = Some(1);
        let steer = PendingSteer {
            tab: 1,
            target: DockState::Undocked,
        };
        sidebar.pending_steer = Some(steer);
        sidebar.perform_toggle();
        assert_eq!(sidebar.pending_steer, Some(steer));
        // A press for another tab supersedes the parked steer.
        sidebar.active_tab = Some(2);
        sidebar.perform_toggle();
        assert!(sidebar.pending_steer.is_none());
    }

    #[test]
    fn floating_instance_closes_once_its_tab_holds_a_tiled_sidebar() {
        // After a retrofit installs a tiled rail, the floating bootstrap
        // actor lingers as an invisible config-matched zombie that keeps
        // receiving pipes; it yields to the tab's tiled sidebar.
        assert!(is_stray_floating_bootstrap(
            true,
            Some(1),
            &[inst(7, 1, true), inst(9, 1, false)]
        ));
        // The tiled sidebar lives in another tab: not superseded.
        assert!(!is_stray_floating_bootstrap(
            true,
            Some(1),
            &[inst(7, 1, true), inst(9, 2, false)]
        ));
        // A tiled instance never closes itself.
        assert!(!is_stray_floating_bootstrap(
            false,
            Some(1),
            &[inst(9, 1, false)]
        ));
        // Only floating siblings around: the retrofit has not landed.
        assert!(!is_stray_floating_bootstrap(
            true,
            Some(1),
            &[inst(7, 1, true), inst(9, 1, true)]
        ));
        // Transient manifest without an own tab yet: never fire.
        assert!(!is_stray_floating_bootstrap(true, None, &[inst(9, 1, false)]));
    }

    #[test]
    fn close_self_fires_only_once_per_instance_lifetime() {
        // close_self does not remove the pane from the very next manifest
        // snapshot the plugin sees (observed live: one instance logged four
        // "Bye" lines), so is_stray_floating_bootstrap can still read true
        // afterward; a fired flag must suppress every call after the first.
        let instances = [inst(7, 1, true), inst(9, 1, false)];
        assert!(should_close_self(false, true, Some(1), &instances));
        assert!(!should_close_self(true, true, Some(1), &instances));
        // A one-shot never re-arms, even once the manifest catches up and
        // the underlying condition reads false again.
        assert!(!should_close_self(true, false, Some(1), &[inst(9, 1, false)]));
    }

    #[test]
    fn stray_floater_sets_the_close_requested_flag_once() {
        let mut sidebar = Sidebar::default();
        sidebar.plugin_id = 7;
        let m = manifest(vec![(
            1,
            vec![sidebar_pane(7, true), sidebar_pane(9, false)],
        )]);
        assert!(!sidebar.close_requested);
        sidebar.update(Event::PaneUpdate(m.clone()));
        assert!(sidebar.close_requested);
        // A second PaneUpdate with the same stray shape (the pane has not
        // been dropped from the manifest yet) must not panic or re-fire —
        // the flag alone gates it.
        sidebar.update(Event::PaneUpdate(m));
        assert!(sidebar.close_requested);
    }

    #[test]
    fn pane_update_tracks_own_floating_state() {
        let mut sidebar = Sidebar::default();
        sidebar.plugin_id = 7;
        let m = manifest(vec![(
            1,
            vec![sidebar_pane(7, true), pane(3, false, "shell", 2, false)],
        )]);
        sidebar.update(Event::PaneUpdate(m));
        assert!(sidebar.own_floating);

        let m = manifest(vec![(
            1,
            vec![sidebar_pane(7, false), pane(3, false, "shell", 2, false)],
        )]);
        sidebar.update(Event::PaneUpdate(m));
        assert!(!sidebar.own_floating);
    }

    #[test]
    fn instance_list_records_tab_and_floating_state() {
        let mut other_plugin = pane(4, true, "tab-bar", 0, false);
        other_plugin.plugin_url = Some("zellij:tab-bar".to_owned());
        let m = manifest(vec![
            (0, vec![sidebar_pane(5, false), other_plugin]),
            (
                2,
                vec![sidebar_pane(9, true), pane(3, false, "shell", 2, false)],
            ),
        ]);
        assert_eq!(
            sidebar_instances(&m),
            vec![inst(5, 0, false), inst(9, 2, true)]
        );
    }

    #[test]
    fn live_focused_tab_overrides_stale_cached_active_tab() {
        // get_focused_pane_info reports the server's stable tab id, which
        // only matches the display position until a tab is closed or moved;
        // the TabUpdate-derived states translate id -> position.
        let mut tab_states = BTreeMap::new();
        tab_states.insert(0, TabState::default());
        tab_states.insert(
            2,
            TabState {
                id: 3,
                ..Default::default()
            },
        );
        assert_eq!(
            active_tab_for_decision(Some(1), Ok((3, PaneId::Terminal(9))), &tab_states),
            Some(2)
        );
        assert_eq!(
            active_tab_for_decision(Some(1), Err("unavailable".to_owned()), &tab_states),
            Some(1)
        );
        // An id the TabUpdate has not caught up with falls back to the cache.
        assert_eq!(
            active_tab_for_decision(Some(1), Ok((9, PaneId::Terminal(9))), &tab_states),
            Some(1)
        );
    }

    #[test]
    fn header_clicks_anywhere_toggle_the_swap_layout() {
        let rows = vec![
            Row {
                pane_id: 4,
                ..Default::default()
            },
            Row {
                pane_id: 8,
                ..Default::default()
            },
        ];
        // The whole header line is one control: clicking it collapses or
        // expands the sidebar via the tab's swap layout, same as Alt-/.
        assert_eq!(decide_click(1, &rows), ClickAction::ToggleDock);
        assert_eq!(decide_click(2, &rows), ClickAction::FocusPane(4));
        assert_eq!(decide_click(5, &rows), ClickAction::FocusPane(8));
        assert_eq!(decide_click(99, &rows), ClickAction::None);
    }

    #[test]
    fn pre_grant_active_tab_comes_from_the_cache_only() {
        // Before the ReadApplicationState grant the host writes no response
        // bytes for get_focused_pane_info and the blocking shim call panics;
        // the cached TabUpdate value is the only safe source.
        let mut sidebar = Sidebar::default();
        sidebar.active_tab = Some(2);
        assert_eq!(sidebar.current_active_tab(), Some(2));
    }

    #[test]
    fn hands_back_focus_only_when_another_pane_can_take_it() {
        assert!(should_hand_back_focus(true, false, true));
        // Session-birth window: no other selectable pane yet — bouncing focus
        // here panics the whole server (get_active_pane_id unwraps None).
        assert!(!should_hand_back_focus(true, false, false));
        assert!(!should_hand_back_focus(true, true, true)); // nav mode keeps focus
        assert!(!should_hand_back_focus(false, false, true));
    }

    #[test]
    fn navigate_enters_nav_only_in_the_resident_instance() {
        let navigate = || PipeMessage {
            source: PipeSource::Keybind,
            name: "navigate".to_owned(),
            payload: None,
            args: BTreeMap::new(),
            is_private: false,
        };
        // The pipe reaches every instance; only the active tab's resident
        // takes nav focus — otherwise every tab's sidebar would call
        // focus_plugin_pane and fight over the client.
        let mut resident = Sidebar::default();
        resident.own_tab = Some(1);
        resident.active_tab = Some(1);
        assert!(resident.pipe(navigate()));
        assert!(resident.nav_mode);

        let mut bystander = Sidebar::default();
        bystander.own_tab = Some(2);
        bystander.active_tab = Some(1);
        assert!(!bystander.pipe(navigate()));
        assert!(!bystander.nav_mode);
    }

    fn toggle() -> PipeMessage {
        PipeMessage {
            source: PipeSource::Keybind,
            name: "toggle".to_owned(),
            payload: None,
            args: BTreeMap::new(),
            is_private: false,
        }
    }

    #[test]
    fn toggle_pipe_before_the_first_manifest_parks_the_press() {
        // Keybind launch-if-missing: the toggle pipe that launched this
        // instance arrives before any PaneUpdate has named it, so every
        // decision input is unknown and an immediate perform_toggle dies.
        let mut sidebar = Sidebar::default();
        sidebar.active_tab = Some(0);
        assert!(!sidebar.pending_bootstrap_toggle);
        sidebar.pipe(toggle());
        assert!(sidebar.pending_bootstrap_toggle);
        // Nothing acted yet.
        assert!(sidebar.pending_steer.is_none());
    }

    #[test]
    fn bootstrap_press_fires_only_when_manifest_and_tab_are_known() {
        let url = Some("file:/x.wasm".to_owned());
        assert!(should_fire_bootstrap_toggle(true, &url, Some(1), Some(1)));
        // No parked press: nothing to fire.
        assert!(!should_fire_bootstrap_toggle(false, &url, Some(1), Some(1)));
        // The manifest has not named us yet.
        assert!(!should_fire_bootstrap_toggle(true, &None, Some(1), Some(1)));
        assert!(!should_fire_bootstrap_toggle(true, &url, None, Some(1)));
        // Without an active tab decide_toggle ignores the press; keep it
        // parked instead of wasting it.
        assert!(!should_fire_bootstrap_toggle(true, &url, Some(1), None));
    }

    #[test]
    fn first_manifest_consumes_the_parked_press_exactly_once() {
        let mut sidebar = Sidebar::default();
        sidebar.plugin_id = 7;
        sidebar.update(Event::TabUpdate(vec![tab_info(1, 4, true, None, false)]));
        sidebar.pipe(toggle());
        assert!(sidebar.pending_bootstrap_toggle);
        // First manifest names the instance (floating, in the active tab):
        // the parked press fires through perform_toggle — its own state is
        // clean, so the debounce gate passes it — and is consumed.
        let m = manifest(vec![(
            1,
            vec![sidebar_pane(7, true), pane(3, false, "shell", 2, false)],
        )]);
        sidebar.update(Event::PaneUpdate(m.clone()));
        assert!(!sidebar.pending_bootstrap_toggle);
        // Later manifests never re-fire it.
        sidebar.update(Event::PaneUpdate(m));
        assert!(!sidebar.pending_bootstrap_toggle);
    }

    #[test]
    fn parked_press_dies_with_a_superseded_floater() {
        // The manifest that would fire the parked press can simultaneously
        // show this floating instance superseded by a tiled rail; the
        // instance is closing, so the press must not retrofit from it.
        let mut sidebar = Sidebar::default();
        sidebar.plugin_id = 7;
        sidebar.update(Event::TabUpdate(vec![tab_info(1, 4, true, None, false)]));
        sidebar.pipe(toggle());
        let m = manifest(vec![(1, vec![sidebar_pane(7, true), sidebar_pane(9, false)])]);
        sidebar.update(Event::PaneUpdate(m));
        assert!(sidebar.close_requested);
        assert!(sidebar.pending_bootstrap_toggle);
    }

    #[test]
    fn session_birth_manifest_with_only_the_sidebar_never_bounces_focus() {
        // A layout that focuses the sidebar produces a birth manifest whose
        // tab holds no other selectable pane; handing focus back then makes
        // the server unwrap a missing active pane and the session dies. The
        // handler derives has_focus_target from the row list.
        let m = manifest(vec![(0, vec![pane(7, true, "sidebar", 0, true)])]);
        let rows = rows_for_own_tab(&m, 7);
        assert!(!should_hand_back_focus(true, false, !rows.is_empty()));
    }

    // A dump_session_layout_for_tab result as zellij v0.44 serializes it:
    // the requesting plugin's own rail already removed by the server, chrome
    // present, nested percent containers, cwds, focus on the focused pane,
    // a floating_panes section, and the session's default-layout template
    // and swap sections trailing the tab node.
    fn dump_fixture() -> &'static str {
        r#"layout {
    tab name="Tab #1" focus=true hide_floating_panes=true {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        pane size="96%" split_direction="vertical" {
            pane name="build" cwd="/Users/clkao/git/zaphod" focus=true size="62%"
            pane size="38%" {
                pane cwd="/tmp" size="50%"
                pane command="claude" cwd="/Users/clkao" size="50%" {
                    args "--continue"
                }
            }
        }
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
        floating_panes {
            pane {
                x 10
                y 5
                width 86
                height 22
            }
        }
    }
    new_tab_template {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        pane
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
    swap_tiled_layout name="stacked" {
        tab min_panes=5 {
            pane stacked=true {
                children
            }
        }
    }
}
"#
    }

    fn jit_config() -> BTreeMap<String, String> {
        let mut config = BTreeMap::new();
        config.insert("rail".to_owned(), "1".to_owned());
        config
    }

    #[test]
    fn split_preserving_layout_wraps_the_dumped_arrangement_in_both_swaps() {
        let kdl = split_preserving_layout_kdl(
            dump_fixture(),
            "file:/tmp/zellij-sidebar.wasm",
            &jit_config(),
        )
        .unwrap();
        assert!(kdl.contains("swap_tiled_layout name=\"docked\""));
        assert!(kdl.contains("swap_tiled_layout name=\"undocked\""));
        // Rail slot: 28 cols in the docked swap and the base tab, 1 col in
        // the undocked swap; the plugin block carries the config identity.
        assert_eq!(
            kdl.matches("pane size=28 borderless=true name=\"sidebar\"")
                .count(),
            2
        );
        assert_eq!(
            kdl.matches("pane size=1 borderless=true name=\"sidebar\"")
                .count(),
            1
        );
        assert_eq!(
            kdl.matches("plugin location=\"file:/tmp/zellij-sidebar.wasm\"")
                .count(),
            3
        );
        assert_eq!(kdl.matches("rail \"1\"").count(), 3);
        // Each chrome pane the dump shows becomes one canonical row in all
        // three tab bodies, so base and swaps stay chrome-consistent with
        // the live tab.
        assert_eq!(kdl.matches("zellij:tab-bar").count(), 3);
        assert_eq!(kdl.matches("zellij:status-bar").count(), 3);
        // The user's arrangement appears in both swaps with splits, sizes,
        // cwds and running commands intact.
        assert_eq!(kdl.matches("size=\"62%\"").count(), 2);
        assert_eq!(kdl.matches("size=\"38%\"").count(), 2);
        assert_eq!(kdl.matches("cwd=\"/tmp\"").count(), 2);
        assert_eq!(kdl.matches("command=\"claude\"").count(), 2);
        assert_eq!(kdl.matches("args \"--continue\"").count(), 2);
        // The base absorbs every pane into a stack; swaps re-seat from it.
        assert!(kdl.contains("pane stacked=true"));
        assert!(kdl.contains("children"));
        // The rail is the left column of the vertical split.
        assert!(kdl.find("name=\"sidebar\"").unwrap() < kdl.find("size=\"62%\"").unwrap());
    }

    #[test]
    fn split_preserving_layout_strips_volatile_dump_attributes() {
        let kdl = split_preserving_layout_kdl(
            dump_fixture(),
            "file:/tmp/zellij-sidebar.wasm",
            &jit_config(),
        )
        .unwrap();
        // focus in an override races the client's real focus; names would
        // pin user titles; the tab name would overwrite the user's on apply
        // (generated tab nodes stay unnamed so the live name survives).
        assert!(!kdl.contains("focus=true"));
        assert!(!kdl.contains("name=\"build\""));
        assert!(!kdl.contains("tab name="));
        assert!(!kdl.contains("hide_floating_panes"));
        // The region container fills whatever remains next to the rail; its
        // dumped size spoke for the pre-rail-removal geometry.
        assert!(!kdl.contains("size=\"96%\""));
        // Floating panes are never part of the tiled swap story.
        assert!(!kdl.contains("floating_panes"));
        assert!(!kdl.contains("width 86"));
        // The session's own template and swap sections do not ride along.
        assert!(!kdl.contains("new_tab_template"));
        assert!(!kdl.contains("min_panes=5"));
        assert!(!kdl.contains("swap_tiled_layout name=\"stacked\""));
    }

    #[test]
    fn split_preserving_layout_rejects_dumps_it_cannot_rebuild_from() {
        let config = jit_config();
        // No tab node at all (e.g. a stale tab id matched nothing).
        assert!(split_preserving_layout_kdl("layout {\n}\n", "file:/x.wasm", &config).is_err());
        // Chrome-only tab: nothing of the user's to preserve.
        let chrome_only = r#"layout {
    tab name="Tab #2" {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
}
"#;
        assert!(split_preserving_layout_kdl(chrome_only, "file:/x.wasm", &config).is_err());
    }

    #[test]
    fn sibling_region_rows_stay_ordered_inside_one_flexible_container() {
        // A tab whose root splits top-to-bottom dumps the rows as siblings
        // of the chrome; they must land next to the rail as one container
        // keeping their order and relative sizes.
        let dump = r#"layout {
    tab name="Tab #3" {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        pane cwd="/upper" size="55%"
        pane cwd="/lower" size="43%"
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
}
"#;
        let kdl = split_preserving_layout_kdl(dump, "file:/x.wasm", &jit_config()).unwrap();
        assert_eq!(kdl.matches("cwd=\"/upper\" size=\"55%\"").count(), 2);
        assert_eq!(kdl.matches("cwd=\"/lower\" size=\"43%\"").count(), 2);
        assert!(kdl.find("/upper").unwrap() < kdl.find("/lower").unwrap());
        // Wrapped, not spliced bare into the vertical split: the two rows
        // would otherwise become columns.
        let docked = &kdl[..kdl.find("undocked").unwrap()];
        let wrapper = docked.find("pane {").expect("row wrapper");
        assert!(wrapper > docked.find("split_direction=\"vertical\"").unwrap());
    }

    #[test]
    fn stacked_arrangements_keep_their_stack_flags() {
        let dump = r#"layout {
    tab name="Tab #4" {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        pane stacked=true {
            pane cwd="/a"
            pane cwd="/b" expanded=true
            pane cwd="/c"
        }
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
}
"#;
        let kdl = split_preserving_layout_kdl(dump, "file:/x.wasm", &jit_config()).unwrap();
        // Both swaps carry the stack (plus the base's absorb stack).
        assert_eq!(kdl.matches("pane stacked=true").count(), 3);
        assert_eq!(kdl.matches("expanded=true").count(), 2);
    }

    #[test]
    fn chrome_baked_into_a_quadrant_is_reemitted_as_a_canonical_row() {
        // Observed live: zellij's absorb seated the tab-bar in a quadrant
        // slot (pane size="50%" borderless=true { zellij:tab-bar }) with no
        // top chrome row. The transform must pull it out wherever it sits
        // and emit one canonical top row per tab body — baking it into the
        // swaps as a user pane mangles the tab permanently.
        let dump = r#"layout {
    tab name="Tab #1" focus=true hide_floating_panes=true {
        pane split_direction="vertical" size="98%" {
            pane size="50%" {
                pane cwd="/a" size="50%"
                pane cwd="/b" size="50%"
            }
            pane size="50%" {
                pane cwd="/c" size="34%"
                pane size="66%" borderless=true {
                    plugin location="zellij:tab-bar"
                }
            }
        }
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
}
"#;
        let kdl = split_preserving_layout_kdl(dump, "file:/x.wasm", &jit_config()).unwrap();
        let tab_bar_row = "pane size=1 borderless=true {\nplugin location=\"zellij:tab-bar\"\n}\n";
        let status_bar_row =
            "pane size=1 borderless=true {\nplugin location=\"zellij:status-bar\"\n}\n";
        assert_eq!(kdl.matches(tab_bar_row).count(), 3);
        assert_eq!(kdl.matches("zellij:tab-bar").count(), 3);
        assert_eq!(kdl.matches(status_bar_row).count(), 3);
        // The quadrant seat is gone entirely, not left as an empty slot.
        assert!(!kdl.contains("size=\"66%\""));
        // The chrome pane's lone sibling takes over the container's slot.
        assert_eq!(kdl.matches("cwd=\"/c\" size=\"50%\"").count(), 2);
        assert!(!kdl.contains("size=\"34%\""));
        // Chrome frames the user region: tab-bar above it, status-bar below.
        assert!(kdl.find("zellij:tab-bar").unwrap() < kdl.find("cwd=\"/a\"").unwrap());
        assert!(kdl.find("cwd=\"/a\"").unwrap() < kdl.find("zellij:status-bar").unwrap());
    }

    #[test]
    fn chrome_inside_a_stack_is_extracted_and_reemitted() {
        // Observed live on a hand-split-then-retrofit tab: the tab-bar
        // arrived as a stack pane. It must leave the stack and become the
        // canonical top row; the stack keeps its surviving panes.
        let dump = r#"layout {
    tab name="Tab #2" {
        pane stacked=true {
            pane cwd="/a"
            pane cwd="/b" expanded=true
            pane borderless=true {
                plugin location="zellij:tab-bar"
            }
        }
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
}
"#;
        let kdl = split_preserving_layout_kdl(dump, "file:/x.wasm", &jit_config()).unwrap();
        let tab_bar_row = "pane size=1 borderless=true {\nplugin location=\"zellij:tab-bar\"\n}\n";
        assert_eq!(kdl.matches(tab_bar_row).count(), 3);
        assert_eq!(kdl.matches("zellij:tab-bar").count(), 3);
        assert_eq!(kdl.matches("zellij:status-bar").count(), 3);
        // The stack survives with its two panes (plus the base absorb stack).
        assert_eq!(kdl.matches("pane stacked=true").count(), 3);
        assert_eq!(kdl.matches("cwd=\"/a\"").count(), 2);
        assert_eq!(kdl.matches("expanded=true").count(), 2);
        assert!(!kdl.contains("pane borderless=true"));
    }

    #[test]
    fn status_bar_absorbed_into_a_stack_is_reemitted_as_the_bottom_row() {
        // Observed live on a hand-split tab after an absorb: the status-bar
        // seated INSIDE the stack as its last pane, with no tab-level
        // bottom row and the tab-bar row intact. A rebuild dump arrives
        // with that damage and must leave it repaired: exactly one
        // canonical status-bar row per tab body, and the stack keeps only
        // the user panes.
        let dump = r#"layout {
    tab name="Tab #2" {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        pane stacked=true {
            pane cwd="/a"
            pane cwd="/b"
            pane cwd="/c"
            pane cwd="/d" expanded=true
            pane borderless=true {
                plugin location="zellij:status-bar"
            }
        }
    }
}
"#;
        let kdl = split_preserving_layout_kdl(dump, "file:/x.wasm", &jit_config()).unwrap();
        let status_bar_row =
            "pane size=1 borderless=true {\nplugin location=\"zellij:status-bar\"\n}\n";
        let tab_bar_row = "pane size=1 borderless=true {\nplugin location=\"zellij:tab-bar\"\n}\n";
        assert_eq!(kdl.matches(status_bar_row).count(), 3);
        assert_eq!(kdl.matches("zellij:status-bar").count(), 3);
        assert_eq!(kdl.matches(tab_bar_row).count(), 3);
        assert_eq!(kdl.matches("zellij:tab-bar").count(), 3);
        // The stack survives with its four user panes and nothing else
        // (plus the base's absorb stack); its status-bar seat is gone.
        assert_eq!(kdl.matches("pane stacked=true").count(), 3);
        assert_eq!(kdl.matches("cwd=\"/a\"").count(), 2);
        assert_eq!(kdl.matches("cwd=\"/d\" expanded=true").count(), 2);
        assert!(!kdl.contains("pane borderless=true {"));
        // Chrome frames the user region: tab-bar above, status-bar below.
        assert!(kdl.find("zellij:tab-bar").unwrap() < kdl.find("cwd=\"/a\"").unwrap());
        assert!(kdl.find("cwd=\"/a\"").unwrap() < kdl.find("zellij:status-bar").unwrap());
    }

    #[test]
    fn container_emptied_by_chrome_extraction_is_dropped() {
        // A container whose only content was chrome must vanish with it,
        // and the cascade may leave its parent with a single child, which
        // then takes over the parent's slot.
        let dump = r#"layout {
    tab name="Tab #3" {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        pane split_direction="vertical" {
            pane cwd="/only" size="70%"
            pane size="30%" {
                pane size=1 borderless=true {
                    plugin location="zellij:status-bar"
                }
            }
        }
    }
}
"#;
        let kdl = split_preserving_layout_kdl(dump, "file:/x.wasm", &jit_config()).unwrap();
        assert_eq!(kdl.matches("zellij:tab-bar").count(), 3);
        assert_eq!(kdl.matches("zellij:status-bar").count(), 3);
        assert_eq!(kdl.matches("cwd=\"/only\"").count(), 2);
        // The emptied 30% wrapper and its parent container are both gone:
        // the only remaining vertical splits are the generated rail splits.
        assert_eq!(kdl.matches("split_direction=\"vertical\"").count(), 3);
        assert!(!kdl.contains("size=\"30%\""));
        assert!(!kdl.contains("size=\"70%\""));
    }

    // The docked tab body of a generated layout, sliced from the marker
    // onward and parsed with extract_tab_body itself — the swap's inner
    // content is a `tab { ... }` node, the same shape extract_tab_body
    // expects from a real dump.
    fn docked_tab_body(kdl: &str) -> Vec<String> {
        let idx = kdl
            .find("swap_tiled_layout name=\"docked\"")
            .expect("docked swap present");
        extract_tab_body(&kdl[idx..]).expect("docked swap wraps a tab node")
    }

    fn normalize(lines: &[String]) -> Vec<String> {
        lines
            .iter()
            .map(|line| line.trim().to_owned())
            .filter(|line| !line.is_empty())
            .collect()
    }

    #[test]
    fn regenerating_from_the_transforms_own_output_is_a_fixed_point() {
        // A tab wearing the transform's own output can be dumped and
        // rebuilt again mid-session — a later toggle, or a remote election
        // (F4) retrofitting a tab that already carries another instance's
        // rail. The dump then shows exactly the E1/E2 costumed shape: a
        // genuine rail pane plus shells wearing rail-like borderless/size
        // dressing from earlier regenerations. Regenerating from that shape
        // must reproduce the same docked body — any accretion here
        // compounds on every toggle.
        let dump = r#"layout {
    tab name="Tab #3" focus=true hide_floating_panes=true {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        pane split_direction="vertical" {
            pane size=28 borderless=true name="sidebar" {
                plugin location="file:/tmp/zellij-sidebar.wasm" {
                    rail "1"
                }
            }
            pane size=28 borderless=true {
                pane cwd="/a" size="33%"
                pane borderless=true cwd="/b" size="33%"
                pane borderless=true cwd="/c" size="34%"
            }
        }
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
}
"#;
        let url = "file:/tmp/zellij-sidebar.wasm";
        let config = jit_config();
        let first = split_preserving_layout_kdl(dump, url, &config).unwrap();
        let docked1 = docked_tab_body(&first);

        // Exactly one rail slot, and only chrome/rail borderless, in the
        // first generation already.
        assert_eq!(docked1.iter().filter(|l| l.contains(url)).count(), 1);
        assert_eq!(
            docked1.iter().filter(|l| l.contains("borderless")).count(),
            3 // tab-bar + rail + status-bar
        );

        let redump = format!("layout {{\n tab {{\n{}\n }}\n}}\n", docked1.join("\n"));
        let second = split_preserving_layout_kdl(&redump, url, &config).unwrap();
        let docked2 = docked_tab_body(&second);

        assert_eq!(docked2.iter().filter(|l| l.contains(url)).count(), 1);
        assert_eq!(
            docked2.iter().filter(|l| l.contains("borderless")).count(),
            3
        );
        assert_eq!(normalize(&docked1), normalize(&docked2));
    }

    #[test]
    fn chromeless_tabs_regenerate_without_inventing_chrome() {
        // Swaps must carry exactly the chrome the tab actually has — adding
        // chrome the tab lacks leaves the swap unfittable (swaps never
        // spawn) and wedges the tab.
        let dump = r#"layout {
    tab name="bare" {
        pane cwd="/only" size="50%"
        pane cwd="/other" size="50%"
    }
}
"#;
        let kdl = split_preserving_layout_kdl(dump, "file:/x.wasm", &jit_config()).unwrap();
        assert!(!kdl.contains("zellij:"));
        assert_eq!(kdl.matches("cwd=\"/only\"").count(), 2);
    }

    #[test]
    fn stray_rail_pane_in_the_region_is_dropped_not_reemitted() {
        // A prior retrofit's rail can ride along in the dump as a genuine
        // plugin pane (not just borderless dressing) — e.g. seated in a
        // sibling's slot after an absorb. rail_pane_kdl always contributes
        // the canonical slot, so any copy found in the region must be
        // dropped outright, not folded in as a second sidebar pane.
        let dump = r#"layout {
    tab name="Tab #1" {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        pane split_direction="vertical" {
            pane size=28 borderless=true name="sidebar" {
                plugin location="file:/tmp/zellij-sidebar.wasm" {
                    rail "1"
                }
            }
            pane cwd="/a" size="72%"
        }
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
}
"#;
        let kdl = split_preserving_layout_kdl(dump, "file:/tmp/zellij-sidebar.wasm", &jit_config())
            .unwrap();
        // Exactly one rail slot per tab body (docked, undocked, base) —
        // contributed solely by rail_pane_kdl.
        assert_eq!(
            kdl.matches("plugin location=\"file:/tmp/zellij-sidebar.wasm\"")
                .count(),
            3
        );
        assert_eq!(kdl.matches("rail \"1\"").count(), 3);
        assert_eq!(kdl.matches("cwd=\"/a\"").count(), 2);
    }

    #[test]
    fn stray_rail_is_recognized_defensively_by_location_substring() {
        // The exact-URL match cannot be the only net: a differently
        // configured or differently pathed copy of the sidebar plugin must
        // still be recognized and dropped.
        let dump = r#"layout {
    tab name="t" {
        pane split_direction="vertical" {
            pane size=28 borderless=true {
                plugin location="file:/other/path/zellij-sidebar.wasm" {
                    rail "2"
                }
            }
            pane cwd="/shell"
        }
    }
}
"#;
        let kdl = split_preserving_layout_kdl(dump, "file:/tmp/zellij-sidebar.wasm", &jit_config())
            .unwrap();
        assert!(!kdl.contains("/other/path/zellij-sidebar.wasm"));
        assert!(!kdl.contains("rail \"2\""));
        assert_eq!(
            kdl.matches("plugin location=\"file:/tmp/zellij-sidebar.wasm\"")
                .count(),
            3
        );
        assert_eq!(kdl.matches("cwd=\"/shell\"").count(), 2);
    }

    #[test]
    fn third_party_plugin_pane_survives_extraction() {
        // A user's non-builtin, non-sidebar plugin pane must ride along into
        // both swaps: dropping it would delete the pane from the swap story
        // (swaps never spawn) and orphan it on application.
        let dump = r#"layout {
    tab name="t" {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        pane cwd="/shell" size="60%"
        pane size="40%" {
            plugin location="file:/other/statusdeck.wasm"
        }
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
}
"#;
        let kdl = split_preserving_layout_kdl(dump, "file:/tmp/zellij-sidebar.wasm", &jit_config())
            .unwrap();
        assert_eq!(
            kdl.matches("plugin location=\"file:/other/statusdeck.wasm\"")
                .count(),
            2
        );
        assert_eq!(kdl.matches("cwd=\"/shell\"").count(), 2);
    }

    #[test]
    fn borderless_costume_is_stripped_from_region_panes() {
        // Observed live: repeated toggles left ordinary shells and
        // containers costumed with borderless=true from a stale rail slot.
        // User panes are never borderless; the only rows that legitimately
        // carry it are chrome and the rail, and those are regenerated from
        // scratch regardless of what the dump says. Sizes are left intact:
        // guessing them wrong corrupts real layouts, while a stray fixed
        // size is merely ugly and user-correctable.
        let dump = r#"layout {
    tab name="Tab #3" {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        pane cwd="/a" size="50%" borderless=true
        pane size="50%" borderless=true {
            pane cwd="/b" size="60%"
            pane cwd="/c" size="40%" borderless=true
        }
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
}
"#;
        let kdl = split_preserving_layout_kdl(dump, "file:/x.wasm", &jit_config()).unwrap();
        // Sizes survive untouched at every level; only borderless is gone.
        assert_eq!(kdl.matches("cwd=\"/a\" size=\"50%\"").count(), 2);
        assert_eq!(kdl.matches("pane size=\"50%\" {").count(), 2);
        assert_eq!(kdl.matches("cwd=\"/b\" size=\"60%\"").count(), 2);
        assert_eq!(kdl.matches("cwd=\"/c\" size=\"40%\"").count(), 2);
        // Only the canonical chrome (2 rows) and rail (1 row) per tab body
        // carry borderless — none of it belongs to the region.
        assert_eq!(kdl.matches("borderless=true").count(), 9);
    }

    #[test]
    fn quoted_strings_do_not_confuse_structure_or_attribute_stripping() {
        // Titles and cwds may contain braces and attr-lookalike text; only
        // real properties outside strings may be touched.
        let dump = r#"layout {
    tab name="Tab {5}" focus=true {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        pane name="build {debug}" cwd="/tmp/name=weird{dir" focus=true size="70%"
        pane cwd="/plain" size="28%"
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
}
"#;
        let kdl = split_preserving_layout_kdl(dump, "file:/x.wasm", &jit_config()).unwrap();
        assert!(!kdl.contains("build {debug}"));
        assert!(!kdl.contains("focus=true"));
        assert_eq!(kdl.matches("cwd=\"/tmp/name=weird{dir\"").count(), 2);
        assert_eq!(kdl.matches("cwd=\"/plain\"").count(), 2);
        // Structure survived: three tab bodies, all balanced.
        assert_eq!(kdl.matches("zellij:status-bar").count(), 3);
        assert_eq!(
            kdl.matches('{').count(),
            kdl.matches('}').count() + kdl.matches("weird{dir").count()
        );
    }

    #[test]
    fn retrofit_layout_carries_swap_set_chrome_and_config_identity() {
        let mut config = BTreeMap::new();
        config.insert("rail".to_owned(), "1".to_owned());
        let kdl = retrofit_layout_kdl("file:/tmp/zellij-sidebar.wasm", &config);
        assert!(kdl.contains("swap_tiled_layout name=\"docked\""));
        assert!(kdl.contains("swap_tiled_layout name=\"undocked\""));
        // docked swap + base tab reserve the full slot; undocked is a sliver
        assert_eq!(
            kdl.matches("pane size=28 borderless=true name=\"sidebar\"")
                .count(),
            2
        );
        assert_eq!(
            kdl.matches("pane size=1 borderless=true name=\"sidebar\"")
                .count(),
            1
        );
        // every sidebar block carries the plugin URL and its config identity
        assert_eq!(
            kdl.matches("plugin location=\"file:/tmp/zellij-sidebar.wasm\"")
                .count(),
            3
        );
        assert_eq!(kdl.matches("rail \"1\"").count(), 3);
        assert!(kdl.contains("zellij:tab-bar"));
        assert!(kdl.contains("zellij:status-bar"));
        assert!(kdl.contains("pane stacked=true"));
        assert!(kdl.contains("children"));
        // The tab node stays unnamed so the user's tab name survives the override.
        assert!(!kdl.contains("tab name="));
    }
}
