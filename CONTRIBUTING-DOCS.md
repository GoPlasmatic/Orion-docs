# Contributing to Orion Documentation

This repository holds the documentation for Orion Community (`orion-server`), Orion Standard and Enterprise (`orion-ext`), and the Orion CLI (`orion-cli`).

Before editing or adding pages, read:
- [`DOCUMENTATION_STANDARD.md`](./DOCUMENTATION_STANDARD.md) for our writing principles, content types, and structure rules.
- [`STYLE_GUIDE.md`](./STYLE_GUIDE.md) for American English spelling, terminology, and formatting conventions.

---

## 1. Site Architecture (The 7-Part Spine)

The documentation is organized around the developer journey in seven top-level parts:

| Part | Purpose | Target Audience |
| :--- | :--- | :--- |
| **Get started** | Run Orion locally, install the CLI and server, build your first services, and evaluate where Orion fits. | Developers evaluating or onboarding to Orion |
| **Core concepts** | Understand the mental model: request lifecycle, workflows, channels, connectors, deployments, and editions. | All developers and architects |
| **Build services** | Task-focused guides for writing workflows, configuring channels/guards, connecting databases/APIs, scheduling cron jobs, testing offline, CI/CD, WebAssembly plugins, and AI authoring. | Application developers |
| **Standard & Enterprise** | Dedicated guides and references for `orion-ext`: Kafka, GraphQL, and SSE channels, the 7 enterprise providers, canary traffic splitting, Redis clustering, and provider packages. | Teams running Orion Standard or Enterprise |
| **Operate in production** | Production checklist, Docker/Kubernetes/HA deployment, security hardening, monitoring, traces/DLQ/audit logs, resilience, backups, upgrades, and troubleshooting. | Platform engineers and operators |
| **Reference** | Complete technical contracts: workflow/channel/connector schemas, task functions, JSONLogic operators, clippy rules, CLI commands, REST API, server configuration, error codes, and metrics. | Developers and operators looking up exact contracts |
| **Releases** | Versioning policy, Orion 2.0 migration guide, and 1.x release history. | Teams upgrading Orion |

### Target Consolidated Page Map (`src/SUMMARY.md`)

To keep information cohesive and easy to scan (`Cmd+F`), related topics are consolidated into comprehensive pages rather than split across hundreds of micro-files:

```markdown
# Summary

[Orion documentation](./introduction.md)

# Get started

- [Quickstart](./get-started/quickstart.md)
- [Install and run Orion](./get-started/install.md)
- [Build your first service](./get-started/first-service.md)
- [Build a Postgres-backed orders API](./get-started/postgres-orders-api.md)
- [Run the example packages](./get-started/examples.md)
- [When to use Orion](./get-started/when-to-use-orion.md)

# Core concepts

- [Architecture and request lifecycle](./concepts/architecture.md)
- [Workflows and data context](./concepts/workflows.md)
- [Channels and ingress](./concepts/channels.md)
- [Connectors and secrets](./concepts/connectors.md)
- [Deployments and revisions](./concepts/deployments.md)
- [Editions and capabilities](./concepts/editions.md)
- [Glossary](./concepts/glossary.md)

# Build services

- [Author workflows](./guides/workflows.md)
- [Configure channels and guards](./guides/channels.md)
- [Connect databases, APIs, and caches](./guides/connectors.md)
- [Schedule jobs with cron](./guides/scheduled-jobs.md)
- [Test and lint offline](./guides/testing.md)
- [CI/CD and deployment workflows](./guides/ci-cd.md)
- [Extend with WebAssembly plugins](./guides/plugins.md)
- [Build with AI assistants](./guides/ai-authoring.md)

# Standard & Enterprise

- [Overview and setup](./enterprise/overview.md)
- [Kafka channels and producer](./enterprise/kafka.md)
- [GraphQL APIs](./enterprise/graphql.md)
- [Server-Sent Events (SSE)](./enterprise/sse.md)
- [Enterprise connector providers](./enterprise/providers.md)
- [Canary rollouts and traffic splitting](./enterprise/traffic-splitting.md)
- [Multi-node clustering and HA](./enterprise/clustering.md)
- [Custom provider packages](./enterprise/provider-packages.md)

# Operate in production

- [Production checklist](./operate/production-checklist.md)
- [Deploy with Docker and Kubernetes](./operate/deploy.md)
- [Security and access control](./operate/security.md)
- [Monitoring, metrics, and alerts](./operate/monitoring.md)
- [Execution traces, DLQ, and audit logs](./operate/traces-and-audit.md)
- [Timeouts, retries, and circuit breakers](./operate/resilience.md)
- [Backups, upgrades, and troubleshooting](./operate/maintenance.md)

# Reference

- [Workflow schema and formatting](./reference/workflows.md)
- [Channel configuration schema](./reference/channels.md)
- [Connector schemas and portable data dialect](./reference/connectors.md)
- [Task functions](./reference/functions.md)
- [Expression language (JSONLogic)](./reference/expressions.md)
- [Static analysis rules (clippy)](./reference/clippy.md)
- [CLI reference](./reference/cli.md)
- [Admin and Data REST API](./reference/api.md)
- [Server configuration and environment variables](./reference/configuration.md)
- [Error codes and metrics](./reference/errors-and-metrics.md)

# Releases

- [Versioning and support policy](./releases/versioning-policy.md)
- [Upgrade to Orion 2.0](./releases/upgrade-to-2.0.md)
- [Orion 1.x release history](./releases/history-1.x.md)
```

