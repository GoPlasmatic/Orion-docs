<!-- description: Learn how Orion channels expose workflows over REST, HTTP, Cron, Kafka, GraphQL, and SSE, and enforce authentication, rate limits, and validation. -->
<!-- type: concept -->
<!-- last_verified: 2026-11-16 -->

# Channels and ingress

A **channel** is a service endpoint in Orion. It defines how work arrives (the protocol and route or schedule), which workflow executes when work arrives, and which ingress guards the caller must pass before the workflow runs.

```orion-diagram
{
  "direction": "LR",
  "nodes": [
    { "id": "rest", "label": "POST /orders", "sublabel": "rest · sync", "type": "channel" },
    { "id": "async", "label": "POST /reports/async", "sublabel": "rest · async", "type": "channel" },
    { "id": "cron", "label": "0 15 2 * * *", "sublabel": "cron · async", "type": "channel" },
    { "id": "wf1", "label": "order-processing", "type": "service" },
    { "id": "wf2", "label": "report-build", "type": "service" },
    { "id": "wf3", "label": "nightly-rollup", "type": "service" }
  ],
  "edges": [
    { "from": "rest", "to": "wf1" },
    { "from": "async", "to": "wf2" },
    { "from": "cron", "to": "wf3" }
  ]
}
```

---

## Channel protocols

A channel's `protocol` field selects the channel driver that serves it.

### Core protocols (all editions)
- **`rest`**: Binds an HTTP method list and a path pattern under `/api/v2/data`, such as `POST /orders` or `GET /orders/{id}`. Path parameters (`{id}`) are extracted into `metadata`. Every REST channel is also addressable by name at `/api/v2/data/{channel_name}`.
- **`http`**: Routes identically to `rest` and can be called by route pattern or channel name.
- **`cron`**: Runs a workflow on a six-field cron schedule (`transport_config.schedule`) with a fixed JSON payload. Cron channels have no external caller and do not register an HTTP route; instead, Orion's scheduler writes a durable **occurrence** row for each scheduled instant and executes it.

### Standard & Enterprise protocols <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>
Available in Orion Standard and Enterprise (`orion-ext`):
- **[`kafka`](../enterprise/kafka.md)**: Consumes messages from one or more Kafka topics using a consumer group (`transport_config`), with at-least-once offset settlement and dead-letter topic routing.
- **[`graphql`](../enterprise/graphql.md)**: Exposes a GraphQL endpoint (`GET` and `POST`) where each root field of the query or mutation runs a workflow execution, protected by depth, complexity, and alias limits.
- **[`sse`](../enterprise/sse.md)**: Serves a `GET` endpoint with `text/event-stream`, executing the workflow on each timer tick (`interval_ms`) and streaming events with keepalives and `Last-Event-ID` resumption.

If you upload a channel with an unknown or unenabled `protocol` (such as `kafka` on a Community node), Orion rejects the upload with `400 VALIDATION_ERROR`.

---

## Synchronous vs. asynchronous execution

The `channel_type` field controls whether the caller waits for the workflow to finish:

| Mode | Response | How the Result Is Read | Timeout Bound |
| :--- | :--- | :--- | :--- |
| **`sync`** | Returns `200 OK` (or a custom shaped status) with the final `data` context | Directly from the HTTP response body | Channel `timeout_ms` |
| **`async`** | Returns `202 Accepted` immediately with `trace_id` and `trace_token` | Poll `GET /api/v2/admin/traces/{id}` | Clamped to `trace_queue.processing_timeout_ms` |

- Any synchronous `rest` or `http` channel also automatically exposes an asynchronous submission path by appending `/async` to its route (`/api/v2/data/orders/async`) or passing `--async-mode` in `orion-cli send`.
- `cron` and `kafka` channels are always `"channel_type": "async"` because there is no interactive HTTP caller waiting on a response.

---

## Ingress guards

A channel declares its security and traffic controls inside its `config` object. Orion enforces these guards before the workflow starts:

| Guard | Purpose | Rejection Status |
| :--- | :--- | :--- |
| **`rate_limit`** | Token-bucket rate limiter (per client IP by default, or custom `key_logic`). | `429 Too Many Requests` |
| **`auth`** | Verifies API keys (`api_key`), webhook HMAC signatures (`hmac`), or bearer tokens (`jwt`). | `401 Unauthorized` |
| **`oauth2_login`** | Browser OAuth2/OIDC authorization-code flow with PKCE and session cookies. | `302` redirect / `401` |
| **`principal_rate_limit`** | Secondary token-bucket quota keyed on verified JWT claims after authentication. | `429 Too Many Requests` |
| **`origin_allow_list`** | Server-side check of the `Origin` header on HTTP requests. | `403 Forbidden` |
| **`validation_logic`** | JSONLogic predicate over `data` and `metadata` to reject bad payloads early. | `400 Bad Request` |
| **`deduplication`** | Rejects replayed idempotency keys within `window_secs`. | `409 Conflict` |
| **`cache`** | Caches synchronous responses keyed by route, query, and payload fields (`ttl_secs`). | Serves cached `200 OK` |
| **`backpressure`** | Caps concurrent in-flight executions per node (`max_concurrent_per_node`). | `503 Service Unavailable` |

Guards apply consistently across all ingresses that support them (sync HTTP, `/async` HTTP, `channel_call`, `cron`, and extension transports). If a channel's guard configuration fails to load at runtime (for example, an `auth` block referencing an unset `env://` secret), Orion **quarantines** the channel—refusing all traffic to it rather than serving it unauthenticated.

---

## Composing services with `channel_call`

A workflow can invoke another channel in the same deployment using the [`channel_call`](../reference/functions.md) task function. The call executes in-process with no network hop or HTTP serialization overhead, while still enforcing the target channel's applicable guards (`rate_limit`, `validation_logic`, `backpressure`) and timeout. Orion detects and rejects circular call chains at compile and deploy time.

## Next steps

- [Configure channels and guards](../guides/channels.md) — Step-by-step walkthrough for setting up routes, authentication, rate limits, deduplication, and response shaping.
- [Schedule jobs with cron](../guides/scheduled-jobs.md) — Configure scheduled channels, misfire policies, and singleton concurrency.
- [Channel configuration schema](../reference/channels.md) — Complete field tables and the guard-by-ingress matrix.
