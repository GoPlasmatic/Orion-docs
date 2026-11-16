<!-- description: The com.orion.elasticsearch provider: connection fields, seven search and document operations, the index allow-list and error codes. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# Elasticsearch provider

Available in Orion Standard and Enterprise only. The connection fields and seven operations of `com.orion.elasticsearch`, which calls a cluster's search and document APIs.

## Synopsis

A definition set declares the connection with `provider` in place of `connector_type`:

```json
{
  "name": "catalog",
  "provider": "com.orion.elasticsearch",
  "config": {
    "url": "https://search.example.com:9200",
    "auth": { "type": "api_key", "api_key": "env://ES_API_KEY" },
    "indices": ["products", "logs-*"],
    "operations": { "delete": false }
  }
}
```

## Description <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

`search`, `count` and `get` read; `index`, `update`, `delete` and `bulk` write. The provider translates each operation into one cluster request and returns the cluster's own result, with the answer read within the response cap.

Every rule is checked before a request is made: the index rules, the connection's allow-list, the search caps and the operation gates. An index name that would widen a call is refused, and a pattern is a read-only form.

## Fields

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `url` | string | yes | — | The cluster's base URL, `http(s)://host[:port][/prefix]`, without a query or fragment. |
| `auth` | object | no | `{"type": "none"}` | How requests authenticate: `none`, `basic`, `api_key` or `bearer`, with `none` the default. |
| `allow_private_urls` | boolean | no | `false` | Whether the cluster may be at a private or internal address. |
| `timeout_ms` | integer | no | `30000` | The ceiling on one request, within the run deadline. |
| `indices` | array of strings | no | `[]` | The indices a call may reach: names, or prefixes ending in `*`. |
| `max_hits` | integer | no | `1000` | The largest `size` a `search` may ask for in one request. |
| `max_window` | integer | no | `10000` | The deepest page a `search` may reach, as `from + size`. |
| `max_bulk_actions` | integer | no | `1000` | The actions one `bulk` call may carry in one request. |
| `max_response_bytes` | integer | no | — | The answer's byte ceiling; unset uses the node's response cap. |
| `operations` | object | no | all allowed | The gates `read`, `insert`, `upsert`, `update` and `delete`, all allowed by default. |

An empty `indices` list reaches any index except a dot-prefixed one, which needs its own entry. `max_hits` may not exceed `max_window`.

### `auth` fields

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `auth.type` | string | no | `"none"` | `none`, `basic`, `api_key` or `bearer`, matching the secret field the connection sets. |
| `auth.username` | string | no | — | The basic-auth user, required when the connection sets `auth.type` to `basic`. |
| `auth.password` | string | no | — | The basic-auth password, always a secret locator at `auth.password`. |
| `auth.api_key` | string | no | — | The encoded API key, always a secret locator at `auth.api_key`. |
| `auth.token` | string | no | — | The bearer token, always a secret locator at `auth.token`. |

## Operations

| Operation | Input | Answer |
|-----------|-------|--------|
| `search` | `index`, `routing?`, and the Query DSL body | `{hits, total, total_relation, max_score, took, timed_out, shards, aggregations?, suggest?}` |
| `count` | `{index, query?, routing?}` | `{count}` |
| `get` | `{index, id, routing?, _source?}` | the document, or `{found: false}` with its index and id |
| `index` | `{index, document, id?, op_type?, refresh?, routing?, if_seq_no?, if_primary_term?, pipeline?}` | the cluster's result |
| `update` | `{index, id, doc or script, upsert?, …}` | the cluster's result |
| `delete` | `{index, id, refresh?, routing?, if_seq_no?, if_primary_term?}` | the cluster's result |
| `bulk` | `{actions, index?, refresh?, pipeline?}` | `{status, succeeded, failed, took, items}` |

### `search`

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `index` | string or array of strings | yes | — | An index name, a pattern, or a list of either. |
| `routing` | string | no | — | The routing value the cluster uses to choose a shard. |
| `query` | object | no | — | The Query DSL query the cluster runs against the indices. |
| `size` | integer | no | `10` | The hits to return, at most the connection's `max_hits` value. |
| `from` | integer | no | `0` | The hits to skip; `from + size` may not pass `max_window`. |
| `sort` | array or object | no | — | The sort the cluster applies to the returned hits. |
| `_source` | boolean, string or array | no | — | The source fields the answer includes, as a field or a list. |
| `aggs` | object | no | — | The aggregations the answer computes beside the returned hits. |
| `search_after` | array | no | — | The sort values to continue after, for deep paging. |

