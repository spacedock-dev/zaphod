// ABOUTME: Clickable pane-switcher sidebar for zellij — lists panes in its own tab with their
// ABOUTME: last terminal line; Alt-/ summons it as a floating left rail in the active tab.

use std::collections::BTreeMap;
use zellij_tile::prelude::*;

mod agent;

const STATUS_POLL_SECS: f64 = 2.0;
const RAIL_WIDTH: usize = 30;

#[derive(Default)]
struct Sidebar {
    rows: Vec<Row>,
    plugin_id: u32,
    hidden: bool,
    rendered_once: bool,
    permissions_requested: bool,
    rail_positioned: bool,
    rail_mode: bool, // sticky: once a floating rail, always re-show as one
    own_tab: Option<usize>,
    own_floating: bool,
    own_url: Option<String>,
    config: BTreeMap<String, String>,
    manifest_seen: bool,
    last_cols: usize,
    nav_mode: bool,
    nav_selected: usize,
    return_focus: Option<u32>,
    active_tab: Option<usize>,
    instances: Vec<(u32, usize)>, // (plugin pane id, tab position) of every sidebar instance
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

#[derive(Debug, Clone, Copy, PartialEq)]
enum ToggleAction {
    HideSelf,
    ShowHere,
    SpawnInActive,
    SwapLayout,
    Ignore,
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
                false
            }
            Event::PaneUpdate(manifest) => {
                self.manifest_seen = true;
                self.own_tab = own_tab_position(&manifest, self.plugin_id);
                self.instances = sidebar_instances(&manifest);
                if let Some(own) = manifest
                    .panes
                    .values()
                    .flatten()
                    .find(|p| p.is_plugin && p.id == self.plugin_id)
                {
                    self.own_floating = own.is_floating;
                    self.own_url = own.plugin_url.clone();
                    // Ground truth beats our flag: drift here caused phantom
                    // show/hide cycles.
                    self.hidden = own.is_suppressed;
                    // If focus ever lands on us outside nav mode (launch,
                    // spawn, show_self), hand it back.
                    if own.is_focused && !self.nav_mode {
                        focus_previous_pane();
                    }
                }
                let old = std::mem::take(&mut self.rows);
                self.rows = rows_for_own_tab(&manifest, self.plugin_id);
                preserve_agent_fields(&mut self.rows, &old);
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
            Event::Mouse(Mouse::LeftClick(line, col)) => {
                self.handle_click(line, col);
                false
            }
            _ => false,
        }
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        // CLI pipes stay blocked until explicitly released; release immediately
        // so `zellij pipe` callers terminate instead of wedging the pipe bus.
        if let PipeSource::Cli(pipe_id) = &pipe_message.source {
            unblock_cli_pipe_input(pipe_id);
        }
        if pipe_message.name == "navigate" {
            if !self.rendered_once {
                // Launching pipe: just summon; nav needs a settled pane.
                show_self(true);
                self.float_as_rail();
                self.hidden = false;
                return true;
            }
            self.ensure_visible_in_active_tab();
            self.enter_nav();
            return true;
        }
        if pipe_message.name != "toggle" {
            return false;
        }
        // A pipe that launched us arrives before the first render. If we were
        // launched into a hidden floating layer (hide_floating_panes tabs), we
        // never render and would stay invisible forever — summon explicitly.
        // NOTE: no blocking shim calls in here (show_floating_panes etc. wait
        // for a server response and deadlock the launch); the pinned rail is
        // visible even while the floating layer is hidden.
        if !self.rendered_once {
            show_self(true);
            self.float_as_rail();
            self.hidden = false;
            return true;
        }
        let action = decide_toggle(
            self.hidden,
            self.own_floating,
            self.own_tab,
            self.current_active_tab(),
            self.plugin_id,
            &self.instances,
        );
        match action {
            ToggleAction::SwapLayout => {
                // A docked tile lives in a swap-layout tab: flip docked <-> undocked
                // by rearranging the tab's existing panes, no hide/show needed.
                next_swap_layout();
            }
            ToggleAction::HideSelf => {
                self.hidden = true;
                hide_self();
            }
            ToggleAction::ShowHere => {
                self.hidden = false;
                if self.rail_mode {
                    // While hidden the manifest reports is_floating=false, so
                    // never trust it here: a rail always re-shows as a rail.
                    show_self(true);
                    self.float_as_rail();
                } else {
                    show_self(false);
                }
            }
            ToggleAction::SpawnInActive => {
                // No cross-tab moves (show_self/break would yank the user's
                // view to our tab): the leader spawns a sibling instance
                // directly in the active tab instead.
                if let Some(url) = self.own_url.clone() {
                    open_plugin_pane_floating(
                        &url,
                        self.config.clone(),
                        Some(rail_coordinates()),
                        BTreeMap::new(),
                    );
                }
            }
            ToggleAction::Ignore => {}
        }
        toggle_action_requests_render(action)
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
        // A floating instance (fresh keybind launch spawns floating, centered)
        // snaps itself to the left rail once.
        if self.own_floating && !self.rail_positioned {
            self.rail_positioned = true;
            self.float_as_rail();
        }
        self.last_cols = cols;
        // Header: click body to hide, click the ⇄ at the right edge to toggle
        // the docked <-> undocked swap layout.
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
    fn float_as_rail(&mut self) {
        self.rail_mode = true;
        let own = PaneId::Plugin(self.plugin_id);
        // Only force the float transition when the manifest confirms we are
        // tiled: ripping a tiled pane out triggers auto-layout reflows (which
        // restack the user's tabs), and keybind launches float from birth.
        if self.manifest_seen && !self.own_floating {
            float_multiple_panes(vec![own]);
        }
        change_floating_panes_coordinates(vec![(own, rail_coordinates())]);
    }

