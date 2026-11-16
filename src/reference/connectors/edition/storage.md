<!-- description: The com.orion.storage S3-compatible provider: bucket connection fields, presign, head, get and put operations, and error codes. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# Storage provider

Available in Orion Standard and Enterprise only. The connection fields and four operations of `com.orion.storage`, which serves one bucket of an S3-compatible store.

## Synopsis

A definition set declares the connection with `provider` in place of `connector_type`:

```json
{
  "name": "media",
  "provider": "com.orion.storage",
  "config": {
    "endpoint": "https://s3.eu-west-1.amazonaws.com",
    "region": "eu-west-1",
    "bucket": "media",
    "access_key": "env://S3_ACCESS_KEY",
    "secret_key": "env://S3_SECRET_KEY"
  }
}
```

## Description <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

The provider signs every request with SigV4 and serves AWS S3, MinIO, Cloudflare R2, Linode, Garage and any S3-compatible endpoint. `presign` answers a time-limited URL computed on the node; `head`, `get` and `put` make one signed request each.

A missing object is data, not failure: `head` and `get` answer `{"exists": false}` for a 404. A refusal from the store is an error with the store's status in `details`.

## Fields

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `endpoint` | string | yes | — | The base URL, `http(s)://host[:port]`, without a path beyond `/`. |
| `region` | string | yes | — | The SigV4 signing region, written as the store's own region code. |
| `bucket` | string | yes | — | The one bucket this connection reads and writes, never chosen per call. |
| `access_key` | string | no | — | The access key, always a secret locator at `access_key`. |
| `secret_key` | string | no | — | The secret key, always a secret locator at `secret_key`. |
| `session_token` | string | no | — | An STS token, sent as `X-Amz-Security-Token` and always a secret locator. |
| `force_path_style` | boolean | no | `false` | Whether requests address `endpoint/bucket/key` instead of the virtual-hosted form. |
| `allow_private_urls` | boolean | no | `false` | Whether the endpoint may be a private or internal address. |
| `timeout_ms` | integer | no | `10000` | The per-request timeout in milliseconds, within the run deadline. |
| `operations` | object | no | reads on, `put` off | The gates `presign_get`, `presign_put`, `head`, `get` and `put`, on unless set otherwise. |

The `operations` gates default to `presign_get: true`, `presign_put: true`, `head: true`, `get: true` and `put: false`. A closed gate refuses the call with `CONNECTOR_OPERATION_FORBIDDEN`.

## Operations

| Operation | Aliases | Input | Answer |
|-----------|---------|-------|--------|
| `presign` | `storage_presign` | `key`, `expires_in`, and the method's fields | `{url, method, expires_in}` |
| `head` | `storage_head` | `{key}` | object metadata, or `{exists: false}` |
| `get` | `storage_get` | `{key, encoding?, max_bytes?}` | metadata plus `encoding` and `body` |
| `put` | `storage_put` | `{key, body, encoding?, content_type?}` | `{key, size, etag}` |

### `presign`

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `key` | string | yes | — | The object key, from 1 to 1024 bytes and not only slashes. |
| `method` | string | no | `"GET"` | `GET` for a download or `PUT` for an upload. |
| `expires_in` | integer or string | yes | — | The URL lifetime in seconds, or as a number with an `s`, `m`, `h` or `d` suffix. |
| `response_content_type` | string | no | — | The `Content-Type` a GET response is served with, signed into the URL. |
| `response_content_disposition` | string | no | — | The `Content-Disposition` a GET response is served with, signed into the URL. |
| `content_type` | string | no | — | The `Content-Type` a PUT must send, signed into the URL. |
| `content_length` | integer | no | — | The exact body length a PUT must send, at least 1 byte. |

`response_content_type` and `response_content_disposition` apply to `GET`; `content_type` and `content_length` apply to `PUT`. A field from the other method is refused.

### `head`

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `key` | string | yes | — | The object key to inspect, within the connection's one bucket. |

The answer carries `exists`, and for a present object `size`, `etag`, `last_modified` and `content_type`.

