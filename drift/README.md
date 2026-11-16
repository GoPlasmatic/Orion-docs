# Drift guards

The tests in `tests/drift/` hold this book to the Orion release it documents.
Each one reads a page (or the committed `openapi.json`) and compares it with
the code that owns the fact — `AppConfig`, the function registry, the router,
the error and audit vocabularies, the metric macros — and fails when the two
disagree. The code is authoritative: fix the page.

Orion arrives as a git dependency pinned to a release tag (`Cargo.toml`).
Every `orion-*` dependency names the same tag. Guards that read Orion's
source files rather than its API find them through `cargo metadata`
(`tests/drift/support.rs`), so they read the checkout cargo compiled.

```sh
cargo test --manifest-path drift/Cargo.toml                         # run the guards
cargo clippy --manifest-path drift/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path drift/Cargo.toml --bin dump-openapi > openapi.json   # regenerate the spec snapshot
```

| Guard | Holds | To |
|---|---|---|
| `audit_actions_drift_test` | `operate/run/audit-logs.md`'s vocabulary table | every `audit_log`/`audit_record` call in Orion's source |
| `clippy_docs_drift_test` | `reference/clippy/` hub table and per-rule pages | the clippy rule registry |
| `config_docs_drift_test` | `reference/configuration/` settings tables, and every `ORION_*` name in this repository | `AppConfig` defaults and the env-override catalogue |
| `docs_link_test` | every relative link and `#fragment` in `src/` | the pages and their headings |
| `docs_routes_drift_test` | every `curl` and `` `METHOD /api/…` `` reference | the generated OpenAPI route table |
| `field_codes_drift_test` | `reference/errors.md`'s field-code table | `orion_api::error::field_codes` |
| `fmt_style_drift_test` | `reference/workflows.md`, `reference/fmt.md`, the function pages' field order | the formatter's tables |
| `functions_docs_drift_test` | `reference/functions/` | the function registry and its retry safety |
| `included_examples_test` | `examples/` (the files pages `{{#include}}`) | Orion's `examples/` at the pinned tag |
| `jsonlogic_operators_docs_test` | `reference/expressions.md`'s operator tables | the engine's operator vocabulary |
| `mdbook_pin_drift_test` | `build.sh`'s mdBook pin | the pin in `.github/workflows/docs.yml` |
| `metrics_docs_drift_test` | `reference/metrics.md` | `orion-common/src/metrics.rs` |
| `openapi_drift_test` | `openapi.json` | the generator `orion-server dump-openapi` prints |
| `release_stamps_drift_test` | the `Tested with Orion <version>.` stamps and the pinned tag | the version cargo resolved |
| `user_facing_ids_test` | `openapi.json` | no tracker IDs |
| `v2_error_codes_drift_test` | `reference/errors.md`'s code registries | `orion_api::ErrorCode` and `V2Code` |

Moving to a new Orion release, and checking against an untagged Orion
working tree, are described in
[`CONTRIBUTING-DOCS.md`](../CONTRIBUTING-DOCS.md#local-setup).
