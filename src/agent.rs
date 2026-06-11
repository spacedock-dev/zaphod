// ABOUTME: Pure agent-awareness classification for terminal panes.
// ABOUTME: Keeps zellij host calls out of detector logic for unit testing.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKind {
    Claude,
    Codex,
    Pi,
    Unknown,
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
}
