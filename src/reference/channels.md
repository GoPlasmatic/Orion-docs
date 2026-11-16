<!-- description: Complete reference for Orion channel definitions: routing fields, cron transport_config, ingress guard matrix, auth, rate limits, caching, and response shaping. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# Channel configuration schema

A channel defines how requests, events, or scheduled ticks enter Orion, which ingress guards protect the boundary, and which workflow executes. Unknown keys at any level of a channel definition are rejected at upload and compile time.

For a walkthrough of common channel patterns, see [Configure channels and guards](../guides/channels.md).

## Example channel definition

```json
{{#include ../../examples/packages/webhook-transform/channel.json}}
```

---

## Top-level routing and protocol fields

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `channel_id` | string | no | `name` | Stable identifier (`1` to `128` characters). Moves onto the deployment member when compiled. |
| `name` | string | yes | — | Channel name (`1` to `255` characters). |
| `description` | string | no | — | Optional summary (`<= 2048` characters). |
| `tags` | array of strings | no | `[]` | Free-form labels. |
| `channel_type` | string | yes | — | `"sync"` (caller waits for the workflow result) or `"async"` (queued; returns `202` with `trace_id`). Must be `"async"` for `cron` and `kafka` channels, and `"sync"` for `graphql` and `sse` channels. |
| `protocol` | string | yes | — | `"rest"`, `"http"`, or `"cron"` on Community; <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> adds [`"kafka"`](../enterprise/kafka.md), [`"graphql"`](../enterprise/graphql.md), and [`"sse"`](../enterprise/sse.md). |
| `methods` | array of strings | conditional | — | Required for `rest`, `http`, `graphql`, and `sse`. Valid HTTP methods: `"GET"`, `"POST"`, `"PUT"`, `"PATCH"`, `"DELETE"`, `"HEAD"`, `"OPTIONS"`. Refused on `cron` and `kafka`. |
| `route_pattern` | string | conditional | — | Required for `rest`, `http`, `graphql`, and `sse` (`<= 255` chars, starting with `/`, no empty `//` or trailing `/`, path parameters written as `{name}`). Captured parameters appear at `metadata.params`. Refused on `cron` and `kafka`. |
| `priority` | number | no | `0` | Route-matching precedence (higher priority wins, then segment count descending, then channel name). |
| `workflow_id` | string | yes | — | Primary workflow executed by this channel. Must be included in the deployment. |
| `connections` | array of strings | no | `[]` | Connections required by the channel's transport driver (such as a `kafka` channel's broker connection). |
| `transport_config` | object | conditional | — | Protocol-specific transport configuration for `cron`, [`kafka`](../enterprise/kafka.md), [`graphql`](../enterprise/graphql.md), and [`sse`](../enterprise/sse.md). |
| `config` | object | no | `{}` | Ingress guards, request/response shaping, caching, timeout, and tracing settings detailed on this page. |

---

## Guards by ingress and execution order

Guards execute in a fixed sequence before the workflow runs:
`rate_limit` → `auth` → `principal_rate_limit` → `origin_allow_list` → `validation_logic` → `deduplication` → `cache` → `backpressure` → `oauth2_login`.

| Guard | HTTP sync | HTTP `/async` | `channel_call` | `cron` | `kafka` <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | `graphql` <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | `sse` <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> |
|---|---|---|---|---|---|---|---|
| `rate_limit` | Yes | Yes | Yes | No | Yes (by topic) | Yes | Yes |
| `principal_rate_limit` | Yes | Yes | No | No | No | Yes | Yes |
| `auth` | Yes | Yes | No | No | No | Yes | Yes |
| `origin_allow_list` | Yes | Yes | No | No | No | Yes | Yes |
| `validation_logic` | Yes | Yes | Yes | Yes | Yes (per record) | Yes (per field) | Yes (per tick) |
| `deduplication` | Yes | Yes | No | No | Yes (by record key) | Yes | No |
| `cache` | Yes | No | No | No | No | No | No |
| `backpressure` | Yes | Yes | Yes | Yes | Yes | Yes | Yes (per stream) |
| `oauth2_login` | Yes | No | No | No | No | No | No |
| `timeout_ms` | Yes | Yes (clamped) | Yes | Yes | Yes | Yes | Yes (per tick) |

Configuring a guard marked **No** on a `cron`, `kafka`, `graphql`, or `sse` channel is rejected at upload and compile time rather than silently ignored.

---

