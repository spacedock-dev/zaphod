// ABOUTME: Clickable pane-switcher sidebar for zellij — lists panes in its own tab,
// ABOUTME: click a row to focus that pane, click the header to hide the sidebar.

use std::collections::BTreeMap;
use zellij_tile::prelude::*;

#[derive(Default)]
struct Sidebar {
    rows: Vec<Row>,
    plugin_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
struct Row {
    pane_id: u32,
    title: String,
    focused: bool,
    agent: bool,
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
            EventType::PermissionRequestResult,
        ]);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PermissionRequestResult(_) => true,
            Event::PaneUpdate(manifest) => {
                self.rows = rows_for_own_tab(&manifest, self.plugin_id);
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
            if row.focused {
                println!("\u{1b}[33m●\u{1b}[0m \u{1b}[1m{}\u{1b}[0m", title);
            } else if row.agent {
                println!("  \u{1b}[36m{}\u{1b}[0m", title);
            } else {
                println!("  {}", title);
            }
        }
    }
}

impl Sidebar {
    fn handle_click(&self, line: isize) {
        if line == 0 {
            hide_self();
        } else if line > 0 {
            if let Some(row) = self.rows.get((line - 1) as usize) {
                focus_terminal_pane(row.pane_id, false, false);
            }
        }
    }
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
}
