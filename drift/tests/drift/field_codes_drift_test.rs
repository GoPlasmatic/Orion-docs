//! Drift guard for the `error.details[].code` vocabulary on the errors page.
//!
//! `orion_api::error::field_codes` is the registry; the Orion repository
//! holds its own source to it (every literal registered, every registered
//! code reachable). This module holds the book to it: the errors reference
//! page documents exactly the registered set — a documented code nothing
//! emits is one a client branching on it would wait for forever.

use std::collections::BTreeSet;

const ERRORS_MD: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/reference/errors.md");

fn registered_codes() -> BTreeSet<String> {
    orion_api::error::field_codes::ALL
        .iter()
        .map(|c| (*c).to_string())
        .collect()
}

#[test]
fn the_errors_reference_documents_the_registered_vocabulary() {
    let md = std::fs::read_to_string(ERRORS_MD).expect("read src/reference/errors.md");
    let heading = "### Field error codes";
    let start = md
        .find(heading)
        .expect("errors.md must carry a '### Field error codes' table");
    let section = &md[start + heading.len()..];
    let end = section.find("\n## ").unwrap_or(section.len());

    let mut documented = BTreeSet::new();
    for line in section[..end].lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        if let Some(cell) = line.split('|').nth(1) {
            for token in cell.split('`').skip(1).step_by(2) {
                let token = token.trim();
                if token.chars().all(|c| c.is_ascii_uppercase() || c == '_') && !token.is_empty() {
                    documented.insert(token.to_string());
                }
            }
        }
    }

    assert_eq!(
        documented,
        registered_codes(),
        "src/reference/errors.md's field-code table and \
         orion_api::error::field_codes disagree"
    );
}
