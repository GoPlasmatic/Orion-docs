<!-- description: The com.orion.redis provider: server connection fields, six cache operations and answers, keyspace prefixing and error codes. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# Redis provider

Available in Orion Standard and Enterprise only. The connection fields and six cache operations of `com.orion.redis`, which stores workflow data in a Redis server.

## Synopsis

A definition set declares the connection with `provider` in place of `connector_type`:

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

## Description <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

The provider serves the memory cache's `get`, `set` and `delete` with the same names, aliases, inputs and answers. A connection therefore moves from per-node memory to a shared server without a workflow change. `incr`, `get_many` and `delete_many` are the operations a shared cache adds.

The server is long-lived state shared with other applications, so the provider isolates a connection by key prefix instead of by revision. The prefix belongs to the connection, so a new revision keeps its keys.

This connection is not the cluster's shared store. Cluster mode reads its store from the node's `[extensions.redis]` section, which is separate.

## Fields

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `host` | string | yes | — | The server host name or IP address; a URL is refused. |
| `port` | integer | no | `6379` | The server port, 6379 for a default Redis deployment. |
| `db` | integer | no | `0` | The logical database, which must be zero or a positive number. |
| `tls` | boolean | no | `false` | Whether the session uses TLS from the first byte, verified against the system trust store. |
| `username` | string | no | — | The ACL user; unset uses the server's default user. |
| `password` | string | no | — | The password, always a secret locator at the `password` configuration path. |
| `allow_private_urls` | boolean | no | `false` | Whether the server may be a private or internal address. |
| `key_prefix` | string | no | `"orion:<connection id>:"` | The prefix every key this connection reads or writes carries, at most 256 bytes. |
| `timeout_ms` | integer | no | `5000` | The connect and per-command timeout in milliseconds, within the run deadline. |
| `max_value_bytes` | integer | no | `1048576` | The largest value `set` stores, measured as serialised JSON bytes. |

## Operations

| Operation | Aliases | Input | Answer |
|-----------|---------|-------|--------|
| `get` | `cache_read`, `cache.get` | `{key}` | `{hit, value}` |
| `set` | `cache_write`, `cache.set` | `{key, value?, ttl_ms? \| ttl_secs?}` | `{stored, key}` |
| `delete` | `cache.delete` | `{key}` | `{deleted, key}` |
| `incr` | `cache_incr`, `cache.incr` | `{key, by?, ttl_ms? \| ttl_secs?}` | `{key, value}` |
| `get_many` | `cache.get_many` | `{keys}` | `{entries}` |
| `delete_many` | `cache.delete_many` | `{keys}` | `{deleted}` |

### `get`

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `key` | string | yes | — | The cache key whose value the call reads from the server. |

A miss answers `{"hit": false, "value": null}`.

### `set`

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `key` | string | yes | — | The cache key that receives the value the call writes. |
| `value` | any JSON | no | `null` | The value to store, serialised as JSON before it is written. |
| `ttl_ms` | integer | no | — | The expiry in milliseconds, from 1 to ten years. |
| `ttl_secs` | integer | no | — | The expiry in seconds, from 1 to ten years. |

A field that is absent or `null` sets nothing. `ttl_ms` and `ttl_secs` together are refused.

### `delete`

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `key` | string | yes | — | The cache key the call deletes from the server. |

The answer reports whether the key existed.

### `incr`

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `key` | string | yes | — | The counter key whose integer value the call increments. |
| `by` | integer | no | `1` | The amount the counter changes by, which may be negative. |
| `ttl_ms` | integer | no | — | The expiry in milliseconds, applied only when the call creates the counter. |
| `ttl_secs` | integer | no | — | The expiry in seconds, applied only when the call creates the counter. |

### `get_many`

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `keys` | array of strings | yes | — | The keys to read, from 1 to 1000 in one call. |

The answer's `entries` holds one `{key, hit, value}` object per key, in input order.

### `delete_many`

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `keys` | array of strings | yes | — | The keys to delete, from 1 to 1000 in one call. |

The answer's `deleted` is the number of keys that existed.

## Errors

| Code | Class | Condition |
|------|-------|-----------|
| `REDIS_COMMAND_FAILED` | provider_failure | The server refused the command, such as an `incr` of a non-integer. |
| `CONNECTOR_INVALID_INPUT` | invalid_input | A key, TTL, counter amount or value size is not valid. |
| `CONNECTOR_EGRESS_DENIED` | forbidden | The server is private and the connection does not allow it. |
| `CONNECTOR_DOWNSTREAM_UNAUTHORIZED` | unauthorized | The server refused the credentials. |
| `CONNECTOR_UNAVAILABLE` | unavailable | The server is unreachable or dropped the connection. |
| `CONNECTOR_DEADLINE_EXCEEDED` | deadline | The run's deadline passed during the command. |

## Examples

The task writes a session with a fifteen-minute expiry:

```json
{
  "name": "connector_call",
  "input": {
    "connection": "sessions",
    "target_id": "com.orion.redis",
    "operation": "set",
    "input": {
      "key": { "var": "data.session_id" },
      "value": { "var": "data.session" },
      "ttl_secs": 900
    },
    "output": "data.cached"
  }
}
```

The answer echoes the caller's key without the connection prefix:

```json
{
  "stored": true,
  "key": "9f2c41"
}
```

## Caveats

- A value is stored as its JSON serialisation. A value another client wrote that is not JSON reads back as a string.
- An `incr` sets its expiry only when the call creates the counter, never on a later increment.
- `ttl_ms` and `ttl_secs` are refused together. Either may be `null`, which sets no expiry.
- `key_prefix` defaults to `orion:<connection id>:`, so two connections on one server do not share keys unless they set the same prefix.
- This connection is not the cluster store. Cluster mode reads `[extensions.redis]` from the node's configuration.
- The largest expiry is ten years, because a write lands before its expiry is applied.

## Related

- [Edition providers](./index.md): the seven providers and the rules they share.
- [`connector_call`](../../functions/connector_call.md): the task that invokes a cache operation.
- [`cache` connectors](../cache.md): the per-node memory cache these operation names also serve.
- [Extension settings](../../configuration/extensions.md): the `[extensions.redis]` cluster store, which is a different connection.
- [Edition capability matrix](../../editions.md): the seven providers with their minimum edition.
