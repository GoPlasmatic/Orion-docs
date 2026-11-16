<!-- description: Every Orion connector type and its config — http, db, cache and the seven Standard+ edition providers — with secrets and gates. -->
<!-- type: hub -->
<!-- last_verified: 2026-11-16 -->

# Connector types

A named connection to an external system, and what every type of one carries.

Connector validation errors return `400 VALIDATION_ERROR` with field paths.
Workflow execution failures that cannot be exposed return `500 ENGINE_ERROR`,
while an open circuit returns `503 CIRCUIT_OPEN`. Correct the connector definition,
and use [Errors & Response
Envelopes](../errors.md) for the complete contract.

A **connector** is a named connection to an external system — an API, a database or a cache. Workflows reference connectors by name. Credentials stay in the connector, never in workflow JSON.

There are three built-in connector types:

| Type | Backs | Task functions |
|------|-------|----------------|
| [`http`](./http.md) | REST APIs and webhooks | `http_call` |
| [`db`](./db.md) | PostgreSQL, MySQL, SQLite | `data_query`, `data_write`, `db_read`, `db_write` |
| [`cache`](./cache.md) | Per-node in-process memory | `cache_read`, `cache_write` |

Type values match case-insensitively. Any other value is refused with the valid list. A deployed connection whose config does not load surfaces as a connector load issue.

The task functions and their inputs are specified in the [Function Reference](../functions/index.md).

| Page | Holds |
|---|---|
| [Shared connector blocks](./common.md) | identity, secret references, secret locators, authentication, request layering, gates and retries. |
| [The three types](./types.md) | the three types, what each backs, and the `config` each takes. |

## Edition providers <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

A Community node ships the three built-in types and no provider. Orion Standard and Enterprise add seven connector providers, which a set names with `provider` in place of `connector_type`. They are available in Orion Standard and Enterprise only. The [Edition providers](./edition/index.md) hub covers the tier model, the naming rules and the immutable revision.

| Provider | Backs | Operations |
|---|---|---|
| [`com.orion.smtp`](./edition/smtp.md) | Transactional email through an SMTP relay | `send` |
| [`com.orion.kafka`](./edition/kafka.md) | Record production to a Kafka cluster | `publish` |
| [`com.orion.redis`](./edition/redis.md) | A Redis cache shared by every node | `get`, `set`, `delete`, `incr`, `get_many`, `delete_many` |
| [`com.orion.storage`](./edition/storage.md) | One bucket of an S3-compatible object store | `presign`, `head`, `get`, `put` |
| [`com.orion.mongodb`](./edition/mongodb.md) | Document queries and writes on one MongoDB cluster | `find`, `aggregate`, `write` |
| [`com.orion.elasticsearch`](./edition/elasticsearch.md) | Search and document APIs on an Elasticsearch cluster | `search`, `count`, `get`, `index`, `update`, `delete`, `bulk` |
| [`com.orion.models`](./edition/models.md) | ONNX inference on a node | `infer` |

## Related

- [Deployment endpoints](../admin-api/deployments.md#connections): the connection revision a connector becomes.
- [Task functions](../functions/index.md): the functions that call through a connector.
- [Portable data dialect](../data-dialect.md): the backend-neutral language `db` serves.
- [Environment variables](../environment-variables.md): `${VAR}` versus `env://`, and what an unset variable does.
