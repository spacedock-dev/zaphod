// ABOUTME: Host-only complete KDL and Zaphod identity validator for real layout smoke evidence.
// ABOUTME: Exit status distinguishes malformed records from valid but stale/wrong identity.

use kdl::KdlDocument;
use std::{env, fs, process};

const MAX_LAYOUT_BYTES: u64 = 4 * 1024 * 1024;
#[cfg(test)]
const EXPECTED: &str = "file:/candidate/zellij-sidebar.wasm";

#[derive(Debug, PartialEq, Eq)]
enum ValidationError {
    Malformed,
    Identity,
}

fn has_exact_rail(node: &kdl::KdlNode) -> bool {
    let Some(children) = node.children() else {
        return false;
    };
    let rails: Vec<_> = children
        .nodes()
        .iter()
        .filter(|child| child.name().value() == "rail")
        .collect();
    rails.len() == 1
        && rails[0].entries().len() == 1
        && rails[0].get(0).and_then(|entry| entry.value().as_string()) == Some("1")
}

fn zaphod_identities<'a>(document: &'a KdlDocument, identities: &mut Vec<(&'a str, bool)>) {
    for node in document.nodes() {
        if node.name().value() == "plugin" {
            if let Some(location) = node.get("location").and_then(|entry| entry.value().as_string()) {
                let path = location.split(['?', '#']).next().unwrap_or(location);
                if path.rsplit('/').next() == Some("zellij-sidebar.wasm") {
                    identities.push((location, has_exact_rail(node)));
                }
            }
        }
        if let Some(children) = node.children() {
            zaphod_identities(children, identities);
        }
    }
}

fn validate_layout(input: &str, expected_url: &str, expect_present: bool) -> Result<(), ValidationError> {
    let document = input
        .parse::<KdlDocument>()
        .map_err(|_| ValidationError::Malformed)?;
    let roots = document.nodes();
    if roots.len() != 1 || roots[0].name().value() != "layout" || roots[0].children().is_none() {
        return Err(ValidationError::Malformed);
    }
    let mut identities = Vec::new();
    zaphod_identities(&document, &mut identities);
    if expect_present {
        if identities.is_empty()
            || identities
                .iter()
                .any(|(location, exact_rail)| *location != expected_url || !exact_rail)
        {
            return Err(ValidationError::Identity);
        }
    } else if !identities.is_empty() {
        return Err(ValidationError::Identity);
    }
    Ok(())
}

fn usage() -> ! {
    eprintln!("usage: zaphod-kdl-validate FILE EXPECTED_URL present|absent");
    process::exit(2);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        usage();
    }
    let expect_present = match args[3].as_str() {
        "present" => true,
        "absent" => false,
        _ => usage(),
    };
    let metadata = fs::metadata(&args[1]).unwrap_or_else(|error| {
        eprintln!("layout read failed: {error}");
        process::exit(2);
    });
    if metadata.len() > MAX_LAYOUT_BYTES {
        eprintln!("malformed layout: {} bytes exceeds {MAX_LAYOUT_BYTES}", metadata.len());
        process::exit(20);
    }
    let input = fs::read_to_string(&args[1]).unwrap_or_else(|error| {
        eprintln!("malformed layout: {error}");
        process::exit(20);
    });
    match validate_layout(&input, &args[2], expect_present) {
        Ok(()) => {}
        Err(ValidationError::Malformed) => {
            eprintln!("malformed layout record");
            process::exit(20);
        }
        Err(ValidationError::Identity) => {
            eprintln!("layout Zaphod identity mismatch");
            process::exit(21);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_layout_with_exact_candidate_is_valid() {
        let input = "layout {\n pane {\n  plugin location=\"file:/candidate/zellij-sidebar.wasm\" {\n   rail \"1\"\n  }\n }\n}\n";
        assert_eq!(validate_layout(input, EXPECTED, true), Ok(()));
    }

    #[test]
    fn candidate_requires_one_exact_rail_child() {
        let missing = "layout {\n plugin location=\"file:/candidate/zellij-sidebar.wasm\"\n}\n";
        let wrong = "layout {\n plugin location=\"file:/candidate/zellij-sidebar.wasm\" {\n  rail \"0\"\n }\n}\n";
        let duplicate = "layout {\n plugin location=\"file:/candidate/zellij-sidebar.wasm\" {\n  rail \"1\"\n  rail \"1\"\n }\n}\n";
        for input in [missing, wrong, duplicate] {
            assert_eq!(
                validate_layout(input, EXPECTED, true),
                Err(ValidationError::Identity)
            );
        }
    }

    #[test]
    fn complete_layout_without_candidate_is_valid_when_absent_is_expected() {
        let input = "layout {\n pane {\n  plugin location=\"zellij:tab-bar\"\n }\n}\n";
        assert_eq!(validate_layout(input, EXPECTED, false), Ok(()));
    }

    #[test]
    fn quoted_brace_is_parsed_as_data_not_structure() {
        let input = "layout {\n pane command=\"printf\" {\n  args \"{\"\n }\n}\n";
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
        let input = "layout {\n plugin location=\"file:/wrong/zellij-sidebar.wasm\"\n}\n";
        assert_eq!(
            validate_layout(input, EXPECTED, true),
            Err(ValidationError::Identity)
        );
    }
}