    // Same show paths as the toggle, minus the hide arm.
    fn ensure_visible_in_active_tab(&mut self) {
        let active_tab = self.current_active_tab();
        match decide_toggle(
            self.hidden,
            self.own_floating,
            self.own_tab,
            active_tab,
            self.plugin_id,
            &self.instances,
        ) {
            // A docked tile is already visible; nav just needs it in place.
            ToggleAction::SwapLayout => {}
            ToggleAction::ShowHere => {
                self.hidden = false;
                if self.rail_mode {
                    show_self(true);
                    self.float_as_rail();
                } else {
                    show_self(false);
                }
            }
            ToggleAction::SpawnInActive => {
                if let Some(url) = self.own_url.clone() {
                    open_plugin_pane_floating(
                        &url,
                        self.config.clone(),
                        Some(rail_coordinates()),
                        BTreeMap::new(),
                    );
                }
            }
            ToggleAction::HideSelf | ToggleAction::Ignore => {}
        }
    }

    fn current_active_tab(&mut self) -> Option<usize> {
        let active_tab = active_tab_for_decision(self.active_tab, get_focused_pane_info());
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

    fn handle_click(&mut self, line: isize, col: usize) {
        match target_for_line(line, self.rows.len()) {
            LineTarget::Header => {
                if header_dock_toggle_hit(col, self.last_cols) {
                    // ⇄ toggles the docked <-> undocked swap layout, same as Alt-/.
                    next_swap_layout();
                } else {
                    self.hidden = true;
                    hide_self();
                }
            }
            LineTarget::Row(idx) => {
                if let Some(row) = self.rows.get(idx) {
                    focus_terminal_pane(row.pane_id, false, false);
                }
            }
            LineTarget::None => {}
        }
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

// The pipe broadcasts to every instance; exactly one may act on it.
// - The active tab's own instance toggles in place.
// - Otherwise, if the active tab has no instance, the leader (lowest pane id)
//   teleports there as a floating rail.
fn decide_toggle(
    hidden: bool,
    own_floating: bool,
    own_tab: Option<usize>,
    active_tab: Option<usize>,
    own_pane_id: u32,
    instances: &[(u32, usize)],
) -> ToggleAction {
    let Some(active) = active_tab else {
        return ToggleAction::Ignore;
    };
    if own_tab == Some(active) {
        if !own_floating {
            // A docked tile toggles the tab's swap layout (docked <-> undocked);
            // only a floating rail hides/shows itself.
            ToggleAction::SwapLayout
        } else if hidden {
            ToggleAction::ShowHere
        } else {
            ToggleAction::HideSelf
        }
    } else if instances
        .iter()
        .any(|(id, tab)| *tab == active && *id != own_pane_id)
    {
        ToggleAction::Ignore
    } else if instances.iter().all(|(id, _)| *id >= own_pane_id) {
        ToggleAction::SpawnInActive
    } else {
        ToggleAction::Ignore
    }
}

fn toggle_action_requests_render(action: ToggleAction) -> bool {
    matches!(action, ToggleAction::ShowHere | ToggleAction::SpawnInActive)
}

fn active_tab_for_decision(
    cached_active_tab: Option<usize>,
    focused_pane_info: Result<(usize, PaneId), String>,
) -> Option<usize> {
    focused_pane_info
        .map(|(tab, _)| tab)
        .ok()
        .or(cached_active_tab)
}

fn sidebar_instances(manifest: &PaneManifest) -> Vec<(u32, usize)> {
    let mut instances: Vec<(u32, usize)> = manifest
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
                .map(move |p| (p.id, *tab))
        })
        .collect();
    instances.sort_unstable();
    instances
}

fn rail_coordinates() -> FloatingPaneCoordinates {
    let mut coords = FloatingPaneCoordinates::default()
        .with_x_fixed(0)
        .with_y_fixed(1)
        .with_width_fixed(RAIL_WIDTH)
        .with_height_percent(97);
    coords.pinned = Some(true);
    coords
}

fn move_selection(current: usize, delta: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (current as isize + delta).clamp(0, len as isize - 1) as usize
}

// The ⇄ control occupies the right edge of the header line.
fn header_dock_toggle_hit(col: usize, total_cols: usize) -> bool {
    total_cols > 4 && col >= total_cols.saturating_sub(4)
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
    fn dock_toggle_hit_zone_is_right_edge_of_header() {
        assert!(header_dock_toggle_hit(26, 30));
        assert!(header_dock_toggle_hit(29, 30));
        assert!(!header_dock_toggle_hit(25, 30));
        assert!(!header_dock_toggle_hit(0, 30));
        assert!(!header_dock_toggle_hit(3, 4)); // degenerate width: never hit
    }

    #[test]
    fn floating_rail_in_own_active_tab_hides_or_shows() {
        let instances = vec![(7, 1)];
        assert_eq!(
            decide_toggle(false, true, Some(1), Some(1), 7, &instances),
            ToggleAction::HideSelf
        );
        assert_eq!(
            decide_toggle(true, true, Some(1), Some(1), 7, &instances),
            ToggleAction::ShowHere
        );
    }

    #[test]
    fn docked_tile_in_own_active_tab_toggles_swap_layout() {
        let instances = vec![(7, 1)];
        // own_floating=false marks the docked layout instance: toggling flips
        // the swap layout instead of hiding the pane.
        assert_eq!(
            decide_toggle(false, false, Some(1), Some(1), 7, &instances),
            ToggleAction::SwapLayout
        );
    }

    #[test]
    fn showing_a_hidden_instance_requests_render() {
        assert!(toggle_action_requests_render(ToggleAction::ShowHere));
        assert!(toggle_action_requests_render(ToggleAction::SpawnInActive));
        assert!(!toggle_action_requests_render(ToggleAction::HideSelf));
        assert!(!toggle_action_requests_render(ToggleAction::SwapLayout));
        assert!(!toggle_action_requests_render(ToggleAction::Ignore));
    }

    #[test]
    fn live_focused_tab_overrides_stale_cached_active_tab() {
        assert_eq!(
            active_tab_for_decision(Some(1), Ok((3, PaneId::Terminal(9)))),
            Some(3)
        );
        assert_eq!(
            active_tab_for_decision(Some(1), Err("unavailable".to_owned())),
            Some(1)
        );
    }

    #[test]
    fn solo_instance_spawns_sibling_in_active_tab() {
        let instances = vec![(7, 1)];
        assert_eq!(
            decide_toggle(false, true, Some(1), Some(3), 7, &instances),
            ToggleAction::SpawnInActive
        );
    }

    #[test]
    fn defers_to_instance_already_in_active_tab() {
        let instances = vec![(7, 1), (9, 3)];
        assert_eq!(
            decide_toggle(false, true, Some(1), Some(3), 7, &instances),
            ToggleAction::Ignore
        );
    }

    #[test]
    fn only_leader_spawns_when_active_tab_is_empty() {
        let instances = vec![(7, 1), (9, 2)];
        // leader (7) spawns; follower (9) ignores
        assert_eq!(
            decide_toggle(false, true, Some(1), Some(5), 7, &instances),
            ToggleAction::SpawnInActive
        );
        assert_eq!(
            decide_toggle(false, true, Some(2), Some(5), 9, &instances),
            ToggleAction::Ignore
        );
    }
}
