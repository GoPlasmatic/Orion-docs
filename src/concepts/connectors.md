<!-- description: Learn how Orion connectors manage database, HTTP, cache, and enterprise provider connections, secret references, operation gates, and circuit breakers. -->
<!-- type: concept -->
<!-- last_verified: 2026-11-16 -->

# Connectors and secrets

A **connector** is a named, reusable connection to an external system—such as a SQL database, an HTTP API, a cache, or a message broker. Instead of embedding URLs, credentials, and retry rules inside individual workflow tasks, you define a connector once in your definition set and reference it by name across any number of workflows.

```orion-diagram
{
  "direction": "LR",
  "groups": [ { "id": "orion", "label": "Orion node" } ],
  "nodes": [
    { "id": "wf1", "label": "order-processing", "type": "service", "group": "orion" },
    { "id": "wf2", "label": "refund-handler", "type": "service", "group": "orion" },
    { "id": "c1", "label": "orders-db", "sublabel": "db connector", "type": "accent", "group": "orion" },
    { "id": "c2", "label": "payments-api", "sublabel": "http connector", "type": "accent", "group": "orion" },
    { "id": "pg", "label": "PostgreSQL", "type": "datastore" },
    { "id": "ext", "label": "Stripe API", "type": "datastore", "shape": "cloud" }
  ],
  "edges": [
    { "from": "wf1", "to": "c1" },
    { "from": "wf1", "to": "c2" },
    { "from": "wf2", "to": "c2" },
    { "from": "c1", "to": "pg" },
    { "from": "c2", "to": "ext" }
  ]
}
```

In the diagram above, both `order-processing` and `refund-handler` reference `payments-api`. They share a single connection pool, credential configuration, retry policy, and circuit breaker.

---

## Connector files vs. connection revisions

A connector exists in two representations across the deployment lifecycle:

| Form | Where It Lives | How It Changes |
| :--- | :--- | :--- |
| **Connector file (`connector.json`)** | In your repository's definition set alongside your workflows and channels | You edit the JSON file in Git |
| **Connection revision** | On the Orion server (`POST /api/v2/admin/connections/{id}/revisions`), referenced by the deployment manifest | Immutable; changing a setting creates the next numbered revision |

Here is the `connector.json` from the `postgres-orders` example package:

```json
{{#include ../../examples/packages/postgres-orders/connector.json}}
```

When you run `orion-cli compile`, the CLI transforms that file into a connection specification (`connections/orders-db.json`), separating non-sensitive settings (`public_config`) from secret references (`secret_locators`):

```json
{
  "connection_id": "orders-db",
  "driver": "org.orion.postgresql",
  "provider_digest": "sha256:5e9d3f17…",
  "public_config": {
    "allow_private_urls": true,
    "operations": { "delete": false },
    "max_connections": 5
  },
  "secret_locators": {
    "connection_string": "env://ORDERS_DB_URL"
  }
}
```

When you run `orion-cli deployments apply -d`, the CLI reuses an existing connection revision if one already matches those exact settings and locators, or creates the next revision (`orders-db@2`) and binds it into the deployment manifest.

---

## Built-in connector types and edition providers

### Core connector types (all editions)

| Type | Target System | Built-in Driver ID | Used by Task Functions |
| :--- | :--- | :--- | :--- |
| **`http`** | REST and HTTP APIs | `org.orion.http` | `http_call` |
| **`db`** | PostgreSQL, MySQL, SQLite | `org.orion.postgresql`, `org.orion.mysql`, `org.orion.sqlite` | `data_query`, `data_write`, `db_read`, `db_write` |
| **`cache`** | Per-node in-memory cache | `org.orion.memory-cache` | `cache_read`, `cache_write`, `cache_delete`, `cache_incr` |

Every `db` connector explicitly declares its `driver` (`"postgresql"`, `"mysql"`, or `"sqlite"`) so `orion-cli compile` knows the database engine even when `connection_string` is an `env://` secret reference.

### Standard & Enterprise connector providers <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

Available in Orion Standard and Enterprise (`orion-ext`). Instead of `"connector_type"`, a connector file declares `"provider"` and workflows invoke its operations using [`connector_call`](../reference/functions.md):

| Provider ID | Target System | Operations |
| :--- | :--- | :--- |
| **`com.orion.smtp`** | SMTP email relays | `send` (alias `send_email`) |
| **`com.orion.kafka`** | Apache Kafka / Redpanda | `publish` (also backs `protocol: "kafka"` channels) |
| **`com.orion.redis`** | Shared Redis cache | `get`, `set`, `delete`, `incr`, `get_many`, `delete_many` |
| **`com.orion.storage`** | S3-compatible object storage | `presign`, `head`, `get`, `put` |
| **`com.orion.mongodb`** | MongoDB clusters | `find`, `write`, `aggregate` |
| **`com.orion.elasticsearch`** | Elasticsearch clusters | `search`, `count`, `get`, `index`, `update`, `delete`, `bulk` |
| **`com.orion.models`** | ONNX models (CPU inference via `tract`) | `infer` |

See [Enterprise connector providers](../enterprise/providers.md) for each provider's configuration and operation schemas.

---

## Secrets by reference (`env://` and `vault://`)

Credentials never live inside a deployment bundle or the Orion state database:
- Use **`env://VAR_NAME`** to resolve a secret from the Orion server's environment variables when the connection loads.
- Use **`vault://path/to/secret#field`** to resolve a secret from HashiCorp Vault.

`orion-cli compile` and the Admin API reject any connector definition that places a literal credential in a secret-shaped field (`password`, `secret`, `token`, `api_key`, or userinfo inside a URL). This guarantees two things:
1. **Database dumps never leak credentials**, because the state database stores only locator strings (`"env://ORDERS_DB_URL"`).
2. **One compiled deployment works across staging and production**, because each environment resolves its own environment variables or Vault paths at runtime.

---

## Operation gates, retries, and circuit breakers

### Operation gates
Every connector has an `operations` block that restricts what workflows are allowed to do through that connection:
- **SQL (`db`) connectors**: Gate `read`, `insert`, `update`, `delete`, `upsert`, and `raw_write` (all default to `true`). Setting `"delete": false` and `"raw_write": false` guarantees that no workflow using that connector can delete rows.
- **HTTP (`http`) connectors**: Set `"operations": { "methods": ["GET"] }` to restrict a connector to read-only requests.
- **Cache (`cache`) connectors**: Gate `read` and `write`.

Operation gates are enforced at the connector layer, independent of workflow definitions.

### Retries and circuit breakers
- **HTTP retries**: `http` connectors can configure automatic exponential backoff retries for transient failures (`502`, `503`, `504`, network timeouts) on idempotent HTTP methods (`GET`, `HEAD`, `PUT`, `DELETE`, `OPTIONS`).
- **Circuit breakers**: When enabled globally via `engine.circuit_breaker.enabled = true`, Orion maintains a circuit breaker for each `(channel, connector)` pair. If a downstream dependency repeatedly fails, the breaker opens and fails fast with `503 CIRCUIT_OPEN` until its recovery cooldown elapses.

## Next steps

- [Connect databases, APIs, and caches](../guides/connectors.md) — Practical guide to configuring SQL, HTTP, OAuth2, and cache connectors.
- [Connector schemas and portable data dialect](../reference/connectors.md) — Full field tables for every connector type and the `data_query` / `data_write` dialect.
- [Enterprise connector providers](../enterprise/providers.md) — Configure Redis, Kafka, S3, MongoDB, Elasticsearch, SMTP, and ONNX models.
