<!-- description: Protect Orion workflows against downstream failures using layered timeouts, idempotent HTTP retries, per-connector circuit breakers, and drain windows. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Timeouts, retries, and circuit breakers

Every workflow that calls an external API, database, or message broker inherits that backend's latency spikes and outages. Orion bounds downstream failures through four mechanisms: **layered timeouts**, **idempotent retries**, **per-connector circuit breakers**, and **graceful shutdown drain windows**.

---

## 1. Layered timeouts

Set timeouts from the inside out so slow downstream calls fail before the outer channel deadline expires:

| Layer | Configuration key | What it bounds |
|---|---|---|
| **Channel deadline** | `config.timeout_ms` on `channel.json` | Total workflow execution time for one request, `/async` run, `channel_call`, or `cron` tick. |
| **Database connector** | `connect_timeout_ms`, `query_timeout_ms` on `db` `connector.json` | Single connection acquisition and single SQL query execution. |
| **HTTP connector** | `timeout_ms` on `http` `connector.json` (backed by `engine.global_http_timeout_secs`) | Single outbound `http_call` request (and each retry attempt). |
| **Health probe** | `engine.health_check_timeout_secs` in `orion.toml` | Maximum time `/health` and `/readyz` wait on a component ping. |

> [!TIP]
> Always set connector `timeout_ms` / `query_timeout_ms` **lower** than the channel's `config.timeout_ms`. Allowing a database query 30 seconds inside a channel that times out at 5 seconds ties up a database connection for 25 seconds after the caller has already received a `504` timeout.

---

## 2. Retries and idempotency rules

Orion never guesses that a timed-out write did not land:

### Built-in Community connectors (`http`, `db`, `cache`)

- **Only `http` connectors retry automatically**: When `retry` is configured on an `http` connector, Orion retries network errors, timeouts, `429 Too Many Requests`, and `5xx` responses using exponential backoff (capped at 60 seconds) **only on idempotent HTTP methods** (`GET`, `HEAD`, `PUT`, `DELETE`, `OPTIONS`, `TRACE`). `POST` and `PATCH` are never retried unless explicitly opted in.
- **`db` and `cache` connectors never retry automatically**: A timed-out `INSERT` or `UPDATE` may already have committed on the database server.

### Standard+ connector providers <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

Every Standard+ provider operation reports a normalized failure class in `error.connector` alongside `error.connector.provider_code`:

| Failure class | Meaning | Safe to retry? |
|---|---|---|
| `unavailable` | Backend is unreachable, dropped the connection, or returned `5xx`. | Only for idempotent read operations (never for `com.orion.kafka` `publish`). |
| `rate_limited` | Backend returned `429` (such as `STORAGE_THROTTLED` or `ELASTICSEARCH_THROTTLED`). | Yes, after backoff. |
| `deadline` | Workflow deadline expired during the call. | No (except pure CPU `MODEL_TIMEOUT` on `com.orion.models`). |
| `provider_failure` | Backend rejected the operation (`SMTP_REJECTED`, `MONGO_FAILED`). | No. |
| `invalid_input` | Input or connection failed schema validation. | No. |
| `forbidden` / `unauthorized` | Operation gate closed, SSRF blocked, or credentials rejected. | No. |

### Higher-level retries for non-idempotent work

- **Async channels (`/async`)**: Failed runs move to `trace_dlq` and are retried by the background DLQ worker (`trace_queue.dlq_max_retries`).
- **Scheduled jobs (`cron`)**: Failed occurrences are recorded in the cron ledger and can be retried via `orion-cli cron retry <occurrence-id>` (`POST /api/v2/admin/cron/occurrences/{id}/retry`).

---

## 3. Circuit breakers

Circuit breakers are disabled by default. Enable them in `orion.toml` whenever workflows call external HTTP APIs or databases:

```toml
[engine.circuit_breaker]
enabled = true
```

### How circuit breakers isolate failures

