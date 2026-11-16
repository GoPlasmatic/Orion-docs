<!-- description: Understand Orion's architecture: the three core primitives, request lifecycle, atomic generation swaps, capability map, and key engineering design decisions. -->
<!-- type: concept -->
<!-- last_verified: 2026-11-16 -->

# Architecture and request lifecycle

Orion is a declarative services runtime. You describe your services as JSON files in your repository, compile them into an immutable deployment with `orion-cli compile`, and apply that deployment to a running Orion server. The server builds an in-memory execution engine from the deployment and serves traffic immediately without restarting.

---

## The three core primitives

Every Orion service is built from three declarative building blocks:

```orion-diagram
{
  "direction": "LR",
  "nodes": [
    { "id": "Channel",   "label": "Channel",   "sublabel": "endpoint + guards", "type": "channel" },
    { "id": "Workflow",  "label": "Workflow",  "sublabel": "task pipeline",     "type": "service" },
    { "id": "Connector", "label": "Connector", "sublabel": "external system",   "type": "datastore" }
  ],
  "edges": [
    { "from": "Channel",  "to": "Workflow" },
    { "from": "Workflow", "to": "Connector" }
  ]
}
```

The diagram shows how the three primitives connect:

| Primitive | Role | Examples |
| :--- | :--- | :--- |
| **[Channel](./channels.md)** | Receives incoming work, enforces ingress security and traffic policies, and routes to a workflow. | `POST /orders`, `GET /users/{id}`, cron schedule `0 15 2 * * *`, Kafka consumer |
| **[Workflow](./workflows.md)** | Executes business logic as an ordered pipeline of task functions and JSONLogic conditions. | Parse payload → validate → query database → call HTTP API → format response |
| **[Connector](./connectors.md)** | Provides a named, pooled connection to an external system with secret references and safety gates. | PostgreSQL, MySQL, SQLite, REST APIs, Redis, S3, MongoDB, Elasticsearch |

---

## From definition files to a serving node

Your workflows, channels, connectors, and optional WebAssembly plugins live in a **definition set**—a directory of files tracked in Git. An Orion server never edits those files in place. Instead, it serves a committed **[deployment](./deployments.md)**.

```orion-diagram
{
  "direction": "LR",
  "nodes": [
    { "id": "files", "label": "Definition set", "sublabel": "workflow · channel · connector", "type": "service" },
    { "id": "compile", "label": "orion-cli compile", "sublabel": "validates & bundles", "type": "ci" },
    { "id": "apply", "label": "deployments apply", "sublabel": "plan & commit head", "type": "gateway" },
    { "id": "node", "label": "Serving node", "sublabel": "atomic generation swap", "type": "channel" }
  ],
  "edges": [
    { "from": "files", "to": "compile" },
    { "from": "compile", "to": "apply" },
    { "from": "apply", "to": "node" }
  ]
}
```

Every change follows four deterministic steps:

1. **Author definitions on disk**: You write `workflow.json`, `channel.json`, and `connector.json` files (and test them offline with `orion-cli lint`, `clippy`, `dry-run`, and `test`).
2. **Compile offline (`orion-cli compile`)**: The CLI validates every file and cross-reference, expands shared fragments (`$from`, `use`, `$each`), extracts `env://` and `vault://` secret locators, and outputs a content-addressed deployment directory.
3. **Plan and apply (`orion-cli deployments apply -d`)**: The CLI uploads the content-addressed definitions, plans the manifest against the server's current head revision, and commits the new manifest as the next numbered revision under compare-and-swap.
4. **Atomic generation swap**: Every node reconciles to the new deployment head, compiles a fresh runtime generation (both the channel routing/guard estate and the workflow engine), and swaps it in atomically. In-flight requests finish on the generation they started on.

---

## One request's lifecycle

When a request or event arrives at an Orion node, it passes through five stages:

