//! The drift guards: each holds a page of this book to the Orion release it
//! documents and fails when the two disagree — the configuration reference to
//! `AppConfig`, the metrics page to the recorders, the functions pages to the
//! registry, the book's routes to the router, the error and audit
//! vocabularies to what the source emits, the committed OpenAPI snapshot to
//! the generator.
//!
//! Orion is a git dependency pinned to a release tag (see `../../Cargo.toml`),
//! so "the code" here is that release. Guards that only hold Orion's code to
//! itself stay in the Orion repository.

mod support;

mod audit_actions_drift_test;
mod clippy_docs_drift_test;
mod config_docs_drift_test;
mod data_dialect_docs_test;
mod docs_link_test;
mod docs_routes_drift_test;
mod field_codes_drift_test;
mod fmt_style_drift_test;
mod functions_docs_drift_test;
mod included_examples_test;
mod jsonlogic_operators_docs_test;
mod mdbook_pin_drift_test;
mod metrics_docs_drift_test;
mod openapi_drift_test;
mod release_stamps_drift_test;
mod user_facing_ids_test;
mod v2_error_codes_drift_test;
