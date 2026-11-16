<!-- description: Complete reference for Orion's HTTP REST APIs: Data API (/api/v2/data/**), v2 Admin API (/api/v2/admin/**), trace and DLQ endpoints, and OpenAPI 3.1 spec. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# Admin and Data REST API

Orion serves two HTTP planes from one binary:
1. **The Data API (`/api/v2/data/**`)**: Executes deployed channels synchronously or asynchronously (`/async`).
2. **The v2 Admin API (`/api/v2/admin/**`)**: Manages immutable definitions, connection revisions, provider packages, deployments, execution traces, dead-letter queues, cron occurrences, circuit breakers, audit logs, and SQLite backups.

Every endpoint is specified in the machine-readable [OpenAPI 3.1 document](#5-health-metrics-and-openapi-31-specification) (`openapi.json`).

---

## 1. Data API (`/api/v2/data/**`)

| Endpoint | Description |
|---|---|
| `POST /api/v2/data/{channel}` | Invoke a channel synchronously by channel name or matching `route_pattern`. |
| `POST /api/v2/data/{channel}/async` | Enqueue an asynchronous execution; returns `202 Accepted` with `{trace_id, trace_token}`. |

### Route resolution order

When a request arrives at `/api/v2/data/{path}` (or an additional prefix configured in `server.data_mounts`), Orion resolves the target channel in three steps:
1. **Async check**: A trailing `/async` suffix switches the execution to asynchronous queue mode.
2. **Route table match**: The HTTP method and path are matched byte-for-byte against deployed `rest`, `http`, `graphql`, and `sse` channel `route_pattern` values, ordered by `priority` descending, then segment count descending, then channel name. Path parameters (`{id}`) are percent-decoded once and written to `metadata.params`.
3. **Channel name fallback**: A single-segment path resolves to the channel with that `name` (`POST /api/v2/data/orders`).

```bash
# Synchronous call by channel name or route pattern
curl -s -X POST http://localhost:8080/api/v2/data/orders \
  -H "Content-Type: application/json" \
  -d '{ "data": { "order_id": "ORD-123", "total": 25000 } }'

# Asynchronous submission
curl -s -X POST http://localhost:8080/api/v2/data/orders/async \
  -H "Content-Type: application/json" \
  -d '{ "data": { "order_id": "ORD-456" } }'
```

### Request envelope (`body_mode: "auto"` vs `"payload"`)

- Under `config.request.body_mode = "auto"` (default), if the request body is a JSON object with a top-level `data` or `metadata` key, Orion unwraps `data` as the payload and merges `metadata`. Any other JSON object, array, or scalar is used directly as `data`.
- Under `config.request.body_mode = "payload"`, Orion uses the entire request body verbatim as `data` and ignores caller-supplied `metadata`.
- Pass `X-Orion-Profile: 1` (or `?profile=1`) when `tracing.debug_profile_enabled = true` to include a per-phase and per-handler `_orion.profile` timing breakdown in the synchronous response.

---

## 2. Admin API authentication and conventions

When `admin_auth.enabled = true`, every `/api/v2/admin/**` request requires a valid admin key in `Authorization: Bearer <key>` (or the custom header configured in `admin_auth.header`). After 5 consecutive failed attempts, the client IP is locked out with exponential backoff up to 30 seconds.

- **Read-only keys (`admin_auth.read_only_api_keys`)**: Permit `GET` and `HEAD` requests plus side-effect-free validation routes (`POST /api/v2/admin/deployment-plans`, `POST /api/v2/admin/authoring/scaffold`, `POST /api/v2/admin/authoring/lint`, `POST /api/v2/admin/authoring/pack`). All mutating routes plus `POST /api/v2/admin/definitions/test` and `POST /api/v2/admin/connections/{id}/revisions/{revision}/probes` return `403 FORBIDDEN`.
- **Cursor pagination**: List endpoints accept `?limit=` (`1` to `1000`), `?cursor=` (from the previous page's `next_cursor`), and `?include_total=true` (when an exact `total` count is needed).
- **Audit correlation**: Pass `X-Orion-Change-Context: <text>` on any admin request to record a change ticket or release identifier under `details.change_context` in `audit_logs`.

---

## 3. Deployment, definition, connection, and provider package endpoints

| Endpoint | Description |
|---|---|
| `GET /api/v2/admin/runtime-capabilities` | Return `api_version` (`2`), `edition`, `admission_policy_id`, `channel_drivers`, `provider_backends`, `deployment_head`, and `node_status`. |
| `POST /api/v2/admin/definitions` | Validate and store a canonical `workflow`, `channel`, or `plugin` definition; returns `{digest, kind, size_bytes, advisories}`. |
| `GET /api/v2/admin/definitions/{digest}` | Read a stored definition by its `sha256:...` digest (channel credentials are masked as `******`). |
| `POST /api/v2/admin/definitions/test` | Execute a workflow definition once against live connectors on this node without deploying it; returns `{matched, output, trace, errors}`. |
| `GET /api/v2/admin/connections` | List all stored revisions across all connections. |
| `GET /api/v2/admin/connections/{id}/revisions` | List stored revisions for connection `{id}`. |
| `POST /api/v2/admin/connections/{id}/revisions` | Create an immutable connection revision (`{revision, provider_digest, public_config, secret_locators, bindings}`). |
| `GET /api/v2/admin/connections/{id}/revisions/{revision}` | Fetch a single connection revision and its `configuration_digest`. |
| `POST /api/v2/admin/connections/{id}/revisions/{revision}/probes` | Probe a connection revision's live backend (`SELECT 1`, TCP connect, cache read, or model admission). |
| `POST /api/v2/admin/deployment-plans` | Validate a candidate manifest against `base_revision`; returns `reviewed_manifest_digest` and added/updated/removed members. |
| `POST /api/v2/admin/deployments` | Commit a reviewed manifest (or `rollback_to` a previous revision) under compare-and-swap on `base_revision` and `idempotency_key`. |
| `GET /api/v2/admin/deployments` | Return `head_revision`, `applied_revision`, `epoch`, and a paginated list of committed revisions (`?limit=`, `?cursor=`). |
| `GET /api/v2/admin/deployments/{revision}` | Fetch a committed deployment revision including its full `manifest`. |
| `GET /api/v2/admin/deployments/{revision}/convergence` | Check revision convergence on this node (`?target=this-node`) or across the cluster (`?target=observed`). |
| `GET /api/v2/admin/connector-packages` | List uploaded connector provider packages (`?limit=`, `?cursor=`). |
| `POST /api/v2/admin/connector-packages` | Upload a packed connector provider package (and optionally admit it). |
| `GET /api/v2/admin/connector-packages/{digest}` | Inspect a connector provider package, its admission state, and its operations. |
| `PUT /api/v2/admin/connector-packages/{digest}/admission` | Admit an uploaded connector provider package (optionally verifying a provider lock). |
| `POST /api/v2/admin/connector-packages/{digest}/revocations` | Emergency-revoke a connector provider package with an audit reason. |
| `POST /api/v2/admin/authoring/scaffold` | Generate a deterministic connector provider package skeleton. |
| `POST /api/v2/admin/authoring/lint` | Validate a connector provider package bundle without storing it. |
| `POST /api/v2/admin/authoring/pack` | Pack a connector provider package bundle and compute its `package_digest`. |

```bash
# Inspect runtime capabilities
curl -s http://localhost:8080/api/v2/admin/runtime-capabilities \
  -H "Authorization: Bearer $ORION_ADMIN_KEY" | jq '{api_version, edition}'

# Upload a definition and read it back
curl -s -X POST http://localhost:8080/api/v2/admin/definitions \
  -H "Authorization: Bearer $ORION_ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{"kind": "workflow", "definition": {"name": "Greet", "tasks": [{"id": "greet", "function": {"name": "map", "input": {"mappings": [{"path": "data.greeting", "logic": "Hello"}]}}}]}}'

curl -s http://localhost:8080/api/v2/admin/definitions/sha256:3f9c \
  -H "Authorization: Bearer $ORION_ADMIN_KEY"

# Test a workflow definition once
curl -s -X POST http://localhost:8080/api/v2/admin/definitions/test \
  -H "Authorization: Bearer $ORION_ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{"definition": {"name": "Greet", "tasks": [{"id": "greet", "function": {"name": "map", "input": {"mappings": [{"path": "data.greeting", "logic": "Hello"}]}}}]}, "data": {"name": "Ada"}}'

# Create, read, and probe a connection revision
curl -s -X POST http://localhost:8080/api/v2/admin/connections/orders-db/revisions \
  -H "Authorization: Bearer $ORION_ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{"revision": 1, "provider_digest": "sha256:...", "public_config": {"max_connections": 5}, "secret_locators": {"connection_string": "env://ORDERS_DB_URL"}}'

curl -s http://localhost:8080/api/v2/admin/connections \
  -H "Authorization: Bearer $ORION_ADMIN_KEY"

curl -s http://localhost:8080/api/v2/admin/connections/orders-db/revisions \
  -H "Authorization: Bearer $ORION_ADMIN_KEY"

curl -s http://localhost:8080/api/v2/admin/connections/orders-db/revisions/1 \
  -H "Authorization: Bearer $ORION_ADMIN_KEY"

curl -s -X POST http://localhost:8080/api/v2/admin/connections/orders-db/revisions/1/probes \
  -H "Authorization: Bearer $ORION_ADMIN_KEY"

# Plan, apply, list, and check convergence of deployments
curl -s -X POST http://localhost:8080/api/v2/admin/deployment-plans \
  -H "Authorization: Bearer $ORION_ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{"base_revision": 0, "manifest": {"schema_version": 2, "providers": [], "members": []}}'

curl -s -X POST http://localhost:8080/api/v2/admin/deployments \
  -H "Authorization: Bearer $ORION_ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{"idempotency_key": "deploy-1", "base_revision": 0, "reason": "initial", "reviewed_manifest_digest": "sha256:...", "manifest": {"schema_version": 2, "providers": [], "members": []}}'

curl -s "http://localhost:8080/api/v2/admin/deployments?limit=20" \
  -H "Authorization: Bearer $ORION_ADMIN_KEY"

curl -s http://localhost:8080/api/v2/admin/deployments/1 \
  -H "Authorization: Bearer $ORION_ADMIN_KEY"

curl -s "http://localhost:8080/api/v2/admin/deployments/1/convergence?target=observed" \
  -H "Authorization: Bearer $ORION_ADMIN_KEY"
```

---

## 4. Operational admin endpoints

| Endpoint | Description |
|---|---|
| `GET /api/v2/admin/engine/status` | Return server version, uptime, loaded workflow count, active generation, `load_issues` (`channels`, `plugins`, `connectors`), and node `capabilities`. |
| `GET /api/v2/admin/functions` | Return the live catalog of built-in and plugin task functions with their `category`, `source`, `retry_safety`, and `input_fields`. |
| `GET /api/v2/admin/traces` | List execution traces (payload-free summaries). Filter with `?channel=`, `?status=`, `?mode=`, `?limit=`, `?cursor=`, `?include_total=true`. |
| `GET /api/v2/admin/traces/{id}` | Fetch one trace including `message`, `error`, and `task_trace_json`. Authorized by `x-trace-token` or an admin API key. |
| `GET /api/v2/admin/trace-dlq` | List dead-letter queue entries (`?limit=`, `?cursor=`, `?include_total=true`). |
| `GET /api/v2/admin/trace-dlq/{id}` | Fetch one DLQ entry including its payload and error details. |
| `POST /api/v2/admin/trace-dlq/{id}/requeue` | Reset `retry_count = 0` on a DLQ entry and schedule it for immediate retry. |
| `POST /api/v2/admin/trace-dlq/purge` | Delete exhausted DLQ entries older than `{"older_than_hours": N}` (required JSON body). |
| `GET /api/v2/admin/cron/occurrences` | List scheduled cron occurrences (`?channel_id=`, `?status=`, `?since=`, `?until=`, `?limit=`, `?cursor=`, `?include_total=true`). |
| `GET /api/v2/admin/cron/occurrences/{id}` | Fetch full details of one cron occurrence (`scheduled_for`, `attempt`, `trace_id`, `singleton_key`, `singleton_slot`). |
| `POST /api/v2/admin/cron/occurrences/{id}/retry` | Retry a `failed`, `skipped_misfire`, or `skipped_singleton` occurrence under its original `scheduled_for` timestamp. |
| `POST /api/v2/admin/cron/occurrences/{id}/cancel` | Cancel an in-flight cron occurrence and release its singleton concurrency slot. |
| `GET /api/v2/admin/cron/status` | List every deployed cron channel with its schedule, next fire time, last run, backlog, and held concurrency slots. |
| `POST /api/v2/admin/channels/{id}/trigger` | Trigger an immediate manual occurrence of a deployed cron channel (`202 Accepted`). |
| `GET /api/v2/admin/connectors/circuit-breakers` | List circuit breaker states on this node (`channel:connector`). |
| `POST /api/v2/admin/connectors/circuit-breakers/{key}` | Reset circuit breaker `{key}` (`channel:connector`) on this node and across all cluster replicas. |
| `POST /api/v2/admin/cache/namespaces/{namespace}/invalidate` | Increment the version counter for response-cache namespace `{namespace}` across all stores. |
| `GET /api/v2/admin/audit-logs` | Query admin audit log entries (`?action=`, `?resource_type=`, `?resource_id=`, `?principal=`, `?start_time=`, `?end_time=`, `?limit=`, `?cursor=`, `?include_total=true`). |
| `POST /api/v2/admin/backups` | Create an online SQLite backup (`VACUUM INTO`) in `storage.backup_dir` (single-node SQLite only). |
| `GET /api/v2/admin/backups` | List SQLite backup files in `storage.backup_dir`. |

```bash
# Engine status and function catalog
curl -s http://localhost:8080/api/v2/admin/engine/status -H "Authorization: Bearer $ORION_ADMIN_KEY"
curl -s http://localhost:8080/api/v2/admin/functions -H "Authorization: Bearer $ORION_ADMIN_KEY"

# Traces and DLQ
curl -s "http://localhost:8080/api/v2/admin/traces?channel=orders&status=completed&limit=20" -H "Authorization: Bearer $ORION_ADMIN_KEY"
curl -s http://localhost:8080/api/v2/admin/traces/550e8400-e29b-41d4-a716-446655440000 -H "x-trace-token: b1946ac92492d2347c6235b4d2611184"
curl -s "http://localhost:8080/api/v2/admin/trace-dlq?limit=20" -H "Authorization: Bearer $ORION_ADMIN_KEY"
curl -s http://localhost:8080/api/v2/admin/trace-dlq/550e8400-e29b-41d4-a716-446655440000 -H "Authorization: Bearer $ORION_ADMIN_KEY"
curl -s -X POST http://localhost:8080/api/v2/admin/trace-dlq/550e8400-e29b-41d4-a716-446655440000/requeue -H "Authorization: Bearer $ORION_ADMIN_KEY"
curl -s -X POST http://localhost:8080/api/v2/admin/trace-dlq/purge -H "Authorization: Bearer $ORION_ADMIN_KEY" -H "Content-Type: application/json" -d '{"older_than_hours": 168}'

# Cron occurrences and manual trigger
curl -s "http://localhost:8080/api/v2/admin/cron/occurrences?channel_id=nightly-order-rollup&limit=20" -H "Authorization: Bearer $ORION_ADMIN_KEY"
curl -s http://localhost:8080/api/v2/admin/cron/status -H "Authorization: Bearer $ORION_ADMIN_KEY"
curl -s -X POST http://localhost:8080/api/v2/admin/channels/nightly-order-rollup/trigger -H "Authorization: Bearer $ORION_ADMIN_KEY"
curl -s -X POST http://localhost:8080/api/v2/admin/cron/occurrences/01a070e1-5e6a-7552-99d3-66dd70c1feff/retry -H "Authorization: Bearer $ORION_ADMIN_KEY"
curl -s -X POST http://localhost:8080/api/v2/admin/cron/occurrences/01a070e1-5e6a-7552-99d3-66dd70c1feff/cancel -H "Authorization: Bearer $ORION_ADMIN_KEY"

# Circuit breakers, cache invalidation, audit logs, and SQLite backups
curl -s http://localhost:8080/api/v2/admin/connectors/circuit-breakers -H "Authorization: Bearer $ORION_ADMIN_KEY"
curl -s -X POST http://localhost:8080/api/v2/admin/connectors/circuit-breakers/orders:orders-db -H "Authorization: Bearer $ORION_ADMIN_KEY"
curl -s -X POST http://localhost:8080/api/v2/admin/cache/namespaces/ladder/invalidate -H "Authorization: Bearer $ORION_ADMIN_KEY"
curl -s "http://localhost:8080/api/v2/admin/audit-logs?action=v2_deployment_apply&limit=20" -H "Authorization: Bearer $ORION_ADMIN_KEY"
curl -s -X POST http://localhost:8080/api/v2/admin/backups -H "Authorization: Bearer $ORION_ADMIN_KEY"
curl -s http://localhost:8080/api/v2/admin/backups -H "Authorization: Bearer $ORION_ADMIN_KEY"
```

---

## 5. Health, metrics, and OpenAPI 3.1 specification

| Endpoint | Description |
|---|---|
| `GET /healthz` | Liveness probe (`200 OK` while process is alive). |
| `GET /readyz` | Readiness probe (`200 OK` when database, startup, background tasks, channel drivers, and cluster store are ready). |
| `GET /health` | Component status (`200 OK` with `"status": "ok"` or `"degraded"`, or `503` when database is down). |
| `GET /metrics` | Prometheus metrics in exposition format (when `metrics.enabled = true`). |
| `GET /docs` | Interactive Swagger UI (enabled outside production by default; controlled by `server.docs.enabled`). |
| `GET /api/v2/openapi.json` | Generated OpenAPI 3.1 JSON specification (also available offline via `orion-server dump-openapi`). |

## Next steps

- [CLI reference](./cli.md): Every `orion-cli` and `orion-server` subcommand that wraps these APIs.
- [Error codes and metrics](./errors-and-metrics.md): Full registry of error codes and Prometheus metrics.
