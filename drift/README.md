# Drift guards

The tests in `tests/drift/` verify that the documentation stays synchronized with the Orion release it documents.
Each guard reads a page (or the committed `openapi.json`) and compares it against the authoritative Rust source (`AppConfig`, the function registry, the HTTP router, the error and audit vocabularies, and the metric macros).

Orion arrives as a git dependency pinned to a release tag in `Cargo.toml`. Guards that inspect Orion's source files resolve them via `cargo metadata` (`tests/drift/support.rs`).

```sh
cargo test --manifest-path drift/Cargo.toml                         # run all drift guards
cargo clippy --manifest-path drift/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path drift/Cargo.toml --bin dump-openapi > openapi.json   # regenerate OpenAPI snapshot
```

| Guard | Holds | To |
|---|---|---|
| `audit_actions_drift_test` | `operate/traces-and-audit.md` audit vocabulary table | every `audit_log` / `audit_record` call in Orion's source |
| `clippy_docs_drift_test` | `reference/clippy.md` rule table and per-rule sections | the clippy rule registry |
| `config_docs_drift_test` | `reference/configuration.md` settings tables and every `ORION_*` token in the repository | `AppConfig` defaults and the env-override catalog |
| `data_dialect_docs_test` | `reference/connectors.md` schema registry example | `orion_engine::query::EntityRegistry` |
| `docs_link_test` | every relative link and `#fragment` in `src/` | target pages and heading IDs |
| `docs_routes_drift_test` | every `curl` and `` `METHOD /api/…` `` reference | the generated OpenAPI route table |
| `field_codes_drift_test` | `reference/errors-and-metrics.md` field-code table | `orion_api::error::field_codes` |
| `fmt_style_drift_test` | `reference/workflows.md` and `reference/functions.md` field order | `orion-authoring` formatter tables |
| `functions_docs_drift_test` | `reference/functions.md` summary, sections, and retry-safety table | `FunctionRegistry::builtin()` |
| `included_examples_test` | `examples/` (files included via `{{#include}}`) | Orion's `examples/` at the pinned tag |
| `jsonlogic_operators_docs_test` | `reference/expressions.md` operator tables | the engine's supported operator vocabulary |
| `mdbook_pin_drift_test` | `build.sh` mdBook pin | the mdBook pin in `.github/workflows/docs.yml` |
| `metrics_docs_drift_test` | `reference/errors-and-metrics.md` metrics tables | `orion-common/src/metrics.rs` |
| `openapi_drift_test` | `openapi.json` | output of `orion-server dump-openapi` |
| `release_stamps_drift_test` | `Tested with Orion <version>.` stamps and pinned git tags | the version Cargo resolved |
| `user_facing_ids_test` | `openapi.json` | no internal tracker IDs |
| `v2_error_codes_drift_test` | `reference/errors-and-metrics.md` code registries | `orion_api::ErrorCode` and `V2Code` |

See [`CONTRIBUTING-DOCS.md`](../CONTRIBUTING-DOCS.md) for instructions on bumping the pinned Orion tag or testing against a local working tree.
