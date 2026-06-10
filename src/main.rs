// ABOUTME: Clickable pane-switcher sidebar for zellij — lists panes in its own tab with their
// ABOUTME: last terminal line; Alt-/ summons it as a floating left rail in the active tab.

use std::collections::BTreeMap;
use zellij_tile::prelude::*;

const STATUS_POLL_SECS: f64 = 2.0;
const TARGET_COLS: usize = 28;
const MAX_DOCK_STEPS: u8 = 10;
const RAIL_WIDTH: usize = 30;

#[derive(Default)]
struct Sidebar {
    rows: Vec<Row>,
    plugin_id: u32,
    dock_steps: u8,
    docked: bool,
    hidden: bool,
    rendered_once: bool,
    permissions_requested: bool,
    rail_positioned: bool,
    own_tab: Option<usize>,
    own_floating: bool,
    active_tab: Option<usize>,
    instances: Vec<(u32, usize)>, // (plugin pane id, tab position) of every sidebar instance
}

#[derive(Debug, Clone, PartialEq, Default)]
struct Row {
    pane_id: u32,
    title: String,
    focused: bool,
    agent: bool,
    busy: bool,
    status: String,
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
    BringToActive(usize),
    Ignore,
}

register_plugin!(Sidebar);

impl ZellijPlugin for Sidebar {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        self.plugin_id = get_plugin_ids().plugin_id;
        subscribe(&[
            EventType::PaneUpdate,
            EventType::TabUpdate,
            EventType::Mouse,
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
                if status == PermissionStatus::Granted {
                    set_selectable(false);
                }
                true
            },
            Event::TabUpdate(tabs) => {
                self.active_tab = tabs.iter().find(|t| t.active).map(|t| t.position);
                false
            },
            Event::PaneUpdate(manifest) => {
                self.own_tab = own_tab_position(&manifest, self.plugin_id);
                self.instances = sidebar_instances(&manifest);
                self.own_floating = manifest
                    .panes
                    .values()
                    .flatten()
                    .find(|p| p.is_plugin && p.id == self.plugin_id)
                    .map(|p| p.is_floating)
                    .unwrap_or(false);
                let old = std::mem::take(&mut self.rows);
                self.rows = rows_for_own_tab(&manifest, self.plugin_id);
                for row in self.rows.iter_mut() {
                    if let Some(prev) = old.iter().find(|r| r.pane_id == row.pane_id) {
                        row.status = prev.status.clone();
                        row.busy = prev.busy;
                    }
                }
                true
            },
            Event::Timer(_) => {
                self.refresh_statuses();
                set_timeout(STATUS_POLL_SECS);
                true
            },
            Event::Mouse(Mouse::LeftClick(line, _col)) => {
                self.handle_click(line);
                false
            },
            _ => false,
        }
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        if pipe_message.name != "toggle" {
            return false;
        }
        // A pipe that launched us arrives before the first render. If we were
        // launched into a hidden floating layer (hide_floating_panes tabs), we
        // never render and would stay invisible forever — summon explicitly.
        if !self.rendered_once {
            let _ = show_floating_panes(None);
            show_self(true);
            self.float_as_rail();
            self.hidden = false;
            return false;
        }
        match decide_toggle(
            self.hidden,
            self.own_tab,
            self.active_tab,
            self.plugin_id,
            &self.instances,
        ) {
            ToggleAction::HideSelf => {
                self.hidden = true;
                hide_self();
            },
            ToggleAction::ShowHere => {
                self.hidden = false;
                if self.own_floating {
                    let _ = show_floating_panes(None);
                }
                show_self(self.own_floating);
            },
            ToggleAction::BringToActive(tab) => {
                if self.hidden {
                    show_self(true);
                    self.hidden = false;
                }
                break_panes_to_tab_with_index(&[PaneId::Plugin(self.plugin_id)], tab, false);
                let _ = show_floating_panes(None);
                self.float_as_rail();
                self.own_tab = Some(tab);
            },
            ToggleAction::Ignore => {},
        }
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
        // A floating instance (fresh keybind launch spawns floating, centered)
        // snaps itself to the left rail once.
        if self.own_floating && !self.rail_positioned {
            self.rail_positioned = true;
            self.float_as_rail();
        }
        self.dock(cols);
        println!(
            "\u{1b}[7m▾ PANES{}\u{1b}[0m",
            " ".repeat(cols.saturating_sub(7))
        );
        for row in &self.rows {
            let title: String = row.title.chars().take(cols.saturating_sub(2)).collect();
            let mark = if row.focused {
                "\u{1b}[33m●\u{1b}[0m " // focused: yellow
            } else if row.busy {
                "\u{1b}[31m●\u{1b}[0m " // agent working: red
            } else {
                "  "
            };
            if row.agent {
                println!("{}\u{1b}[36m{}\u{1b}[0m", mark, title);
            } else if row.focused {
                println!("{}\u{1b}[1m{}\u{1b}[0m", mark, title);
            } else {
                println!("{}{}", mark, title);
            }
            let status: String = row.status.chars().take(cols.saturating_sub(4)).collect();
            println!("    \u{1b}[2m{}\u{1b}[0m", status);
        }
    }
}

