<!-- description: Orion is a declarative services runtime in Rust. Define workflows, channels, and connectors as JSON and run them on one governed runtime. -->
<!-- type: hub -->
<!-- last_verified: 2026-11-16 -->

# Orion documentation

Orion is a declarative services runtime written in Rust. Instead of writing and deploying a separate application server for every API, webhook handler, or background job, you define your business logic as a JSON **workflow**, expose it through a **channel**, and connect external systems with **connectors**.

When you compile and apply those files with `orion-cli`, Orion serves the endpoints immediately—handling routing, authentication, rate limiting, payload validation, retries, circuit breaking, execution traces, and zero-downtime rollbacks in a single runtime.

<div class="hero-logo">
  <img src="images/plasmatic-logo.png" alt="Plasmatic logo">
  <p class="hero-sub">Write JSON definitions, test them offline, and deploy live endpoints without rebuilding or restarting your server.</p>
</div>

## How a service looks

An Orion service is defined by files in your repository:

1. **`workflow.json`** — An ordered pipeline of task functions (parsing, data mapping, validation, SQL queries, HTTP calls) controlled by JSONLogic conditions.
2. **`channel.json`** — The ingress endpoint (`POST /orders`, a cron schedule, or a Kafka consumer) and its traffic guards (API key/HMAC/JWT auth, rate limits, deduplication, caching).
3. **`connector.json`** *(optional)* — A reusable connection to PostgreSQL, MySQL, SQLite, an HTTP API, a cache, or an enterprise provider, with credentials referenced via `env://` or `vault://`.

You validate, test, compile, and deploy those files from your terminal or CI pipeline using `orion-cli`:

```bash
orion-cli lint ./orders-service                          # Validate schemas and cross-references offline
orion-cli dry-run -w ./orders-service/workflow.json \
  -i sample-order.json                                   # Run the workflow locally without a server
orion-cli compile ./orders-service -o dist/deployment    # Compile into an immutable deployment
orion-cli deployments apply -d dist/deployment           # Deploy with zero downtime
orion-cli send orders -d '{"order_id": "ORD-1", "total": 25000}'
```

## Where to start

Pick the path that matches what you want to do right now:

| Your goal | Start here |
| :--- | :--- |
| **See Orion working in 5 minutes** | [Quickstart](./get-started/quickstart.md) — Run Orion, create a workflow and channel, test offline, deploy, and call your endpoint. |
| **Install the server and CLI** | [Install and run Orion](./get-started/install.md) — Install `orion-server` and `orion-cli` via Homebrew, shell scripts, Docker, or Cargo. |
| **Build a database-backed API** | [Build a Postgres-backed orders API](./get-started/postgres-orders-api.md) — Validate inputs, query PostgreSQL safely, run offline tests with stubs, and inspect traces. |
| **Evaluate if Orion fits your stack** | [When to use Orion](./get-started/when-to-use-orion.md) — Honest trade-offs and comparisons with API gateways, Temporal/Restate, rule engines, and n8n. |
| **Understand the architecture** | [Architecture and request lifecycle](./concepts/architecture.md) — How requests flow through channels, guards, workflows, and atomic deployment generations. |
| **Use AI to author services** | [Build with AI assistants](./guides/ai-authoring.md) — Let Claude Code or any LLM author, dry-run, and deploy services using the Orion agent skill. |
| **Deploy to production** | [Production checklist](./operate/production-checklist.md) — Security, database sizing, TLS, rate limits, observability, and high-availability clustering. |

## Editions

Orion is available in three editions built on the same core runtime:

- **Orion Community (`orion-server`)**: The open-source single-node runtime under Apache-2.0. Includes REST, HTTP, and Cron channels; HTTP, SQL (PostgreSQL, MySQL, SQLite), and in-memory cache connectors; all 27 built-in task functions; and sandboxed WebAssembly plugins.
- **Orion Standard & Enterprise (`orion-ext`)**: The commercial binary that composes Community with Kafka, GraphQL, and Server-Sent Events (SSE) channels; seven additional connector providers (Kafka, Redis, S3 storage, MongoDB, Elasticsearch, SMTP, and ONNX model inference); canary traffic splitting; and multi-node clustering backed by Redis. Standard and Enterprise ship as the same artifact and differ only in support terms. See [Editions and capabilities](./concepts/editions.md) for the full feature matrix.

## Documentation sections

| Section | What it covers |
| :--- | :--- |
| **[Get started](./get-started/quickstart.md)** | Quickstart, installation, hands-on tutorials, example packages, and architecture fit evaluation. |
| **[Core concepts](./concepts/architecture.md)** | Request lifecycle, workflows, channels, connectors, deployments, editions, and glossary. |
| **[Build services](./guides/workflows.md)** | Authoring workflows, configuring channel guards, querying databases, cron jobs, offline testing, CI/CD, plugins, and AI authoring. |
| **[Standard & Enterprise](./enterprise/overview.md)** | Kafka, GraphQL, and SSE channels, enterprise connector providers, canary rollouts, multi-node clustering, and custom provider packages. |
| **[Operate in production](./operate/production-checklist.md)** | Production checklist, Docker and Kubernetes deployments, security hardening, monitoring, traces, resilience, backups, and troubleshooting. |
| **[Reference](./reference/workflows.md)** | Complete JSON schemas, task functions, JSONLogic operators, clippy rules, CLI flags, REST API endpoints, server config, error codes, and metrics. |
| **[Releases](./releases/versioning-policy.md)** | Versioning and support policy, upgrading to Orion 2.0, and 1.x release history. |

Orion Community is developed in the open at [GoPlasmatic/Orion](https://github.com/GoPlasmatic/Orion). Ask questions in [GitHub Discussions](https://github.com/GoPlasmatic/Orion/discussions) or report bugs in [GitHub Issues](https://github.com/GoPlasmatic/Orion/issues).
