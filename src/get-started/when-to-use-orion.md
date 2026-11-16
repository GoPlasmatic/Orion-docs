<!-- description: Evaluate when Orion is the right fit for your architecture and how it compares to API gateways, Temporal, Restate, n8n, OPA, Drools, and dataflow-rs. -->
<!-- type: concept -->
<!-- last_verified: 2026-11-16 -->

# When to use Orion

Orion is a declarative services runtime: you define business logic as a JSON workflow, expose it through a channel, and connect external systems through connectors. Because it sits at the intersection of API development, workflow execution, and rule evaluation, it helps to be clear about where Orion excels, where it is the wrong tool, and how it pairs with neighboring systems.

---

## Quick fit check

### When Orion is a strong fit
- **Your service logic fits an ordered pipeline**: Parse input, validate fields, query or write a database, call an upstream HTTP API, transform the result, and return a response.
- **You want consistent operational guardrails across many endpoints**: Rate limiting, API key/HMAC/JWT authentication, payload validation, deduplication, response caching, retries, circuit breakers, Prometheus metrics, and execution traces are built in and configured per channel or connector.
- **You want instant, reversible deployments**: Deploying a new service or changing logic is a reviewed JSON deployment (`orion-cli compile` → `deployments apply`) that hot-reloads in milliseconds with zero downtime and one-command rollback.
- **Developers or AI coding agents author services**: Because every service is declarative JSON validated by strict schemas (`orion-cli lint`, `clippy`, `dry-run`, `test`), AI assistants can safely generate, test, and deploy services under human review.
- **Work is request-, event-, or schedule-driven**: HTTP/REST APIs, webhooks, cron jobs (and in Standard/Enterprise: Kafka consumers, GraphQL APIs, and SSE streams) that complete within a bounded timeout.

### When to choose another tool (or pair it with Orion)
- **Work spans hours or days, or waits on human approval**: Orion executes each request or event as a bounded run and does not journal intermediate steps to resume mid-pipeline after a crash. Use a **durable execution engine** like Temporal or Restate.
- **You need general-purpose code with arbitrary I/O**: Sandboxed WebAssembly plugins in Orion handle pure JSON-to-JSON computation (such as custom binary/text codecs or complex math), but cannot open sockets or read files. Put general-purpose I/O services in your application language and call them from Orion via `http_call`.
- **You need heavy compute on the hot path**: Video/image transcoding, large in-memory joins, or training ML models belong in dedicated compute workers. (Lightweight CPU ONNX inference is supported in Orion Standard/Enterprise via `com.orion.models`).
- **You need a drag-and-drop SaaS connector catalog for non-developers**: Orion is built for engineering teams managing definitions in Git, not a browser-based no-code builder with 400 SaaS OAuth templates.

---

## Learning paths by use case

| What you want to build | Start here | Next guides to read | Production checklist |
| :--- | :--- | :--- | :--- |
| **REST API or decision service** | [Build your first service](./first-service.md) | [Author workflows](../guides/workflows.md) · [Configure channels](../guides/channels.md) | [Security and access control](../operate/security.md) |
| **Database-backed CRUD or enrichment API** | [Build a Postgres-backed orders API](./postgres-orders-api.md) | [Connect databases, APIs, and caches](../guides/connectors.md) | [Timeouts, retries, and circuit breakers](../operate/resilience.md) |
| **Webhook receiver & normalizer** | [Run the example packages (`webhook-transform`)](./examples.md) | [Configure channels (HMAC auth & dedup)](../guides/channels.md) | [Execution traces, DLQ, and audit logs](../operate/traces-and-audit.md) |
| **Scheduled background job** | [Schedule jobs with cron](../guides/scheduled-jobs.md) | [Channels and ingress](../concepts/channels.md) | [Monitoring, metrics, and alerts](../operate/monitoring.md) |
| **Kafka consumer, GraphQL, or SSE stream** <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Standard & Enterprise overview](../enterprise/overview.md) | [Kafka](../enterprise/kafka.md) · [GraphQL](../enterprise/graphql.md) · [SSE](../enterprise/sse.md) | [Multi-node clustering and HA](../enterprise/clustering.md) |
| **AI-authored microservice** | [Build with AI assistants](../guides/ai-authoring.md) | [Test and lint offline](../guides/testing.md) | [CI/CD and deployment workflows](../guides/ci-cd.md) |

---

## How Orion compares to neighboring tools

