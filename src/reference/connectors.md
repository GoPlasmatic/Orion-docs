<!-- description: Reference for Orion http, db, and cache connector schemas, managed OAuth2, the portable SQL data dialect (data_query and data_write), and plugin manifests. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# Connector schemas and portable data dialect

This page specifies the configuration schemas for Orion's three built-in Community connector types (`http`, `db`, and `cache`), the portable SQL data dialect used by [`data_query` and `data_write`](./functions.md), and the `plugin.toml` manifest contract for WebAssembly plugins.

For Standard and Enterprise connector providers (`com.orion.smtp`, `com.orion.kafka`, `com.orion.redis`, `com.orion.storage`, `com.orion.mongodb`, `com.orion.elasticsearch`, `com.orion.models`), see [Enterprise connector providers](../enterprise/providers.md).

---

## 1. Built-in connector schemas

In a definition set, a built-in connector declares `name`, `connector_type` (`"http"`, `"db"`, or `"cache"`), and `config`. All credentials in `config` must use `env://VAR` or `vault://path#field` secret locators; `orion-cli compile` extracts them into `secret_locators` on the compiled connection revision and rejects plaintext secrets.

### `http` connectors (`org.orion.http`)

Used by [`http_call`](./functions.md) (and `connector_call` over `org.orion.http`).

```json
{
  "name": "payments-api",
  "connector_type": "http",
  "config": {
    "type": "http",
    "url": "https://api.stripe.com/v1",
    "auth": { "type": "bearer", "token": "env://STRIPE_API_KEY" },
    "headers": { "x-source": "orion" },
    "retry": { "max_retries": 3, "retry_delay_ms": 1000 }
  }
}
```

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `url` | string or reference | yes | — | Base URL (`http://` or `https://`) for requests through this connector. |
| `method` | string | no | `""` | Default HTTP method when the calling task omits `method`. |
| `headers` | object | no | `{}` | Default request headers (task-level `headers` override matching keys; `auth` headers always win). |
| `query_params` | object | no | `{}` | Default query parameters appended to every request (values may be secret locators). |
| `auth` | object | no | — | Outbound authentication: `bearer`, `basic`, `apikey`, or managed `oauth2` (see table below). |
| `retry` | object | no | `{"max_retries": 3, "retry_delay_ms": 1000}` | Exponential backoff retry policy (`max_retries <= 16`, backoff capped at `60s`). |
| `retry_non_idempotent` | boolean | no | `false` | When `true`, also retries `POST` and `PATCH` requests on retryable errors (`>= 500`, `429`, `408`, timeouts, and I/O errors). |
| `max_response_size` | integer | no | `10485760` | Maximum successful response body size in bytes (`10 MiB`). Non-2xx error bodies are previewed up to `512` bytes. |
| `allow_private_urls` | boolean or reference | no | `false` | Allow requests to RFC 1918 private, loopback, link-local, and CGNAT IP addresses. |
| `operations` | object | no | all methods | Method allowlist `{"methods": ["GET", "POST", ...]}`. |

#### HTTP `config.auth` schemes

| `auth.type` | Required fields | Optional fields | Description |
|---|---|---|---|
| `"bearer"` | `token` (`env://` / `vault://`) | — | Sends `Authorization: Bearer <token>`. |
| `"basic"` | `username`, `password` (`env://` / `vault://`) | — | Sends HTTP Basic authentication. |
| `"apikey"` | `header`, `key` (`env://` / `vault://`) | — | Sends `<header>: <key>`. |
| `"oauth2"` | `grant` (`"client_credentials"`, `"refresh_token"`, or `"account_credentials"`), `token_url`, `client_id`, `client_secret` | `client_auth` (`"basic"` or `"body"`), `refresh_token` (required for `"refresh_token"` grant), `account_id` (required for Zoom `"account_credentials"`), `scopes`, `audience`, `resource`, `extra_params`, `refresh_margin_secs` (default `60`, max `3600`) | Acquires, caches in memory, refreshes single-flight, and persists rotated refresh tokens in `connector_oauth_state` (encrypted when `storage.connector_encryption_key` is set). |

---

### `db` connectors (`org.orion.postgresql`, `org.orion.mysql`, `org.orion.sqlite`)

Used by [`data_query`, `data_write`, `db_read`, and `db_write`](./functions.md).