## Cron `transport_config` (`"protocol": "cron"`)

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `schedule` | string | yes | — | Six-field cron expression (`second minute hour day-of-month month day-of-week`). Five-field and seven-field expressions are refused. |
| `timezone` | string | no | `"UTC"` | IANA time zone name (such as `"America/New_York"` or `"Asia/Kolkata"`). Three-letter abbreviations (`EST`, `IST`) are refused. |
| `payload` | object | no | `{}` | Static JSON input object (`<= 1 MiB`). Secret locators (`env://`, `vault://`, `secret://`, `var://`) are refused here because `payload` is stored in trace rows. |
| `misfire_policy` | string | no | `"latest"` | `"skip"`, `"latest"`, or `"catch_up"` when scheduled ticks pass while the scheduler is down. |
| `max_catch_up` | number | conditional | — | Required when `misfire_policy` is `"catch_up"` (`1` to `1000`). |
| `concurrency.policy` | string | no | `"allow"` | `"allow"` (overlapping runs permitted) or `"forbid"` (at most `slots` concurrent runs per `key`). |
| `concurrency.key` | string | no | `channel_id` | Lock identifier (`[A-Za-z0-9][A-Za-z0-9_.-]{0,127}`). Channels sharing a `key` share its concurrency slots. |
| `concurrency.slots` | integer | no | `1` | Maximum concurrent runs (`1` to `64`, valid only when `concurrency.policy` is `"forbid"`). |

Each cron run stamps `metadata.trigger` with `{type, occurrence_id, scheduled_for, started_at, timezone, attempt, singleton_key?, singleton_slot?}`.

---

## Authentication (`config.auth`)

