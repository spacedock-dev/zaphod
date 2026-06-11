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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewportDetection {
    pub state: AgentState,
    pub status: String,
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
}