- **Scoped per `channel:connector`**: Each `(channel, connector)` pair maintains its own breaker state (`closed`, `open`, `half_open`). A failing downstream path on one channel does not trip the breaker for other channels sharing the same connector.
- **Fast failure (`503 CIRCUIT_OPEN`)**: While a breaker is `open`, calls to that connector fail immediately with `503 CIRCUIT_OPEN` instead of queueing threads against an unresponsive backend. After the cool-down window, the breaker allows probe calls through in `half_open` and closes automatically once calls succeed.
- **Cluster behavior**: In cluster mode, breakers trip independently on each replica based on its own observed errors, while an admin reset propagates across every replica on the next reconcile tick.

Inspect and manually reset circuit breakers via the CLI or Admin API:

<!-- tabs:start -->
### **CLI**
```bash
orion-cli circuit-breakers list
orion-cli circuit-breakers reset <key>
```

### **curl**
```bash
curl -s -H "Authorization: Bearer $ORION_ADMIN_KEY" \
  http://localhost:8080/api/v2/admin/connectors/circuit-breakers

curl -s -X POST -H "Authorization: Bearer $ORION_ADMIN_KEY" \
  http://localhost:8080/api/v2/admin/connectors/circuit-breakers/<key>
```
<!-- tabs:end -->

---

## 4. Task-level failure and rejection handling in workflows

Within a workflow definition, two separate task properties control what happens when a step does not succeed:

| Task property | What it governs | Default | Options |
|---|---|---|---|
| `"on_failure"` | Handler/connector errors and `5xx` statuses (`IO_ERROR`, `TIMEOUT_ERROR`, `CIRCUIT_OPEN`). | `"halt"` (stops the workflow and returns the error) | `"halt"` or `"continue"` (records the error in `errors` and `metadata._orion_errors` and continues to the next task). |
| `"on_rejection"` | Client/validation `4xx` rejections (such as a failed [`validation`](../reference/functions.md) rule). | `"continue"` (records the `400` error and continues) | `"halt"` (stops the workflow immediately on validation failure) or `"continue"`. |

> [!IMPORTANT]
> Always set `"on_rejection": "halt"` on `validation` tasks that act as security or schema gates before database writes or external calls. `orion-cli lint` and `orion-cli clippy` warn when a `validation` task omits `"on_rejection": "halt"` (`engine.unguarded_validation`).

When `"on_failure": "continue"` is set on a task, downstream fallback tasks can inspect `metadata._orion_errors` to branch on the error code:

```json
{
  "id": "use_cached_fallback",
  "name": "Read fallback value when upstream times out",
  "condition": {
    "in": [
      { "var": "metadata._orion_errors.0.code" },
      ["TIMEOUT_ERROR", "IO_ERROR", "CIRCUIT_OPEN"]
    ]
  },
  "function": {
    "name": "cache_read",
    "input": {
      "connector": "local_cache",
      "key": { "var": "data.product_id" },
      "output": "product"
    }
  }
}
```

---

## 5. Shut down without dropping requests

When Orion receives `SIGTERM` or `SIGINT`, it executes a four-stage graceful drain designed for zero-downtime rolling deployments behind load balancers:

1. **Withdraw readiness immediately**: `GET /readyz` switches to `503` at once so the load balancer or Kubernetes Service removes the replica from rotation.
2. **Keep accepting during load-balancer propagation (`server.shutdown_drain_secs`)**: Orion continues accepting and serving new requests for `shutdown_drain_secs` (default `30s`) so requests routed by load balancers that have not yet polled `/readyz` still succeed.
3. **Drain in-flight requests (`server.shutdown_force_timeout_secs`)**: Listener accept stops, and Orion waits up to `shutdown_force_timeout_secs` (default `30s`) for active workflows to finish.
4. **Flush background queues and telemetry**: Background maintenance jobs stop, the audit queue drains (`audit.drain_timeout_secs`), OpenTelemetry spans flush, and the process exits `0`.

```toml
[server]
shutdown_drain_secs = 30
shutdown_force_timeout_secs = 30
```

Configure your container orchestrator's termination grace period (`stop_grace_period` in Docker Compose or `terminationGracePeriodSeconds` in Kubernetes) to at least `shutdown_drain_secs + shutdown_force_timeout_secs + 10s`.

## Next steps

- [Monitoring, metrics, and alerts](./monitoring.md): Monitor circuit breaker state and retry counters.
- [Backups, upgrades, and troubleshooting](./maintenance.md): Diagnose open circuit breakers and timeout errors.