Authenticates HTTP callers on `/api/v2/data/{channel}` and `/async`. All authentication failures return a uniform `401` response.

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `mode` | string | yes | — | `"api_key"`, `"hmac"`, or `"jwt"`. |
| `keys` | array of strings | conditional | — | Required for `api_key`: accepted keys or `env://VAR` locators (compared in constant time via SHA-256). |
| `header` | string | no | `"Authorization"` (`api_key`) / `"X-Signature"` (`hmac`) | Request header carrying the credential or signature. |
| `scheme` | string | no | `"Bearer"` (when `header` is `"Authorization"`) | `api_key` only: RFC 9110 authentication scheme prefix (`""` reads the raw header value). |
| `secret` / `secrets` | string / array | conditional | — | Required for `hmac`: shared secret(s) via `env://VAR` locator. |
| `preset` | string | no | — | `hmac` only: `"zoom"`, `"slack"`, `"stripe"`, `"github"`, `"shopify"`, or `"webex"`. |
| `algorithm` | string | no | `"sha256"` | `hmac` only: `"sha1"`, `"sha256"`, or `"sha512"`. |
| `message` | string | no | `"{body}"` | `hmac` only: signing string template using `{body}`, `{header:<name>}`, and `{header:<name>:<key>}`. |
| `signature_prefix` | string | no | — | `hmac` only: prefix stripped before decoding (such as `"sha256="`). Mutually exclusive with `signature_key`. |
| `signature_key` | string | no | — | `hmac` only: key extracted from a comma-separated `k=v` header (such as Stripe's `"v1"`). |
| `encoding` | string | no | auto (`hex` then `base64`) | `hmac` only: `"hex"`, `"base64"`, or `"base64url"`. |
| `timestamp` / `tolerance_secs` | string / integer | no | — | `hmac` only: header (`<header>` or `<header>:<key>`) holding a Unix-seconds timestamp and replay window in seconds (must be set together). |
| `algorithms` | array of strings | conditional | — | Required for `jwt`: non-empty allowlist (`HS256`/`384`/`512`, `RS256`/`384`/`512`, `PS256`/`384`/`512`, `ES256`/`384`, `EdDSA`). |
| `jwt_keys` | array of objects | conditional | — | `jwt` only: static verification keys `[{algorithm, key, kid?, key_encoding?}]`. At least one of `jwt_keys` or `jwks_url` is required. |
| `jwks_url` | string | conditional | — | `jwt` only: HTTPS JWKS URL cached process-wide with background refresh. |
| `issuer` / `audience` | string or array | no | — | `jwt` only: expected `iss` and `aud` claims. |
| `leeway_secs` | integer | no | `30` | `jwt` only: clock-skew leeway in seconds (`0` to `300`). |
| `require_exp` | boolean | no | `true` | `jwt` only: require the `exp` claim on every token. |
| `required` | boolean | no | `true` | `jwt` only: when `false`, requests without a token proceed with no `metadata.auth` (invalid tokens are still rejected). |
| `source` | object | no | `{"header": "Authorization", "scheme": "Bearer"}` | `jwt` only: `{"header": "...", "scheme": "..."}` or `{"cookie": "..."}`. |
| `max_token_bytes` | integer | no | `8192` | `jwt` only: maximum token byte length. |
| `claims_to_metadata` | array of strings | no | all claims | `jwt` only: subset of verified claims copied to `metadata.auth.claims`. |
| `authorization_logic` | JSONLogic | no | — | `jwt` only: evaluated over `{"claims": ...}`; falsy returns `403 insufficient_scope`. |

---

## Browser OAuth2 / OIDC sign-in (`config.oauth2_login`)

Configures a `rest` channel as the relying party in an OAuth2/OIDC Authorization Code + PKCE flow (`route_pattern` initiates the redirect; `callback_path` handles the provider callback, verifies state/nonce/`id_token`, and populates `metadata.oauth` and `metadata.identity`).

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `issuer` | string | no | — | OIDC issuer URL (`https://...`). Discovers `authorize_url`, `token_url`, `jwks_uri`, and `userinfo_url` from `/.well-known/openid-configuration`. |
| `authorize_url` | string | conditional | — | Authorization endpoint (`https://...`). Required unless `issuer` or `providers` is set. |
| `token_url` | string | conditional | — | Token endpoint (`https://...`). Required unless `issuer` or `providers` is set. |
| `client_id` | string | conditional | — | OAuth2 client ID (`var://`, `env://`, or literal). Required unless `providers` is set. |
| `client_secret` | string | conditional | — | Client secret locator (`env://` or `vault://`). Required unless `providers` is set. |
| `client_auth` | string | no | `"basic"` | Token endpoint authentication method: `"basic"` or `"body"`. |
| `redirect_uri` | string | yes | — | Registered callback URL (`https://...`, or `var://` / `env://`). Contains `{provider}` in multi-provider mode. |
| `callback_path` | string | yes | — | Callback route on this channel (must differ from `route_pattern`). |
| `state_secret` | string | yes | — | HS256 secret locator (`env://` or `vault://`, `>= 32` bytes) used to sign the CSRF state cookie. |
| `pkce` | boolean | no | `true` | Enable RFC 7636 S256 PKCE challenge. |
| `scopes` | array of strings | no | `[]` | OAuth2 scopes requested on the authorize redirect. |
| `extra_authorize_params` | object | no | `{}` | Additional query parameters appended to `authorize_url` (`prompt`, `hd`, etc.). |
| `state_cookie` | object | no | `{name: "orion_oauth_state", secure: true, same_site: "lax", path: "/", max_age: 600}` | State cookie attributes (`max_age` between `1` and `86400` seconds; `same_site: "strict"` is refused). |
| `id_token` | object | no | discovered | Explicit OIDC verification block: `{issuer, jwks_url, audience?, algorithms?, required?, nonce?}`. |
| `userinfo_url` | string | no | discovered | Optional HTTPS userinfo endpoint fetched when identity is not carried in `id_token`. |
| `identity` | object | no | OIDC defaults | Field mapping `{subject, login, name, email, picture}` populating `metadata.identity`. |
| `providers` | object | no | — | Map of provider slugs to per-provider settings when serving `/auth/{provider}` and `/auth/{provider}/callback`. |
| `providers_from_instance` | boolean | no | `false` | Merge instance-level `[oauth2_login.providers]` from `orion.toml`. |
| `run_workflow_on_authorize` | boolean | no | `false` | Execute the workflow on the authorize leg before redirecting. |
| `return_to` | object | no | — | `{param, allow_list}` validating and sealing a post-login redirect URL into the signed state. |

---

## Rate limiting, backpressure, validation, origins, and deduplication

### `config.rate_limit` and `config.principal_rate_limit`

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `requests_per_second` | integer | yes | — | Steady-state token refill rate per bucket. |
| `burst` | integer | no | `requests_per_second / 2 + 1` | Burst capacity above the steady rate. |
| `key_logic` | JSONLogic | conditional | caller IP / channel | Bucket key expression evaluated over `{client_ip, channel, headers}` (plus `auth` claims on `principal_rate_limit`). Required on `principal_rate_limit`. |
| `key_headers` | array of strings | no | — | Additional request headers exposed in `headers.*` for `key_logic` (merged with `authorization`, `x-api-key`, `x-forwarded-for`, `x-real-ip`, `user-agent`, `content-type`, `origin`, `x-tenant-id`). |
| `on_backend_error` | string | no | `"allow"` | Cluster Redis store failure policy: `"allow"` (fail open) or `"deny"` (`503`). |

### `config.backpressure`

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `max_concurrent_per_node` | integer | yes | — | Maximum concurrent in-flight executions for this channel on each replica; excess requests fail fast with `503`. |

### `config.deduplication`

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `header` | string | yes | — | Request header carrying the caller's idempotency key (repeats within `window_secs` return `409 Conflict`). |
| `window_secs` | integer | no | `300` | Deduplication window in seconds. |
| `connector` | string | no | in-memory (or cluster Redis store) | Optional `cache` connector name on single-node deployments (refused in cluster mode). |
| `on_backend_error` | string | no | `"allow"` | `"allow"` or `"deny"` (`503`) when the backing store cannot be reached. |

### `config.validation_logic` and `config.origin_allow_list`

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `validation_logic` | JSONLogic | no | — | Predicate evaluated over `{data, metadata}` before workflow execution; falsy values reject the request with `400`. |
| `origin_allow_list` | array of strings | no | no check | Allowed `Origin` header values enforced server-side on HTTP requests; unlisted `Origin` headers return `403`. |

---

## Request, response shaping, caching, timeout, and tracing

### `config.request`

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `body_mode` | string | no | `"auto"` | `"auto"` unwraps top-level `{"data": ..., "metadata": ...}` envelopes; `"payload"` treats the entire HTTP body as `data` verbatim and ignores caller-supplied `metadata`. |
| `cookies_to_metadata` | array of strings | no | — | Request cookie names copied into `metadata.cookies.<name>` while keeping the raw `Cookie` header masked. |

### `config.response`

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `mode` | string | no | `"envelope"` | `"envelope"` returns `{id, status, data, errors}`; `"shaped"` reads `data._orion.response` (`{status, headers, cookies, body_path, raw}`) to set custom HTTP status codes, headers, cookies, and raw bodies. |
| `allowed_headers` | array of strings | no | `["content-type", "location", "cache-control", "etag", "last-modified", "retry-after", "content-language", "link"]` | Response headers the workflow is allowed to set in `data._orion.response.headers`. |
| `cookies` | boolean | no | `false` | Allow the workflow to set `Set-Cookie` headers via `data._orion.response.cookies` (`[{name, value, path?, domain?, max_age?, expires?, same_site?, http_only?, secure?}]`). |
| `error_bodies` | object | no | — | Per-status (`"400"`–`"599"` or `"default"`) custom error templates `{body, content_type?}` for guard rejections using `{status}`, `{code}`, `{message}`, `{request_id}`, `{channel}`, and `{timestamp}`. |

### `config.cache`

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `enabled` | boolean | yes | — | Enable or disable synchronous HTTP response caching. |
| `ttl_secs` | integer | no | `300` | Cache entry lifetime in seconds. |
| `cache_key_fields` | array of strings | no | whole payload | Subset of payload paths included in the cache key. |
| `key_logic` | JSONLogic | no | — | Custom cache key expression over `{data, metadata}` (overrides `cache_key_fields`). |
| `connector` | string | no | in-memory (or cluster Redis store) | Backing `cache` connector on single-node deployments. |
| `namespaces` | array of strings | no | — | `1` to `8` invalidation namespaces (`[a-z0-9_.:-]{1,64}`) invalidated via [`cache_invalidate`](./functions.md) or `POST /api/v2/admin/cache/namespaces/{namespace}/invalidate`. |
| `coalesce_misses` | boolean | no | `false` | Collapse concurrent cache misses for the same key on a node into a single workflow execution. |

### `config.timeout_ms` and `config.tracing`

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `timeout_ms` | integer | no | per ingress | Maximum workflow execution time in milliseconds (clamped to `trace_queue.processing_timeout_ms` on `/async`). |
| `tracing.mode` | string | no | `trace_storage.mode` | Per-channel trace persistence override: `"sync"`, `"async"`, `"batch"`, or `"off"`. |
| `tracing.sample_rate` | number | no | `trace_storage.sample_rate` | Synchronous trace persistence sampling rate (`0.0` to `1.0`). |
| `tracing.errors_only` | boolean | no | `trace_storage.errors_only` | Persist traces only when the workflow run ends with an error. |
| `tracing.task_details` | boolean | no | `false` | Record per-task input/output snapshots in `task_trace_json`. |

## Next steps

- [Configure channels and guards](../guides/channels.md): Practical examples for authentication, rate limiting, and response shaping.
- [Kafka](../enterprise/kafka.md), [GraphQL](../enterprise/graphql.md), and [SSE](../enterprise/sse.md): Reference for Standard+ channel protocols.
