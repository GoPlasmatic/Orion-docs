<!-- description: Reference and usage guide for the seven Orion Standard+ connector providers: SMTP, Kafka, Redis, S3 storage, MongoDB, Elasticsearch, and ONNX models. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# Enterprise connector providers

<span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

Orion Standard and Enterprise include seven first-party connector providers compiled into the `orion-ext` binary. Unlike the three built-in Community connector types (`http`, `db`, and `cache`), a provider connection declares `"provider": "<driver_id>"` in place of `"connector_type"` and is invoked from workflows through the [`connector_call`](../reference/functions.md) task function.

## Summary of Standard+ providers

| Provider ID | System | Operations (and compatibility aliases) |
|---|---|---|
| [`com.orion.smtp`](#smtp-provider-comorionsmtp) | Transactional email over SMTP (STARTTLS or implicit TLS) | `send` (`send_email`) |
| [`com.orion.kafka`](./kafka.md) | Apache Kafka producer and channel broker | `publish` (`publish_kafka`) |
| [`com.orion.redis`](#redis-provider-comorionredis) | Shared Redis cache and atomic counters | `get` (`cache_read`, `cache.get`), `set` (`cache_write`, `cache.set`), `delete` (`cache.delete`), `incr` (`cache_incr`, `cache.incr`), `get_many` (`cache.get_many`), `delete_many` (`cache.delete_many`) |
| [`com.orion.storage`](#object-storage-provider-comorionstorage) | Single bucket in an S3-compatible object store | `presign` (`storage_presign`), `head` (`storage_head`), `get` (`storage_get`), `put` (`storage_put`) |
| [`com.orion.mongodb`](#mongodb-provider-comorionmongodb) | MongoDB document collections (Extended JSON v2) | `find` (`mongo_read`), `aggregate` (`mongo_aggregate`), `write` (`mongo_write`) |
| [`com.orion.elasticsearch`](#elasticsearch-provider-comorionelasticsearch) | Elasticsearch search, document, and bulk APIs | `search`, `count`, `get`, `index`, `update`, `delete`, `bulk` |
| [`com.orion.models`](#onnx-models-provider-comorionmodels) | Local CPU ONNX model inference (`tract`) | `infer` (`model_infer`) |

---

## Shared connection and error rules

1. **Connection definition (`connector.json`)**: Set `"provider"` to the provider ID (for example, `"com.orion.smtp"`). A connector file that sets both `provider` and `connector_type`, or neither, is refused. Store all credentials as `env://` or `vault://` secret locators.
2. **Task invocation (`connector_call`)**: Set `connection` to your connection name, `target_id` to the provider ID, `operation` to one of the provider's operations, `input` to the operation input object, and `output` to the destination path (such as `"data.mail"`).
3. **Common error codes**: Across all seven providers, `error.connector.provider_code` surfaces either a provider-specific code (listed in each section below) or one of the shared connector codes:
   - `CONNECTOR_INVALID_INPUT` (`invalid_input`): Unknown or invalid field in the connection or operation input.
   - `CONNECTOR_OPERATION_FORBIDDEN` (`forbidden`): Operation is disabled by the connection's `operations` gates.
   - `CONNECTOR_EGRESS_DENIED` (`forbidden`): Target host resolves to a private IP while `allow_private_urls` is `false`.
   - `CONNECTOR_DOWNSTREAM_UNAUTHORIZED` (`unauthorized`): Backend rejected the credentials.
   - `CONNECTOR_UNAVAILABLE` (`unavailable`): Backend could not be reached or dropped the connection.
   - `CONNECTOR_DEADLINE_EXCEEDED` (`deadline`): Run deadline expired during the call.

---

## SMTP provider (`com.orion.smtp`)

Delivers RFC 5322 email messages through an SMTP relay. Once the message body begins transmitting, the provider marks the effect as `Unknown` so a post-body timeout is never retried to avoid duplicate emails.

```json
{
  "name": "mailer",
  "provider": "com.orion.smtp",
  "config": {
    "host": "smtp.example.com",
    "port": 587,
    "tls": "starttls",
    "auth": {
      "type": "basic",
      "username": "mailer",
      "password": "env://SMTP_PASSWORD"
    },
    "from": "Orders <orders@example.com>",
    "max_connections": 10
  }
}
```

### Connection `config` fields

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `host` | string | yes | — | SMTP relay hostname or IP address (URLs are refused). |
| `port` | integer | no | `587` | Relay port (`587` for STARTTLS, `465` for implicit TLS, `25` for internal relays). |
| `tls` | string | no | `"starttls"` | Session security: `"starttls"` or `"implicit"`. |
| `auth.type` | string | no | `"none"` | `"none"` for a trusted relay or `"basic"` for PLAIN/LOGIN authentication. |
| `auth.username` | string | no | — | Login username (required when `auth.type` is `"basic"`). |
| `auth.password` | string | no | — | Secret locator at `auth.password` (required when `auth.type` is `"basic"`). |
| `from` | string | yes | — | Default sender (`"addr@example.com"` or `"Name <addr@example.com>"`). |
| `allow_from_override` | boolean | no | `false` | Whether an operation input may override `from`. |
| `allow_private_urls` | boolean | no | `false` | Allow the relay hostname to resolve to a private or internal IP. |
| `timeout_ms` | integer | no | `10000` | Connect and per-command timeout in milliseconds. |
| `max_connections` | integer | no | `10` | Maximum concurrent sessions opened to the relay by this node. |

### Operation: `send` (alias `send_email`)

| Input field | Type | Required | Default | Description |
|---|---|---|---|---|
| `to` | string or array of strings | yes | — | One or more recipient addresses. |
| `cc` | string or array of strings | no | — | Carbon-copy recipients visible in headers. |
| `bcc` | string or array of strings | no | — | Blind recipients sent only in the SMTP envelope. |
| `subject` | string | yes | — | Message `Subject` header. |
| `text` | string | no | — | Plaintext body (at least one of `text` or `html` is required). |
| `html` | string | no | — | HTML body (when paired with `text`, sends `multipart/alternative`). |
| `reply_to` | string | no | — | `Reply-To` address. |
| `headers` | object | no | `{}` | Extra string headers (structured headers such as `From`, `To`, `Subject`, `Content-Type` are refused). |
| `from` | string | no | — | Per-message sender override (permitted only when `allow_from_override: true`). |

- **Output**: `{"message_id": "<8f3c1a9d@example.com>", "response": "250 2.0.0 Ok: queued as 4Wx1"}`
- **Provider error codes**: `SMTP_REJECTED` (5xx reply), `SMTP_DEFERRED` (4xx reply), `SMTP_TLS_FAILED`, `SMTP_PROTOCOL`, `SMTP_CONNECTION_INVALID`, `SMTP_CONNECTION_INCOMPLETE`.

---

## Redis provider (`com.orion.redis`)

Provides a shared Redis cache for workflows. Because `get`, `set`, and `delete` accept the same inputs and return the same shapes as the built-in memory cache, you can switch a connection from in-memory to Redis without modifying your workflow tasks.

```json
{
  "name": "sessions",
  "provider": "com.orion.redis",
  "config": {
    "host": "cache.example.com",
    "port": 6379,
    "db": 0,
    "tls": true,
    "username": "orion",
    "password": "env://REDIS_PASSWORD"
  }
}
```

### Connection `config` fields

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `host` | string | yes | — | Redis server hostname or IP address (URLs are refused). |
| `port` | integer | no | `6379` | Redis TCP port. |
| `db` | integer | no | `0` | Logical database number (`>= 0`). |
| `tls` | boolean | no | `false` | Enable TLS from the first byte, verified against the system trust store. |
| `username` | string | no | default user | Redis ACL username. |
| `password` | string | no | — | Secret locator (`env://` or `vault://`) at `password`. |
| `allow_private_urls` | boolean | no | `false` | Allow `host` to resolve to a private or internal IP address. |
| `key_prefix` | string | no | `"orion:<connection id>:"` | Prefix prepended to every key read or written by this connection (max `256` bytes). |
| `timeout_ms` | integer | no | `5000` | Connect and per-command timeout in milliseconds. |
| `max_value_bytes` | integer | no | `1048576` | Largest serialized JSON value `set` stores in bytes. |

### Operations

| Operation | Aliases | Input fields | Output |
|---|---|---|---|
| `get` | `cache_read`, `cache.get` | `key` (string, required) | `{"hit": true, "value": <json>}` or `{"hit": false, "value": null}` |
| `set` | `cache_write`, `cache.set` | `key` (string, required), `value` (any JSON), `ttl_ms` or `ttl_secs` (integer, up to 10 years; setting both is refused) | `{"stored": true, "key": "..."}` |
| `delete` | `cache.delete` | `key` (string, required) | `{"deleted": true, "key": "..."}` |
| `incr` | `cache_incr`, `cache.incr` | `key` (string, required), `by` (integer, default `1`), `ttl_ms` or `ttl_secs` (applied only when creating the counter) | `{"key": "...", "value": 42}` |
| `get_many` | `cache.get_many` | `keys` (array of 1 to 1000 strings, required) | `{"entries": [{"key": "...", "hit": true, "value": ...}]}` |
| `delete_many` | `cache.delete_many` | `keys` (array of 1 to 1000 strings, required) | `{"deleted": 2}` |

- **Provider error codes**: `REDIS_COMMAND_FAILED` (such as calling `incr` on a non-integer value).

---

## Object storage provider (`com.orion.storage`)

Serves one bucket in an S3-compatible store (AWS S3, Cloudflare R2, MinIO, Garage) using SigV4 signing. By default, read operations (`presign_get`, `presign_put`, `head`, `get`) are enabled and `put` is disabled until explicitly opened in `config.operations`.

```json
{
  "name": "media",
  "provider": "com.orion.storage",
  "config": {
    "endpoint": "https://s3.eu-west-1.amazonaws.com",
    "region": "eu-west-1",
    "bucket": "media",
    "access_key": "env://S3_ACCESS_KEY",
    "secret_key": "env://S3_SECRET_KEY",
    "operations": { "put": true }
  }
}
```

### Connection `config` fields

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `endpoint` | string | yes | — | Base URL (`http(s)://host[:port]`) without a path beyond `/`. |
| `region` | string | yes | — | SigV4 signing region code. |
| `bucket` | string | yes | — | Target bucket name (scoped to the connection, never passed per call). |
| `access_key` | string | no | — | Access key secret locator at `access_key`. |
| `secret_key` | string | no | — | Secret key locator at `secret_key`. |
| `session_token` | string | no | — | Optional STS token secret locator (`X-Amz-Security-Token`). |
| `force_path_style` | boolean | no | `false` | Address `endpoint/bucket/key` instead of virtual-hosted style. |
| `allow_private_urls` | boolean | no | `false` | Allow private or internal endpoint addresses. |
| `timeout_ms` | integer | no | `10000` | Per-request timeout in milliseconds. |
| `operations` | object | no | reads `true`, `put: false` | Boolean operation gates: `presign_get`, `presign_put`, `head`, `get`, `put`. |

### Operations

| Operation | Aliases | Input fields | Output |
|---|---|---|---|
| `presign` | `storage_presign` | `key` (string, required), `expires_in` (seconds or `"15m"`/`"1h"`, max 7 days, required), `method` (`"GET"` or `"PUT"`, default `"GET"`), `response_content_type` / `response_content_disposition` (for `GET`), `content_type` / `content_length` (for `PUT`) | `{"url": "https://...", "method": "GET", "expires_in": 900}` |
| `head` | `storage_head` | `key` (string, required) | `{"exists": true, "size": 1024, "etag": "...", "last_modified": "...", "content_type": "..."}` or `{"exists": false}` |
| `get` | `storage_get` | `key` (string, required), `encoding` (`"utf8"`, `"base64"`, or `"json"`, default `"utf8"`), `max_bytes` (integer, optional) | `{"exists": true, "size": 1024, "etag": "...", "last_modified": "...", "content_type": "...", "encoding": "utf8", "body": ...}` or `{"exists": false}` |
| `put` | `storage_put` | `key` (string, required), `body` (string or JSON, required), `encoding` (`"utf8"`, `"base64"`, or `"json"`, default `"utf8"`), `content_type` (string, optional) | `{"key": "...", "size": 1024, "etag": "..."}` |

- **Provider error codes**: `STORAGE_OBJECT_TOO_LARGE`, `STORAGE_OBJECT_UNDECODABLE`, `STORAGE_THROTTLED` (`429`), `STORAGE_REJECTED`, `STORAGE_CONNECTION_INCOMPLETE`.

---

## MongoDB provider (`com.orion.mongodb`)

Queries, aggregates, and writes documents on a MongoDB cluster using Extended JSON v2. Results exceeding `max_result_documents` fail with `MONGO_RESULT_TOO_LARGE` rather than silently truncating.

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

### Connection `config` fields

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `connection_string` | string | no | — | `mongodb://` or `mongodb+srv://` URL (required at use in `public_config` or `secret_locators`). |
| `database` | string | no | — | Default database name when omitted from operation inputs. |
| `username` | string | no | — | SCRAM username when not embedded in `connection_string`. |
| `password` | string | no | — | SCRAM password secret locator at `password`. |
| `allow_private_urls` | boolean | no | `false` | Allow private or internal cluster addresses. |
| `max_connections` | integer | no | `10` | Driver connection pool size. |
| `connect_timeout_ms` | integer | no | `10000` | Connect and server-selection timeout in milliseconds. |
| `query_timeout_ms` | integer | no | `30000` | Ceiling on one operation in milliseconds. |
| `max_result_documents` | integer | no | `1000` | Maximum documents a `find` or `aggregate` call may return. |
| `max_skip` | integer | no | `10000` | Largest `skip` value allowed on `find`. |
| `max_write_documents` | integer | no | `1000` | Maximum documents in a single `insert_many` call. |
| `allow_unfiltered` | boolean | no | `false` | Allow an update, replace, or delete with an empty `{}` filter when the call also sets `"all": true`. |
| `aggregate_write_stages` | boolean | no | `false` | Allow `$out` and `$merge` stages in `aggregate` pipelines. |
| `operations` | object | no | all `true` | Boolean gates: `read`, `insert`, `update`, `upsert`, `delete`. |

### Operations

| Operation | Aliases | Input fields | Output |
|---|---|---|---|
| `find` | `mongo_read` | `collection` (required), `database`, `filter` (default `{}`), `projection`, `sort`, `limit`, `skip` | Array of documents in relaxed Extended JSON v2 |
| `aggregate` | `mongo_aggregate` | `collection` (required), `database`, `pipeline` (non-empty array of stage objects, required), `allow_disk_use` (boolean, default `false`) | Array of documents |
| `write` | `mongo_write` | `collection` (required), `database`, `op` (`"insert_one"`, `"insert_many"`, `"update_one"`, `"update_many"`, `"replace_one"`, `"delete_one"`, or `"delete_many"`), plus `document`, `documents`, `ordered`, `filter`, `update`, `upsert`, `array_filters`, `all` | `{"status": "ok", "inserted": ...}` / `{"status": "ok", "matched": ..., "modified": ..., "upserted_id": ...}` / `{"status": "ok", "deleted": ...}` |

- **Provider error codes**: `MONGO_DUPLICATE_KEY`, `MONGO_RESULT_TOO_LARGE`, `MONGO_FAILED`, `MONGO_CONNECTION_INVALID`, `MONGO_CONNECTION_INCOMPLETE`.

---

## Elasticsearch provider (`com.orion.elasticsearch`)

Calls an Elasticsearch cluster's search, document, and bulk APIs under an index allowlist and search window bounds.

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

### Connection `config` fields

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `url` | string | yes | — | Cluster base URL (`http(s)://host[:port][/prefix]`). |
| `auth.type` | string | no | `"none"` | `"none"`, `"basic"`, `"api_key"`, or `"bearer"`. |
| `auth.username` / `auth.password` | string | no | — | Basic auth username and secret locator (when `auth.type` is `"basic"`). |
| `auth.api_key` | string | no | — | Encoded API key secret locator (when `auth.type` is `"api_key"`). |
| `auth.token` | string | no | — | Bearer token secret locator (when `auth.type` is `"bearer"`). |
| `allow_private_urls` | boolean | no | `false` | Allow private or internal cluster addresses. |
| `timeout_ms` | integer | no | `30000` | Per-request timeout in milliseconds. |
| `indices` | array of strings | no | `[]` | Allowed index names or prefix patterns ending in `*` (patterns are permitted for read operations only). |
| `max_hits` | integer | no | `1000` | Maximum `size` on `search` (`<= max_window`). |
| `max_window` | integer | no | `10000` | Maximum `from + size` deep-paging window on `search`. |
| `max_bulk_actions` | integer | no | `1000` | Maximum actions in a single `bulk` call. |
| `max_response_bytes` | integer | no | node cap | Maximum response byte size. |
| `operations` | object | no | all `true` | Boolean gates: `read`, `insert`, `upsert`, `update`, `delete`. |

### Operations

| Operation | Input fields | Output |
|---|---|---|
| `search` | `index` (string or array, required), `query`, `size` (default `10`), `from` (default `0`), `sort`, `_source`, `aggs`, `search_after`, `routing` | `{hits, total, total_relation, max_score, took, timed_out, shards, aggregations?, suggest?}` |
| `count` | `index` (required), `query`, `routing` | `{"count": 42}` |
| `get` | `index` (required), `id` (required), `routing`, `_source` | Document with `found: true`, or `{"found": false, "_index": "...", "_id": "..."}` |
| `index` | `index` (required), `document` (required), `id`, `op_type` (`"index"` or `"create"`), `refresh`, `routing`, `if_seq_no`, `if_primary_term`, `pipeline` | Cluster result (`_id`, `result`, `_version`) |
| `update` | `index` (required), `id` (required), `doc` or `script`, `upsert`, `doc_as_upsert`, `scripted_upsert`, `detect_noop`, `refresh`, `routing`, `retry_on_conflict`, `if_seq_no`, `if_primary_term` | Cluster result |
| `delete` | `index` (required), `id` (required), `refresh`, `routing`, `if_seq_no`, `if_primary_term` | Cluster result |
| `bulk` | `actions` (1 to `max_bulk_actions` objects with `op`: `"index"`, `"create"`, `"update"`, or `"delete"`, required), `index`, `refresh`, `pipeline` | `{status, succeeded, failed, took, items}` |

- **Provider error codes**: `ELASTICSEARCH_NOT_FOUND` (`404`), `ELASTICSEARCH_CONFLICT` (`409`), `ELASTICSEARCH_THROTTLED` (`429`), `ELASTICSEARCH_REJECTED`, `ELASTICSEARCH_RESPONSE_TOO_LARGE`, `ELASTICSEARCH_BULK_FAILED`, `ELASTICSEARCH_CONNECTION_INCOMPLETE`.

---

## ONNX models provider (`com.orion.models`)

Admits an ONNX model pinned by SHA-256 digest and runs CPU inference on `tract`. On first use (or during a connection probe), Orion fetches the artifact, verifies `digest`, validates the declared input/output tensor shapes and dtypes, and runs 5 warm-up probe inferences over zero-filled tensors within `max_probe_ms`.

```json
{
  "name": "c4-policy",
  "provider": "com.orion.models",
  "config": {
    "artifact": {
      "s3": {
        "endpoint": "https://s3.eu-west-1.amazonaws.com",
        "region": "eu-west-1",
        "bucket": "models",
        "key": "c4-tiny/1.onnx",
        "access_key": "env://MODELS_ACCESS_KEY",
        "secret_key": "env://MODELS_SECRET_KEY"
      }
    },
    "digest": "sha256:a846b06040de77b5c51789ba7b02e05618008f6524f1fc2789e4382f3f811619",
    "inputs": [{ "name": "board", "dtype": "f32", "shape": [1, 2, 6, 7] }],
    "outputs": [{ "name": "policy", "dtype": "f32", "shape": [1, 7] }]
  }
}
```

### Connection `config` fields

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `artifact` | object | yes | — | Artifact source: `artifact.storage` (`{key}` fetched via a bound `artifact_store` `com.orion.storage` connection), `artifact.s3` (`{endpoint, region, bucket, key, access_key?, secret_key?, session_token?, force_path_style?}`), or `artifact.http` (`{url, token?}`). |
| `digest` | string | yes | — | Expected `sha256:<64 lowercase hex>` digest of the `.onnx` artifact. |
| `format` | string | no | `"onnx"` | Model format (`"onnx"`). |
| `inputs` | array | yes | — | Declared input tensors: `{name, dtype, shape}`. Supported `dtype`: `"bool"`, `"i8"`, `"u8"`, `"i16"`, `"u16"`, `"i32"`, `"u32"`, `"i64"`, `"u64"`, `"f32"`, `"f64"`. Dimensions in `shape` are positive integers or symbolic names like `"N"`. |
| `outputs` | array | yes | — | Declared output tensors: `{name, dtype, shape}`. |
| `probe_dims` | object | no | `{}` | Dimension sizes used when probing symbolic dimensions (defaults to `1`). |
| `allow_private_urls` | boolean | no | `false` | Allow fetching the artifact from a private or internal address. |
| `max_artifact_bytes` | integer | no | `268435456` | Maximum artifact size in bytes (`1` byte to `2 GiB`). |
| `max_parameters` | integer | no | — | Optional ceiling on total graph parameters. |
| `max_input_elements` | integer | no | `1048576` | Maximum total input elements per inference call. |
| `max_output_elements` | integer | no | `1048576` | Maximum total output elements per inference call. |
| `timeout_ms` | integer | no | `1000` | Per-inference wall-clock ceiling in milliseconds (up to `60000`). |
| `max_probe_ms` | integer | no | `250` | Maximum allowed median latency across the 5 admission probe runs. |
| `fetch_timeout_ms` | integer | no | `300000` | Maximum artifact download time in milliseconds. |
| `max_concurrency` | integer | no | `16` | Maximum concurrent inferences for this model on one node (`1` to `1024`). |

### Operation: `infer` (alias `model_infer`)

| Input field | Type | Required | Default | Description |
|---|---|---|---|---|
| `inputs` | object | yes | — | Map of input tensor names to nested JSON arrays (or `{shape, data}` objects when `layout` is `"flat"`). |
| `timeout_ms` | integer | no | — | Per-call timeout that can shorten the connection's `timeout_ms`. |
| `layout` | string | no | `"nested"` | Tensor encoding format: `"nested"` or `"flat"`. |

- **Output**: `{"outputs": {"policy": [[...]]}, "stats": {"digest": "...", "runtime": "tract", "device": "cpu", "inference_ms": 0.3, ...}}`
- **Provider error codes**: `MODEL_UNAVAILABLE` (admission failed at `details.stage`), `MODEL_BUSY`, `MODEL_TIMEOUT` (only retryable code), `MODEL_OUTPUT_TOO_LARGE`, `MODEL_RUN_FAILED`, `MODEL_CONNECTION_INCOMPLETE`.

## Next steps

- [Kafka channels and producer](./kafka.md): Full guide for `com.orion.kafka` and `kafka` channels.
- [Custom provider packages](./provider-packages.md): Build and admit your own connector provider packages.