```orion-diagram
{
  "direction": "LR",
  "nodes": [
    { "id": "req", "label": "Request / Event", "type": "service" },
    { "id": "resolve", "label": "Route resolution", "sublabel": "pattern → channel", "type": "gateway" },
    { "id": "guards", "label": "Ingress guards", "sublabel": "rate limit · auth · validation", "type": "ci" },
    { "id": "matcher", "label": "Workflow match", "sublabel": "condition + rollout", "type": "gateway" },
    { "id": "pipeline", "label": "Task pipeline", "sublabel": "ordered execution", "type": "gateway" },
    { "id": "resp", "label": "Response & trace", "type": "channel" }
  ],
  "edges": [
    { "from": "req", "to": "resolve" },
    { "from": "resolve", "to": "guards" },
    { "from": "guards", "to": "matcher" },
    { "from": "matcher", "to": "pipeline" },
    { "from": "pipeline", "to": "resp" }
  ]
}
```

1. **Snapshot the runtime generation**: The ingress loads the current `RuntimeGeneration` once via lock-free `ArcSwap`. Route resolution, guard enforcement, and workflow execution all use this exact generation snapshot.
2. **Resolve the channel**: For HTTP requests under `/api/v2/data/*`, Orion matches the HTTP method and path against deployed REST route patterns (such as `GET /orders/{id}`), falling back to a direct channel name lookup (`/api/v2/data/{channel}`).
3. **Enforce ingress guards**: Before any workflow logic runs, Orion executes the channel's configured guards in a fixed order: rate limiting → authentication → origin allow-list → payload validation (`validation_logic`) → deduplication → response cache lookup → backpressure concurrency permit.
4. **Select the workflow revision**: The channel points to a required workflow (or, during a Standard/Enterprise canary rollout, selects between workflow revisions using a deterministic hash bucket) and checks the workflow's top-level `condition`.
5. **Run the task pipeline and return**: Tasks execute sequentially against a shared JSON context (`data`, `metadata`, `temp_data`). Synchronous channels return the final `data` context (or a custom shaped response) immediately; asynchronous and scheduled runs persist the result to the execution trace store.

### Synchronous, asynchronous, and scheduled execution

```text
Sync HTTP   POST /api/v2/data/orders         → Executes workflow and returns JSON response immediately
Async HTTP  POST /api/v2/data/orders/async   → Enqueues work and returns HTTP 202 with trace_id + trace_token
REST route  GET  /api/v2/data/orders/{id}    → Matches path pattern and exposes {id} in metadata
Cron job    schedule: "0 15 2 * * *"         → Claims a durable occurrence on schedule with no HTTP caller
```

---

## Development vs. production topology

The same binary runs locally on a laptop and in a production fleet—only configuration and backing stores change:

| Dimension | Local Development | Production |
| :--- | :--- | :--- |
| **Deployment flow** | `orion-cli compile` and `deployments apply -d` from your terminal | Automated in CI/CD with PR lint/test gates and staged `plan` / `apply` |
| **State database** | Embedded SQLite (`./orion.db`), zero external setup | PostgreSQL or MySQL with connection pooling |
| **Database migrations** | Applied automatically at startup (`auto_migrate = true`) | Run once as a pre-deploy step (`orion-server migrate`) with `auto_migrate = false` |
| **High availability** | Single process | Multiple stateless replicas behind a load balancer + Redis cluster store (<span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>) |

---

## Runtime capability map

Everything in the map below is built into the Orion runtime—click any node to expand its capabilities or jump to the documentation page that covers it:

