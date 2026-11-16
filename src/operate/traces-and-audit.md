<!-- description: Manage execution trace persistence, async channel queues, dead-letter queue (DLQ) retries, and admin audit logs with change-context correlation. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Execution traces, DLQ, and audit logs

Orion maintains two distinct operational trails in the state database:
1. **Execution traces (`traces` and `trace_dlq`)** record data-plane requests (`/api/v2/data/**`), capturing which workflow matched, every task step that executed or skipped, and the final output. Traces also serve as the result delivery mechanism for asynchronous (`/async`) channels.
2. **Audit logs (`audit_logs`)** record admin-plane mutations (`/api/v2/admin/**`), capturing who changed or tested a deployment, definition, connection, or operational resource, and when.

---

## Part 1: Execution traces and `/async` processing

### Choose a trace storage mode

`trace_storage.mode` in `orion.toml` sets the global persistence policy for synchronous requests (any channel can override this via `config.tracing`):

| Mode | Behavior | Best for |
|---|---|---|
| `sync` (default) | Waits for the database write attempt before returning the HTTP response. | Debugging and compliance trails where immediate trace visibility is required. |
| `async` | Enqueues completed traces to a bounded background worker queue (`trace_storage.max_pending`). | Decoupling synchronous request latency from database write spikes. |
| `batch` | Background workers commit up to `trace_storage.batch_size` trace rows per database transaction. | High-throughput synchronous APIs where individual row inserts cap throughput. |
| `off` | Disables database trace persistence for synchronous requests. | Ultra-high-throughput synchronous endpoints you observe purely via metrics and OTLP spans. |

> [!NOTE]
> `trace_storage.mode = "off"` and `trace_storage.sample_rate` apply **only to synchronous requests**. Asynchronous submissions (`/async`) and scheduled `cron` runs always persist their trace rows (with `off` upgraded to `sync` for `cron`) because the trace row is how callers and operators read the execution result. To bound storage for async and cron runs, use `trace_storage.errors_only = true` or `trace_queue.retention_hours`.

### Submit and poll `/async` channel requests

When a client posts to `/api/v2/data/<route>/async`, Orion writes an initial `pending` trace row and returns HTTP `202 Accepted` with `trace_id` and `trace_token`. The caller polls the trace using `x-trace-token` (or an admin API key):

<!-- tabs:start -->
#### **CLI**
```bash
# Submit to /async and wait for completion automatically
orion-cli send orders -f order.json --async-mode --wait

# Or fetch/wait on a trace ID later using its trace token
orion-cli traces get <trace-id> --token <token>
orion-cli traces wait <trace-id> --token <token> --timeout 120
```

#### **curl**
```bash
# 1. Submit to /async
curl -s -X POST http://localhost:8080/api/v2/data/orders/async \
  -H "Content-Type: application/json" \
  -d '{ "data": { "order_id": "ORD-123" } }'

# 2. Poll the trace with x-trace-token
curl -s -H "x-trace-token: <token>" \
  http://localhost:8080/api/v2/admin/traces/<trace-id>
```
<!-- tabs:end -->

### Configure the async worker queue and bounds

```toml
[trace_queue]
workers = 4                          # Concurrent async execution workers
buffer_size = 1000                   # Max queued submissions before 503 load shedding
processing_timeout_ms = 60000        # Per-trace execution deadline
max_result_size_bytes = 1048576      # Max result payload size (1 MiB)
max_queue_memory_bytes = 104857600   # Total queued payload memory cap (100 MiB)
retention_hours = 72                 # Delete traces older than 72 hours (0 = keep forever)
cleanup_interval_secs = 3600         # Background retention sweep interval
```

When either `buffer_size` or `max_queue_memory_bytes` is reached during a traffic burst, new `/async` submissions fail fast with `503 SERVICE_UNAVAILABLE` rather than growing process memory without bound.

### Query traces via the Admin API

Filter traces by channel, status (`pending`, `running`, `completed`, `failed`), and cursor pagination:

<!-- tabs:start -->
#### **CLI**
```bash
orion-cli traces list --channel orders --status failed --limit 20
orion-cli traces get <trace-id>
```

#### **curl**
```bash
curl -s -H "Authorization: Bearer $ORION_ADMIN_KEY" \
  "http://localhost:8080/api/v2/admin/traces?channel=orders&status=failed&limit=20"

curl -s -H "Authorization: Bearer $ORION_ADMIN_KEY" \
  "http://localhost:8080/api/v2/admin/traces/<trace-id>"
```
<!-- tabs:end -->

---

## Part 2: Drain and manage the trace dead-letter queue (DLQ)

When an `/async` workflow execution fails, Orion records the failure in `trace_dlq` and retries it in the background with exponential backoff (`2^retries` seconds):

```toml
[trace_queue]
dlq_retry_enabled = true
dlq_max_retries = 5          # 1 to 16 attempts
dlq_poll_interval_secs = 30
dlq_batch_size = 20          # Rows claimed per tick
dlq_lease_secs = 60
```

Once a row exhausts `dlq_max_retries`, it is marked exhausted and kept in `trace_dlq` for inspection, manual requeueing, or purging:

<!-- tabs:start -->
### **CLI**
```bash
orion-cli dlq list --limit 20
orion-cli dlq requeue <dlq-id>
orion-cli dlq purge --older-than-hours 168
```

