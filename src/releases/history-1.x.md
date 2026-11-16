<!-- description: Historical release and upgrade notes for Orion 1.x (1.8.0, 1.6.0, 1.2.0, 1.1.0, and 1.0.0), preserved for teams migrating older estates to Orion 2.0. -->
<!-- type: migration -->
<!-- last_verified: 2026-11-16 -->

# Orion 1.x release history

This page consolidates the behavioral changes, breaking renames, and migration notes across the Orion 1.x release series (`1.0.0` through `1.8.0`). If you are migrating an older 1.x or 0.3.x deployment to Orion 2.0, review the sections below alongside [Upgrade to Orion 2.0](./upgrade-to-2.0.md).

---

## Orion 1.8.0

Orion 1.8.0 enabled the 20 tensor operators in the JSONLogic expression engine and added the `engine.ops_budget` evaluation ceiling.

### 1. Twenty tensor operator names became live in templates

Enabling the `tensor` feature in `dataflow-rs` made 20 operator names active in template positions (`map` mapping `logic` and expression-typed function inputs): `tensor`, `zeros`, `full`, `scatter`, `rle_expand`, `one_hot`, `stack`, `concat`, `unstack`, `reshape`, `transpose`, `pad`, `crop`, `cast`, `normalize`, `argmax`, `gather`, `to_list`, `shape`, and `dtype`.

In template positions, a single-key object whose key matches an active operator is evaluated as an operator call rather than emitted as a literal object. If a workflow previously emitted a single-key literal object using one of those names, escape the key with a leading `$`:

```json
{ "path": "data.board", "logic": { "$shape": [6, 7] } }
```

See [Expression language (JSONLogic)](../reference/expressions.md) for the full operator table and `$` escaping rules.

### 2. `engine.ops_budget` added (default `0`)

The `[engine]` section gained `ops_budget = 0` (`0` disables the ceiling). When set to a positive integer, expressions that exceed the node budget fail their task with `BUDGET_EXCEEDED` (or evaluate to `false` in routing/task conditions). See [Server configuration and environment variables](../reference/configuration.md).

---

## Orion 1.6.0

Orion 1.6.0 introduced the WebAssembly plugin sandbox (`[plugins]`, off by default), scheduled cron channels (`[cron]`, on by default), stricter `db_read` statement validation, and concrete SQL driver decoding.

### 1. `db_read` enforces read-only SQL statements

Previously, `db_read` checked the connector's `read` permission gate and passed the raw SQL string to the database driver, which allowed `DELETE`, `UPDATE`, or `INSERT` statements (including `DELETE ... RETURNING`) to execute. From 1.6.0 onward, `db_read` statements must begin with `SELECT`, `WITH` (without data-modifying CTEs), `VALUES`, or `TABLE`. Move any mutating SQL statements to `db_write` or `data_write`.

### 2. Concrete SQL driver decoding (`numeric_as` and `binary_as`)

SQL connectors switched from `sqlx::AnyPool` to concrete PostgreSQL, MySQL, and SQLite drivers so `uuid`, `numeric`, `timestamptz`, `jsonb`, arrays, and `inet` columns decode cleanly.

- **`numeric` / `decimal`**: Decodes to a JSON number (`f64`) by default. Set `numeric_as: "string"` on `db_read` or `data_query` to preserve exact decimal digits on currency columns.
- **Binary columns (`bytea`, `blob`, `varbinary`)**: Set `binary_as` (`"hex"` or `"base64"`) explicitly rather than relying on UTF-8 fallback detection.

### 3. Trace token query parameter (`?token=`) deprecated

Passing an async trace capability token in the URL query string (`?token=...`) was deprecated in favor of the `x-trace-token` HTTP header so capability tokens do not leak into proxy access logs or browser history. Trace responses also added `Cache-Control: no-store`.

---

## Orion 1.2.0

Orion 1.2.0 added recursive task groups (`children`, `mode`, `terminal`), shared definition fragments, and directory-level static analysis (`lint`), while tightening test case paths and removing the legacy CLI MCP server.

### 1. Test case `expect` paths must include a root segment

In `*.case.json` files used by `orion-cli test`, every key in `expect` must start with a valid message root (`data.`, `metadata.`, `temp_data.`, `calls`, or `audit_trail`). Previously, unrooted paths implicitly defaulted to `data.`, which caused `"metadata.foo": null` to read `data.metadata.foo` and pass silently. See [Test and debug locally](../guides/testing.md).

### 2. Empty workflow task lists (`"tasks": []`) rejected

Workflows with an empty `tasks` array are rejected at validation and compile time (`tasks` must contain at least one step).

### 3. CLI MCP server (`orion-cli mcp serve`) removed

The unauthenticated `orion-cli mcp serve` subcommand was removed in 1.2.0. To use AI coding assistants with Orion, install the agent skill or prompt pack so the assistant runs `orion-cli` directly under your shell credentials. See [AI-assisted authoring](../guides/ai-authoring.md).

### 4. Fragment task IDs inside task groups namespaced (1.2.1+)

When a shared fragment contains a nested task group, every task ID inside the group is prefixed flatly with `{call-site}.{id}` so multiple inclusions cannot collide with `DUPLICATE_TASK_ID`.