```orion-mindmap
{
  "centre": "Orion",
  "height": 680,
  "branches": [
    {
      "name": "Observability", "type": "observability",
      "children": [
        { "name": "Structured logging", "link": "operate/monitoring.html",
          "children": ["JSON or pretty output", "Five levels", "Per-crate RUST_LOG control", "x-request-id on every request"] },
        { "name": "Prometheus metrics", "link": "operate/monitoring.html",
          "children": ["Request counts & error rates", "Latency histograms", "Engine & queue gauges", "Breaker state", "Rate-limit rejections"] },
        { "name": "Distributed tracing", "link": "operate/monitoring.html",
          "children": ["W3C trace context", "OTLP export", "Configurable sample rate", "One span per task"] },
        { "name": "Health endpoints", "link": "operate/monitoring.html",
          "children": ["/healthz — liveness", "/readyz — readiness", "/health — component state", "Two-tier detail"] },
        { "name": "Execution traces", "link": "operate/traces-and-audit.html",
          "children": ["Per-task input, output, timing", "sync / async / batch / off", "Per-channel override"] }
      ]
    },
    {
      "name": "Resilience", "type": "infra",
      "children": [
        { "name": "Timeouts", "link": "operate/resilience.html",
          "children": ["Channel timeout", "Connector query & connect", "HTTP client backstop", "Per-ingress ceilings"] },
        { "name": "Retries", "link": "operate/resilience.html",
          "children": ["HTTP connectors only", "Exponential backoff, capped 60s", "Idempotent methods only"] },
        { "name": "Circuit breakers", "link": "operate/resilience.html",
          "children": ["Per connector", "Cooldown before probing", "Off by default", "Inspect & reset via API"] },
        { "name": "Dead letter queue", "link": "operate/traces-and-audit.html",
          "children": ["Failed async traces", "Automatic retry", "Requeue through the API"] },
        { "name": "Graceful shutdown", "link": "operate/resilience.html",
          "children": ["SIGTERM / SIGINT", "Bounded drain window", "Force timeout"] },
        { "name": "Panic recovery", "link": "operate/resilience.html",
          "children": ["Request fails, process lives"] }
      ]
    },
    {
      "name": "Security", "type": "gateway",
      "children": [
        { "name": "Admin authentication", "link": "operate/security.html",
          "children": ["Constant-time comparison", "sha256: digests", "Several keys for rotation", "Startup error in production"] },
        { "name": "Data-plane authentication", "link": "operate/security.html",
          "children": ["api_key per channel", "hmac — templates & presets", "jwt with claims in context", "Uniform 401"] },
        { "name": "Secrets by reference", "link": "concepts/connectors.html",
          "children": ["env:// and vault://", "References only in a deployment", "OAuth2 tokens encrypted at rest"] },
        { "name": "Payload validation", "link": "guides/channels.html",
          "children": ["JSONLogic rules per channel", "Body size limit", "Runs before any logic"] },
        { "name": "Egress control", "link": "operate/security.html",
          "children": ["SSRF private-IP blocking", "Per-connector opt-in", "Operation gates"] },
        { "name": "Query safety", "link": "reference/connectors.html",
          "children": ["Parameterized queries", "Dialect guards on db"] },
        { "name": "Transport & headers", "link": "operate/security.html",
          "children": ["TLS termination", "CSP, X-Frame-Options, HSTS", "Server-side origin checks"] }
      ]
    },
    {
      "name": "Scalability", "type": "ci",
      "children": [
        { "name": "Rate limiting", "link": "reference/channels.html",
          "children": ["Token bucket with burst", "JSONLogic client key", "Per-channel and platform-wide", "429 on an empty bucket"] },
        { "name": "Backpressure", "link": "reference/channels.html",
          "children": ["Per-channel semaphore", "503 load shedding", "Shared across ingresses"] },
        { "name": "Deduplication", "link": "reference/channels.html",
          "children": ["Idempotency key window", "Sync and async HTTP", "Shared store when clustered"] },
        { "name": "Async processing", "link": "operate/traces-and-audit.html",
          "children": ["Trace id returned at once", "Bounded worker queue", "DLQ behind it"] },
        { "name": "Cluster mode", "link": "enterprise/clustering.html",
          "children": ["N replicas, one system", "Shared store for limits and caches", "Epoch config propagation", "Leased scheduled jobs"] },
        { "name": "Database backends", "link": "reference/configuration.html",
          "children": ["SQLite embedded", "PostgreSQL", "MySQL", "Chosen by URL scheme"] }
      ]
    },
    {
      "name": "Deployability", "type": "service",
      "children": [
        { "name": "Single binary", "link": "get-started/install.html",
          "children": ["Embedded database", "Homebrew tap", "Shell & PowerShell installers", "Multi-platform binaries"] },
        { "name": "Containers", "link": "operate/deploy.html",
          "children": ["Multi-stage image", "Non-root execution", "Health probes", "Reference HA compose"] },
        { "name": "Kubernetes", "link": "operate/deploy.html",
          "children": ["Community Helm chart", "Liveness & readiness probes", "Migrations as a Job", "Mounted edition config"] },
        { "name": "Configuration", "link": "reference/configuration.html",
          "children": ["TOML file", "ORION_SECTION__KEY overrides", "Defaults for everything", "Misspelling is a startup error"] },
        { "name": "Pre-flight checks", "link": "guides/testing.html",
          "children": ["validate-config", "test-connectivity", "lint"] }
      ]
    },
    {
      "name": "Extensibility", "type": "connector",
      "children": [
        { "name": "Connector types", "link": "concepts/connectors.html",
          "children": ["http", "db — PG / MySQL / SQLite", "cache — per-node memory"] },
        { "name": "Task functions", "link": "reference/functions.html",
          "children": ["Parse, map, filter, validate", "HTTP calls", "Portable dialect & raw SQL", "Cache read and write", "Hashing, HMAC and JWT"] },
        { "name": "Portable data dialect", "link": "reference/connectors.html",
          "children": ["One filter syntax", "Lowered to PostgreSQL", "Lowered to MySQL", "Lowered to SQLite"] },
        { "name": "Channel protocols", "link": "concepts/channels.html",
          "children": ["REST with route patterns", "Plain HTTP by name", "Cron schedules", "Sync or async"] },
        { "name": "In-process composition", "link": "reference/functions.html",
          "children": ["No network hop", "No serialization", "Callee guards still apply"] }
      ]
    },
    {
      "name": "Availability", "type": "workflow",
      "children": [
        { "name": "Hot reload", "link": "concepts/deployments.html",
          "children": ["Atomic engine swap", "In-flight requests unaffected", "No restart"] },
        { "name": "Immutable revisions", "link": "concepts/deployments.html",
          "children": ["A committed revision never changes", "Change is a new revision", "History kept"] },
        { "name": "Rollback", "link": "concepts/deployments.html",
          "children": ["Commit an earlier revision", "History moves forward", "Nothing to rebuild"] },
        { "name": "Response caching", "link": "reference/channels.html",
          "children": ["Keyed from the request", "Memory or the shared store", "Never reaches the workflow"] },
        { "name": "Connection pooling", "link": "concepts/connectors.html",
          "children": ["Pools cached per connector", "Reused across requests"] }
      ]
    },
    {
      "name": "Maintainability", "type": "channel",
      "children": [
        { "name": "Admin API", "link": "reference/api.html",
          "children": ["Definitions and connections", "Plan, apply, roll back", "Engine status", "Convergence"] },
        { "name": "OpenAPI", "link": "reference/api.html",
          "children": ["Full admin surface", "Swagger UI outside production", "Closed in production"] },
        { "name": "Offline testing", "link": "guides/testing.html",
          "children": ["Lint a workflow file", "Dry-run with stubs", "*.case.json suites", "No server needed"] },
        { "name": "Deployments", "link": "concepts/deployments.html",
          "children": ["compile, plan, apply", "Content-addressed definitions", "Revision history"] },
        { "name": "CI/CD", "link": "guides/ci-cd.html",
          "children": ["Offline PR gates", "Plan-then-apply pipelines", "Rollback by revision"] },
        { "name": "Audit logs", "link": "operate/traces-and-audit.html",
          "children": ["Who changed what, when", "Filterable", "Bounded retention", "Change-context grouping"] },
        { "name": "Backups", "link": "operate/maintenance.html",
          "children": ["SQLite backup via API", "Offline restore procedure", "PG/MySQL snapshot tooling"] }
      ]
    }
  ]
}
```

