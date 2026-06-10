// ABOUTME: Clickable pane-switcher sidebar for zellij — lists panes in its own tab with their
// ABOUTME: last terminal line, click a row to focus that pane, click the header to hide.

use std::collections::BTreeMap;
use zellij_tile::prelude::*;

const STATUS_POLL_SECS: f64 = 2.0;

#[derive(Default)]
struct Sidebar {
    rows: Vec<Row>,
    plugin_id: u32,
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

register_plugin!(Sidebar);

impl ZellijPlugin for Sidebar {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        self.plugin_id = get_plugin_ids().plugin_id;
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);
        subscribe(&[
            EventType::PaneUpdate,
            EventType::Mouse,
            EventType::Timer,
            EventType::PermissionRequestResult,
        ]);
        // Never take focus: clicks are delivered to the plugin without focusing
        // it (same mechanism as the built-in tab-bar).
        set_selectable(false);
        set_timeout(STATUS_POLL_SECS);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PermissionRequestResult(_) => {
                set_selectable(false);
                true
            },
            Event::PaneUpdate(manifest) => {
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

    fn render(&mut self, _rows: usize, cols: usize) {
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
    fn handle_click(&self, line: isize) {
        match target_for_line(line, self.rows.len()) {
            LineTarget::Header => hide_self(),
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

// Rows for the tab this plugin lives in: terminal panes only, top-to-bottom,
// excluding suppressed/unselectable panes. Agent panes are those whose title
// carries the ✳ marker set by Claude/codex.
fn rows_for_own_tab(manifest: &PaneManifest, own_plugin_id: u32) -> Vec<Row> {
    let own_tab = manifest.panes.iter().find_map(|(tab, panes)| {
        panes
            .iter()
            .any(|p| p.is_plugin && p.id == own_plugin_id)
            .then_some(*tab)
    });
    let Some(tab) = own_tab else {
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
}
