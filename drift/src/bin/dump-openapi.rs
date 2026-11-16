//! Print the OpenAPI document of the pinned Orion release, exactly as
//! `orion-server dump-openapi` prints it:
//!
//! ```sh
//! cargo run --manifest-path drift/Cargo.toml --bin dump-openapi > openapi.json
//! ```
//!
//! `openapi_drift_test` compares the committed `openapi.json` against the same
//! accessor, so regenerating with this binary is what makes that guard pass.

fn main() {
    println!("{}", orion_http::routes::openapi::pretty_json());
}