### `get`

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `key` | string | yes | — | The object key to read, within the connection's one bucket. |
| `encoding` | string | no | `"utf8"` | How the bytes travel: `utf8`, `base64` or `json`, with `utf8` the default. |
| `max_bytes` | integer | no | — | The largest body to read, at least 1 byte and no more than the node's cap. |

The answer carries the metadata, the `encoding` and the decoded `body`. A body over the bound fails the call rather than truncating.

### `put`

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `key` | string | yes | — | The object key to write, within the connection's one bucket. |
| `body` | string or any JSON | yes | — | The object body; any JSON is accepted with `encoding: "json"`. |
| `encoding` | string | no | `"utf8"` | How the body travels: `utf8`, `base64` or `json`, with `utf8` the default. |
| `content_type` | string | no | — | The request's content type; a `json` body defaults to `application/json`. |

`put` is closed unless the connection sets `operations.put` to `true`.

## Returns

| Operation | Fields |
|-----------|--------|
| `presign` | `url`, `method`, `expires_in` |
| `head` | `exists`, `size`, `etag`, `last_modified`, `content_type` |
| `get` | `exists`, `size`, `etag`, `last_modified`, `content_type`, `encoding`, `body` |
| `put` | `key`, `size`, `etag` |

An `etag` is answered without its surrounding quotes. A field the store does not send is `null`.

## Errors

| Code | Class | Condition |
|------|-------|-----------|
| `STORAGE_OBJECT_TOO_LARGE` | provider_failure | A `get` object is over `max_bytes` or the node's response cap. |
| `STORAGE_OBJECT_UNDECODABLE` | provider_failure | A `get` object does not decode as its `encoding`. |
| `STORAGE_THROTTLED` | rate_limited | The store answered 429. |
| `STORAGE_REJECTED` | provider_failure | The store answered any other refusal; `details.status` names it. |
| `STORAGE_CONNECTION_INCOMPLETE` | provider_failure | A key is not resolved from its secret locator. |
| `CONNECTOR_INVALID_INPUT` | invalid_input | A key, endpoint, encoding, expiry or method field is not valid. |
| `CONNECTOR_OPERATION_FORBIDDEN` | forbidden | The connection's `operations` block closes the requested operation. |
| `CONNECTOR_EGRESS_DENIED` | forbidden | The endpoint is private and the connection does not allow it. |
| `CONNECTOR_DOWNSTREAM_UNAUTHORIZED` | unauthorized | The store answered 401 or 403. |
| `CONNECTOR_UNAVAILABLE` | unavailable | The store is unreachable or answered 5xx. |

## Examples

The task signs a fifteen-minute download URL for a video:

```json
{
  "name": "connector_call",
  "input": {
    "connection": "media",
    "target_id": "com.orion.storage",
    "operation": "presign",
    "input": {
      "key": { "var": "data.video_key" },
      "expires_in": "15m"
    },
    "output": "data.download"
  }
}
```

The answer carries the signed URL and its lifetime in seconds:

```json
{
  "url": "https://media.s3.eu-west-1.amazonaws.com/videos/1042.mp4?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Expires=900&X-Amz-Signature=…",
  "method": "GET",
  "expires_in": 900
}
```

## Caveats

- One bucket belongs to one connection. An input has no `bucket` field, and a `bucket` key is refused.
- `presign` computes the signature locally and makes no request, so it works while the store is unreachable.
- `get` reads at most the lower of `max_bytes` and the node's response cap.
- `put` is closed by default. A read-only media connection can also close `presign_put`.
- A key is 1 to 1024 bytes and may not consist only of slashes or whitespace.
- A 404 answers `{"exists": false}` for `head` and `get`, which is data rather than a failure.
- A presigned URL lives at most 604800 seconds, S3's own ceiling of 7 days.

## Related

- [Edition providers](./index.md): the seven providers and the rules they share.
- [`connector_call`](../../functions/connector_call.md): the task that invokes a storage operation.
- [Models provider](./models.md): ONNX artifacts read from a bound storage connection.
- [Edition capability matrix](../../editions.md): the seven providers with their minimum edition.
