//! `src/reference/clippy/` lists exactly the registry — id, level, scope
//! and summary on the hub's table, and one page per rule — so the book cannot
//! describe a rule that does not exist or omit one that does.

use orion_authoring::definitions::clippy;

const CLIPPY_MD: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/reference/clippy.md");

fn hub() -> String {
    std::fs::read_to_string(CLIPPY_MD).expect("src/reference/clippy.md")
}

/// The rows of the first table whose header starts with `| Rule |`.
fn documented() -> Vec<Vec<String>> {
    hub()
        .lines()
        .skip_while(|l| !l.starts_with("| Rule |"))
        .skip(2)
        .take_while(|l| l.starts_with("| `") || l.starts_with("| ["))
        .map(|l| {
            l.trim_matches('|')
                .split(" | ")
                .map(|c| {
                    let c = c.trim();
                    let c = match (c.find('['), c.find("](")) {
                        (Some(a), Some(b)) if a < b => &c[a + 1..b],
                        _ => c,
                    };
                    c.trim().trim_matches('`').to_string()
                })
                .collect()
        })
        .collect()
}

#[test]
fn the_rule_table_is_the_registry() {
    let rows = documented();
    let registry = clippy::registry();
    assert_eq!(
        rows.len(),
        registry.len(),
        "the table has {} rows, the registry {} rules",
        rows.len(),
        registry.len()
    );
    for (row, rule) in rows.iter().zip(registry) {
        assert_eq!(row[0], rule.id(), "row order must be registry order");
        assert_eq!(row[1], rule.level().as_str(), "{}", rule.id());
        assert_eq!(row[2], rule.scope().as_str(), "{}", rule.id());
        assert_eq!(row[3], rule.summary(), "{}: summary", rule.id());
    }
}

#[test]
fn every_rule_has_its_own_page() {
    let hub = hub();
    for rule in clippy::registry() {
        assert!(
            hub.contains(&format!("\n### `{}`\n", rule.id())),
            "clippy.md must contain a `### \\`{}\\`` section",
            rule.id()
        );
    }
}