---

## Orion 1.1.0

Orion 1.1.0 added JWT channel authentication, managed OAuth2 client credentials, and specific task error codes.

### 1. Unresolved `rate_limit.key_logic` headers return `429`

If a channel's `rate_limit.key_logic` evaluates to `null` or `""` (for example, because it reads a custom request header not listed in `rate_limit.key_headers`), the request is rejected with `429 RATE_LIMITED` and increments `orion_rate_limit_key_unavailable_total` instead of silently bucketing all callers under the literal string `"null"`. Eight headers are always available in the rate-limit key context (`authorization`, `x-api-key`, `x-forwarded-for`, `x-real-ip`, `user-agent`, `content-type`, `origin`, and `x-tenant-id`); list any additional headers in `rate_limit.key_headers`.

### 2. Specific `errors[].code` values replace generic `TASK_ERROR`

When a task fails on a channel with `continue_on_error: true`, `errors[].code` and `metadata._orion_errors` report the specific failure category (`IO_ERROR`, `TIMEOUT_ERROR`, `FUNCTION_ERROR`, or `circuit_open`) instead of always reporting `TASK_ERROR`.

### 3. Connector header values masked and error URLs redacted

Admin API reads mask all values inside `headers`, `query_params`, and `extra_params` regardless of key name, and `http_call` error messages redact connector URLs and truncate upstream error bodies to 512 bytes.

---

## Orion 1.0.0

Orion 1.0.0 hardened configuration validation, SSRF protection, channel configuration schemas, and the portable data dialect over the 0.3.0 release.

### 1. Renamed and removed configuration keys

Four TOML sections and one environment variable were renamed in 1.0.0, and two obsolete settings were removed:

| Pre-1.0 setting | 1.0+ replacement |
|---|---|
| `[queue]` (`ORION_QUEUE__WORKERS`, etc.) | `[trace_queue]` (`ORION_TRACE_QUEUE__WORKERS`, etc.) |
| `queue.trace_retention_hours` | `trace_queue.retention_hours` |
| `queue.trace_cleanup_interval_secs` | `trace_queue.cleanup_interval_secs` |
| `queue.audit_retention_days` | `audit.retention_days` (plus `audit.cleanup_interval_secs`) |
| `[channels]` | `[channel_filter]` |
| `[tracing.storage]` | `[trace_storage]` |
| `ORION_ENV` | `ORION_ENVIRONMENT` |
| `kafka.max_inflight` (`ORION_KAFKA__MAX_INFLIGHT`) | *Removed* — reports `removed in 1.0 (K4)` because Kafka messages are processed sequentially per consumer |
| `engine.reload_timeout_secs` (`ORION_ENGINE__RELOAD_TIMEOUT_SECS`) | *Removed* — lock-free atomic engine swap eliminated the reload lock wait |

### 2. Misspelled `ORION_*__` environment overrides fail startup

Any `ORION_*` environment variable containing the `__` section separator that does not match a known server setting (or a near-miss of `ORION_ENVIRONMENT`) stops the server at startup instead of being silently ignored:

- `ORION_SERVER__PORTT` fails startup with `did you mean ORION_SERVER__PORT?`.
- `ORION_ADMIN_AUTH__API_KEY` (singular) fails startup; the valid setting is `ORION_ADMIN_AUTH__API_KEYS` (plural).
- Single-underscore names like `ORION_SERVER_PORT` or Kubernetes service links (`ORION_SERVICE_HOST`) are ignored by the config loader because they do not contain `__`.
- If you store connector secrets in environment variables referenced via `env://`, do not name them with a double underscore under `ORION_` (such as `ORION_DB__PASSWORD`), or the config loader rejects them as unknown server settings. Use `ORION_SECRET_*` (for example, `ORION_SECRET_DB_PASSWORD`), which is reserved for secrets and skipped by the config check.

### 3. Private network connectors require `allow_private_urls: true`

SSRF protection applies to all connector types. Any connector pointing to a loopback, link-local, or RFC 1918 private address (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`) must explicitly set `"allow_private_urls": true` in its connection configuration.

### 4. Channel and workflow field renames

- **`data_write` envelope nested under `write`**: The mutation fields (`op`, `target`, `values`, `set`, `filter`, `on_conflict`, `returning`, `all`) must be nested inside an `"input": { "write": { ... } }` object, matching `data_query`'s `"query"` object.
- **Channel `cors` renamed to `origin_allow_list`**: Per-channel origin filtering uses `"origin_allow_list": ["https://app.example.com"]`.
- **Channel `backpressure` fields**: `backpressure.max_concurrent` was renamed to `backpressure.max_concurrent_per_node`, and the unused `backpressure.queue_depth` field was removed.
- **Trace read endpoints moved to Admin API**: `GET /api/v2/admin/traces` and `GET /api/v2/admin/traces/{id}` replaced the pre-1.0 `/data/traces` paths.

## Related pages

- [Upgrade to Orion 2.0](./upgrade-to-2.0.md) — current major-version migration guide
- [Versioning and support policy](./versioning-policy.md) — support window and compatibility contract
- [Server configuration and environment variables](../reference/configuration.md) — current server settings and environment variable rules
