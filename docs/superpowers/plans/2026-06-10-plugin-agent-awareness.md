# Plugin Agent Awareness Implementation Plan

> **For Claude:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a plugin-only agent awareness sidebar that identifies Claude, Codex, and Pi panes and renders herdr-like state rows without a helper daemon.

**Architecture:** Add a pure `src/agent.rs` module for command classification, viewport state detection, status extraction, and stale-state preservation. Keep zellij host calls in `src/main.rs`; the timer poll calls zellij APIs, hands results to the pure module, and renders the enriched rows.

**Tech Stack:** Rust 2021, `zellij-tile` 0.44.x, `wasm32-wasip1`, existing ANSI terminal rendering, pure Rust unit tests.

---

## Chunk 1: Plugin-Local Agent Awareness

### File Structure

- Create: `src/agent.rs`
  - Owns `AgentKind`, `AgentState`, `AgentFields`, command classification, viewport classification, status extraction, and `enrich_fields`.
  - Contains pure unit tests for detector behavior and stale-state preservation.
- Modify: `src/main.rs`
  - Imports `mod agent;`.
  - Replaces `agent: bool`, `busy: bool`, and loose status handling with `AgentFields`.
  - Calls `get_pane_running_command` and `get_pane_scrollback` from `refresh_statuses`.
  - Renders the two-line agent-aware rows.
  - Updates existing row-building and click-mapping tests.
- No changes: `Cargo.toml`, `SPEC.md`.
- Optional: `README.md` only if manual verification finds behavior that changes
  the documented feature list.

### Task 1: Add Pure Agent Identity Model

**Files:**
- Create: `src/agent.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write failing tests for command and title classification**

Add `mod agent;` near the top of `src/main.rs` so the new module is compiled.
Then create `src/agent.rs` with this initial module and tests:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKind {
    Claude,
    Codex,
    Pi,
    Unknown,
}

pub fn agent_from_command(_argv: &[String]) -> AgentKind {
    todo!("classify command basenames")
}

pub fn agent_from_title(_title: &str) -> AgentKind {
    todo!("classify title fallback")
}

pub fn agent_from_viewport(_viewport: &[String]) -> AgentKind {
    todo!("classify viewport fallback")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|part| (*part).to_owned()).collect()
    }

    #[test]
    fn classifies_command_basename_exactly() {
        assert_eq!(agent_from_command(&argv(&["/opt/bin/claude"])), AgentKind::Claude);
        assert_eq!(agent_from_command(&argv(&["claude-code"])), AgentKind::Claude);
        assert_eq!(agent_from_command(&argv(&["codex"])), AgentKind::Codex);
        assert_eq!(agent_from_command(&argv(&["/usr/local/bin/pi"])), AgentKind::Pi);
    }

    #[test]
    fn command_detection_does_not_false_positive_pi() {
        assert_eq!(agent_from_command(&argv(&["pip"])), AgentKind::Unknown);
        assert_eq!(agent_from_command(&argv(&["python"])), AgentKind::Unknown);
        assert_eq!(agent_from_command(&argv(&["/tmp/pi/codex-helper"])), AgentKind::Unknown);
        assert_eq!(agent_from_command(&argv(&["sh", "-lc", "pi"])), AgentKind::Unknown);
    }

    #[test]
    fn title_fallback_identifies_known_agents() {
        assert_eq!(agent_from_title("claude code"), AgentKind::Claude);
        assert_eq!(agent_from_title("Codex literature"), AgentKind::Codex);
        assert_eq!(agent_from_title("pi reviewer"), AgentKind::Pi);
    }

    #[test]
    fn title_marker_without_name_is_unknown() {
        assert_eq!(agent_from_title("\u{2733} helper"), AgentKind::Unknown);
    }

    #[test]
    fn viewport_fallback_identifies_known_agents() {
        assert_eq!(
            agent_from_viewport(&["Claude Code".to_owned(), "esc to interrupt".to_owned()]),
            AgentKind::Claude
        );
        assert_eq!(
            agent_from_viewport(&["Codex".to_owned(), "press enter to confirm".to_owned()]),
            AgentKind::Codex
        );
        assert_eq!(agent_from_viewport(&["Working...".to_owned()]), AgentKind::Pi);
    }
}
```

