//! Drift guards for the formatter's tables and numbers, against the pages
//! that document them.
//!
//! The canonical key order is a hand-written table per document shape, and
//! a table is a promise that goes stale the moment a shape gains a field. The
//! Orion repository pins the entity tables to the request schemas and the
//! built-in input tables to the function registry; this module pins what the
//! book states: the step tables to the workflow reference's own field tables,
//! each function page's field order to the formatter's, and the style numbers
//! to the page that documents them.

use std::collections::BTreeSet;

use orion_authoring::definitions::fmt::style::{self, STYLE};

const FMT_MD: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/reference/fmt.md");
const WORKFLOWS_MD: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/reference/workflows.md");
const FUNCTIONS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/reference/functions");

fn table(keys: &[&str]) -> BTreeSet<String> {
    keys.iter().map(|k| k.to_string()).collect()
}

/// The step tables against the workflow reference's own field tables.
/// The engine's task definition is `#[non_exhaustive]` and not `Serialize`, so the
/// page — which `docs_routes_drift_test`'s siblings already keep honest —
/// is the closest pin available. A task's `description` is accepted by the
/// engine but not documented, and is allowed as the one extra.
#[test]
fn step_tables_match_the_workflow_reference() {
    let page = std::fs::read_to_string(WORKFLOWS_MD).unwrap();
    let section = |heading: &str| -> BTreeSet<String> {
        let body = page
            .split(heading)
            .nth(1)
            .unwrap_or_else(|| panic!("workflows.md lacks `{heading}`"));
        let body = body.split("\n##").next().unwrap();
        body.lines()
            .filter(|l| l.starts_with("| `"))
            .map(|l| {
                l.trim_start_matches("| `")
                    .split('`')
                    .next()
                    .unwrap()
                    .to_string()
            })
            .collect()
    };
    // The `## Tasks` section documents the task table and then the
    // `function` object's table; only the first is the task's own keys.
    let tasks_section = page.split("\n## Tasks").nth(1).unwrap();
    let task_keys: BTreeSet<String> = tasks_section
        .split("\n| `name` | string | **yes** | — | One of")
        .next()
        .unwrap()
        .lines()
        .filter(|l| l.starts_with("| `"))
        .map(|l| {
            l.trim_start_matches("| `")
                .split('`')
                .next()
                .unwrap()
                .to_string()
        })
        .collect();
    let mut table_minus_description = table(style::TASK_KEYS);
    table_minus_description.remove("description");
    assert_eq!(
        table_minus_description, task_keys,
        "TASK_KEYS vs the Tasks table"
    );
    assert_eq!(
        table(style::GROUP_KEYS),
        section("\n### Task groups"),
        "GROUP_KEYS vs the Task groups table"
    );
}

/// Each function's reference page lists its fields in a table; the
/// formatter orders an input by the registry (Orion functions) or
/// `BUILTIN_INPUT_KEYS` (engine built-ins). Both are canonical order, so
/// they must agree — a reader who learned the fields from the page should
/// see them in that order in every formatted file.
#[test]
fn documented_field_order_is_the_formatters() {
    let mut checked = 0;
    let mut mismatches = Vec::new();
    let mut pages: Vec<_> = std::fs::read_dir(FUNCTIONS_DIR)
        .expect("list reference/functions")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .collect();
    pages.sort();
    for path in pages {
        // The page is named after the function; the hub and the discovery
        // page name no function and are skipped by `input_key_order`.
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let name = name.as_str();
        let section = std::fs::read_to_string(&path).unwrap();
        // The page's *first* table: `data_write` follows its field table
        // with one for the `write` envelope's own members.
        // A row may document several fields at once (`cc` / `bcc`); every
        // backticked name in its first cell counts, in order.
        let documented: Vec<String> = section
            .lines()
            .skip_while(|l| !l.starts_with("| `"))
            .take_while(|l| l.starts_with("| `"))
            .flat_map(|l| {
                let cell = l.trim_start_matches("| ").split(" | ").next().unwrap_or("");
                cell.split('`')
                    .skip(1)
                    .step_by(2)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            // Top-level fields only: `mappings[].path` describes a member.
            .filter(|f| !f.contains('[') && !f.contains('.'))
            .collect();
        let Some(formatter) =
            orion_authoring::definitions::fmt::formatter().input_key_order(Some(name))
        else {
            continue;
        };
        if documented.is_empty() {
            continue;
        }
        let formatter: Vec<String> = formatter.iter().map(|s| s.to_string()).collect();
        if documented != formatter {
            mismatches.push(format!(
                "`{name}`: documented {documented:?}, formatter (registry) {formatter:?}"
            ));
        }
        checked += 1;
    }
    assert!(
        mismatches.is_empty(),
        "the reference page lists these functions' fields in a different order from the \
         registry, and the formatter follows the registry:\n{}",
        mismatches.join("\n")
    );
    assert!(
        checked >= 20,
        "only {checked} function sections were compared"
    );
}

#[test]
fn the_style_page_quotes_the_numbers_the_code_uses() {
    let page = std::fs::read_to_string(FMT_MD).expect("src/reference/fmt.md");
    let quoted = |label: &str| -> usize {
        let row = page
            .lines()
            .find(|l| l.starts_with(&format!("| {label} |")))
            .unwrap_or_else(|| panic!("fmt.md has no `| {label} |` row"));
        row.split('|')
            .nth(2)
            .unwrap()
            .trim()
            .split(' ')
            .next()
            .unwrap()
            .parse()
            .unwrap_or_else(|_| panic!("`{label}` row does not start with a number"))
    };
    assert_eq!(quoted("Line width"), STYLE.width);
    assert_eq!(quoted("Indent"), STYLE.indent);
    assert_eq!(quoted("Scalar array inline cap"), STYLE.max_scalar_inline);
}
