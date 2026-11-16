<!-- description: The com.orion.mongodb provider: cluster connection fields, find, aggregate and write operations, and the safety limits on each. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# MongoDB provider

Available in Orion Standard and Enterprise only. The connection fields and three operations of `com.orion.mongodb`, which queries and writes one MongoDB cluster.

## Synopsis

A definition set declares the connection with `provider` in place of `connector_type`:

```json
{
  "name": "orders-docs",
  "provider": "com.orion.mongodb",
  "config": {
    "connection_string": "env://MONGO_URL",
    "database": "orders",
    "max_result_documents": 500,
    "operations": { "delete": false }
  }
}
```

## Description <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

`find` reads documents by filter, `aggregate` runs a pipeline, and `write` runs one insert, update, replace or delete. `mongo_read`, `mongo_aggregate` and `mongo_write` are compatibility aliases for the three.

Every check runs before a connection opens, so a filter, pipeline, cap or gate refusal never reaches the cluster. A result past `max_result_documents` fails rather than truncating, because a short answer would misstate the data.

Documents are MongoDB Extended JSON v2, relaxed or canonical. Answers are relaxed Extended JSON v2, so an answer's values round-trip into the next call's filter unchanged.

## Fields

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `connection_string` | string | no | — | A `mongodb://` or `mongodb+srv://` URL, and a secret locator when it carries credentials. |
| `database` | string | no | — | The database a call uses when its input names none. |
| `username` | string | no | — | The SCRAM login name, when the connection string carries none. |
| `password` | string | no | — | The SCRAM password, always a secret locator at `password`. |
| `allow_private_urls` | boolean | no | `false` | Whether the cluster may be at a private or internal address. |
| `max_connections` | integer | no | `10` | The driver's connection pool size, which bounds concurrent operations. |
| `connect_timeout_ms` | integer | no | `10000` | The connect and server-selection timeout in milliseconds, within the run deadline. |
| `query_timeout_ms` | integer | no | `30000` | The ceiling on one operation, within the run deadline. |
| `max_result_documents` | integer | no | `1000` | The documents a read may answer; a larger result fails. |
| `max_skip` | integer | no | `10000` | The largest `skip` a `find` may ask for, which bounds deep paging. |
| `max_write_documents` | integer | no | `1000` | The documents one `insert_many` may write in a single call. |
| `allow_unfiltered` | boolean | no | `false` | Whether a write with an empty filter may run when the call also sets `all`. |
| `aggregate_write_stages` | boolean | no | `false` | Whether `aggregate` may run the write stages `$out` and `$merge`. |
| `operations` | object | no | all allowed | The gates `read`, `insert`, `update`, `upsert` and `delete`, all allowed by default. |

A `connection_string` is required at use, either in `public_config` or as a secret locator. The username and password go together, and the provider refuses a half-configured pair with `MONGO_CONNECTION_INCOMPLETE`.

## Operations

| Operation | Aliases | Input | Answer |
|-----------|---------|-------|--------|
| `find` | `mongo_read` | `collection`, `database?`, `filter?`, `projection?`, `sort?`, `limit?`, `skip?` | an array of documents |
| `aggregate` | `mongo_aggregate` | `collection`, `database?`, `pipeline`, `allow_disk_use?` | an array of documents |
| `write` | `mongo_write` | `collection`, `database?`, `op`, and the op's fields | `{status, …}` |

`find` and `aggregate` need the `read` gate. A write needs the gate its operation implies: `insert`, `update`, `upsert` or `delete`.

### `find`

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `collection` | string | yes | — | The collection the query reads, on the connection's cluster. |
| `database` | string | no | — | The database, defaulting to the one the connection names. |
| `filter` | object | no | `{}` | The Extended JSON query filter; empty matches every document. |
| `projection` | object | no | — | The fields the answer includes or excludes, as a projection document. |
| `sort` | object | no | — | The sort document, such as `{"created": -1}` for newest first. |
| `limit` | integer | no | — | The page size, at most the connection's `max_result_documents` value. |
| `skip` | integer | no | — | The documents to skip, at most the connection's `max_skip` value. |

An absent `limit` is not a page size: the call answers every match up to the cap.

### `aggregate`

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `collection` | string | yes | — | The collection the pipeline reads, on the connection's cluster. |
| `database` | string | no | — | The database, defaulting to the one the connection names. |
| `pipeline` | array | yes | — | The stages, at least one, each an object with one stage key. |
| `allow_disk_use` | boolean | no | `false` | Whether the server may write to disk during the aggregation. |

### `write`

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `collection` | string | yes | — | The collection the write targets, on the connection's cluster. |
| `database` | string | no | — | The database, defaulting to the one the connection names. |
| `op` | string | yes | — | One of `insert_one`, `insert_many`, `update_one`, `update_many`, `replace_one`, `delete_one` or `delete_many`. |
| `document` | object | no | — | The document for `insert_one` or the replacement for `replace_one`. |
| `documents` | array | no | — | The documents for `insert_many`, at most the connection's `max_write_documents` value. |
| `ordered` | boolean | no | `true` | Whether `insert_many` stops at its first failure or attempts every document. |
| `filter` | object | no | `{}` | The filter for every operation that takes one, as Extended JSON. |
| `update` | object | no | — | The operator update for `update_one` or `update_many`, using `$` operators. |
| `upsert` | boolean | no | `false` | Whether a missing match is inserted; needs the `upsert` gate. |
| `array_filters` | array | no | — | The filters for the update's `$[identifier]` paths, one per identifier. |
| `all` | boolean | no | `false` | Whether an empty filter may affect every document, which `allow_unfiltered` must also permit. |