```json
{
  "name": "orders-db",
  "connector_type": "db",
  "config": {
    "type": "db",
    "driver": "postgresql",
    "connection_string": "env://ORDERS_DB_URL",
    "max_connections": 10,
    "connect_timeout_ms": 5000,
    "query_timeout_ms": 30000
  }
}
```

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `driver` | string | yes (in a definition set) | — | `"postgresql"`, `"mysql"`, or `"sqlite"`. Must match the scheme of `connection_string` (`postgres://`, `mysql://`, `sqlite:`). |
| `connection_string` | string | yes | — | Database URL or secret locator (`env://` or `vault://`). For SQLite, relative and absolute file paths are confined inside `engine.sqlite_connector_root`. |
| `max_connections` | integer | no | — | Maximum connections in the pool. |
| `connect_timeout_ms` | integer | no | — | Connection acquisition timeout in milliseconds. |
| `query_timeout_ms` | integer | no | — | Per-query execution timeout in milliseconds. |
| `allow_private_urls` | boolean | no | `false` | Allow connecting to private or internal database IP addresses (ignored for `sqlite:`). |
| `operations` | object | no | all `true` | Operation gates: `read`, `insert`, `update`, `delete`, `upsert`, `raw_write`. |
| `dialect` | object | no | `{require_schema: false, allowed_entities: []}` | Portable dialect table guards: `require_schema` (boolean) and `allowed_entities` (array of physical table names). |

---

### `cache` connectors (`org.orion.memory-cache`)

