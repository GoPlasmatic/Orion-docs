<!-- description: Configure Orion channels and ingress guards: REST routes, async mode, API key/HMAC/JWT/OAuth2 auth, rate limiting, validation, dedup, caching, and shaping. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Configure channels and guards

A channel defines a service's endpoint and the ingress policies enforced before its workflow runs. This guide walks through configuring REST routes, synchronous vs. asynchronous modes, caller authentication, rate limiting, payload validation, deduplication, response caching, backpressure, timeouts, and custom HTTP response shaping.

For the complete field-by-field specification and guard-by-ingress matrix, see [Channel configuration schema](../reference/channels.md).

---

## 1. Configure REST routes and path parameters

A `rest` channel binds HTTP methods and a `route_pattern` under `/api/v2/data` to a workflow:

```json
{
  "channel_id": "order-lookup",
  "name": "order-lookup",
  "channel_type": "sync",
  "protocol": "rest",
  "route_pattern": "/orders/{id}",
  "methods": ["GET"],
  "workflow_id": "get-order"
}
```

- **Path parameters**: Segments written as `{name}` match any single URL segment and are exposed to your workflow at `metadata.params.id`.
- **Exact matching**: Routes match case-sensitively (`GET /Orders/123` does not match `/orders/{id}`).
- **Name fallback**: Every `rest` and `http` channel can also be invoked by its `name` at `/api/v2/data/{name}`, which is what `orion-cli send` uses.

---

## 2. Choose synchronous or asynchronous execution

- **Synchronous (`"channel_type": "sync"`)**: The caller waits while the workflow runs and receives the final `data` context in the HTTP response body.
- **Asynchronous (`"channel_type": "async"`)**: Orion enqueues the request onto the trace queue and immediately returns `202 Accepted` with a `trace_id` and `trace_token`. The caller polls `GET /api/v2/admin/traces/{trace_id}` (using the `trace_token` or an admin key) to retrieve the completed result.

You can also invoke any synchronous REST channel asynchronously by appending `/async` to its URL (`POST /api/v2/data/orders/async`) or passing `--async-mode` to `orion-cli send`.

---

## 3. Authenticate callers (`auth` and `oauth2_login`)

By default, data-plane channels are unauthenticated so local development works out of the box. Before exposing a channel to external traffic, configure an `auth` block under `config`.

### API key authentication (`api_key`)
List one or more secret references in `keys` (supporting zero-downtime key rotation):

```json
{
  "config": {
    "auth": {
      "mode": "api_key",
      "keys": ["env://ORDERS_API_KEY", "env://ORDERS_API_KEY_PREVIOUS"],
      "header": "X-API-Key"
    }
  }
}
```

### Webhook HMAC verification (`hmac`)
Verify webhook signatures (such as GitHub, Stripe, or Shopify) over the raw request body before JSON parsing:

```json
{
  "config": {
    "auth": {
      "mode": "hmac",
      "secret": "env://GITHUB_WEBHOOK_SECRET",
      "header": "X-Hub-Signature-256",
      "signature_prefix": "sha256="
    }
  }
}
```

### JWT verification (`jwt`)
Verify Bearer tokens against static keys or a remote JWKS URL. Verified claims are placed at `metadata.auth.claims` for use in downstream guards and workflow tasks:

```json
{
  "config": {
    "auth": {
      "mode": "jwt",
      "jwks_url": "https://auth.example.com/.well-known/jwks.json",
      "algorithms": ["RS256"],
      "issuer": "https://auth.example.com/",
      "audience": ["orders-api"]
    }
  }
}
```

Any authentication failure returns a uniform `401 Unauthorized`. If an `env://` secret referenced by `auth` is unset on a node, Orion quarantines the channel rather than serving it unauthenticated.

### Browser OAuth2 / OIDC sign-in (`oauth2_login`)
For browser-facing channels, you can configure `config.oauth2_login` to act as an OAuth2/OIDC relying party (handling the authorization-code redirect, PKCE, state cookie, and `id_token` verification). See [Channel configuration schema](../reference/channels.md) for all `oauth2_login` fields.

---

## 4. Rate-limit traffic (`rate_limit` and `principal_rate_limit`)

Configure a token-bucket rate limiter on the channel:

```json
{
  "config": {
    "rate_limit": {
      "requests_per_second": 100,
      "burst": 50
    }
  }
}
```

