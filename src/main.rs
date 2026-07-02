// ABOUTME: Clickable pane-switcher sidebar for zellij — lists panes in its own tab with their
// ABOUTME: last terminal line; Alt-/ flips it between a docked rail and a 1-col sliver.

use std::collections::BTreeMap;
use zellij_tile::prelude::*;

mod agent;

const STATUS_POLL_SECS: f64 = 2.0;
// Sidebar widths in the two swap-layout states (mirrors layouts/zaphod.kdl).
const DOCKED_COLS: usize = 28;
const UNDOCKED_COLS: usize = 1;

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
    // Per-tab is_swap_layout_dirty: a damaged tab needs two next_swap_layout
    // calls to advance. Caveat: zellij reports (None, false) for tabs with at
    // most one selectable tiled pane, so damage there is invisible.
    tab_swap_dirty: BTreeMap<usize, bool>,
    own_floating: bool,
    instances: Vec<SidebarInstance>,
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

#[derive(Debug, Clone, Copy, PartialEq)]
enum ToggleAction {
    SwapLayout { calls: u8 },
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
                self.tab_swap_dirty = tabs
                    .iter()
                    .map(|t| (t.position, t.is_swap_layout_dirty))
                    .collect();
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

    fn handle_click(&mut self, line: isize) {
        match decide_click(line, &self.rows) {
            ClickAction::ToggleDock => self.perform_toggle(),
            ClickAction::FocusPane(id) => focus_terminal_pane(id, false, false),
            ClickAction::None => {}
        }
    }