---

## Core engineering design notes

The sections below explain the rationale behind Orion's key runtime behaviors.

### 1. How hot reload swaps runtime generations (`ArcSwap`)
When a deployment revision is applied, Orion builds a complete `RuntimeGeneration` off to the side while existing traffic continues running. A generation bundles **both** the compiled `dataflow-rs` workflow engine and the `ChannelSnapshot` (route table, rate limiters, compiled validation predicates, deduplication stores, response caches, and backpressure semaphores) into a single immutable value held in an `Arc<ArcSwap<RuntimeGeneration>>`.

Why bundle both into one atomic pointer swap? If the channel routing table and the workflow engine were updated in two separate steps, a request arriving between the two updates could match a new channel whose workflow did not exist yet (or reach a workflow before its channel guards were active). Loading the combined generation once at request entry guarantees that routing, guards, and workflow execution always agree.

### 2. Why ingress guards run in a fixed order
Every transport passes through `apply_guards` in a strict, deliberate sequence:
1. **Rate limit (`rate_limit`)**: Runs first so rejected traffic is cheap and still consumes rate-limit tokens (preventing unmetered brute-force probing of downstream checks).
2. **Authentication (`auth`, then `principal_rate_limit`)**: Runs before any stateful store (deduplication or cache) so unauthenticated callers cannot probe cached responses or claim idempotency keys belonging to legitimate users.
3. **Origin allow-list (`origin_allow_list`) and payload validation (`validation_logic`)**: Rejects malformed or unauthorized origins before touching shared state.
4. **Deduplication (`deduplication`)**: Runs before the response cache so a replayed idempotency key receives `409 Conflict` rather than a cached `200 OK`.
5. **Response cache (`cache`)**: Serves cached responses before acquiring a concurrency permit.
6. **Backpressure (`backpressure`)**: Acquires a concurrency permit last, only when the workflow is actually about to execute. If backpressure sheds the request with `503`, Orion releases any deduplication claim made in step 4 so the caller can retry cleanly.