impl Sidebar {
    fn float_as_rail(&self) {
        let own = PaneId::Plugin(self.plugin_id);
        float_multiple_panes(vec![own]);
        let mut coords = FloatingPaneCoordinates::default()
            .with_x_fixed(0)
            .with_y_fixed(1)
            .with_width_fixed(RAIL_WIDTH)
            .with_height_percent(90);
        coords.pinned = Some(true);
        change_floating_panes_coordinates(vec![(own, coords)]);
    }

    // Tiled instances only: re-shrink toward the target width whenever layout
    // reflows (new panes, swap layouts) inflate us. Floating rails keep their
    // coordinates and skip this.
    fn dock(&mut self, cols: usize) {
        if self.own_floating {
            return;
        }
        if cols > TARGET_COLS + 6 {
            self.docked = false;
        }
        if self.docked {
            return;
        }
        if cols <= TARGET_COLS + 4 {
            self.docked = true;
            self.dock_steps = 0;
            return;
        }
        if self.dock_steps < MAX_DOCK_STEPS {
            self.dock_steps += 1;
            resize_pane_with_id(
                ResizeStrategy::new(Resize::Decrease, Some(Direction::Right)),
                PaneId::Plugin(self.plugin_id),
            );
        } else {
            self.docked = true; // give up until the next successful dock resets steps
        }
    }

    fn handle_click(&mut self, line: isize) {
        match target_for_line(line, self.rows.len()) {
            LineTarget::Header => {
                self.hidden = true;
                hide_self();
            },
            LineTarget::Row(idx) => {
                if let Some(row) = self.rows.get(idx) {
                    focus_terminal_pane(row.pane_id, false, false);
                }
            },
            LineTarget::None => {},
        }
    }

    fn refresh_statuses(&mut self) {
        for row in self.rows.iter_mut() {
            if let Ok(contents) = get_pane_scrollback(PaneId::Terminal(row.pane_id), false) {
                row.status = last_meaningful_line(&contents.viewport);
                row.busy = contents
                    .viewport
                    .iter()
                    .any(|l| l.contains("esc to interrupt"));
            }
        }
    }
}

// The pipe broadcasts to every instance; exactly one may act on it.
// - The active tab's own instance toggles in place.
// - Otherwise, if the active tab has no instance, the leader (lowest pane id)
//   teleports there as a floating rail.
fn decide_toggle(
    hidden: bool,
    own_tab: Option<usize>,
    active_tab: Option<usize>,
    own_pane_id: u32,
    instances: &[(u32, usize)],
) -> ToggleAction {
    let Some(active) = active_tab else {
        return ToggleAction::Ignore;
    };
    if own_tab == Some(active) {
        if hidden {
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
        ToggleAction::BringToActive(active)
    } else {
        ToggleAction::Ignore
    }
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
                            .map_or(false, |u| u.contains("zellij-sidebar"))
                })
                .map(move |p| (p.id, *tab))
        })
        .collect();
    instances.sort_unstable();
    instances
}

