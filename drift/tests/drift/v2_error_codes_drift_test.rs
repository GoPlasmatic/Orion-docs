//! Drift guard for the API's `error.code` vocabularies on the errors page.
//!
//! `orion_api::ErrorCode` (the general registry) and `orion_api::V2Code` (the
//! v2 admin API's closed registry) are authoritative; the Orion repository
//! holds its own source to them (every raised code registered, every
//! registered code reachable). This module holds the book to them: the errors
//! reference page documents exactly each registered set, each code with the
//! status it answers.

use std::collections::BTreeMap;

use orion_api::{ErrorCode, V2Code};

const ERRORS_MD: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/reference/errors.md");

/// `code -> status` from the table under `heading`: a row whose first cell is
/// a backticked code and whose second is a numeric status.
fn documented(md: &str, heading: &str) -> BTreeMap<String, u16> {
    let start = md
        .find(heading)
        .unwrap_or_else(|| panic!("errors.md must carry a '{heading}' table"));
    let section = &md[start + heading.len()..];
    let end = section.find("\n## ").unwrap_or(section.len());

    let mut documented = BTreeMap::new();
    for line in section[..end].lines() {
        let cells: Vec<&str> = line.trim().split('|').map(str::trim).collect();
        if cells.len() < 4 {
            continue;
        }
        let (Some(code), Ok(status)) = (
            cells[1].strip_prefix('`').and_then(|c| c.strip_suffix('`')),
            cells[2].parse::<u16>(),
        ) else {
            continue;
        };
        documented.insert(code.to_string(), status);
    }
    documented
}

#[test]
fn the_errors_reference_documents_the_general_code_registry() {
    let md = std::fs::read_to_string(ERRORS_MD).expect("read src/reference/errors.md");
    let expected: BTreeMap<String, u16> = ErrorCode::ALL
        .iter()
        .map(|c| (c.as_str().to_string(), c.status()))
        .collect();
    assert_eq!(
        documented(&md, "## The code registry"),
        expected,
        "src/reference/errors.md's general code table and orion_api::ErrorCode \
         disagree (code or status)"
    );
}

#[test]
fn the_errors_reference_documents_the_registry() {
    let md = std::fs::read_to_string(ERRORS_MD).expect("read src/reference/errors.md");
    let expected: BTreeMap<String, u16> = V2Code::ALL
        .iter()
        .map(|c| (c.as_str().to_string(), c.kind().status()))
        .collect();
    assert_eq!(
        documented(&md, "## The v2 admin code registry"),
        expected,
        "src/reference/errors.md's v2 admin code table and orion_api::V2Code \
         disagree (code or status)"
    );
}