    fn perform_toggle(&mut self) {
        let active_tab = self.current_active_tab();
        let active_swap_dirty = active_tab
            .and_then(|tab| self.tab_swap_dirty.get(&tab).copied())
            .unwrap_or(false);
        match decide_toggle(
            self.own_tab,
            self.own_floating,
            active_tab,
            active_swap_dirty,
            self.plugin_id,
            &self.instances,
        ) {
            ToggleAction::SwapLayout { calls } => {
                // Flip docked <-> sliver by rearranging the tab's existing
                // panes; the plugin pane itself never hides or moves. Zellij
                // inserts every tab's birth layout as swap position 0, named
                // BASE and constrained to the exact birth pane count, so the
                // first press on a tab can be visually silent: it steps
                // BASE -> docked, which are geometrically identical.
                for _ in 0..calls {
                    next_swap_layout();
                }
            }
            ToggleAction::Retrofit => {
                // A tab without a docked sidebar gets its layout replaced
                // once, docking the sidebar and installing the swap set;
                // every later toggle there is a pure swap cycle.
                if let Some(url) = self.own_url.clone() {
                    override_layout(
                        LayoutInfo::Stringified(retrofit_layout_kdl(&url, &self.config)),
                        true, // retain existing terminal panes
                        true, // retain existing plugin panes
                        true, // apply only to the active tab
                        BTreeMap::new(),
                    );
                }
            }
            ToggleAction::Ignore => {}
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

// The pipe broadcasts to every config-matched instance; exactly one may act.
// - The active tab's tiled resident cycles the tab's swap layout — with two
//   next_swap_layout calls when the tab is damaged (manual split/resize):
//   zellij's first call then only re-applies the current template.
// - A floating resident (keybind bootstrap) has no swap set to cycle: it
//   retrofits its own tab, docking itself and installing the swap set.
// - A tab with no sidebar gets a one-time override_layout retrofit; the
//   lowest-pane-id instance is the single actor so the override runs once.
fn decide_toggle(
    own_tab: Option<usize>,
    own_floating: bool,
    active_tab: Option<usize>,
    active_swap_dirty: bool,
    own_pane_id: u32,
    instances: &[SidebarInstance],
) -> ToggleAction {
    let Some(active) = active_tab else {
        return ToggleAction::Ignore;
    };
    if own_tab == Some(active) && !own_floating {
        ToggleAction::SwapLayout {
            calls: if active_swap_dirty { 2 } else { 1 },
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
        ToggleAction::Retrofit
    } else {
        ToggleAction::Ignore
    }
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

// Handing focus back with nowhere to send it panics the server in the
// session-birth window (get_active_pane_id unwraps None); only bounce when
// the manifest shows another selectable pane in our tab.
fn should_hand_back_focus(own_focused: bool, nav_mode: bool, has_focus_target: bool) -> bool {
    own_focused && !nav_mode && has_focus_target
}

// One-time override for a tab without a sidebar: a full replacement layout
// that docks the sidebar and installs the docked/undocked swap set. It must
// carry the tab-bar/status-bar chrome and a stacked main that absorbs the
// existing panes — override_layout replaces the whole tab, and anything the
// KDL omits is dropped. The tab node stays unnamed so the user's tab name
// survives the override.
fn retrofit_layout_kdl(plugin_url: &str, config: &BTreeMap<String, String>) -> String {
    let config_lines: String = config
        .iter()
        .map(|(key, value)| format!("                {key} \"{value}\"\n"))
        .collect();
    let tab_body = |sidebar_cols: usize| {
        format!(
            r#"  tab {{
    pane size=1 borderless=true {{
        plugin location="zellij:tab-bar"
    }}
    pane split_direction="vertical" {{
        pane size={sidebar_cols} borderless=true name="sidebar" {{
            plugin location="{plugin_url}" {{
{config_lines}            }}
        }}
        pane stacked=true {{
            children
        }}
    }}
    pane size=1 borderless=true {{
        plugin location="zellij:status-bar"
    }}
  }}
"#
        )
    };
    format!(
        "layout {{\n swap_tiled_layout name=\"docked\" {{\n{docked} }}\n swap_tiled_layout name=\"undocked\" {{\n{undocked} }}\n{base}}}\n",
        docked = tab_body(DOCKED_COLS),
        undocked = tab_body(UNDOCKED_COLS),
        base = tab_body(DOCKED_COLS),
    )
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
    fn resident_instance_in_active_tab_cycles_swap_layout() {
        let instances = [inst(7, 1, false)];
        assert_eq!(
            decide_toggle(Some(1), false, Some(1), false, 7, &instances),
            ToggleAction::SwapLayout { calls: 1 }
        );
    }

    #[test]
    fn dirty_tab_needs_two_swap_calls_to_actually_flip() {
        // On a damaged tab (manual split/resize) zellij's first
        // next_swap_layout only re-applies the current template without
        // advancing, so one press must issue two calls.
        let instances = [inst(7, 1, false)];
        assert_eq!(
            decide_toggle(Some(1), false, Some(1), true, 7, &instances),
            ToggleAction::SwapLayout { calls: 2 }
        );
    }

    #[test]
    fn defers_to_the_resident_instance_of_the_active_tab() {
        let instances = [inst(7, 1, false), inst(9, 3, false)];
        assert_eq!(
            decide_toggle(Some(1), false, Some(3), false, 7, &instances),
            ToggleAction::Ignore
        );
    }

    #[test]
    fn lowest_id_instance_retrofits_a_tab_without_a_sidebar() {
        let instances = [inst(7, 1, false), inst(9, 2, false)];
        assert_eq!(
            decide_toggle(Some(1), false, Some(5), false, 7, &instances),
            ToggleAction::Retrofit
        );
        assert_eq!(
            decide_toggle(Some(2), false, Some(5), false, 9, &instances),
            ToggleAction::Ignore
        );
    }

    #[test]
    fn ignores_toggle_when_active_tab_is_unknown() {
        assert_eq!(
            decide_toggle(Some(1), false, None, false, 7, &[inst(7, 1, false)]),
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
            decide_toggle(Some(1), true, Some(1), false, 7, &instances),
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
            decide_toggle(Some(1), true, Some(1), false, 7, &instances),
            ToggleAction::Ignore
        );
        assert_eq!(
            decide_toggle(Some(1), false, Some(1), false, 9, &instances),
            ToggleAction::SwapLayout { calls: 1 }
        );
    }

    #[test]
    fn lowest_id_floating_resident_is_the_single_retrofit_actor() {
        // Two floating sidebars in one tab (stray pipe launches) must not
        // both fire the override.
        let instances = [inst(7, 1, true), inst(9, 1, true)];
        assert_eq!(
            decide_toggle(Some(1), true, Some(1), false, 7, &instances),
            ToggleAction::Retrofit
        );
        assert_eq!(
            decide_toggle(Some(1), true, Some(1), false, 9, &instances),
            ToggleAction::Ignore
        );
    }

    #[test]
    fn floating_bystander_defers_to_any_active_tab_resident() {
        // Floating bootstrap instance parked in tab 2; the active tab's own
        // resident acts, whether tiled or floating.
        let instances = [inst(7, 2, true), inst(9, 1, false)];
        assert_eq!(
            decide_toggle(Some(2), true, Some(1), false, 7, &instances),
            ToggleAction::Ignore
        );
        let instances = [inst(7, 2, true), inst(9, 1, true)];
        assert_eq!(
            decide_toggle(Some(2), true, Some(1), false, 7, &instances),
            ToggleAction::Ignore
        );
    }

    #[test]
    fn floating_bystander_joins_the_election_for_a_sidebarless_tab() {
        // No sidebar in the active tab: the lowest-id instance session-wide
        // retrofits it, floating or not.
        let instances = [inst(7, 2, true), inst(9, 3, false)];
        assert_eq!(
            decide_toggle(Some(2), true, Some(5), false, 7, &instances),
            ToggleAction::Retrofit
        );
        assert_eq!(
            decide_toggle(Some(3), false, Some(5), false, 9, &instances),
            ToggleAction::Ignore
        );
    }

    #[test]
    fn tab_update_records_swap_dirty_state_per_tab() {
        let tab = |position: usize, active: bool, dirty: bool| TabInfo {
            position,
            active,
            is_swap_layout_dirty: dirty,
            ..Default::default()
        };
        let mut sidebar = Sidebar::default();
        sidebar.update(Event::TabUpdate(vec![
            tab(0, false, false),
            tab(1, true, true),
        ]));
        assert_eq!(sidebar.active_tab, Some(1));
        assert_eq!(sidebar.tab_swap_dirty.get(&0), Some(&false));
        assert_eq!(sidebar.tab_swap_dirty.get(&1), Some(&true));
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
