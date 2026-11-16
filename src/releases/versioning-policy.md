<!-- description: Versioning, support window, compatibility guarantees, Rust MSRV policy, database backends, and platform support for Orion 2.x. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# Versioning and support policy

This page defines what "supported" means for Orion 2.x, which releases receive fixes, what an upgrade guarantees, and which toolchains, databases, and platforms each release is tested against.

## Supported versions

**The latest release is the supported release.** Bug fixes and security fixes land on `main` and ship as a patch release of the current minor version. Older releases are not back-patched.

Orion ships as a single binary with embedded database migrations for every supported backend and a documented upgrade path. Because no supported configuration requires staying on an older release, upgrading to the newest release is always the path to receive fixes.

| Version | Status |
|---|---|
| Latest 2.x release | Supported — receives all bug and security fixes |
| Older 2.x releases | Upgrade to the latest release for fixes |
| 1.x releases | Not back-patched — [upgrade to 2.0](./upgrade-to-2.0.md) (requires a fresh database and definition migration) |
| 0.x releases | End of life |

Report security vulnerabilities privately following the [security policy](https://github.com/GoPlasmatic/Orion/blob/main/SECURITY.md).

## Editions and release alignment

Orion is available in three editions:

- **Community**: Open-source `orion-server` under Apache-2.0, supported through the public GitHub issue tracker.
- **Standard and Enterprise**: The licensed `orion-ext` binary, which combines the Community node with enterprise channel drivers, connector providers, traffic splitting, and the Redis cluster state store. Standard and Enterprise share the same binary and features; they differ in commercial support terms. See [Editions](../concepts/editions.md) and [Standard & Enterprise overview](../enterprise/overview.md).

The `orion-ext` binary tracks the exact Community version it builds on, and every `orion-*` workspace dependency pins a single revision (`=2.x.y`). Upgrading moves the server, the CLI, and the edition together. Mixed-version clusters or mismatched CLI and server versions are not supported.

## Semantic versioning

Orion follows [Semantic Versioning](https://semver.org):

- **Patch (`2.0.x`)**: Bug and security fixes. No configuration, API, or database schema changes beyond what the fix requires. Always safe to roll forward.
- **Minor (`2.x.0`)**: New capabilities, new configuration keys (with defaults that preserve existing behavior), additive database migrations, and potential MSRV updates (see [Rust toolchain and MSRV](#rust-toolchain-and-msrv)).
- **Major (`3.0.0`)**: Reserved for breaking changes to the Admin or Data APIs, configuration semantics, or workflow and channel definitions. Major releases may start a clean state database, as 2.0.0 did, without an in-place database migration path.

The versioned API prefix (`/api/v2/`) identifies the HTTP contract generation. All `/api/v2/` endpoints keep their request and response schemas for the life of the 2.x series. Adding new endpoints or optional response fields is non-breaking.

The v2 Admin API also reports an `api_version` in `/api/v2/admin/info`, and `orion-cli` checks this version before running commands. If a minor release introduces a wire addition that requires a newer CLI, `api_version` increments and `orion-cli` prompts you to upgrade the CLI alongside the server.

### Compatibility guarantees in 2.x

| Surface | Covered by 2.x compatibility? |
|---|---|
| HTTP Admin and Data APIs under `/api/v2/` | **Yes** — request and response contracts remain stable across 2.x |
| Configuration keys and semantics | **Yes** — minor releases only add keys with behavior-preserving defaults |
| Workflow, channel, and connection definitions | **Yes** — definitions that validate on 2.0 remain valid across 2.x |
| Prometheus metric names and labels | **Yes** — metric renames wait for a major version |
| `orion-api` and `orion-client` Rust APIs | **No** — internal Rust types may change in minor releases; pin exact crate versions |
| Extension API (`orion-extension-api`, `orion::extension_api`) | **No** — internal trait surface; pin exact crate versions (`=2.x.y`) |
| State database schema | **No** — internal implementation detail; query Orion state through the HTTP API |

**Client and extension crates**: `orion-api`, `orion-client`, and `orion-extension-api` share the server's version number (`2.x.y`). While the HTTP wire format is stable across 2.x, the Rust struct and trait definitions may evolve in minor releases. If you depend on these crates in Rust code, pin an exact version (`=2.0.0`).

**Database schema**: Tables, views, triggers, and column names in the state database may change in any minor release through embedded migrations. Always read operational data through the HTTP Admin API rather than querying SQLite, PostgreSQL, or MySQL tables directly.

## Deprecation policy

Orion 2.0.0 removes all deprecated 1.x spellings cleanly and rejects unknown configuration or definition fields at load time.

Any deprecation introduced *after* 2.0.0 is announced in a minor release with the old spelling continuing to work, and is removed no earlier than the next major release (`3.0.0`).

## Upgrade guarantees

- **Release-to-release upgrades**: Upgrades are tested and supported between consecutive minor releases. When skipping minor versions, review the release notes for each intermediate version.
- **Embedded forward-only migrations**: Database migrations are compiled into the server binary for SQLite, PostgreSQL, and MySQL. They run automatically at startup when `storage.auto_migrate = true` (the default for single-node deployments) or explicitly via `orion-server migrate` in clustered deployments.
- **Back up before upgrading**: Because database migrations are forward-only, rolling a binary back after a migration has applied is not supported without restoring from a backup. See [Backups, upgrades, and troubleshooting](../operate/maintenance.md).

## Rust toolchain and MSRV

The minimum supported Rust version (MSRV) for building Orion from source is **1.98**, declared as `rust-version` in `Cargo.toml` and enforced in CI. MSRV increases happen only in minor or major releases and are noted in the changelog. Prebuilt binaries and container images are self-contained and do not require a Rust toolchain.

Because Orion embeds the Wasmtime WebAssembly runtime for the plugin sandbox, Orion's MSRV also tracks [Wasmtime's MSRV policy](https://docs.wasmtime.dev/stability-release.html) (stable minus two). Wasmtime and Cranelift are compiled into all release targets (adding roughly 6 MB to the binary) but remain inert at runtime unless `plugins.enabled = true`.

## Supported database backends

Orion selects the state database backend at runtime from the `storage.url` scheme. All three backends are tested in CI.

| Backend | Support notes |
|---|---|
| **SQLite** | Default embedded backend (`sqlite:data/orion.db?mode=rwc`). Zero external dependencies; ideal for local development and single-node deployments. Cannot be used for multi-node clustering. |
| **PostgreSQL** | Production backend for single-node and clustered deployments. Tested against PostgreSQL 16 in CI and official Helm/Compose manifests. |
| **MySQL** | Production backend for single-node and clustered deployments. Requires MySQL 8.0+. |

## Supported platforms

- **Container images**: Multi-arch images published to `ghcr.io/goplasmatic/orion` for `linux/amd64` and `linux/arm64` (based on Debian `trixie-slim`).
- **Prebuilt binaries**: Released on GitHub and Homebrew for `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, and `x86_64-pc-windows-msvc`.
- **Source builds**: Supported on Linux and macOS using the Rust toolchain version specified in `Cargo.toml`.

## Related pages

- [Upgrade to Orion 2.0](./upgrade-to-2.0.md) — breaking changes and migration steps from 1.x
- [Backups, upgrades, and troubleshooting](../operate/maintenance.md) — operational upgrade and backup procedures
- [Server configuration and environment variables](../reference/configuration.md) — full configuration reference
