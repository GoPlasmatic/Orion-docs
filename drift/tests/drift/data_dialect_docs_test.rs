//! The data dialect page's schema-registry example parses and means what
//! its prose says (moved from orion-engine's `query::schema` unit tests with
//! the docs).

use orion_engine::query::EntityRegistry;
use orion_engine::query::schema::UnmappedPolicy;

// -----------------------------------------------------------------
// W5: the documented example must parse and mean what it says
// -----------------------------------------------------------------

/// The first fenced ```json block after "The schema registry" heading in
/// `src/reference/data-dialect.md`, exactly as a reader would copy it.
fn documented_schema_example() -> serde_json::Value {
    let path = crate::support::docs_path("src/reference/connectors.md");
    let source =
        std::fs::read_to_string(path).expect("src/reference/connectors.md must be readable");
    let section = source
        .split("## The schema registry")
        .nth(1)
        .expect("data-dialect.md must have a 'The schema registry' section");
    let fence_start = section
        .find("```json")
        .expect("the schema registry section must carry a ```json example")
        + "```json".len();
    let body = &section[fence_start..];
    let fence_end = body
        .find("```")
        .expect("the ```json example must be closed");
    // The doc shows a `"schema": { … }` task fragment; wrap it in braces
    // to make it a standalone document, changing nothing else.
    let wrapped: serde_json::Value =
        serde_json::from_str(&format!("{{{}}}", body[..fence_end].trim()))
            .expect("the documented schema example must be valid JSON");
    wrapped
        .get("schema")
        .expect("the example must be a 'schema' fragment")
        .clone()
}

/// The single canonical example of the feature the dialect's security
/// model hangs off was copy-paste broken in two directions: `"table"`
/// where the field is `physical` (silently dropped, so the rename never
/// applied) and `"type": "string"`, which is not a `FieldType` variant
/// (hard parse error). Parse the doc's example verbatim and assert it
/// *means* what the surrounding prose says, so it cannot drift again.
#[test]
fn the_documented_schema_example_parses_and_means_what_it_says() {
    let reg = EntityRegistry::from_json(&documented_schema_example())
        .expect("the documented schema example must parse as an EntityRegistry");

    // The prose promises a physical rename (users → app_users)…
    assert_eq!(
        reg.physical_table("users").expect("users is declared"),
        "app_users",
        "the entity rename in the example must actually apply"
    );
    // …a column rename with a type hint (id → user_id)…
    assert_eq!(
        reg.resolve_field("users", "id", "filter")
            .expect("id is declared")
            .physical,
        "user_id",
        "the column rename in the example must actually apply"
    );
    // …a column hidden from reads and writes…
    assert!(
        reg.resolve_field("users", "secret", "filter").is_err(),
        "queryable: false must hide the column from reads"
    );
    assert!(
        reg.resolve_write_column("users", "secret", "values")
            .is_err(),
        "writable: false must protect the column from writes"
    );
    // …allowlist mode, and a declared relation for some/all/none.
    assert_eq!(reg.unmapped, UnmappedPolicy::Reject);
    assert!(
        reg.resolve_relation("users", "orders", "filter").is_ok(),
        "the example's relation must resolve"
    );
}