> [!WARNING]
> By default, `rate_limit` is applied **per caller IP**, not globally across the channel. To enforce a per-tenant quota or a single channel-wide limit, set `key_logic` (for example, `{"var": "data.tenant_id"}` or a constant `"global"`). When running behind a reverse proxy, also configure `rate_limit.trusted_proxies` in `orion.toml`.

To enforce a second quota based on verified JWT claims *after* authentication runs, add `principal_rate_limit`:

```json
{
  "config": {
    "principal_rate_limit": {
      "requests_per_second": 20,
      "burst": 10,
      "key_logic": { "var": "metadata.auth.claims.sub" }
    }
  }
}
```

---

## 5. Validate payloads before running the workflow (`validation_logic`)

Use `validation_logic` to reject malformed requests with `400 Bad Request` at the ingress before any workflow task or database query executes:

```json
{
  "config": {
    "validation_logic": {
      "and": [
        { "!!": { "var": "data.order_id" } },
        { ">": [{ "var": "data.total" }, 0] }
      ]
    }
  }
}
```

Use `validation_logic` for fast structural checks, and use the workflow `validation` task when you want rule failures recorded in execution traces or returned as detailed field error lists.

---

## 6. Deduplicate replayed requests (`deduplication`)

Prevent duplicate execution of retried requests by naming an idempotency key header and a time window in seconds:

```json
{
  "config": {
    "deduplication": {
      "header": "Idempotency-Key",
      "window_secs": 300
    }
  }
}
```

If a second request arrives with the same `Idempotency-Key` within 300 seconds, Orion rejects it with `409 Conflict` before the workflow runs. On Orion Standard and Enterprise in cluster mode, deduplication claims are shared across all nodes via the Redis cluster store.

---

## 7. Cache synchronous responses (`cache`)

Cache responses for repeated identical requests so hits return immediately without running the workflow:

```json
{
  "config": {
    "cache": {
      "enabled": true,
      "ttl_secs": 60,
      "cache_key_fields": ["data.customer_id"],
      "namespace": "customer-profile"
    }
  }
}
```

- **Cache keys ignore HTTP headers**: If your response depends on a tenant ID or user identity from a header, include that value in the payload and list it in `cache_key_fields`, or use `key_logic`.
- **Invalidation**: Workflows can invalidate a cache namespace on write using the [`cache_invalidate`](../reference/functions.md) task function, and operators can invalidate namespaces via `orion-cli cache invalidate <namespace>`.

---

## 8. Shed excess load (`backpressure`) and bound execution time (`timeout_ms`)

Protect downstream systems by capping concurrent in-flight executions per node and setting a strict execution deadline:

```json
{
  "config": {
    "backpressure": { "max_concurrent_per_node": 50 },
    "timeout_ms": 5000
  }
}
```

When 50 requests are already executing on a node, request 51 is rejected immediately with `503 Service Unavailable` rather than queuing indefinitely.

---

## 9. Customize HTTP responses (`response`)

By default, a synchronous channel returns HTTP `200` with `{"id": "...", "status": "ok", "data": {...}, "errors": []}`. To let your workflow control the HTTP status code, custom headers, cookies, or raw response body (for example, returning `201 Created`, `204 No Content`, or a custom webhook payload), enable response shaping on the channel:

```json
{
  "config": {
    "response": {
      "shape": "custom",
      "allowed_headers": ["X-Order-Id", "Location"]
    }
  }
}
```

Your workflow can then write `data._orion.response` using `map`:

```json
{
  "path": "data._orion.response",
  "logic": {
    "status": 201,
    "headers": { "X-Order-Id": { "var": "data.order.id" } },
    "body_path": "data.order"
  }
}
```

## Verify your channel configuration

Compile your definition set and run `deployments plan` to verify that all guard keys and route patterns are valid:

```bash
orion-cli compile ./definitions -o dist/deployment
orion-cli deployments plan -d dist/deployment
```

Orion rejects unknown keys anywhere inside `config` with `400 VALIDATION_ERROR` so a misspelled guard (such as `rate_limits`) never silently leaves an endpoint unprotected.

## Next steps

- [Channel configuration schema](../reference/channels.md) — Complete field reference for routing, transports, and all guard blocks.
- [Kafka](../enterprise/kafka.md), [GraphQL](../enterprise/graphql.md), and [SSE](../enterprise/sse.md) — Configure Standard and Enterprise channel protocols.
- [Security and access control](../operate/security.md) — Configure TLS, CORS, and `trusted_proxies` at the server level.
