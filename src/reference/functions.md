<!-- description: Complete reference for Orion's built-in task functions: data transforms, HTTP/SQL/cache connector calls, channel composition, crypto/JWT, and retry safety. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# Task functions

Every workflow step invokes a task function by setting `"function": { "name": "<function>", "input": { ... } }`. Some functions are contributed by the `dataflow-rs` engine (`Data` category), while the remaining functions are Orion handlers that interact with [connectors](./connectors.md), compose channels, or perform cryptographic and JWT operations.

Query `GET /api/v2/admin/functions` on any running node to inspect the live function catalog (including loaded WebAssembly plugin functions).

---

## Function summary

<div class="table-filter" data-label="Filter functions"></div>

| Function | Category | Connector | Purpose |
|----------|----------|:---------:|---------|
| [`parse_json`](#parse_json) | Data | — | Parse a raw JSON string or request payload into the `data` context |
| [`parse_xml`](#parse_xml) | Data | — | Parse a raw XML string into the `data` context |
| [`map`](#map) | Data | — | Transform and reshape context fields using JSONLogic expressions |
| [`filter`](#filter) | Data | — | Gate workflow execution on a JSONLogic condition |
| [`validation`](#validation) | Data | — | Evaluate JSONLogic validation rules and collect error messages |
| [`log`](#log) | Data | — | Emit a structured log entry with evaluated JSONLogic fields |
| [`publish_json`](#publish_json) | Data | — | Serialize a `data` field to a JSON string |
| [`publish_xml`](#publish_xml) | Data | — | Serialize a `data` field to an XML string |
| [`http_call`](#http_call) | Connector | HTTP | Send an outbound HTTP request with retry and circuit breaker support |
| [`data_query`](#data_query) | Connector | SQL | Run a portable, backend-neutral SQL read query |
| [`data_write`](#data_write) | Connector | SQL | Run a portable, backend-neutral SQL `insert`, `update`, `delete`, or `upsert` |
| [`db_read`](#db_read) | Connector | SQL | Execute a parameterized raw SQL `SELECT` query |
| [`db_write`](#db_write) | Connector | SQL | Execute a parameterized raw SQL `INSERT`, `UPDATE`, or `DELETE` statement |
| [`cache_read`](#cache_read) | Connector | Cache | Read one key or a batch of keys from an in-memory cache connector |
| [`cache_write`](#cache_write) | Connector | Cache | Write a key-value entry to an in-memory cache connector with optional TTL |
| [`cache_delete`](#cache_delete) | Connector | Cache | Delete exact keys from an in-memory cache connector |
| [`cache_incr`](#cache_incr) | Connector | Cache | Atomically increment or decrement an integer counter in cache |
| [`connector_call`](#connector_call) | Connector | — | Invoke a v2 connector provider operation (including <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> providers) |
| [`channel_call`](#channel_call) | Composition | — | Invoke another channel's workflow in-process |
| [`cache_invalidate`](#cache_invalidate) | Utility | — | Bump channel response-cache invalidation namespaces across stores |
| [`crypto`](#crypto) | Utility | — | Compute SHA/MD5 digests, compute/verify HMACs, and hash/verify passwords |
| [`jwt_sign`](#jwt_sign) | Utility | — | Mint a signed JWT (`HS*`, `RS*`, `PS*`, `ES*`, `EdDSA`) |
| [`jwt_verify`](#jwt_verify) | Utility | — | Verify a JWT against static keys or a remote JWKS |

---

## Data functions (engine built-ins)

### `parse_json`

Parses a raw JSON string (or `"payload"` for the incoming request body) and writes the parsed value under `data.<target>`.

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `source` | string | yes | — | Source field to parse (use `"payload"` for the raw request payload) |
| `target` | string | yes | — | Destination key under `data` (`data.<target>`) |

```json
{ "name": "parse_json", "input": { "source": "payload", "target": "order" } }
```

### `parse_xml`

Parses an XML string into a JSON object under `data.<target>`.

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `source` | string | yes | — | Source XML field (for example, `"payload"`) |
| `target` | string | yes | — | Destination key under `data` (`data.<target>`) |

### `map`

Applies an ordered list of JSONLogic mappings to write, append, extend, or unset paths in `data` or `temp_data`.

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `mappings` | array | yes | — | Ordered list of mapping objects applied sequentially |
| `mappings[].path` | string \| JSONLogic | yes | — | Dotted target path (such as `"data.order.total"` or `"temp_data.rate"`) |
| `mappings[].logic` | JSONLogic | yes, unless `unset` | — | Expression evaluated against `{data, metadata, temp_data}` |
| `mappings[].mode` | string | no | `"set"` | `"set"` replaces the value at `path`; `"append"` pushes onto an array; `"extend"` concatenates an array result |
| `mappings[].on_null` | string | no | `"skip"` | Behavior when `logic` evaluates to `null`: `"skip"` leaves `path` unchanged; `"unset"` removes the key |
| `mappings[].unset` | bool | no | `false` | When `true`, deletes the key at `path` (takes no `logic`) |

```json
{
  "name": "map",
  "input": {
    "mappings": [
      { "path": "data.order.tax", "logic": { "*": [{ "var": "data.order.subtotal" }, 0.08] } },
      { "path": "data.order.internal_token", "unset": true }
    ]
  }
}
```

### `filter`

Evaluates a JSONLogic `condition` and either halts the workflow (`299`) or skips the current step when falsy.

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `condition` | JSONLogic | yes | — | Boolean predicate evaluated against `{data, metadata, temp_data}` |
| `on_reject` | string | no | `"halt"` | `"halt"` stops the workflow (including enclosing loops); `"skip"` skips only this task |

### `validation`

Evaluates an array of JSONLogic rules (`validate` is an accepted alias). Each rule whose `logic` does not evaluate to `true` appends `message` to the response's `errors` list and records status `400`. Set `"on_rejection": "halt"` on the task to stop the workflow when any rule fails.

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `rules` | array | yes | — | Non-empty list of `{ "logic", "message" }` validation rules |
| `rules[].logic` | JSONLogic | yes | — | Expression that must evaluate to `true` to pass |
| `rules[].message` | string | yes | — | Error message recorded when `logic` is falsy |

### `log`

Emits a structured log message with optional key-value fields.

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `message` | JSONLogic | yes | — | Log message string or JSONLogic expression |
| `level` | string | no | `"info"` | `"trace"`, `"debug"`, `"info"`, `"warn"`, or `"error"` |
| `fields` | object | no | `{}` | Map of structured field names to JSONLogic expressions |

### `publish_json`

Serializes `data.<source>` into a JSON string at `data.<target>`.

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `source` | string | yes | — | Field name under `data` to serialize |
| `target` | string | yes | — | Field name under `data` to receive the JSON string |
| `pretty` | bool | no | `false` | Format the output with indentation |

### `publish_xml`

Serializes `data.<source>` into an XML string at `data.<target>`.

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `source` | string | yes | — | Field name under `data` to serialize |
| `target` | string | yes | — | Field name under `data` to receive the XML string |
| `root_element` | string | no | `"root"` | XML root element tag name |

---

## Connector functions

### `http_call`

Sends an HTTP request through a configured [`http` connector](./connectors.md#http-connectors-orgorionhttp).

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `connector` | string \| JSONLogic | yes | — | Name of the `http` connector |
| `method` | string | no | `"GET"` | `"GET"`, `"POST"`, `"PUT"`, `"PATCH"`, or `"DELETE"` |
| `path` | string \| JSONLogic | no | — | Path appended to the connector's base `url` |
| `headers` | object | no | `{}` | Request headers (each value is evaluated as JSONLogic) |
| `body` | any \| JSONLogic | no | — | Request body payload |
| `body_format` | string \| JSONLogic | no | `"json"` | Request body serialization: `"json"`, `"form"` (`application/x-www-form-urlencoded`), or `"text"` |
| `output` | string \| JSONLogic | no | — | Dotted context path where the response body is written (omit to discard) |
| `response_format` | string \| JSONLogic | no | `"json"` | Response decoding mode: `"json"` or `"text"` |
| `timeout_ms` | number \| JSONLogic | no | `30000` | Per-attempt timeout in milliseconds |

### `data_query`

Executes a backend-neutral read query using the [portable data dialect](./connectors.md#2-portable-data-dialect-data_query-and-data_write).

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `connector` | string | yes | — | Name of the `db` connector |
| `query` | object | yes | — | Portable query envelope: `source`, `filter`, `fields`, `sort`, `limit`, `skip`, `after`, `include`, `count` |
| `params` | object | no | `{}` | Named parameter map referenced via `{"param": "<name>"}` inside `query.filter` and `query.after` |
| `schema` | object | yes | — | Inline [entity schema registry](./connectors.md#the-schema-registry) (`entities` and `unmapped`) |
| `numeric_as` | string | no | `"number"` | Decimal decoding mode: `"number"` or `"string"` (use `"string"` for exact currency values) |
| `binary_as` | string | no | `"auto"` | Binary column decoding mode: `"auto"`, `"hex"`, `"base64"`, or `"text"` |
| `output` | string \| JSONLogic | no | `"data"` | Dotted path where the result array (or `{"count": n}`) is written |

### `data_write`

Executes a backend-neutral mutation (`insert`, `update`, `delete`, `upsert`) using the [portable data dialect](./connectors.md#write-envelope-data_write-inputwrite).

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `connector` | string | yes | — | Name of the `db` connector |
| `write` | object | yes | — | Portable write envelope: `op`, `target`, `values`, `set`, `filter`, `on_conflict`, `returning`, `all` |
| `params` | object | no | `{}` | Named parameter map referenced via `{"param": "<name>"}` in `values`, `set`, and `filter` |
| `schema` | object | yes | — | Inline [entity schema registry](./connectors.md#the-schema-registry) |
| `numeric_as` | string | no | `"number"` | Decimal decoding mode for `returning` rows: `"number"` or `"string"` |
| `binary_as` | string | no | `"auto"` | Binary decoding mode for `returning` rows: `"auto"`, `"hex"`, `"base64"`, or `"text"` |
| `output` | string \| JSONLogic | no | `"data"` | Dotted path where `{"status": "ok", "rows_affected": n, ...}` is written |

### `db_read`

Executes a parameterized raw SQL `SELECT` query (`?` placeholders on SQLite/MySQL; `$1, $2, ...` on PostgreSQL). The statement may also be loaded from a file at compile time via `{"$sql": "sql/query.sql"}`.

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `connector` | string | yes | — | Name of the `db` connector |
| `query` | string | yes | — | Read-only SQL statement (`SELECT`, `WITH`, `VALUES`, or `TABLE`; data-modifying CTEs and `EXPLAIN` are refused) |
| `params` | array | no | — | Positional parameter array (`{"var": "..."}` references are resolved in order) |
| `numeric_as` | string | no | `"number"` | How `numeric`/`decimal` columns decode: `"number"` (float64) or `"string"` (exact decimal text) |
| `binary_as` | string | no | `"auto"` | How `bytea`/`blob` columns decode: `"auto"` (UTF-8 text or hex), `"hex"`, `"base64"`, or `"text"` |
| `output` | string \| JSONLogic | no | `"data"` | Dotted path where the returned JSON row array is written |

### `db_write`

Executes a parameterized raw SQL `INSERT`, `UPDATE`, or `DELETE` statement (gated by `operations.raw_write` on the connector).

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `connector` | string | yes | — | Name of the `db` connector |
| `query` | string | yes | — | SQL mutation statement with bind placeholders (`?` or `$1`) |
| `params` | array | no | — | Positional parameter array (`{"var": "..."}` references are resolved in order) |
| `output` | string \| JSONLogic | no | `"data"` | Dotted path where `{"rows_affected": n}` (plus `last_insert_id` on MySQL/SQLite inserts) is written |

### `cache_read`

Reads one key (`key`) or up to 1,000 keys (`keys`) from an in-memory `cache` connector.

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `connector` | string | yes | — | Name of the `cache` connector |
| `key` | string \| JSONLogic | one of | — | Single cache key to read |
| `keys` | array \| JSONLogic | one of | — | Array of up to `1000` cache keys to read in one call |
| `output` | string \| JSONLogic | no | `"data"` | Dotted path where the cached value (or array of values for `keys`) is written |

### `cache_write`

Writes a value to an in-memory `cache` connector.

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `connector` | string | yes | — | Name of the `cache` connector |
| `key` | string | yes | — | Cache key to write |
| `value` | any | yes | — | Value to store |
| `ttl_secs` | number | no | no expiry | Optional time-to-live in seconds |

### `cache_delete`

Deletes exact keys from an in-memory `cache` connector.

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `connector` | string | yes | — | Name of the `cache` connector |
| `keys` | array \| JSONLogic | yes | — | Array of `1` to `1000` exact keys to delete |
| `output` | string \| JSONLogic | no | nothing recorded | Dotted path where `{"deleted": n}` is written |

### `cache_incr`

Atomically increments (or decrements) an integer counter in an in-memory `cache` connector.

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `connector` | string | yes | — | Name of the `cache` connector |
| `key` | string \| JSONLogic | yes | — | Counter key (initialized to `0` before adding `by` when absent) |
| `by` | integer \| JSONLogic | no | `1` | Signed integer delta to add |
| `ttl_secs` | number \| JSONLogic | no | no expiry | Expiration in seconds applied **only when the key is created** |
| `output` | string \| JSONLogic | no | nothing recorded | Dotted path where the updated counter value is written |

### `connector_call`

Invokes an operation on any v2 connector provider—including built-in drivers (`org.orion.*`), the seven [Enterprise connector providers](../enterprise/providers.md) (`com.orion.*`), and admitted [custom provider packages](../enterprise/provider-packages.md).

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `connection` | string | yes | — | Deployed connection ID |
| `operation` | string | yes | — | Provider operation name (such as `"send"`, `"publish"`, `"get"`, `"find"`, `"search"`, `"infer"`) |
| `kind` | string | no | `"rust"` | Provider implementation kind: `"rust"`, `"workflow"`, or `"wasm"` |
| `target_id` | string | no | `connection` | Provider driver ID (`"com.orion.smtp"`) or package digest (`"sha256:..."`) |
| `input` | object | no | `{}` | Operation input validated against the provider operation's input schema |
| `idempotency_key` | string \| JSONLogic | no | — | Optional idempotency key forwarded to the provider |
| `output` | string \| JSONLogic | no | `"data"` | Dotted context path where the validated operation result is written |

---

## Composition and utility functions

### `channel_call`

Invokes another channel's workflow in-process (bounded by `engine.max_channel_call_depth`, default `8`, and cycle-checked at compile time).

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `channel` | string \| JSONLogic | yes | — | Target channel name |
| `data` | any \| JSONLogic | no | request payload | Input `data` passed to the target channel |
| `output` | string \| JSONLogic | no | `"data"` | Dotted path where the target channel's result is written |
| `timeout_ms` | number | no | from config | Per-call timeout override in milliseconds |

### `cache_invalidate`

Increments the version counter for one or more [channel response-cache namespaces](./channels.md#configcache), invalidating cached entries across all stores on the node (and in the Redis cluster store).

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `namespaces` | array \| JSONLogic | yes | — | `1` to `64` namespace names (`[a-z0-9_.:-]{1,64}`) to invalidate |
| `output` | string \| JSONLogic | no | nothing recorded | Dotted path where `{"namespaces": n, "stores": m}` is written |

### `crypto`

Performs local cryptographic hashing, constant-time HMAC generation/verification, and Argon2id/bcrypt password hashing/verification.

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `op` | string | yes | — | `"hash"`, `"hmac"`, `"hmac_verify"`, `"password_hash"`, or `"password_verify"` |
| `algorithm` | string | no | per `op` | `hash`: `"sha256"` (default), `"sha512"`, `"sha1"`, `"md5"`. `hmac`/`hmac_verify`: `"sha256"` (default), `"sha512"`, `"sha1"`. `password_hash`: `"argon2id"` (default), `"bcrypt"` |
| `data` | any | conditional | — | Input payload for `hash`, `hmac`, and `hmac_verify` |
| `input_encoding` | string | no | `"utf8"` | How string `data` is decoded into bytes: `"utf8"`, `"hex"`, or `"base64"` |
| `key` | string | conditional | — | Secret key for `hmac` and `hmac_verify` (use `{"secret": "<name>"}` or `env://VAR`) |
| `key_encoding` | string | no | `"utf8"` | Key decoding format: `"utf8"`, `"hex"`, or `"base64"` |
| `signature` | string | conditional | — | Presented MAC to check in `hmac_verify` (`hex`, `base64`, or `base64url` auto-detected and compared in constant time) |
| `password` | string | conditional | — | Plaintext password for `password_hash` and `password_verify` |
| `hash` | string | conditional | — | Stored PHC password hash string (`$argon2id$...` or `$2b$...`) for `password_verify` |
| `encoding` | string | no | `"hex"` | Output encoding for `hash` and `hmac`: `"hex"`, `"base64"`, or `"base64url"` |
| `params` | object | no | safe defaults | Cost tuning for `password_hash`: Argon2id `{memory_kib, iterations, parallelism}` or bcrypt `{cost}` |
| `output` | string \| JSONLogic | no | `"data"` | Dotted path where the digest/hash string or verification boolean is written |

### `jwt_sign`

Mints a signed JSON Web Token (JWS).

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `algorithm` | string | yes | — | `"HS256"`, `"HS384"`, `"HS512"`, `"RS256"`, `"RS384"`, `"RS512"`, `"PS256"`, `"PS384"`, `"PS512"`, `"ES256"`, `"ES384"`, or `"EdDSA"` |
| `key` | string | yes | — | Signing secret or private key PEM (`{"secret": "<name>"}` or `env://VAR`) |
| `key_encoding` | string | no | `"utf8"` | Encoding for `HS*` secrets: `"utf8"`, `"base64"`, or `"hex"` |
| `claims` | object | no | `{}` | Custom claims object (`{"var": "..."}` references are folded from the context) |
| `expires_in` | number \| string | conditional | — | Token lifetime in seconds or `"<n>s\|m\|h\|d"` (required unless `claims.exp` is set) |
| `claims.iat` | number | no | current time | Optional explicit issued-at Unix timestamp |
| `issuer` / `audience` / `not_before` | — | no | — | Convenience fields setting `iss`, `aud`, and `nbf` |
| `kid` | string | no | — | Optional `kid` header stamped on the token |
| `output` | string \| JSONLogic | no | `"data"` | Dotted path where the compact JWT string is written |

### `jwt_verify`

Verifies a compact JWS inside a workflow against static keys or an HTTPS JWKS endpoint.

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `token` | string | yes | — | Compact JWT string (or `{"var": "..."}` reference) |
| `algorithms` | array | yes | — | Non-empty allowlist of accepted algorithms |
| `keys` | array | one of | — | Static verification keys `[{algorithm, key, kid?, key_encoding?}]` |
| `jwks_url` | string | one of | — | HTTPS JWKS URL |
| `issuer` / `audience` | string \| array | no | — | Expected `iss` and `aud` claim values |
| `leeway_secs` | number | no | `30` | Clock-skew allowance in seconds (`<= 300`) |
| `require_exp` | boolean | no | `true` | Require the `exp` claim on verified tokens |
| `output` | string \| JSONLogic | no | `"data"` | Dotted path where the verified claims object is written |

---

## Retry safety

Every Orion handler declares its retry safety in `GET /api/v2/admin/functions` (`pure`, `read`, `idempotent_write`, `unsafe_write`, or `depends_on <input>`). Engine `Data` built-ins (`map`, `filter`, `validation`, `parse_json`, `parse_xml`, `publish_json`, `publish_xml`, `log`) are all `pure`.

## Functions

| Function | Retry safety | Notes |
|---|---|---|
| [`crypto`](#crypto) | `pure` | Local computation with no external side effects. |
| [`jwt_sign`](#jwt_sign) | `pure` | Local token signing. |
| [`cache_read`](#cache_read) | `read` | Reads in-memory cache state without modifying it. |
| [`db_read`](#db_read) | `read` | Executes a read-only SQL query. |
| [`data_query`](#data_query) | `read` | Executes a read-only portable query. |
| [`jwt_verify`](#jwt_verify) | `read` | Local verification (with cached JWKS lookup when `jwks_url` is used). |
| [`cache_write`](#cache_write) | `idempotent_write` | Re-writing the same key and value yields the same cached state. |
| [`cache_invalidate`](#cache_invalidate) | `idempotent_write` | Bumping a namespace counter a second time causes at most one extra cache miss. |
| [`cache_delete`](#cache_delete) | `idempotent_write` | Deleting already-deleted keys leaves the cache in the same state. |
| [`cache_incr`](#cache_incr) | `unsafe_write` | Retrying adds `by` a second time. |
| [`http_call`](#http_call) | `depends_on` `method` | `GET`, `HEAD`, `PUT`, and `DELETE` are idempotent; `POST` and `PATCH` may duplicate side effects unless an idempotency key is used. |
| [`db_write`](#db_write) | `depends_on` `sql` | `UPDATE ... SET x = 1` is idempotent; `INSERT` without conflict handling is not. |
| [`data_write`](#data_write) | `depends_on` `op` | `upsert` and `delete` are idempotent; `insert` is not. |
| [`connector_call`](#connector_call) | `depends_on` `operation` | Read operations (`get`, `head`, `find`, `search`, `count`, `infer`) are safe to retry; write operations (`send`, `publish`) are not. |
| [`channel_call`](#channel_call) | `depends_on` `channel` | Inherits the retry safety of the target channel's workflow. |

---

## Runtime function discovery

Query `GET /api/v2/admin/functions` (or run `orion-cli functions list` / `orion-cli functions get <name>`) to inspect the live catalog of built-in and plugin functions on a running server:

<!-- tabs:start -->
### **CLI**
```bash
orion-cli functions list
orion-cli functions get http_call
```

### **curl**
```bash
curl -s http://localhost:8080/api/v2/admin/functions \
  -H "Authorization: Bearer $ORION_ADMIN_KEY" | jq .
```
<!-- tabs:end -->

## Next steps

- [Workflow schema and formatting](./workflows.md): How tasks, conditions, loops, and `for_each` compose functions.
- [Expression language (JSONLogic)](./expressions.md): All operators available inside `map`, `filter`, `validation`, and dynamic task inputs.