Used by [`cache_read`, `cache_write`, `cache_delete`, and `cache_incr`](./functions.md), and optionally by single-node channel deduplication and response caches.

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `backend` | string | yes | — | Must be `"memory"` (bounded per connector by `engine.max_memory_cache_entries`). For a shared Redis cache across nodes, use [`com.orion.redis`](../enterprise/providers.md#redis-provider-comorionredis). |
| `operations` | object | no | `{read: true, write: true}` | Boolean gates (`write` governs `cache_write`, `cache_delete`, `cache_incr`, and channel stores). |

---

## 2. Portable data dialect (`data_query` and `data_write`)

The portable data dialect expresses SQL reads and writes as JSON envelopes that compile through `sea-query` into parameterized PostgreSQL, MySQL, or SQLite statements.

### Token model

| Token | Meaning | Where valid |
|---|---|---|
| `{ "field": "column_name" }` | References a declared column | `filter` (and as keys in `values` / `set`) |
| `{ "param": "param_name" }` | Binds a named scalar parameter from the task's `params` map | `filter`, `values`, `set`, `after` |
| `{ "var": "data.x" }` | Reads a value from the workflow context | Inside the task's `params` map only |

### Query envelope (`data_query` `input.query`)

| Field | Type | Description |
|---|---|---|
| `source` | string | Logical entity name resolved through `schema.entities`. |
| `filter` | JSONLogic | Predicate using `and`, `or`, `!`, `==`, `!=`, `<`, `<=`, `>`, `>=` (including ternary ranges `{ "<=": [1, {"field": "x"}, 10] }`), `in`, `starts_with`, `ends_with`, `missing`, and relation quantifiers `some`, `all`, `none`. |
| `fields` | array of strings | Columns to project (omit for all queryable columns). |
| `sort` | array of objects | Sort order `[{"age": "asc"}, {"name": "desc"}]`. Nulls sort first on `asc` and last on `desc` across all three SQL backends. |
| `limit` / `skip` | number | Offset pagination (defaults to `query.default_limit`; values exceeding `query.max_limit` or `query.max_skip` are rejected). |
| `after` | object | Keyset cursor containing the previous page's final row values for each `sort` key (requires `sort`, forbids `skip`, and requires every sort key to appear in `fields`). |
| `include` | object | Relation hydration map: `{"orders": {"fields": ["id", "total"], "sort": [{"total": "desc"}], "limit": 10}}` (`sort` is required). |
| `count` | boolean | When `true`, returns `{"count": n}` instead of rows (cannot be combined with `fields`, `sort`, `limit`, `skip`, `after`, or `include`). |

### Write envelope (`data_write` `input.write`)

| Field | Used by | Description |
|---|---|---|
| `op` | all | `"insert"`, `"update"`, `"delete"`, or `"upsert"`. |
| `target` | all | Logical entity name resolved through `schema.entities`. |
| `values` | `insert`, `upsert` | Single row object or array of row objects (bulk inserts run atomically in a transaction, capped at `write.max_rows`, default `1000`). |
| `set` | `update`, `upsert` | Map of column names to literal values or `{"param": "..."}` references. |
| `filter` | `update`, `delete` | Row selection predicate using the query filter dialect. |
| `on_conflict` | `upsert` | `{"target": ["email"], "action": "update" \| "nothing"}` (note: on MySQL, `ON DUPLICATE KEY UPDATE` triggers on any unique index on the table). |
| `returning` | all | Columns to return from mutated rows (supported on PostgreSQL and SQLite; rejected on MySQL, which returns `last_insert_id` on single-row inserts). |
| `all` | `update`, `delete` | Must be `true` (alongside server `write.allow_unfiltered = true`) to permit an unfiltered table-wide update or delete. |

## The schema registry

Every `data_query` and `data_write` task requires an inline `schema` object declaring the entities, column mappings, permissions, and relations the query can access (or an explicit `"schema": {"unmapped": "identity"}` when `dialect.require_schema` is `false`):

```json
"schema": {
  "entities": {
    "users": {
      "physical": "app_users",
      "columns": {
        "id":     { "name": "user_id", "type": "int" },
        "email":  { "type": "text" },
        "secret": { "queryable": false, "writable": false }
      },
      "relations": {
        "orders": { "to": "orders", "kind": "has_many", "local": "id", "foreign": "user_id" }
      }
    }
  },
  "unmapped": "reject"
}
```

- **Physical table and column renames**: `"physical": "app_users"` maps the logical `users` entity to `app_users`; `"name": "user_id"` maps the logical `id` field to `user_id`.
- **Column access control**: `"queryable": false` hides a column from reads and filters; `"writable": false` prevents writes to generated or protected columns.
- **Relations**: `"kind"` accepts `"has_one"`, `"has_many"`, or `"many_to_many"` (with `"through"`), enabling `some`/`all`/`none` filter quantifiers and `include` hydration.

---

## 3. WebAssembly plugin manifest and ABI (`plugin.toml`)

A WebAssembly plugin bundles a `plugin.toml` manifest and a compiled WASI Preview 2 component (`component.wasm`) implementing the `orion:plugin@1.0.0` WIT world.

```toml
abi = "orion:plugin@1.0.0"
name = "acme.iso8583"
version = "1.2.0"
component = "component.wasm"

[[functions]]
name = "acme.iso8583.parse"
description = "Parse an ISO 8583 message into field-numbered JSON"
category = "transform"
output_default_root = "data"

[[functions.input_fields]]
name = "message"
kind = "string"
required = true
resolvable = true

[[functions.input_fields]]
name = "spec"
kind = "string"
required = true
```

| Manifest key | Rule |
|---|---|
| `abi` | Must be `"orion:plugin@1.0.0"`. |
| `name` | Lowercase reverse-domain namespace with at least two labels (`[a-z][a-z0-9-]*`, excluding `orion.*`). |
| `version` | Informational semantic version string. |
| `component` | Relative path to the `.wasm` file within the plugin directory. |
| `functions[].name` | Must be `<name>.<label>` (for example, `"acme.iso8583.parse"`). |
| `functions[].output_default_root` | Optional default root (`"data"`, `"temp_data"`, or `"metadata"`) when a task omits a root prefix on `output`. |
| `functions[].input_fields[]` | `{name, kind ("string"\|"number"\|"bool"\|"object"\|"array"\|"any"), required, template_at?, resolvable?}`. `{"secret": ...}` expressions are refused in plugin task inputs. |

### WIT interface (`orion:plugin@1.0.0`)

```wit
package orion:plugin@1.0.0;

interface functions {
  enum error-class { caller-input, internal }
  record plugin-error { code: string, class: error-class, message: string }

  invoke: func(function: string, input: string) -> result<string, plugin-error>;
}

world plugin {
  export functions;
}
```

## Next steps

- [Task functions](./functions.md): Input schemas for `http_call`, `data_query`, `data_write`, `db_read`, `db_write`, `cache_*`, and `connector_call`.
- [Enterprise connector providers](../enterprise/providers.md): Schemas for the seven Standard+ `com.orion.*` providers.
- [Extend with WebAssembly plugins](../guides/plugins.md): Step-by-step guide to building, signing, and testing a plugin.