A field that does not apply to the chosen `op` is refused. An update must use operators such as `$set`; a replacement must not.

### Allowed aggregate stages

`aggregate` accepts these read stages:

- `$addFields`, `$bucket`, `$bucketAuto`, `$count`, `$densify`, `$facet`, `$fill`, `$geoNear`
- `$graphLookup`, `$group`, `$limit`, `$lookup`, `$match`, `$project`, `$redact`
- `$replaceRoot`, `$replaceWith`, `$sample`, `$search`, `$searchMeta`, `$set`
- `$setWindowFields`, `$skip`, `$sort`, `$sortByCount`, `$unionWith`, `$unset`, `$unwind`, `$vectorSearch`

`$out` and `$merge` are refused unless the connection sets `aggregate_write_stages`. The pipeline is judged after the workflow fills it from the message, so a stage from message data meets the same allowlist.

## Returns

| Operation | Answer |
|-----------|--------|
| `find` | An array of documents, in the sort order. |
| `aggregate` | An array of documents, which is empty after `$out` or `$merge`. |
| `write` | `{status, inserted, ids}` for an insert, `{status, matched, modified, upserted_id?}` for an update or replace, `{status, deleted}` for a delete. |

An `insert_many` the server partly refused answers `{"status": "partial", …}` with `failed`, `skipped` and a per-item `items` array. A clean write answers `"status": "ok"`.

## Errors

| Code | Class | Condition |
|------|-------|-----------|
| `MONGO_DUPLICATE_KEY` | conflict | A write hit a unique index; an `insert_many` reports it only when nothing was written. |
| `MONGO_RESULT_TOO_LARGE` | invalid_input | A result is past `max_result_documents`. |
| `MONGO_FAILED` | provider_failure | The server refused the operation otherwise. |
| `MONGO_CONNECTION_INVALID` | provider_failure | The connection string does not parse into client options. |
| `MONGO_CONNECTION_INCOMPLETE` | provider_failure | A required setting or credential is missing. |
| `CONNECTOR_INVALID_INPUT` | invalid_input | A filter, document, pipeline stage or cap is not valid. |
| `CONNECTOR_OPERATION_FORBIDDEN` | forbidden | A gate, `allow_unfiltered` or `aggregate_write_stages` refuses the call. |
| `CONNECTOR_EGRESS_DENIED` | forbidden | The cluster is private and the connection does not allow it. |
| `CONNECTOR_DOWNSTREAM_UNAUTHORIZED` | unauthorized | The cluster refused the credentials. |
| `CONNECTOR_UNAVAILABLE` | unavailable | The cluster is unreachable. |
| `CONNECTOR_DEADLINE_EXCEEDED` | deadline | The run's deadline passed during the operation. |

## Examples

The task reads the open orders for one customer:

```json
{
  "name": "connector_call",
  "input": {
    "connection": "orders-docs",
    "target_id": "com.orion.mongodb",
    "operation": "find",
    "input": {
      "collection": "orders",
      "filter": { "customer_id": { "var": "data.customer_id" }, "status": "open" },
      "sort": { "created": -1 },
      "limit": 20
    },
    "output": "data.open_orders"
  }
}
```

The answer is an array of documents in relaxed Extended JSON v2:

```json
[
  {
    "_id": { "$oid": "6712f3c1a4b9d20e5c8a1f30" },
    "customer_id": "c-1042",
    "status": "open",
    "total": { "$numberDecimal": "84.50" },
    "created": { "$date": "2026-11-02T09:14:00Z" }
  }
]
```

## Caveats

- An update, replace or delete with an empty filter needs the call's `all: true` and the connection's `allow_unfiltered`. Both are required.
- `$out` and `$merge` are the only write stages, and both need `aggregate_write_stages`.
- A result over `max_result_documents` fails. Add a filter or a `limit`, or raise the cap.
- The input names a collection and a database, never a cluster. The connection string decides the cluster.
- A replacement document with a `$` key is refused. Use `update_one` for operator updates.
- An `array_filters` entry must match a `$[identifier]` in the update, and an identifier without a filter is refused.
- Answers are relaxed Extended JSON v2, so an int64 reads as a number and an ObjectId as `{"$oid": …}`.

## Related

- [Edition providers](./index.md): the seven providers and the rules they share.
- [`connector_call`](../../functions/connector_call.md): the task that invokes `find`, `aggregate` or `write`.
- [Shared connector blocks](../common.md): the revision and secret rules every connector shares.
- [Edition capability matrix](../../editions.md): the seven providers with their minimum edition.
