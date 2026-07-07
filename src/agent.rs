// ABOUTME: Pure agent-awareness classification for terminal panes.
// ABOUTME: Keeps zellij host calls out of detector logic for unit testing.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKind {
    Claude,
    Codex,
    Pi,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Blocked,
    Working,
    Done,
    Idle,
    Unknown,
}

// Maps grout's session-state strings (the agent-event row protocol) onto the
// marker states; anything unrecognized renders the neutral marker.
pub fn marker_for_state(state: &str) -> AgentState {
    match state {
        "blocked" => AgentState::Blocked,
        "working" => AgentState::Working,
        "idle" => AgentState::Idle,
        "done" => AgentState::Done,
        _ => AgentState::Unknown,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewportDetection {
    pub state: AgentState,
    pub status: String,
}

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
        return ViewportDetection {
            state: AgentState::Unknown,
            status,
        };
    }
    if let Some(status) = blocker_line(viewport) {
        return ViewportDetection {
            state: AgentState::Blocked,
            status,
        };
    }
    if has_working_signal(agent, viewport) {
        return ViewportDetection {
            state: AgentState::Working,
            status,
        };
    }
    ViewportDetection {
        state: AgentState::Idle,
        status,
    }
}

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
    let mut line = format!(
        "{} . {}",
        state_label(fields.state),
        agent_label(fields.kind)
    );
    if !fields.status.is_empty() {
        line.push_str(" . ");
        line.push_str(&fields.status);
    }
    truncate_chars(&line, max_width)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|part| (*part).to_owned()).collect()
    }

    #[test]
    fn classifies_command_basename_exactly() {
        assert_eq!(
            agent_from_command(&argv(&["/opt/bin/claude"])),
            AgentKind::Claude
        );
        assert_eq!(
            agent_from_command(&argv(&["claude-code"])),
            AgentKind::Claude
        );
        assert_eq!(agent_from_command(&argv(&["codex"])), AgentKind::Codex);
        assert_eq!(
            agent_from_command(&argv(&["/usr/local/bin/pi"])),
            AgentKind::Pi
        );
    }

    #[test]
    fn command_detection_does_not_false_positive_pi() {
        assert_eq!(agent_from_command(&argv(&["pip"])), AgentKind::Unknown);
        assert_eq!(agent_from_command(&argv(&["python"])), AgentKind::Unknown);
        assert_eq!(
            agent_from_command(&argv(&["/tmp/pi/codex-helper"])),
            AgentKind::Unknown
        );
        assert_eq!(
            agent_from_command(&argv(&["sh", "-lc", "pi"])),
            AgentKind::Unknown
        );
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
        assert_eq!(
            agent_from_viewport(&["Working...".to_owned()]),
            AgentKind::Pi
        );
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
        assert_eq!(
            detect_viewport(AgentKind::Codex, &codex).state,
            AgentState::Working
        );

        let pi = vec!["Working...".to_owned()];
        assert_eq!(
            detect_viewport(AgentKind::Pi, &pi).state,
            AgentState::Working
        );
    }

    #[test]
    fn known_agent_without_signal_is_idle() {
        let viewport = vec!["> ".to_owned()];
        assert_eq!(
            detect_viewport(AgentKind::Claude, &viewport).state,
            AgentState::Idle
        );
    }

    #[test]
    fn unknown_agent_stays_unknown() {
        let viewport = vec!["plain shell".to_owned()];
        assert_eq!(
            detect_viewport(AgentKind::Unknown, &viewport).state,
            AgentState::Unknown
        );
    }

    #[test]
    fn status_uses_last_non_empty_line() {
        let viewport = vec![
            "first".to_owned(),
            "  ".to_owned(),
            "last".to_owned(),
            "".to_owned(),
        ];
        assert_eq!(detect_viewport(AgentKind::Claude, &viewport).status, "last");
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
        let fields = enrich_fields(
            &previous,
            "claude",
            Ok(argv(&["claude"])),
            Err("denied".to_owned()),
        );
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

    #[test]
    fn grout_state_strings_map_onto_marker_states() {
        assert_eq!(marker_for_state("blocked"), AgentState::Blocked);
        assert_eq!(marker_for_state("working"), AgentState::Working);
        assert_eq!(marker_for_state("idle"), AgentState::Idle);
        assert_eq!(marker_for_state("done"), AgentState::Done);
        assert_eq!(marker_for_state("exited"), AgentState::Unknown);
        assert_eq!(marker_for_state(""), AgentState::Unknown);
    }

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
}
