// ABOUTME: Host-only complete KDL and Zaphod identity validator for real layout smoke evidence.
// ABOUTME: Exit status distinguishes malformed records from valid but stale/wrong identity.

const EXPECTED: &str = "file:/candidate/zellij-sidebar.wasm";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_layout_with_exact_candidate_is_valid() {
        let input = r#"layout { pane { plugin location="file:/candidate/zellij-sidebar.wasm" { rail "1" } } }"#;
        assert_eq!(validate_layout(input, EXPECTED, true), Ok(()));
    }

    #[test]
    fn complete_layout_without_candidate_is_valid_when_absent_is_expected() {
        let input = r#"layout { pane { plugin location="zellij:tab-bar" } }"#;
        assert_eq!(validate_layout(input, EXPECTED, false), Ok(()));
    }

    #[test]
    fn quoted_brace_is_parsed_as_data_not_structure() {
        let input = r#"layout { pane command="printf" { args "{" } }"#;
        assert_eq!(validate_layout(input, EXPECTED, false), Ok(()));
    }

    #[test]
    fn malformed_complete_record_is_distinct_from_identity_mismatch() {
        assert_eq!(
            validate_layout("layout { pane", EXPECTED, true),
            Err(ValidationError::Malformed)
        );
        assert_eq!(
            validate_layout("layout { pane; }", EXPECTED, true),
            Err(ValidationError::Identity)
        );
    }

    #[test]
    fn wrong_candidate_url_is_an_identity_mismatch() {
        let input = r#"layout { plugin location="file:/wrong/zellij-sidebar.wasm" }"#;
        assert_eq!(
            validate_layout(input, EXPECTED, true),
            Err(ValidationError::Identity)
        );
    }
}
