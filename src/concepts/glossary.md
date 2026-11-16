<!-- description: Definitions of core Orion terms: channels, workflows, connectors, connections, deployments, revisions, generations, quarantine, rollouts, and traces. -->
<!-- type: glossary -->
<!-- last_verified: 2026-11-16 -->

# Glossary

Concise definitions of the core terms used across Orion documentation and the CLI.

- **Admin API (Control plane)**: The HTTP management endpoints under `/api/v2/admin/*` used to upload definitions, manage connections, plan and apply deployments, and inspect traces, schedules, and health. Driven from the terminal by `orion-cli`. See [Admin and Data REST API](../reference/api.md).
- **Backpressure**: A per-channel concurrency limit (`max_concurrent_per_node`) shared across all ingresses. When in-flight requests reach the cap, additional requests are shed immediately with HTTP `503`. See [Channel configuration schema](../reference/channels.md).
- **Case file (`*.case.json`)**: An offline regression test file executed by `orion-cli test` that runs a workflow against sample input and connector stubs and asserts expected output paths. See [Test and lint offline](../guides/testing.md).
- **Channel**: A service endpoint that receives incoming traffic (over HTTP/REST, Cron, Kafka, GraphQL, or SSE), enforces ingress guards, and triggers a workflow. See [Channels and ingress](./channels.md).
- **`channel_call`**: The built-in task function that invokes another channel's workflow in-process without a network hop while still enforcing the target channel's guards. See [Task functions](../reference/functions.md).
- **Circuit breaker**: A per-`(channel, connector)` state machine on each node that opens after repeated downstream failures and fails fast with `503 CIRCUIT_OPEN` until a cooldown period elapses. See [Connectors and secrets](./connectors.md).
- **Cluster mode**: A multi-replica deployment topology (<span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>) where nodes coordinate over a shared database and Redis cluster store. See [Multi-node clustering and HA](../enterprise/clustering.md).
- **Cluster store**: The shared low-latency store (`[extensions.redis]` in `orion-ext`) that holds fleet-wide rate-limit counters, deduplication claims, and response caches across cluster nodes.
- **Connection revision**: An immutable, numbered version of a connector's configuration stored on the server (`(connection_id, revision)`), containing public settings and `env://` or `vault://` secret locators. See [Connectors and secrets](./connectors.md).
- **Connector**: A named connection to an external system (`http`, `db`, `cache`, or an edition provider) authored as a JSON file in a definition set and referenced by name in workflows. See [Connectors and secrets](./connectors.md).
- **Convergence**: The state tracking which nodes in an instance or cluster have built and started serving a committed deployment revision. See [Deployments and revisions](./deployments.md).
- **Cron channel**: A channel with `"protocol": "cron"` that runs a workflow on a six-field cron schedule with no external caller, recording each scheduled run as a durable occurrence. See [Schedule jobs with cron](../guides/scheduled-jobs.md).
- **Data API (Data plane)**: The endpoints under `/api/v2/data/*` (and configured `server.data_mounts`) where deployed channels receive requests. See [Admin and Data REST API](../reference/api.md).
- **Data context**: The per-request JSON document shared by all tasks in a workflow run, consisting of three top-level namespaces: `data`, `metadata`, and `temp_data`. See [Workflows and data context](./workflows.md).
- **Deduplication (Idempotency key)**: A channel guard that claims a caller-supplied header value for `window_secs` and rejects duplicate deliveries within that window with `409 Conflict`. See [Configure channels and guards](../guides/channels.md).
- **Definition**: A single workflow, channel, or WebAssembly plugin document stored on the server by the `sha256:` digest of its canonical bytes. See [Deployments and revisions](./deployments.md).
- **Definition set**: A directory of workflow, channel, connector, shared fragment, and plugin files tracked in Git and compiled into a deployment by `orion-cli compile`.
- **Deployment**: A manifest listing every workflow, channel, connection, and plugin revision a server serves, committed as an immutable numbered revision. See [Deployments and revisions](./deployments.md).
- **Deployment head**: The latest committed deployment revision on an Orion instance. Every node reconciles toward serving the head revision.
- **Driver**: A native channel transport or connector backend compiled into the Orion binary (such as `org.orion.postgresql` or `com.orion.kafka`).
- **Edition**: The capability and support tier of an Orion node: `community` (`orion-server`), `standard` (`orion-ext`), or `enterprise` (`orion-ext`). See [Editions and capabilities](./editions.md).
- **Engine**: The in-memory workflow execution runtime (`dataflow-rs`) compiled from the active deployment's workflows and plugins.
- **Fragment**: A reusable sequence of tasks or JSON values defined in a shared file (`constants`, `errors`, `fragments`) and expanded at compile time via `$from`, `use`, `$use`, or `$each`. See [Author workflows](../guides/workflows.md).
- **Generation (`RuntimeGeneration`)**: The combined in-memory snapshot containing both the compiled workflow engine and the channel snapshot (routes and guards), published atomically via `ArcSwap` during hot reload. See [Architecture and request lifecycle](./architecture.md).
- **Hot reload**: Building a new runtime generation in the background and swapping it in atomically so new deployments take effect with zero dropped requests and no process restart.
- **Ingress**: Any entry path through which work reaches a channel: synchronous HTTP, `/async` HTTP, in-process `channel_call`, scheduled `cron` occurrences, or extension drivers (`kafka`, `graphql`, `sse`).
- **Ingress guards**: The security and traffic policies (`rate_limit`, `auth`, `oauth2_login`, `principal_rate_limit`, `origin_allow_list`, `validation_logic`, `deduplication`, `cache`, `backpressure`) enforced on a channel before its workflow runs.
- **Loop**: A workflow configuration that repeats the workflow's task list sequentially up to a bounded `max` iteration count. See [Workflow schema and formatting](../reference/workflows.md).
- **Manifest**: In a deployment, the JSON list of member entities (`workflow`, `channel`, `connection`, `plugin`), their digests, and their `requires` dependencies. In a WebAssembly plugin, the `plugin.toml` file.
- **Occurrence**: A single scheduled execution instant of a cron channel, persisted in the database ledger with status tracking (`pending`, `running`, `completed`, `failed`, `misfired`, `cancelled`). See [Schedule jobs with cron](../guides/scheduled-jobs.md).
- **Operation gates**: Per-connector boolean or method allow-lists under `operations` (such as `"delete": false` or `"methods": ["GET"]`) enforced at the connector level. See [Connectors and secrets](./connectors.md).
- **Plugin**: A sandboxed WebAssembly component (`orion-plugin-sdk`) deployed as a definition that adds custom pure JSON-to-JSON task functions (`<plugin_id>.<function_name>`). See [Extend with WebAssembly plugins](../guides/plugins.md).
- **Portable data dialect**: Orion's backend-neutral JSON query and write language (`data_query` and `data_write`) that lowers to parameterized SQL for PostgreSQL, MySQL, and SQLite. See [Connector schemas and portable data dialect](../reference/connectors.md).
- **Provider**: A connector backend invoked via `connector_call`—either a built-in edition driver (`com.orion.*`) or an admitted custom provider package.
- **Provider package**: A content-addressed bundle declaring a custom connector provider's schemas and workflow/Wasm implementation, managed with `orion-cli provider`. See [Custom provider packages](../enterprise/provider-packages.md).
- **Quarantine**: The state of a channel or workflow that failed to load on a node (for example, due to a missing `env://` secret). Quarantined channels reject traffic while the rest of the deployment continues serving.
- **Revision**: An immutable integer version number (`1`, `2`, `3`, ...) assigned to a deployment, manifest member, or connection.
- **Rollout bucket**: A deterministic hash (`0..99`) of the caller identity used in Orion Standard and Enterprise to split channel traffic across multiple workflow revisions during a canary rollout. See [Canary rollouts and traffic splitting](../enterprise/traffic-splitting.md).
- **Shaped response**: A channel response mode where the workflow explicitly controls the HTTP status code, headers, cookies, and body via `data._orion.response`. See [Configure channels and guards](../guides/channels.md).
- **Stub file**: A JSON file mapping function names and connector IDs to canned responses so `orion-cli dry-run` and `orion-cli test` can run workflows offline without external databases or APIs. See [Test and lint offline](../guides/testing.md).
- **Task**: A single step in a workflow (`id`, `name`, optional `condition`, and `function`).
- **Task group**: A step in `tasks` that wraps a nested `tasks` array under a single shared `condition` and optional `terminal: true` flag.
- **Terminal step**: A task or task group with `"terminal": true`, which stops the workflow cleanly as soon as that step completes.
- **Trace**: The execution record of a workflow run, including status, timing, final output, and optional per-step changes. See [Execution traces, DLQ, and audit logs](../operate/traces-and-audit.md).
- **Trace DLQ (Dead-letter queue)**: The persistence table where failed `/async` executions are stored and retried with backoff.
- **Workflow**: An ordered pipeline of tasks and JSONLogic conditions executed by the Orion engine to process a request or event. See [Workflows and data context](./workflows.md).
