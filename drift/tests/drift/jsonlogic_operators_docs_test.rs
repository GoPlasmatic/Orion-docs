//! The JSONLogic operator vocabulary the book documents is the one the pinned
//! Orion release dispatches.
//!
//! `orion_engine::operators::operator_names()` derives from a running engine
//! (dataflow-rs's operator names over datalogic's own opcode table), and the
//! Orion repository proves every name in it evaluates to its documented
//! result. This module holds `reference/expressions.md`'s "Available
//! operators" tables to that set in both directions: a supported operator the
//! page omits is one no reader learns, and a documented one the build does not
//! dispatch is one a reader writes and gets either an error or — in a `map`
//! mapping — a silent object literal.

use std::collections::BTreeSet;

const EXPRESSIONS_MD: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../src/reference/expressions.md"
);

/// Operator names documented in the "Available operators" section.
///
/// A row is a table row whose first cell contains one or more backticked
/// tokens; `` `sort` `` yields one name and `` `upper` / `lower` `` yields two,
/// which is how the tables group operators that share an example. Reading stops
/// at the next `## ` heading so prose elsewhere in the file cannot contribute
/// names.
fn documented_operators() -> BTreeSet<String> {
    let md = std::fs::read_to_string(EXPRESSIONS_MD).expect("read src/reference/expressions.md");
    let heading = "## Available operators";
    let start = md
        .find(heading)
        .expect("expressions.md must document the operator vocabulary");
    let section = &md[start + heading.len()..];
    let end = section.find("\n## ").unwrap_or(section.len());

    let mut out = BTreeSet::new();
    for line in section[..end].lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        let Some(first_cell) = line.split('|').nth(1) else {
            continue;
        };
        for token in first_cell.split('`').skip(1).step_by(2) {
            let token = token.trim();
            if !token.is_empty() {
                out.insert(token.to_string());
            }
        }
    }
    out
}

/// The reference page documents exactly the operators the engine dispatches —
/// no more, no fewer.
#[test]
fn documented_operators_match_the_supported_set() {
    let supported: BTreeSet<String> = orion_engine::operators::operator_names().clone();
    assert!(
        supported.len() > 20,
        "the engine reports only {} operators — the accessor has stopped \
         reading the vocabulary",
        supported.len()
    );
    let documented = documented_operators();

    let undocumented: Vec<&String> = supported.difference(&documented).collect();
    assert!(
        undocumented.is_empty(),
        "operators supported but absent from the 'Available operators' tables in \
         src/reference/expressions.md: {undocumented:?}"
    );

    let invented: Vec<&String> = documented.difference(&supported).collect();
    assert!(
        invented.is_empty(),
        "src/reference/expressions.md documents operators this build does not \
         support: {invented:?}. A reader will write these and get either an error \
         or a silent object literal."
    );
}