### **curl**
```bash
curl -s -H "Authorization: Bearer $ORION_ADMIN_KEY" \
  "http://localhost:8080/api/v2/admin/trace-dlq?limit=20"

curl -s -X POST -H "Authorization: Bearer $ORION_ADMIN_KEY" \
  "http://localhost:8080/api/v2/admin/trace-dlq/purge" \
  -H "Content-Type: application/json" \
  -d '{"older_than_hours": 168}'
```
<!-- tabs:end -->

> [!WARNING]
> Setting `trace_queue.dlq_retry_enabled = false` also stops refreshing the `orion_trace_dlq_depth` Prometheus gauge. Keep `dlq_retry_enabled = true` if your alerts depend on `orion_trace_dlq_depth`.

---

## Part 3: Admin audit logs

Every admin mutation (plus `POST /api/v2/admin/definitions/test` and connection `probes`, which reach live backends) writes an entry to `audit_logs` recording the principal, action, resource type, resource ID, timestamp, and `details` JSON.

### Filter audit logs

All query parameters (`action`, `resource_type`, `resource_id`, `principal`, `start_time`, `end_time`) combine with `AND` and match exact values. Unrecognized query parameters are rejected with `400` so a typo never returns an unfiltered result set.

<!-- tabs:start -->
#### **CLI**
```bash
orion-cli audit-logs list
orion-cli audit-logs list --action v2_deployment_apply --resource-type deployment
orion-cli audit-logs list --resource-id nightly-order-rollup --start-time 2026-07-01T00:00:00Z
```

#### **curl**
```bash
curl -s -H "Authorization: Bearer $ORION_ADMIN_KEY" \
  "http://localhost:8080/api/v2/admin/audit-logs?action=v2_deployment_apply&resource_type=deployment"
```
<!-- tabs:end -->

### Complete `action` and `resource_type` vocabulary

Because `action` and `resource_type` use exact-match filtering, use the exact identifiers from this table in your compliance queries:

| `action` | `resource_type` | Written by |
|---|---|---|
| `create` | `backup` | `POST /backups` |
| `reset` | `circuit_breaker` | `POST /connectors/circuit-breakers/{key}` |
| `invalidate` | `cache_namespace` | `POST /cache/namespaces/{namespace}/invalidate`. A workflow's `cache_invalidate` is not an admin action and writes no row |
| `purge`, `requeue` | `trace_dlq` | The [trace DLQ](../reference/api.md) endpoints |
| `retry` | `cron_occurrence` | `POST /cron/occurrences/{id}/retry` |
| `cancel` | `cron_occurrence` | `POST /cron/occurrences/{id}/cancel` |
| `trigger` | `channel` | `POST /channels/{id}/trigger` — a manual run of a cron channel |
| `v2_package_upload` | `connector_package` | `POST /connector-packages`; `resource_id` is the package digest |
| `v2_package_admit` | `connector_package` | `PUT /connector-packages/{digest}/admission` |
| `v2_package_revoke` | `connector_package` | `POST /connector-packages/{digest}/revocations` |
| `v2_definition_upload` | `definition` | `POST /definitions`; `resource_id` is the definition digest |
| `v2_definition_test` | `definition` | `POST /definitions/test`, before the run; `resource_id` is the digest the workflow definition would have |
| `v2_connection_create` | `connection` | `POST /connections/{id}/revisions` — one row per connection revision; `resource_id` is `{connection_id}@{revision}` |
| `v2_connection_probe` | `connection` | `POST /connections/{id}/revisions/{revision}/probes` — written before the probe connects, since it reaches a real backend with the connection's credentials; `resource_id` is `{connection_id}@{revision}` |
| `v2_deployment_apply` | `deployment` | `POST /deployments` with a `manifest`; `resource_id` is the committed revision. Written in the transaction that moves the deployment head, so it is there the moment the apply returns, and an apply that commits nothing (an idempotent replay, a lost race on the base revision) writes none |
| `v2_deployment_rollback` | `deployment` | `POST /deployments` with `rollback_to`; `resource_id` is `{rollback_to}->{committed_revision}`. In the head move's transaction, like an apply's |

### Correlate multi-step deployments with `X-Orion-Change-Context`

Attach a ticket number, release tag, or CI build ID across all API calls in a deployment by setting `ORION_CHANGE_CONTEXT` (or passing `--change-context` on the CLI / `X-Orion-Change-Context` via HTTP):

```bash
export ORION_CHANGE_CONTEXT="ticket=OPS-4412"
orion-cli deployments apply -d dist/deployment

orion-cli audit-logs list --start-time 2026-07-01T00:00:00Z --output json \
  | jq '.data[] | select((.details | fromjson).change_context == "ticket=OPS-4412")'
```

### Configure audit retention and queue bounds

```toml
[audit]
retention_days = 90            # Delete audit rows older than 90 days (0 = keep forever)
cleanup_interval_secs = 3600
max_pending = 1000             # Bounded async audit writer queue
drain_timeout_secs = 5         # Shutdown drain wait for pending audit rows (must be > 0)
```

`v2_deployment_apply` and `v2_deployment_rollback` rows commit inside the same database transaction that advances the deployment head, so they can never be dropped by queue overflow. All other audit rows flow through `audit.max_pending`; alert on any non-zero value of `orion_audit_events_dropped_total`.

## Next steps

- [Monitoring, metrics, and alerts](./monitoring.md): Configure alerts for `orion_audit_events_dropped_total`, `orion_trace_persistence_failures_total`, and `orion_trace_dlq_depth`.
- [Admin and Data REST API](../reference/api.md): Full endpoint contracts for `/api/v2/admin/traces`, `/api/v2/admin/trace-dlq`, and `/api/v2/admin/audit-logs`.