All later `src/agent.rs` test snippets in this plan go inside this same
`#[cfg(test)] mod tests` module so they can reuse `argv`.

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
rtk cargo test agent::tests
```

Expected: FAIL because the functions contain `todo!()`.

- [ ] **Step 3: Implement command, title, and viewport classification**

Implement in `src/agent.rs`:

```rust
fn normalized_basename(command: &str) -> String {
    let base = command
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(command)
        .to_ascii_lowercase();
    base.strip_suffix(".exe").unwrap_or(&base).to_owned()
}

fn contains_word(haystack: &str, needle: &str) -> bool {
    haystack
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .any(|part| part == needle)
}

pub fn agent_from_command(argv: &[String]) -> AgentKind {
    let Some(command) = argv.first() else {
        return AgentKind::Unknown;
    };
    match normalized_basename(command).as_str() {
        "claude" | "claude-code" => AgentKind::Claude,
        "codex" => AgentKind::Codex,
        "pi" => AgentKind::Pi,
        _ => AgentKind::Unknown,
    }
}

pub fn agent_from_title(title: &str) -> AgentKind {
    let lower = title.to_ascii_lowercase();
    if lower.contains("claude code") || contains_word(&lower, "claude") {
        AgentKind::Claude
    } else if contains_word(&lower, "codex") {
        AgentKind::Codex
    } else if contains_word(&lower, "pi") {
        AgentKind::Pi
    } else {
        AgentKind::Unknown
    }
}