Every other Query DSL body field is passed through. `pit` is refused, because the indices of a point-in-time search travel inside the point in time.

### `count`

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `index` | string or array of strings | yes | — | An index name, a pattern, or a list of either. |
| `query` | object | no | — | The query the cluster counts, or every document when unset. |
| `routing` | string | no | — | The routing value the cluster uses to choose a shard. |

### `get`

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `index` | string | yes | — | The index holding the document that the operation targets. |
| `id` | string | yes | — | The document id the read targets, from 1 to 512 bytes. |
| `routing` | string | no | — | The routing value the document was indexed with, if it set one. |
| `_source` | boolean, string or array | no | — | The source fields the answer includes, as a field or a list. |

### `index`

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `index` | string | yes | — | The index that receives the document the call writes. |
| `document` | object | yes | — | The document to index, written as a JSON object. |
| `id` | string | no | — | The document id; absent lets the cluster generate one. |
| `op_type` | string | no | `"index"` | `index` to overwrite an id, or `create` to refuse an existing one. |
| `refresh` | boolean or string | no | — | `true`, `false` or `"wait_for"`, controlling when the write becomes visible. |
| `routing` | string | no | — | The routing value the document was indexed with, if one was set. |
| `if_seq_no` | integer | no | — | The sequence number the write requires, paired with `if_primary_term`. |
| `if_primary_term` | integer | no | — | The primary term the write requires, paired with `if_seq_no`. |
| `pipeline` | string | no | — | The ingest pipeline the cluster runs before it stores the document. |

Without an `id`, or with `op_type` set to `create`, the call needs the `insert` gate. With an `id` it needs the `upsert` gate.

### `update`

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `index` | string | yes | — | The index holding the document that the operation targets. |
| `id` | string | yes | — | The document id the operation targets, from 1 to 512 bytes. |
| `doc` | object | no | — | The partial document to merge; exactly one of `doc` and `script`. |
| `script` | object or string | no | — | The script that updates the document, as an object or a string. |
| `upsert` | object | no | — | The document inserted when no document matches the update. |
| `doc_as_upsert` | boolean | no | `false` | Whether `doc` is also the document inserted by an upsert. |
| `scripted_upsert` | boolean | no | `false` | Whether the script runs for the document an upsert inserts. |
| `detect_noop` | boolean | no | — | Whether an unchanged document skips the write and reports no operation. |
| `refresh` | boolean or string | no | — | `true`, `false` or `"wait_for"`, controlling when the write becomes visible. |
| `routing` | string | no | — | The routing value the document was indexed with, if one was set. |
| `retry_on_conflict` | integer | no | — | The retries the cluster makes on a version conflict. |
| `if_seq_no` | integer | no | — | The sequence number the write requires, paired with `if_primary_term`. |
| `if_primary_term` | integer | no | — | The primary term the write requires, paired with `if_seq_no`. |

An update needs the `update` gate, and an update that may create a document needs `upsert` as well.

### `delete`

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `index` | string | yes | — | The index holding the document that the operation targets. |
| `id` | string | yes | — | The document id the operation targets, from 1 to 512 bytes. |
| `refresh` | boolean or string | no | — | `true`, `false` or `"wait_for"`, controlling when the write becomes visible. |
| `routing` | string | no | — | The routing value the document was indexed with, if one was set. |
| `if_seq_no` | integer | no | — | The sequence number the delete requires, paired with `if_primary_term`. |
| `if_primary_term` | integer | no | — | The primary term the delete requires, paired with `if_seq_no`. |

### `bulk`

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `actions` | array | yes | — | The actions, from 1 to `max_bulk_actions`, each an object with `op`. |
| `index` | string | no | — | The default index for actions that name none of their own. |
| `refresh` | boolean or string | no | — | `true`, `false` or `"wait_for"`, controlling when the write becomes visible. |
| `pipeline` | string | no | — | The ingest pipeline the cluster runs for every action. |

An action's `op` is `index`, `create`, `update` or `delete`. An `index` or `create` action takes `document`; an `update` action takes the `update` fields; a `delete` action takes none. Every action accepts `index`, `id`, `routing`, `if_seq_no` and `if_primary_term`, and needs the gate its operation implies.