// Each pane occupies two display lines (title + status) below the header.
fn target_for_line(line: isize, row_count: usize) -> LineTarget {
    if line == 0 {
        return LineTarget::Header;
    }
    if line < 0 {
        return LineTarget::None;
    }
    let idx = (line as usize - 1) / 2;
    if idx < row_count {
        LineTarget::Row(idx)
    } else {
        LineTarget::None
    }
}

fn last_meaningful_line(viewport: &[String]) -> String {
    viewport
        .iter()
        .rev()
        .map(|l| l.trim())
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .to_owned()
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
// excluding suppressed/unselectable panes. Agent panes are those whose title
// carries the ✳ marker set by Claude/codex.
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
            agent: p.title.contains('✳'),
            ..Default::default()
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
    }

    #[test]
    fn marks_agent_panes_by_title_marker() {
        let m = manifest(vec![(
            0,
            vec![
                pane(7, true, "sidebar", 0, false),
                pane(1, false, "✳ Claude Code", 1, false),
                pane(2, false, "shell", 2, false),
            ],
        )]);
        let rows = rows_for_own_tab(&m, 7);
        assert!(rows[0].agent);
        assert!(!rows[1].agent);
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
        assert_eq!(target_for_line(0, 2), LineTarget::Header);
        assert_eq!(target_for_line(1, 2), LineTarget::Row(0));
        assert_eq!(target_for_line(2, 2), LineTarget::Row(0));
        assert_eq!(target_for_line(3, 2), LineTarget::Row(1));
        assert_eq!(target_for_line(4, 2), LineTarget::Row(1));
        assert_eq!(target_for_line(5, 2), LineTarget::None);
        assert_eq!(target_for_line(-3, 2), LineTarget::None);
    }

    #[test]
    fn last_meaningful_line_skips_trailing_blanks() {
        let viewport = vec![
            "first".to_owned(),
            "> do the thing".to_owned(),
            "   ".to_owned(),
            "".to_owned(),
        ];
        assert_eq!(last_meaningful_line(&viewport), "> do the thing");
        assert_eq!(last_meaningful_line(&[]), "");
    }

    #[test]
    fn toggle_in_own_active_tab_hides_or_shows() {
        let instances = vec![(7, 1)];
        assert_eq!(
            decide_toggle(false, Some(1), Some(1), 7, &instances),
            ToggleAction::HideSelf
        );
        assert_eq!(
            decide_toggle(true, Some(1), Some(1), 7, &instances),
            ToggleAction::ShowHere
        );
    }

    #[test]
    fn solo_instance_follows_to_active_tab() {
        let instances = vec![(7, 1)];
        assert_eq!(
            decide_toggle(false, Some(1), Some(3), 7, &instances),
            ToggleAction::BringToActive(3)
        );
    }

    #[test]
    fn defers_to_instance_already_in_active_tab() {
        let instances = vec![(7, 1), (9, 3)];
        assert_eq!(
            decide_toggle(false, Some(1), Some(3), 7, &instances),
            ToggleAction::Ignore
        );
    }

    #[test]
    fn only_leader_teleports_when_active_tab_is_empty() {
        let instances = vec![(7, 1), (9, 2)];
        // leader (7) goes; follower (9) ignores
        assert_eq!(
            decide_toggle(false, Some(1), Some(5), 7, &instances),
            ToggleAction::BringToActive(5)
        );
        assert_eq!(
            decide_toggle(false, Some(2), Some(5), 9, &instances),
            ToggleAction::Ignore
        );
    }
}