pub fn agent_from_viewport(viewport: &[String]) -> AgentKind {
    let joined = viewport.join("\n").to_ascii_lowercase();
    if joined.contains("claude code") {
        AgentKind::Claude
    } else if joined.contains("codex") || joined.contains("press enter to confirm") {
        AgentKind::Codex
    } else if viewport.iter().any(|line| line.trim() == "Working...") {
        AgentKind::Pi
    } else {
        AgentKind::Unknown
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
rtk cargo test agent::tests
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add src/main.rs src/agent.rs
rtk git commit -m "Add agent identity detection"
```

### Task 2: Add Viewport State and Status Extraction

**Files:**
- Modify: `src/agent.rs`

- [ ] **Step 1: Write failing tests for viewport classification**

Add:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Blocked,
    Working,
    Done,
    Idle,
    Unknown,
}

pub struct ViewportDetection {
    pub state: AgentState,
    pub status: String,
}

pub fn detect_viewport(_agent: AgentKind, _viewport: &[String]) -> ViewportDetection {
    todo!("detect state and status")
}

#[test]
fn blocked_beats_working() {
    let viewport = vec![
        "esc to interrupt".to_owned(),
        "Allow command?".to_owned(),
        "press enter to confirm".to_owned(),
    ];
    let detection = detect_viewport(AgentKind::Codex, &viewport);
    assert_eq!(detection.state, AgentState::Blocked);
    assert_eq!(detection.status, "press enter to confirm");
}

#[test]
fn working_matches_interrupt_controls_and_pi_working_line() {
    let codex = vec!["Esc to cancel".to_owned()];
    assert_eq!(detect_viewport(AgentKind::Codex, &codex).state, AgentState::Working);

    let pi = vec!["Working...".to_owned()];
    assert_eq!(detect_viewport(AgentKind::Pi, &pi).state, AgentState::Working);
}

#[test]
fn known_agent_without_signal_is_idle() {
    let viewport = vec!["> ".to_owned()];
    assert_eq!(detect_viewport(AgentKind::Claude, &viewport).state, AgentState::Idle);
}

#[test]
fn unknown_agent_stays_unknown() {
    let viewport = vec!["plain shell".to_owned()];
    assert_eq!(detect_viewport(AgentKind::Unknown, &viewport).state, AgentState::Unknown);
}

#[test]
fn status_uses_last_non_empty_line() {
    let viewport = vec!["first".to_owned(), "  ".to_owned(), "last".to_owned(), "".to_owned()];
    assert_eq!(detect_viewport(AgentKind::Claude, &viewport).status, "last");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
rtk cargo test agent::tests
```

Expected: FAIL because detection is not implemented.

- [ ] **Step 3: Implement minimal detector helpers**

Implement case-insensitive matching:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewportDetection {
    pub state: AgentState,
    pub status: String,
}

fn last_non_empty_line(viewport: &[String]) -> String {
    viewport
        .iter()
        .rev()
        .map(|line| line.trim())
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .to_owned()
}

fn blocker_line(viewport: &[String]) -> Option<String> {
    viewport.iter().rev().find_map(|line| {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        let blocked = lower.contains("allow command")
            || lower.contains("enter to confirm")
            || lower.contains("press enter to confirm")
            || lower.contains("enter to submit answer")
            || lower.contains("enter to submit all")
            || lower.contains("[y/n]")
            || lower.contains("yes (y)")
            || lower.contains("submit answer");
        blocked.then(|| trimmed.to_owned())
    })
}

fn has_working_signal(agent: AgentKind, viewport: &[String]) -> bool {
    viewport.iter().any(|line| {
        let trimmed = line.trim();
        let lower = trimmed.to_ascii_lowercase();
        lower.contains("esc to interrupt")
            || lower.contains("esc to cancel")
            || lower.contains("esc to stop")
            || (agent == AgentKind::Pi && trimmed == "Working...")
    })
}

pub fn detect_viewport(agent: AgentKind, viewport: &[String]) -> ViewportDetection {
    let status = last_non_empty_line(viewport);
    if agent == AgentKind::Unknown {
        return ViewportDetection { state: AgentState::Unknown, status };
    }
    if let Some(status) = blocker_line(viewport) {
        return ViewportDetection { state: AgentState::Blocked, status };
    }
    if has_working_signal(agent, viewport) {
        return ViewportDetection { state: AgentState::Working, status };
    }
    ViewportDetection { state: AgentState::Idle, status }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
rtk cargo test agent::tests
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add src/agent.rs
rtk git commit -m "Add agent viewport state detection"
```

### Task 3: Add Pure Enrichment and Failure Preservation

**Files:**
- Modify: `src/agent.rs`

- [ ] **Step 1: Write failing tests for `AgentFields` enrichment**

Add:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentFields {
    pub kind: AgentKind,
    pub state: AgentState,
    pub status: String,
    pub running_command: Option<Vec<String>>,
}

impl Default for AgentFields {
    fn default() -> Self {
        Self {
            kind: AgentKind::Unknown,
            state: AgentState::Unknown,
            status: String::new(),
            running_command: None,
        }
    }
}

pub fn enrich_fields(
    _previous: &AgentFields,
    _title: &str,
    _command: Result<Vec<String>, String>,
    _viewport: Result<Vec<String>, String>,
) -> AgentFields {
    todo!("preserve stale fields and enrich successful polls")
}

#[test]
fn enriches_from_command_and_viewport() {
    let fields = enrich_fields(
        &AgentFields::default(),
        "ignored",
        Ok(argv(&["codex"])),
        Ok(vec!["Esc to cancel".to_owned()]),
    );
    assert_eq!(fields.kind, AgentKind::Codex);
    assert_eq!(fields.state, AgentState::Working);
    assert_eq!(fields.status, "Esc to cancel");
    assert_eq!(fields.running_command, Some(argv(&["codex"])));
}

#[test]
fn command_failure_preserves_command_but_uses_title_and_viewport_identity() {
    let previous = AgentFields {
        kind: AgentKind::Codex,
        running_command: Some(argv(&["codex"])),
        ..AgentFields::default()
    };
    let fields = enrich_fields(
        &previous,
        "pi reviewer",
        Err("denied".to_owned()),
        Ok(vec!["Working...".to_owned()]),
    );
    assert_eq!(fields.kind, AgentKind::Pi);
    assert_eq!(fields.state, AgentState::Working);
    assert_eq!(fields.running_command, Some(argv(&["codex"])));
}

#[test]
fn scrollback_failure_preserves_state_and_status() {
    let previous = AgentFields {
        kind: AgentKind::Claude,
        state: AgentState::Working,
        status: "Esc to interrupt".to_owned(),
        running_command: Some(argv(&["claude"])),
    };
    let fields = enrich_fields(&previous, "claude", Ok(argv(&["claude"])), Err("denied".to_owned()));
    assert_eq!(fields.kind, AgentKind::Claude);
    assert_eq!(fields.state, AgentState::Working);
    assert_eq!(fields.status, "Esc to interrupt");
}

#[test]
fn both_failures_preserve_previous_enriched_fields() {
    let previous = AgentFields {
        kind: AgentKind::Codex,
        state: AgentState::Blocked,
        status: "allow command?".to_owned(),
        running_command: Some(argv(&["codex"])),
    };
    let fields = enrich_fields(
        &previous,
        "claude",
        Err("command denied".to_owned()),
        Err("scrollback denied".to_owned()),
    );
    assert_eq!(fields, previous);
}

#[test]
fn uses_viewport_identity_when_command_and_title_are_unknown() {
    let fields = enrich_fields(
        &AgentFields::default(),
        "plain",
        Ok(argv(&["bash"])),
        Ok(vec!["Working...".to_owned()]),
    );
    assert_eq!(fields.kind, AgentKind::Pi);
    assert_eq!(fields.state, AgentState::Working);
}

#[test]
fn successful_unknown_sources_clear_stale_agent_kind() {
    let previous = AgentFields {
        kind: AgentKind::Codex,
        state: AgentState::Idle,
        status: ">".to_owned(),
        running_command: Some(argv(&["codex"])),
    };
    let fields = enrich_fields(
        &previous,
        "plain shell",
        Ok(argv(&["bash"])),
        Ok(vec![">".to_owned()]),
    );
    assert_eq!(fields.kind, AgentKind::Unknown);
    assert_eq!(fields.state, AgentState::Unknown);
    assert_eq!(fields.running_command, Some(argv(&["bash"])));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
rtk cargo test agent::tests
```

Expected: FAIL because `enrich_fields` is not implemented.

- [ ] **Step 3: Implement `AgentFields` and `enrich_fields`**

Implement:

```rust
pub fn enrich_fields(
    previous: &AgentFields,
    title: &str,
    command: Result<Vec<String>, String>,
    viewport: Result<Vec<String>, String>,
) -> AgentFields {
    let current_command = command.ok();
    let viewport = viewport.ok();

    if current_command.is_none() && viewport.is_none() {
        return previous.clone();
    }

    let command_kind = current_command
        .as_deref()
        .map(agent_from_command)
        .unwrap_or(AgentKind::Unknown);
    let running_command = current_command.or_else(|| previous.running_command.clone());
    let title_kind = agent_from_title(title);
    let viewport_kind = viewport
        .as_deref()
        .map(agent_from_viewport)
        .unwrap_or(AgentKind::Unknown);
    let kind = [command_kind, title_kind, viewport_kind]
        .into_iter()
        .find(|kind| *kind != AgentKind::Unknown)
        .unwrap_or(AgentKind::Unknown);

    let Some(viewport) = viewport else {
        return AgentFields {
            kind,
            state: previous.state,
            status: previous.status.clone(),
            running_command,
        };
    };

    let detection = detect_viewport(kind, viewport.as_slice());
    AgentFields {
        kind,
        state: detection.state,
        status: detection.status,
        running_command,
    }
}
```

- [ ] **Step 4: Run all agent module tests**

Run:

```bash
rtk cargo test agent
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add src/agent.rs
rtk git commit -m "Add agent row enrichment"
```

### Task 4: Add Rendering Helpers and Integrate the Sidebar

**Files:**
- Modify: `src/agent.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write failing tests for rendering helpers and row integration**

Add pure helper tests to `src/agent.rs`:

```rust
#[test]
fn state_and_agent_labels_are_stable() {
    assert_eq!(state_label(AgentState::Blocked), "blocked");
    assert_eq!(state_label(AgentState::Working), "working");
    assert_eq!(state_label(AgentState::Done), "done");
    assert_eq!(state_label(AgentState::Idle), "idle");
    assert_eq!(state_label(AgentState::Unknown), "unknown");

    assert_eq!(agent_label(AgentKind::Claude), "claude");
    assert_eq!(agent_label(AgentKind::Codex), "codex");
    assert_eq!(agent_label(AgentKind::Pi), "pi");
    assert_eq!(agent_label(AgentKind::Unknown), "unknown");
}

#[test]
fn status_line_omits_empty_status() {
    let fields = AgentFields {
        kind: AgentKind::Codex,
        state: AgentState::Idle,
        status: String::new(),
        running_command: Some(argv(&["codex"])),
    };
    assert_eq!(status_line(&fields, 80), "idle . codex");
}

#[test]
fn status_line_truncates_by_chars() {
    let fields = AgentFields {
        kind: AgentKind::Claude,
        state: AgentState::Blocked,
        status: "approve command now".to_owned(),
        running_command: Some(argv(&["claude"])),
    };
    assert_eq!(status_line(&fields, 12), "blocked . c.");
}
```

Then change `Row` in `src/main.rs` to store `agent::AgentFields`:


```rust
#[derive(Debug, Clone, PartialEq, Default)]
struct Row {
    pane_id: u32,
    title: String,
    focused: bool,
    agent: agent::AgentFields,
}
```

Update existing tests:

- `marks_agent_panes_by_title_marker` should become `initial_rows_start_with_unknown_agent_fields` or verify the title only. Agent enrichment now happens during timer polling.
- `lists_only_terminal_panes_of_own_tab_sorted_top_to_bottom` should assert default `AgentFields`.
- Existing click mapping should still pass because row height stays two lines.

Add this test in `src/main.rs`:

```rust
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
```

Add a focused preservation helper test in `src/main.rs`:

```rust
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
```

Run:

```bash
rtk cargo test agent::tests
rtk cargo test initial_rows_start_with_unknown_agent_fields
```

Expected: FAIL until rendering helpers, `Row`, and tests are updated.

- [ ] **Step 2: Update `rows_for_own_tab` preservation**

Add this helper and call it from `Event::PaneUpdate` after rebuilding rows:

```rust
fn preserve_agent_fields(rows: &mut [Row], old: &[Row]) {
    for row in rows.iter_mut() {
        if let Some(prev) = old.iter().find(|r| r.pane_id == row.pane_id) {
            row.agent = prev.agent.clone();
        }
    }
}
```

Make `rows_for_own_tab` create rows with `agent::AgentFields::default()`.

- [ ] **Step 3: Update `refresh_statuses` to call zellij APIs**

Replace the old `busy` and `status` mutation with:

```rust
fn refresh_statuses(&mut self) {
    for row in self.rows.iter_mut() {
        let command = get_pane_running_command(PaneId::Terminal(row.pane_id));
        let viewport = get_pane_scrollback(PaneId::Terminal(row.pane_id), false)
            .map(|contents| contents.viewport);
        row.agent = agent::enrich_fields(&row.agent, &row.title, command, viewport);
    }
}
```

If the exact zellij return types differ, adapt only at the call boundary. Keep
`agent::enrich_fields` pure.

- [ ] **Step 4: Implement rendering helpers and update rendering**

Add pure helpers in `src/agent.rs`:

```rust
pub fn state_label(state: AgentState) -> &'static str {
    match state {
        AgentState::Blocked => "blocked",
        AgentState::Working => "working",
        AgentState::Done => "done",
        AgentState::Idle => "idle",
        AgentState::Unknown => "unknown",
    }
}

pub fn agent_label(kind: AgentKind) -> &'static str {
    match kind {
        AgentKind::Claude => "claude",
        AgentKind::Codex => "codex",
        AgentKind::Pi => "pi",
        AgentKind::Unknown => "unknown",
    }
}

fn truncate_chars(text: &str, max_width: usize) -> String {
    let len = text.chars().count();
    if len <= max_width {
        return text.to_owned();
    }
    if max_width == 0 {
        return String::new();
    }
    if max_width == 1 {
        return ".".to_owned();
    }
    let prefix: String = text.chars().take(max_width - 1).collect();
    format!("{prefix}.")
}

pub fn status_line(fields: &AgentFields, max_width: usize) -> String {
    let mut line = format!("{} . {}", state_label(fields.state), agent_label(fields.kind));
    if !fields.status.is_empty() {
        line.push_str(" . ");
        line.push_str(&fields.status);
    }
    truncate_chars(&line, max_width)
}
```

Add the ANSI state marker in `src/main.rs`:

```rust
fn state_marker(fields: &agent::AgentFields) -> &'static str {
    match fields.state {
        agent::AgentState::Blocked => "\u{1b}[31m\u{25cf}\u{1b}[0m ",
        agent::AgentState::Working => "\u{1b}[33m\u{25cf}\u{1b}[0m ",
        agent::AgentState::Done => "\u{1b}[36m\u{25cf}\u{1b}[0m ",
        agent::AgentState::Idle => "\u{1b}[32m\u{2713}\u{1b}[0m ",
        agent::AgentState::Unknown => "  ",
    }
}
```

Render line 1 as state marker plus title. Render line 2 as dimmed
`agent::status_line(&row.agent, cols.saturating_sub(4))`.

- [ ] **Step 5: Run focused tests**

Run:

```bash
rtk cargo test agent::tests
rtk cargo test rows_for_own_tab
rtk cargo test target_for_line
rtk cargo test selection_moves_within_bounds
```

Expected: PASS.

- [ ] **Step 6: Run full tests**

Run:

```bash
rtk cargo test
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
rtk git add src/main.rs src/agent.rs
rtk git commit -m "Render plugin-local agent awareness"
```

### Task 5: Build and Manual Verification Notes

**Files:**
- Modify: `README.md` only if observed behavior changes the documented feature list.

- [ ] **Step 1: Build the plugin**

Run:

```bash
rtk ./build.sh
```

Expected: `target/wasm32-wasip1/release/zellij-sidebar.wasm` is rebuilt.

- [ ] **Step 2: Run the full test suite again**

Run:

```bash
rtk cargo test
```

Expected: PASS.

- [ ] **Step 3: Verify in a live zellij session**

Use an existing development session or create a background one:

```bash
rtk zellij attach zaphod-agent-awareness --create-background
```

Manual checks:

- Sidebar opens without stealing focus after permissions are granted.
- Claude, Codex, and Pi panes show known agent labels when available.
- Shell panes show lower-emphasis `unknown` rows.
- Blocked prompts outrank working prompts.
- Click-to-focus still works for every two-line row.
- `Alt .` navigation still highlights and jumps correctly.

- [ ] **Step 4: Inspect zellij logs if behavior is wrong**

Run:

```bash
rtk sh -lc 'tail -n 120 "$TMPDIR/zellij-$(id -u)/zellij-log/zellij.log"'
```

Expected: no repeated permission denial spam, no WASM panic, no deadlock signs
from response-reading calls in `load()` or `pipe()`.

- [ ] **Step 5: Commit README change only if needed**

If `README.md` changes:

```bash
rtk git add README.md
rtk git commit -m "Document agent awareness states"
```

If `README.md` does not change, skip this commit.

### Final Verification

- [ ] Run:

```bash
rtk cargo test
rtk ./build.sh
rtk git status --short
```

Expected: tests pass, build succeeds, and only intentional files are modified.

- [ ] Summarize:
  - final test/build commands
  - live zellij checks completed or skipped
  - any detector limitations discovered for `get_pane_running_command`