| Category | Examples | Relationship to Orion | Summary |
| :--- | :--- | :--- | :--- |
| **Custom microservices** | Go, Rust, Spring Boot, FastAPI, Express | **Replaces** (for pipeline-shaped services) | Replaces boilerplate CRUD, webhook, and orchestration services with declarative JSON definitions on one shared runtime. |
| **[Durable execution engines](#orion-vs-durable-execution-engines)** | Temporal, Restate, AWS Step Functions, Airflow | **Pairs with** | Durable engines journal every step to survive crashes and sleep for days; Orion runs bounded request/event pipelines in milliseconds. |
| **[API gateways](#orion-vs-api-gateways)** | Kong, Envoy, APISIX, KrakenD | **Pairs with** | Gateways route and police traffic at the edge in front of upstreams; Orion *is* the upstream service that executes the business logic. |
| **[Automation platforms](#orion-vs-automation-platforms)** | n8n, Zapier, Make, Node-RED | **Different job** | Automation tools optimize for browser-based SaaS wiring at low volume; Orion is engineered for high-throughput production traffic and Git/CI workflows. |
| **[Rule engines](#orion-vs-rule-engines)** | OPA, Drools, GoRules | **Overlaps** (on single-payload decisions) | Rule engines evaluate unordered rule sets or recursive inference; Orion runs ordered JSONLogic pipelines combined with database/API calls. |
| **[Embedding `dataflow-rs`](#orion-vs-embedding-dataflow-rs)** | `dataflow-rs` crate | **Sits under Orion** | `dataflow-rs` is the Rust workflow engine library inside Orion. Embed the crate when you want an in-process library; run Orion when you want a full server. |

---

## Orion vs. durable execution engines

*(Compared against Temporal 1.31 and Restate 1.7)*

A durable execution engine journals the result of every step so a workflow can survive a process crash, sleep for days, or wait for a human callback and resume from the exact step where it paused.

Orion does not journal intermediate steps for resumption. It runs a bounded pipeline within a request or scheduled occurrence and records a completed [execution trace](../operate/traces-and-audit.md). If a run fails and is retried (for example, via the async trace dead-letter queue or a cron retry), Orion runs the workflow again from the first task.

| Dimension | Durable Execution Engines (Temporal, Restate) | Orion |
| :--- | :--- | :--- |
| **Core mechanism** | Replay/journal log that resumes execution mid-workflow | In-memory task pipeline with post-execution trace storage |
| **Unit of work** | Resumable workflow execution (milliseconds to months) | Single HTTP request, Kafka message, or cron occurrence (milliseconds to seconds) |
| **Authoring model** | Code written in TypeScript, Go, Java, Python, or Rust | Declarative JSON definitions compiled and deployed via CLI/API |
| **State model** | Engine-managed durable timers, awakeables, and keyed virtual objects | Per-request `data` context; persistent state lives in your SQL/Redis/S3 connectors |
| **Deployment** | Build and deploy worker binaries alongside the orchestrator | Single binary; definitions hot-reload atomically with no worker fleet |

### When to use each—and how to run both
- **Use Temporal or Restate** when a workflow must sleep for hours, wait for an external human approval webhook, or guarantee that step 3 is never re-executed if step 4 crashes after moving money.
- **Use Orion** when the work completes within a single request or event, and you want the HTTP endpoint, ingress guards, database queries, and versioned deployments in one lightweight runtime.
- **Running both together**: An Orion channel can validate and normalize an incoming request, then trigger a long-running Temporal or Restate workflow via `http_call` and return the run ID immediately. Conversely, a Temporal activity can call an Orion channel to execute hot-reloadable business rules without redeploying worker code.

---

## Orion vs. API gateways

*(Compared against Kong 3.9 and Envoy 1.39)*

An API gateway is a reverse proxy that routes traffic to upstream services. Orion **is** the upstream service: it terminates the request, executes the business logic workflow, queries databases, and produces the response.

| Dimension | API Gateways (Kong, Envoy, APISIX) | Orion |
| :--- | :--- | :--- |
| **Role in the stack** | Edge proxy routing traffic to backend services | Backend service runtime that executes workflows and returns responses |
| **Business logic** | None (or lightweight header/body transforms) | Full ordered task pipeline (SQL queries, HTTP calls, validation, branching) |
| **Traffic guards** | Fleet-wide TLS, WAF, mTLS, OIDC, IP rate limiting | Per-channel API key/HMAC/JWT/OAuth2 auth, JSONLogic payload rate limiting & validation, dedup, caching |

### When to use each—and how to run both
- **Keep an API gateway in front of Orion** when you run multiple backend services across teams, need edge TLS/mTLS termination, WAF/DDoS protection, or fleet-wide routing.
- **Use Orion's channel guards** for service-specific policies—especially rules that depend on the parsed JSON payload (such as rate limiting by `data.tenant_id`, validating business fields with JSONLogic before hitting the database, or caching responses by specific payload fields).
- When running Orion behind a gateway or load balancer, configure [`rate_limit.trusted_proxies`](../operate/security.md) in `orion.toml` so Orion reads the real client IP from `X-Forwarded-For` rather than rate-limiting the proxy's IP.

---

## Orion vs. automation platforms

*(Compared against n8n 2.34, Zapier, and Make)*

Workflow automation platforms are designed for wiring SaaS applications together quickly in a visual drag-and-drop editor. Orion is designed for software engineers running production APIs and event pipelines with high throughput, low latency, and Git-based CI/CD governance.

| Dimension | Automation Platforms (n8n, Zapier) | Orion |
| :--- | :--- | :--- |
| **Primary audience** | Operations teams,citizen integrators, rapid prototyping | Backend/platform engineers and AI coding agents |
| **Authoring & CI/CD** | Visual canvas in a browser | JSON files in Git, validated offline with `fmt`, `lint`, `clippy`, `dry-run`, and `test` |
| **Connectors** | Hundreds of pre-built SaaS app nodes with browser OAuth | High-performance pooled drivers for SQL, HTTP, Redis, Kafka, S3, MongoDB, Elasticsearch, SMTP, and ONNX |
| **Performance** | Seconds per flow; tens to thousands of runs per day | Single-digit millisecond overhead; thousands of requests per second per node |
| **Rollouts & rollback** | Manual edits in the UI | Immutable content-addressed deployments, compare-and-swap apply, canary splits, one-command rollback |

---

## Orion vs. rule engines

*(Compared against Open Policy Agent [OPA] 1.19 and Drools 10.2)*

Rule engines evaluate a collection of rules against input facts. Forward-chaining engines (like Drools) re-fire rules automatically as new facts are inferred; policy engines (like OPA/Rego) evaluate hierarchical authorization policies.

Orion evaluates [JSONLogic expressions](../reference/expressions.md) in the exact sequential order your workflow declares.

| Dimension | Rule Engines (OPA, Drools) | Orion |
| :--- | :--- | :--- |
| **Execution model** | Inference/RETE graph (Drools) or declarative policy query (OPA) | Sequential pipeline of tasks, task groups, and JSONLogic conditions |
| **Side effects & I/O** | Pure evaluation (caller must fetch data first or act on the decision after) | Integrated: the same workflow fetches data from Postgres/HTTP, evaluates rules, writes results, and responds |
| **Validation collection** | Policy allow/deny or mutated working memory | Built-in `validation` task collects all rule failures into `errors[]` or halts with `on_rejection: "halt"` |

- **Choose OPA or Drools** when you have hundreds of interdependent rules where asserting one fact must recursively re-trigger earlier rules, or when you need a unified Rego authorization sidecar across polyglot services.
- **Choose Orion** when your decision logic is a deterministic forward pass over a request payload combined with database lookups, external API calls, and response formatting.

---

## Orion vs. embedding `dataflow-rs`

*(Compared against `dataflow-rs` 4.0)*

[`dataflow-rs`](https://github.com/GoPlasmatic/dataflow-rs) is the open-source Rust workflow engine crate that powers Orion. Choosing between them is a **library vs. runtime** decision:

- **Embed `dataflow-rs` directly in your Rust application** when you want an in-process JSONLogic workflow evaluator inside an existing binary, or when you need to write custom Rust task handlers that perform arbitrary network/filesystem I/O.
- **Run Orion** when you want a complete server around that engine: HTTP/REST/Cron/Kafka/GraphQL/SSE channels, ingress security guards, pooled SQL/HTTP/cache connectors with circuit breakers, immutable deployment revisions with zero-downtime hot reload, execution traces, Prometheus metrics, and `orion-cli`.

## Next steps

- [Quickstart](./quickstart.md) — Try Orion locally in five minutes.
- [Architecture and request lifecycle](../concepts/architecture.md) — See how channels, guards, workflows, and connectors work under the hood.
- [Editions and capabilities](../concepts/editions.md) — Compare Orion Community with Orion Standard and Enterprise.