### 3. Why deduplication claims stand for the full window
Once a request passes ingress guards, its idempotency key is claimed for the configured `window_secs` regardless of whether the workflow later succeeds or fails. Only a guard rejection *before* execution (such as `503` backpressure shedding) releases the claim. If a caller retries a failed workflow execution, it must supply a new idempotency key or wait for the window to expire. In-process `channel_call` invocations are never deduplicated so a parent workflow can call a child channel multiple times in a loop.

### 4. Why response-cache keys ignore HTTP headers
Orion computes response cache keys from the channel ID, HTTP method, route parameters, query string, and request payload (or the subset of fields listed in `cache_key_fields`). Request headers are never hashed into the cache key. If a workflow's output varies based on a header (such as a tenant ID or locale header), pass that value in the payload and include it in `cache_key_fields`, or disable response caching on that channel.

### 5. Why `trusted_proxies` defaults to empty
`X-Forwarded-For` and `X-Real-IP` headers can be forged by any client connecting directly to Orion. By default, `rate_limit.trusted_proxies` is empty and Orion uses the direct peer socket IP for rate limiting, admin auth backoff, and audit logging. When running behind a reverse proxy or load balancer, add the proxy's CIDR block to `rate_limit.trusted_proxies`; Orion then extracts the rightmost non-trusted IP from `X-Forwarded-For`.

### 6. Why `/async` timeouts clamp to the trace queue deadline
On synchronous HTTP calls, a channel's `timeout_ms` applies directly because only the caller's connection waits. On `/async` submissions, execution runs on a shared pool of background trace-queue workers (`trace_queue.workers`). To prevent one misconfigured channel from starving all async workers, `/async` clamps `timeout_ms` to `trace_queue.processing_timeout_ms`.

### 7. Why list endpoints use cursor pagination
All v2 list endpoints (`/deployments`, `/traces`, `/audit-logs`, `/cron/occurrences`, `/connections`) paginate newest-first using an opaque `next_cursor` encoding `(created_at, id)` rather than `OFFSET`. Keyset cursors execute as an index seek with constant latency at any depth and never skip or duplicate rows when new records are inserted concurrently. Total row counts (`total`) require a table scan on PostgreSQL and MySQL and are computed only when `?include_total=true` is requested.

## Next steps

- [Workflows and data context](./workflows.md) — Learn how tasks, JSONLogic conditions, and the shared context work.
- [Channels and ingress](./channels.md) — Explore protocols, sync/async modes, and ingress guards.
- [Connectors and secrets](./connectors.md) — See how Orion manages external connections, secrets, and circuit breakers.
- [Deployments and revisions](./deployments.md) — Understand content-addressed definitions and atomic rollouts.