## Returns

| Operation | Answer |
|-----------|--------|
| `search` | `{hits, total, total_relation, max_score, took, timed_out, shards, aggregations?, suggest?}`; each hit as the cluster returns it. |
| `count` | `{count}`. |
| `get` | The document with `found: true`, `_source`, `_version` and its metadata, or `{found: false}` with the index and id. |
| `index`, `update`, `delete` | The cluster's result, with `_id`, `result` and `_version`. |
| `bulk` | `{status, succeeded, failed, took, items}`. |

A `bulk` `items` array has one entry per action, in order: `index`, `op`, `status` (`ok` or `error`), `_id`, and `result` or `error`. The envelope's `status` is `ok` when every action applied and `partial` when some did. An `update` or `delete` that matches no document answers `result: "not_found"` rather than failing.

## Errors

| Code | Class | Condition |
|------|-------|-----------|
| `ELASTICSEARCH_NOT_FOUND` | not_found | The cluster answered 404, such as a missing index. |
| `ELASTICSEARCH_CONFLICT` | conflict | The cluster answered 409, such as a version conflict or a `create` of an existing id. |
| `ELASTICSEARCH_THROTTLED` | rate_limited | The cluster answered 429. |
| `ELASTICSEARCH_REJECTED` | provider_failure | The cluster answered another refusal; `details.status` and `details.type` name it. |
| `ELASTICSEARCH_RESPONSE_TOO_LARGE` | provider_failure | The answer is over the response cap. |
| `ELASTICSEARCH_BULK_FAILED` | provider_failure | A `bulk` applied none of its actions. |
| `ELASTICSEARCH_CONNECTION_INCOMPLETE` | provider_failure | An `auth` secret is not resolved from its locator. |
| `CONNECTOR_INVALID_INPUT` | invalid_input | An index, id, action, field or cap is not valid. |
| `CONNECTOR_OPERATION_FORBIDDEN` | forbidden | The connection's `operations` block closes the call. |
| `CONNECTOR_EGRESS_DENIED` | forbidden | The cluster is private and the connection does not allow it. |
| `CONNECTOR_DOWNSTREAM_UNAUTHORIZED` | unauthorized | The cluster answered 401 or 403. |
| `CONNECTOR_UNAVAILABLE` | unavailable | The cluster is unreachable or answered 5xx. |

## Examples

The task searches the product index for a term:

```json
{
  "name": "connector_call",
  "input": {
    "connection": "catalog",
    "target_id": "com.orion.elasticsearch",
    "operation": "search",
    "input": {
      "index": "products",
      "query": { "match": { "title": { "var": "data.q" } } },
      "size": 20
    },
    "output": "data.results"
  }
}
```

The answer carries the hits and the total:

```json
{
  "hits": [
    { "_index": "products", "_id": "p-1042", "_score": 1.2,
      "_source": { "title": "Aeropress filter", "price": 12.5 } }
  ],
  "total": 1,
  "total_relation": "eq",
  "max_score": 1.2,
  "took": 4,
  "timed_out": false,
  "shards": { "total": 1, "successful": 1, "skipped": 0, "failed": 0 }
}
```

## Caveats

- Index names cannot widen a call. A comma, a `remote:` prefix, a `-exclusion` and `_all` are refused.
- A pattern with `*` is a read form only. A write names one index.
- A dot-prefixed index needs an entry of its own in `indices`; an empty list reaches any other index.
- The allow-list also holds the indices a query names itself: a terms lookup, a `more_like_this` document and a percolated document.
- A point-in-time search is refused, because its indices travel inside the point in time.
- `bulk` answers `partial` when some actions applied and fails with `ELASTICSEARCH_BULK_FAILED` when none did. An action the cluster did not report is counted failed, never assumed applied.
- An index's ingest pipeline runs inside the cluster and may route a document elsewhere, which the allow-list cannot bound.
- `search` defaults to `size` 10, capped at `max_hits`, and `from + size` may not pass `max_window`.

## Related

- [Edition providers](./index.md): the seven providers and the rules they share.
- [`connector_call`](../../functions/connector_call.md): the task that invokes a search or document operation.
- [Definition and identity](../identity.md): the connection revision and the secret locators it holds.
- [Edition capability matrix](../../editions.md): the seven providers with their minimum edition.
