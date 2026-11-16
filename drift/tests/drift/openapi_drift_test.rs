//! The committed `openapi.json` is what the pinned Orion release emits.
//!
//! `orion_http::routes::openapi::pretty_json` is the accessor
//! `orion-server dump-openapi` prints, so the snapshot this repository
//! publishes cannot drift from the binary it describes as the API (or the
//! crate version, which the document carries) changes. If this fails,
//! regenerate it:
//!
//! ```sh
//! cargo run --manifest-path drift/Cargo.toml --bin dump-openapi > openapi.json
//! ```

#[test]
fn committed_openapi_json_is_up_to_date() {
    let committed = std::fs::read_to_string(crate::support::docs_path("openapi.json")).expect(
        "openapi.json should exist — run \
         `cargo run --manifest-path drift/Cargo.toml --bin dump-openapi > openapi.json`",
    );

    let generated = orion_http::routes::openapi::pretty_json();

    // `println!` adds a trailing newline to the committed file; trim both ends.
    assert_eq!(
        committed.trim_end(),
        generated.trim_end(),
        "openapi.json is stale — regenerate with: \
         cargo run --manifest-path drift/Cargo.toml --bin dump-openapi > openapi.json"
    );
}