Whenever a file path is consolidated or moved, add a redirect in `book.toml` under `[output.html.redirect]` so existing links continue to resolve.

---

## 2. Page Conventions

### 2.1 Front-Matter Comments
Every page in `src/` (except `SUMMARY.md`) must begin with three HTML comment lines:

```markdown
<!-- description: Concise 110-160 character description of the page, unique across the book. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->
```

- **Valid types**: `quickstart`, `tutorial`, `concept`, `guide`, `reference`, `troubleshooting`, `hub`, `release-note`, `migration`, `glossary`.
- **Freshness stamp**: Use `<!-- last_verified: YYYY-MM-DD -->` for authored pages, or `<!-- generated_from: <version> on YYYY-MM-DD -->` for generated pages.
- **Executable tutorials**: Pages that walk through live commands state `Tested with Orion <version>.` in their prerequisites section, matching the pinned Orion tag in `drift/Cargo.toml`.

### 2.2 Code Tabs Order (`CLI` → `curl`)
When showing equivalent operations across interfaces, put **`CLI` first** and **`curl` second** (followed by `Console` if applicable):

```html
<div class="tabs">
<section data-tab="CLI">

```bash
orion-cli send orders -d '{"order_id": "ORD-1001", "total": 25000}'
```

</section>
<section data-tab="curl">

```bash
curl -sS -X POST http://localhost:8080/api/v2/data/orders \
  -H "Content-Type: application/json" \
  -d '{"data": {"order_id": "ORD-1001", "total": 25000}}'
```

</section>
</div>
```

### 2.3 Edition Marking (`Standard+`)
Orion Community is the unmarked default. Features available only in Orion Standard and Enterprise (`orion-ext`) are documented in the **Standard & Enterprise** section and marked in shared reference tables or headings with:

```html
<span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>
```

- Always state the edition requirement in plain prose in the opening paragraph of a gated section or page (for example, *"Available in Orion Standard and Enterprise (`orion-ext`)."*).
- Do not place `<span class="edition-badge ...">` inside an `# H1` heading.
- Never use an `Enterprise` badge—Orion Standard and Enterprise ship as the same `orion-ext` binary and share the same feature set.

---

## 3. Building and Verifying Locally

Run these checks before opening a pull request:

```bash
bash build.sh                               # Build the mdBook, llms-full.txt, and SEO metadata
bash lint.sh                                # Check links, anchors, JSON examples, descriptions, and page structure
vale sync && vale src                       # Lint prose for buzzwords, ease words, and spelling consistency
cargo test --manifest-path drift/Cargo.toml # Verify docs against the pinned Orion codebase
```

### Updating the Pinned Orion Release (`drift/Cargo.toml`)

The Rust drift guards in `drift/tests/drift/` verify that our documentation matches the Orion codebase (`AppConfig` defaults, `ORION_*` environment variables, built-in task functions, JSONLogic operators, `clippy` rules, OpenAPI routes, error codes, audit log actions, and Prometheus metrics).

To update the book for a new Orion release:
1. Update the `tag` on all `orion-*` dependencies in `drift/Cargo.toml` and run `cargo update --manifest-path drift/Cargo.toml`.
2. Regenerate the OpenAPI snapshot:
   `cargo run --manifest-path drift/Cargo.toml --bin dump-openapi > openapi.json`
3. Sync any updated example files from `../Orion/examples/` into `examples/`.
4. Update the `Tested with Orion <version>.` lines in the tutorials.

To test against an unreleased local checkout of `../Orion`, add a gitignored `.cargo/config.toml` patch at the root of this repository:

```toml
[patch."https://github.com/GoPlasmatic/Orion"]
orion-api = { path = "../Orion/crates/orion-api" }
orion-authoring = { path = "../Orion/crates/orion-authoring" }
orion-common = { path = "../Orion/crates/orion-common" }
orion-definitions = { path = "../Orion/crates/orion-definitions" }
orion-engine = { path = "../Orion/crates/orion-engine" }
orion-extension-api = { path = "../Orion/crates/orion-extension-api" }
orion-http = { path = "../Orion/crates/orion-http" }
orion-plugin-host = { path = "../Orion/crates/orion-plugin-host" }
orion-runtime = { path = "../Orion/crates/orion-runtime" }
orion-storage = { path = "../Orion/crates/orion-storage" }
```
