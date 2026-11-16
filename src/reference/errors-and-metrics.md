<!-- description: Complete reference for Orion response envelopes, general and v2 admin error code registries, field validation codes, and all Prometheus metric series. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# Error codes and metrics

This reference specifies Orion's HTTP response envelopes, the complete `error.code` and v2 Admin API error registries, field-level validation codes, and every Prometheus metric series exported at `GET /metrics`.

---

## Part 1: Response envelopes and error registries

### The shared error envelope

Every non-2xx response on both the admin plane and the data plane returns a single JSON error envelope:

```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "Workflow with id 'wf_orders' not found",
    "request_id": "9b2d64ec-6f0a-4f6e-9c1a-6d1f2b3c4d5e"
  }
}
```

| Field | Type | Present | Description |
|---|---|---|---|
| `code` | string | always | Machine-readable error code from [The code registry](#the-code-registry) or [The v2 admin code registry](#the-v2-admin-code-registry). Always branch on `code`, never on `message`. |
| `message` | string | always | Human-readable diagnostic summary. |
| `details` | array | validation errors only | Field-pathed validation errors (`[{path, code, message, expected?, got?}]`). |
| `request_id` | string | when request has an ID | Echoes the `x-request-id` response header for log and trace correlation. |

### Synchronous data-plane result envelope

Unless a channel sets `config.response.mode = "shaped"`, synchronous data-plane calls return HTTP `200` with:

```json
{
  "id": "6e7f0a4b-8c2d-4e1f-9a3b-5c6d7e8f9a0b",
  "status": "ok",
  "data": { "order": { "order_id": "ORD-123", "flagged": true } },
  "errors": []
}
```

> [!WARNING]
> An HTTP `200` envelope can still contain task-level errors in `errors[]` (`[{code, message, task_id}]`). Always check that `errors` is empty before treating `data` as a clean result. In production (`server.verbose_errors = false`), task error messages in `errors[]` are sanitized to `"Task processing failed; full detail is available in the trace"`.

### Asynchronous `/async` acknowledgment

Submitting to `/api/v2/data/<channel>/async` writes an initial trace row and returns HTTP `202 Accepted`:

```json
{
  "trace_id": "550e8400-e29b-41d4-a716-446655440000",
  "trace_token": "b1946ac92492d2347c6235b4d2611184"
}
```

Poll `GET /api/v2/admin/traces/{trace_id}` with `x-trace-token: <trace_token>` (or an admin API key) to read the result once execution finishes.

---

## The code registry

Every general `error.code` emitted on the data plane and operational admin endpoints:

<div class="table-filter" data-label="Filter error codes"></div>

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `VALIDATION_ERROR` | 400 | Invalid input — malformed body, failed strict validation, or bad query parameter |
| `UNAUTHORIZED` | 401 | Missing or invalid credentials |
| `FORBIDDEN` | 403 | Access denied — a read-only admin key on a mutating method, an `Origin` outside a channel's `origin_allow_list`, or a `channel_call` relaying a target channel's `403` |
| `NOT_FOUND` | 404 | Resource not found |
| `METHOD_NOT_ALLOWED` | 405 | The path exists but not for this HTTP method |
| `CONFLICT` | 409 | Duplicate idempotency key or uncaught database unique/foreign-key constraint violation |
| `PAYLOAD_TOO_LARGE` | 413 | Request body exceeded `ingest.max_payload_size` (data plane) or `server.max_admin_body_size` (admin plane) |
| `UNSUPPORTED_MEDIA_TYPE` | 415 | Invalid `Content-Type` header |
| `RATE_LIMITED` | 429 | Caller exceeded a rate limit (includes a `Retry-After` header) |
| `INTERNAL_ERROR` | 500 | Internal server error |
| `ENGINE_ERROR` | 500 | Workflow execution failed inside the engine |
| `STORAGE_ERROR` | 500 | State database operation failed |
| `SERIALIZATION_ERROR` | 500 | Stored row could not be decoded or response could not be serialized |
| `CONFIG_ERROR` | 500 | Configuration error surfaced at request time |
| `RESPONSE_TOO_LARGE` | 500 | Workflow result exceeded `trace_queue.max_result_size_bytes` |
| `CONNECTOR_CONTRACT_VIOLATION` | 500 | Provider backend returned a response that violated its declared contract |
| `CONNECTOR_PROVIDER_FAILURE` | 502 | Provider backend execution failed at the upstream target |
| `CONNECTOR_PROVIDER_UNAVAILABLE` | 503 | Provider backend is temporarily unavailable or refused the call |
| `SERVICE_UNAVAILABLE` | 503 | Backpressure shed the request, a guard's backend failed closed, a quarantined channel was called, or the server is draining |
| `CIRCUIT_OPEN` | 503 | Target connector's circuit breaker is open |
| `LOOP_DETECTED` | 508 | A `channel_call` chain formed a cycle or exceeded `engine.max_channel_call_depth` |
| `TIMEOUT` | 504 | Workflow execution exceeded the channel's `timeout_ms` deadline |

---

## The v2 admin code registry

Closed registry (`orion_api::V2Code`) returned by the v2 deployment, definition, connection, and provider package endpoints:

<div class="table-filter" data-label="Filter v2 admin codes"></div>

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `PACKAGE_NOT_FOUND` | 404 | No connector package is stored under the digest |
| `ADMISSION_NOT_FOUND` | 404 | The package was never admitted, so there is nothing to revoke |
| `BASE_REVISION_NOT_FOUND` | 404 | The plan or apply names a `base_revision` that was never committed |
| `ROLLBACK_TARGET_NOT_FOUND` | 404 | `rollback_to` names a revision that was never committed |
| `CONNECTION_REVISION_NOT_FOUND` | 404 | The connection has no such revision, or a manifest names one that is not stored |
| `REVISION_NOT_FOUND` | 404 | No committed deployment revision has that number |
| `PROVIDER_NOT_FOUND` | 404 | This node registers no backend for the connection's provider package |
| `PUBLISHER_VERSION_CONFLICT` | 409 | A package with this publisher, provider id, and version is stored with other content |
| `IMMUTABLE_CONTENT_CONFLICT` | 409 | An immutable artifact is stored under this identity with other content |
| `IMMUTABLE_CONNECTION_CONFLICT` | 409 | The connection revision is stored with other content; create the next revision instead |
| `IDEMPOTENCY_KEY_CONFLICT` | 409 | The principal used this `idempotency_key` for a different request |
| `DEPLOYMENT_HEAD_CONFLICT` | 409 | The head moved since `base_revision`; plan again on the new head |
| `REVIEWED_DIGEST_MISMATCH` | 409 | The manifest no longer plans to the `reviewed_manifest_digest` the caller reviewed |
| `CANDIDATE_SUPERSEDED` | 409 | A newer revision superseded the one a node was building |
| `PROVIDER_NOT_ADMITTED` | 403 | A manifest names a provider package that is not admitted |
| `PROVIDER_REVOKED` | 403 | The provider package is revoked |
| `ADMISSION_INVALIDATED` | 403 | The package's admission was recorded under another policy or runtime and no longer holds |
| `TIER_NOT_ENABLED` | 403 | A package above the node's edition, or a canary rollout on a Community node |
| `PACKAGE_INVALID_PROVIDER_ID` | 400 | The admission policy refuses the package's provider id |
| `PACKAGE_INVALID_TARGET` | 400 | The admission policy refuses the package's target |
| `ADMISSION_REJECTED` | 403 | The edition's admission policy refused the package |
| `DB_TRANSACTION_ERROR` | 503 | The state database could not begin a transaction |
| `DB_WRITE_ERROR` | 503 | The state database could not be written |
| `DB_COMMIT_ERROR` | 503 | The state database could not commit |
| `DB_READ_ERROR` | 503 | The state database could not be read |
| `CORRUPT_PERSISTED_STATE` | 500 | A stored row could not be read back |
| `SERIALIZE_ERROR` | 500 | A row the store wrote could not be serialized |
| `REQUEST_HASH_INVALID` | 500 | The request's hash could not be computed |
| `CANDIDATE_REVISION_MISSING` | 500 | A node was asked to build a revision the store does not hold |
| `PROBE_FAILED` | 500 | A health probe's run could not be started |
| `ARCHIVE_ENTRY_COUNT_EXCEEDED` | 400 | The package archive: entry count exceeded |
| `ARCHIVE_SYMLINK_DENIED` | 400 | The package archive: symlink denied |
| `ARCHIVE_DUPLICATE_ENTRY` | 400 | The package archive: duplicate entry |
| `ARCHIVE_COMPRESSION_RATIO_EXCEEDED` | 400 | The package archive: compression ratio exceeded |
| `ARCHIVE_SIZE_LIMIT_EXCEEDED` | 400 | The package archive: size limit exceeded |
| `ARCHIVE_INVENTORY_MISSING` | 400 | The package archive: inventory missing |
| `ARCHIVE_INVENTORY_INVALID` | 400 | The package archive: inventory invalid |
| `ARCHIVE_PATH_INVALID` | 400 | The package archive: path invalid |
| `INVALID_PACKAGE_ENCODING` | 400 | A file in an upload is neither UTF-8 text nor valid `base64:` |
| `PACK_VALIDATION_FAILED` | 400 | `POST /authoring/pack` could not pack the files |
| `PACKAGE_INVALID` | 400 | A package finding with no dedicated code; the message carries the finding code |
| `INVENTORY_CONTENT_MISMATCH` | 400 | The package inventory: content mismatch |
| `INVENTORY_LIMIT_EXCEEDED` | 400 | The package inventory: limit exceeded |
| `INVENTORY_NOT_CANONICAL` | 400 | The package inventory: not canonical |
| `INVENTORY_ORDER_INVALID` | 400 | The package inventory: order invalid |
| `INVENTORY_PATH_COLLISION` | 400 | The package inventory: path collision |
| `INVENTORY_PATH_INVALID` | 400 | The package inventory: path invalid |
| `INVENTORY_PAYLOAD_MISMATCH` | 400 | The package inventory: payload mismatch |
| `INVENTORY_REQUIRED_FILE_MISSING` | 400 | The package inventory: required file missing |
| `INVENTORY_SELF_REFERENCE` | 400 | The package inventory: self reference |
| `INVENTORY_SHAPE_INVALID` | 400 | The package inventory: shape invalid |
| `CANONICAL_VALUE_UNSUPPORTED` | 400 | A value canonical JSON cannot carry, such as a non-integer number |
| `MANIFEST_INVALID_JSON` | 400 | The package's `connector.json`: invalid JSON |
| `MANIFEST_SHAPE_INVALID` | 400 | The package's `connector.json`: shape invalid |
| `MANIFEST_ARTIFACT_MISSING` | 400 | The package's `connector.json`: artifact missing |
| `MANIFEST_DEPENDENCY_ALIAS_DUPLICATE` | 400 | The package's `connector.json`: dependency alias duplicate |
| `MANIFEST_ENTRY_MISMATCH` | 400 | The package's `connector.json`: entry mismatch |
| `MANIFEST_IMPORT_PROFILE_CAPABILITY_MISSING` | 400 | The package's `connector.json`: import profile capability missing |
| `MANIFEST_LIMIT_EXCEEDS_PROFILE` | 400 | The package's `connector.json`: limit exceeds profile |
| `MANIFEST_OPERATION_RESERVED` | 400 | The package's `connector.json`: operation reserved |
| `MANIFEST_PATH_INVALID` | 400 | The package's `connector.json`: path invalid |
| `MANIFEST_RETRY_UNSAFE` | 400 | The package's `connector.json`: retry unsafe |
| `MANIFEST_SCHEMA_VERSION_UNSUPPORTED` | 400 | The package's `connector.json`: schema version unsupported |
| `DISTRIBUTION_INVALID_JSON` | 400 | `distribution.json`: invalid JSON |
| `DISTRIBUTION_SHAPE_INVALID` | 400 | `distribution.json`: shape invalid |
| `DOCUMENT_INVALID_JSON` | 400 | A document in the package: invalid JSON |
| `DOCUMENT_LIMIT_EXCEEDED` | 400 | A document in the package: limit exceeded |
| `SCHEMA_INVALID_JSON` | 400 | A schema in the package: invalid JSON |
| `SCHEMA_INVALID` | 400 | A schema in the package: invalid |
| `SCHEMA_PROFILE_LIMIT` | 400 | A schema in the package: profile limit |
| `SCHEMA_REFERENCE_CYCLE` | 400 | A schema in the package: reference cycle |
| `SCHEMA_REFERENCE_DENIED` | 400 | A schema in the package: reference denied |
| `SCHEMA_REFERENCE_MISSING` | 400 | A schema in the package: reference missing |
| `SCHEMA_REFERENCE_NOT_SCHEMA` | 400 | A schema in the package: reference not schema |
| `SCHEMA_UNSUPPORTED_KEYWORD` | 400 | A schema in the package: unsupported keyword |
| `PACKAGE_INVALID_TIER` | 400 | `distribution.json` names a tier outside `community`, `standard`, and `enterprise` |
| `ARTIFACT_NOT_ACTIVE` | 400 | A manifest names an artifact the collector drained; upload it again |
| `ARTIFACT_DRAINING_OR_DELETED` | 400 | The artifact is draining or deleted |
| `LOCK_ALIAS_DUPLICATE` | 400 | The dependency lock: alias duplicate |
| `LOCK_ARTIFACT_UNAVAILABLE` | 400 | The dependency lock: artifact unavailable |
| `LOCK_BINDING_SLOT_MISMATCH` | 400 | The dependency lock: binding slot mismatch |
| `LOCK_CLOSURE_MISSING` | 400 | The dependency lock: closure missing |
| `LOCK_CYCLE` | 400 | The dependency lock: cycle |
| `LOCK_DECLARATION_MISMATCH` | 400 | The dependency lock: declaration mismatch |
| `LOCK_DUPLICATE_NODE` | 400 | The dependency lock: duplicate node |
| `LOCK_GRANT_MISMATCH` | 400 | The dependency lock: grant mismatch |
| `LOCK_IDENTITY_CONFLICT` | 400 | The dependency lock: identity conflict |
| `LOCK_METADATA_MISMATCH` | 400 | The dependency lock: metadata mismatch |
| `LOCK_ORDER_INVALID` | 400 | The dependency lock: order invalid |
| `LOCK_SHAPE_INVALID` | 400 | The dependency lock: shape invalid |
| `LOCK_UNREACHABLE_NODE` | 400 | The dependency lock: unreachable node |
| `LOCK_VERSION_REQUIREMENT_UNQUALIFIED` | 400 | The dependency lock: version requirement unqualified |
| `CONNECTION_REVISION_INVALID` | 400 | The connection revision is malformed (revision `0` or empty id) |
| `CONNECTION_PUBLIC_SECRET_FORBIDDEN` | 400 | A credential sits in `public_config`; move it to `secret_locators` |
| `CONNECTION_SECRET_LOCATOR_INVALID` | 400 | A secret locator is not a valid reference (`env://`, `vault://`) |
| `CONNECTION_DIGEST_MISMATCH` | 400 | A member's `definition_digest` is not the connection revision's configuration digest |
| `CONNECTION_PROVIDER_NOT_DECLARED` | 400 | A connection's provider is neither declared in `manifest.providers` nor built in |
| `PROVIDER_METADATA_MISMATCH` | 400 | A `manifest.providers` entry's publisher, id, or version does not match the stored package |
| `MEMBER_DEFINITION_NOT_FOUND` | 400 | A manifest member names a definition digest that is not stored |
| `OWNING_PACKAGE_NOT_DECLARED` | 400 | A member's `owning_package` is not declared in `manifest.providers` |
| `MEMBER_ARTIFACT_NOT_IN_PACKAGE` | 400 | A member names an artifact its owning package does not carry |
| `ROLLBACK_TARGET_COMPACTED` | 400 | The collector compacted the target revision's members |
| `ENTITY_REF_INVALID` | 400 | A member identity is malformed |
| `PROVIDER_REF_INVALID` | 400 | A `manifest.providers` entry is malformed |
| `ROLLOUT_PARTITION_INVALID` | 400 | A member's `rollouts` do not partition `[0, 100)` contiguously |
| `MANIFEST_VERSION_UNSUPPORTED` | 400 | The deployment manifest uses an unsupported `schema_version` |
| `MANIFEST_DUPLICATE_PROVIDER` | 400 | The deployment manifest: duplicate provider |
| `MANIFEST_OWNER_MISSING` | 400 | The deployment manifest: owner missing |
| `MANIFEST_OWNERSHIP_CONFLICT` | 400 | The deployment manifest: ownership conflict |
| `MANIFEST_DUPLICATE_SINGLETON` | 400 | The deployment manifest: duplicate singleton |
| `MANIFEST_DUPLICATE_MEMBER` | 400 | The deployment manifest: duplicate member |
| `MANIFEST_MISSING_REQUIREMENT` | 400 | The deployment manifest: missing requirement |
| `MANIFEST_CANONICAL_ERROR` | 400 | The deployment manifest cannot be canonicalized |
| `INVALID_MANIFEST_JSON` | 400 | The deployment manifest is not a valid manifest |
| `DEPLOYMENT_WOULD_NOT_BUILD` | 400 | This node could not build the candidate; nothing is committed |
| `ENGINE_RELOAD_FAILED` | 400 | The node could not rebuild its generation from the candidate |

### Field error codes

Closed vocabulary (`orion_api::error::field_codes`) carried inside `error.details[].code` when a definition or request body fails field-level validation:

| Code | Meaning |
|---|---|
| `REQUIRED` | The field was absent and is always required. |
| `REQUIRED_FOR_PROTOCOL` | Required for this `protocol` or `channel_type` (for example, `methods` on a `rest` channel). |
| `INVALID` | Present and well-typed, but not an acceptable value. |
| `TYPE_MISMATCH` | Present with the wrong JSON type. |
| `TOO_LONG` | Exceeds the maximum length allowed by the schema or protocol. |
| `UNKNOWN_FIELD` | Unrecognized key rejected by the strict parser. |
| `DUPLICATE_FIELD` | The same key appeared twice in one JSON object. |
| `DUPLICATE_TASK_ID` | Two steps in one workflow declare the same `id`. |
| `UNKNOWN_FUNCTION` | A task names a function the engine does not register. |
| `UNCOMPILED_SOURCE` | The document still contains uncompiled source constructs (`$from`, `use`, `$use`, `$each`, `$sql`); compile the directory with `orion-cli compile` first. |
| `UNRESOLVED_SECRET_REF` | A secret locator (`env://`, `vault://`) was placed in a workflow field that does not resolve secret locators. |

---

## Part 2: Prometheus metrics catalog

Every Prometheus metric exported at `GET /metrics` carries the `orion_` prefix (and an `instance` label in cluster mode).

### Core runtime metrics

| Name | Type | Labels | Description |
|------|------|--------|-------------|
| `orion_build_info` | Gauge | `version`, `git_hash`, `build_timestamp` | Always `1`; identifies the build running on each replica. |
| `orion_jwt_rejections_total` | Counter | `reason` | JWTs rejected at channel authentication, by typed reason (`expired`, `bad_signature`, `alg_rejected`, `scheme_mismatch`, etc.). |
| `orion_oauth_token_requests_total` | Counter | `connector`, `outcome` | Outbound managed-OAuth2 token requests (`ok`, `rejected`, `transport_error`). |
| `orion_oauth_login_total` | Counter | `channel`, `provider`, `leg`, `outcome` | Inbound browser OAuth2 sign-in legs (`authorize` or `callback`) by outcome. |
| `orion_messages_total` | Counter | `channel`, `status` | Messages processed per channel by outcome (`ok`, `error`, `timeout`, `duplicate`). |
| `orion_message_duration_seconds` | Histogram | `channel` | End-to-end channel message processing latency. |
| `orion_channel_streams_open` | Gauge | `channel` | Currently open streaming channel connections (such as SSE streams). |
| `orion_channel_stream_duration_seconds` | Histogram | `channel` | Wall-clock duration of completed or closed channel streams. |
| `orion_active_workflows` | Gauge | — | Number of workflows loaded in the active engine generation. |
| `orion_errors_total` | Counter | `reason` | Runtime errors by cause (`engine`, `timeout`, `panic`, `channel_quarantined`). |
| `orion_admin_auth_failures_total` | Counter | `reason` | Rejected admin requests (`missing_or_malformed`, `invalid_key`, `locked_out`, `read_only_write`). |
| `orion_trace_token_query_reads_total` | Counter | — | Trace reads authorized via the deprecated `?token=` query parameter. |
| `orion_http_requests_total` | Counter | `method`, `path`, `status` | HTTP requests served by route template and status code. |
| `orion_http_request_duration_seconds` | Histogram | `method`, `path`, `status_class` | HTTP request latency by route template and status class (`2xx`, `4xx`, `5xx`). |
| `orion_db_query_duration_seconds` | Histogram | `operation` | State database query latency. |
| `orion_db_pool_size` | Gauge | — | Total connections in the state database pool. |
| `orion_db_pool_idle` | Gauge | — | Idle connections in the state database pool. |
| `orion_session_pool_entries` | Gauge | `cache` | Cached sessions held by a version-fenced provider pool. |
| `orion_memory_cache_partitions` | Gauge | — | Active connection-identity partitions in `org.orion.memory-cache`. |
| `orion_engine_reloads_total` | Counter | `status` | Engine generation reload events (`ok` or `error`). |
| `orion_engine_reload_duration_seconds` | Histogram | — | Engine reload build duration. |
| `orion_circuit_breaker_trips_total` | Counter | `connector`, `channel` | Circuit breaker transitions to `open`. |
| `orion_circuit_breaker_rejections_total` | Counter | `connector`, `channel` | Calls fast-failed with `503 CIRCUIT_OPEN`. |
| `orion_connector_requests_total` | Counter | `connector`, `channel`, `status` | Outbound connector calls by outcome. |
| `orion_connector_request_duration_seconds` | Histogram | `connector`, `channel` | Outbound connector call duration. |
| `orion_provider_invocations_total` | Counter | `driver`, `kind`, `outcome` | Provider backend invocations (`rust`, `workflow`, `wasm`) by outcome (`ok`, `error`). |
| `orion_provider_invocation_duration_seconds` | Histogram | `driver`, `kind` | Wall-clock duration of provider backend invocations. |
| `orion_task_duration_seconds` | Histogram | `workflow`, `task`, `function` | Per-task execution duration across all built-in and connector functions. |
| `orion_workflow_duration_seconds` | Histogram | `workflow` | Total workflow execution duration. |
| `orion_rate_limit_rejections_total` | Counter | `scope` | Requests rejected with `429` by channel name or route group. |
| `orion_rate_limit_key_unavailable_total` | Counter | `channel` | Rate-limit rejections where `key_logic` failed or evaluated to `null`/empty. |
| `orion_response_drops_total` | Counter | `channel`, `kind` | Shaped response cookies or headers dropped (`RESPONSE_COOKIE_DROPPED`, `RESPONSE_HEADER_DROPPED`, `RESPONSE_HEADER_NOT_ALLOWED`, `RESPONSE_COOKIES_DISABLED`). |
| `orion_response_cache_hits_total` | Counter | `channel` | Channel response-cache hits. |
| `orion_response_cache_misses_total` | Counter | `channel` | Channel response-cache misses. |
| `orion_response_cache_coalesced_total` | Counter | `channel` | Concurrent cache misses served from a single in-flight workflow run (`coalesce_misses: true`). |
| `orion_response_cache_invalidations_total` | Counter | `source` | Response-cache namespace invalidations by source (`workflow` or `admin`). |
| `orion_deployment_artifacts_collected_total` | Counter | `phase` | Unreferenced deployment artifacts processed by garbage collection (`draining` or `deleted`). |
| `orion_deployment_artifact_bytes_collected_total` | Counter | — | Total bytes reclaimed from deleted deployment artifacts. |
| `orion_job_last_success_timestamp_seconds` | Gauge | `job` | Unix timestamp of the last successful tick per background job (`trace_cleanup`, `audit_cleanup`, `dlq_retry`, `epoch_watcher`, `cron_reconcile`, `cron_cleanup`, `deployment_gc`, `v2_reconciler`). |

### WebAssembly plugin and native work metrics

| Name | Type | Labels | Description |
|------|------|--------|-------------|
| `orion_plugin_invocations_total` | Counter | `plugin`, `function`, `outcome` | Plugin function invocations (`ok` or `error`). |
| `orion_plugin_invocation_duration_seconds` | Histogram | `plugin`, `function` | Wall-clock duration of a plugin invocation. |
| `orion_plugin_queue_seconds` | Histogram | `plugin`, `function` | Time spent waiting for a per-function concurrency permit. |
| `orion_plugin_failures_total` | Counter | `plugin`, `function`, `category` | Failed plugin invocations by category (`caller_input`, `guest_error`, `fuel`, `memory`, `timeout`, `trap`, etc.). |
| `orion_plugin_loads_total` | Counter | `plugin`, `outcome` | Plugin WebAssembly component compilations (`ok` or `error`). |
| `orion_plugin_compile_duration_seconds` | Histogram | `plugin` | Cranelift compilation duration per component digest. |
| `orion_plugin_live_instances` | Gauge | — | Currently active WebAssembly instances across all plugins. |
| `orion_native_work_in_flight` | Gauge | — | Blocking native work jobs currently executing. |
| `orion_native_work_refusals_total` | Counter | `reason` | Native work jobs refused (`queue_deadline`, `caller_deadline`, `shutting_down`, `panicked`). |

### Cron scheduling metrics

| Name | Type | Labels | Description |
|------|------|--------|-------------|
| `orion_cron_occurrences_total` | Counter | `status` | Scheduled occurrences reaching `pending`, `completed`, `failed`, `skipped_misfire`, or `skipped_singleton`. |
| `orion_cron_execution_duration_seconds` | Histogram | `channel` | Execution duration of scheduled cron runs. |
| `orion_cron_schedule_lag_seconds` | Histogram | — | Delay (`started_at - scheduled_for`) before an occurrence begins running. |
| `orion_cron_pending_occurrences` | Gauge | — | Scheduled occurrences currently waiting for a worker. |
| `orion_cron_singleton_contention_total` | Counter | — | Occurrences skipped because their `concurrency.key` slot was already held. |
| `orion_cron_lease_renewal_failures_total` | Counter | — | Running cron occurrences canceled after losing their database lease. |
| `orion_cron_reconcile_duration_seconds` | Histogram | — | Duration of each cron scheduler reconciliation pass. |

### Trace pipeline and audit trail metrics

| Name | Type | Labels | Description |
|------|------|--------|-------------|
| `orion_trace_queue_depth` | Gauge | — | Queued `/async` submissions waiting for a worker. |
| `orion_trace_queue_memory_bytes` | Gauge | — | Total memory held by queued `/async` payloads. |
| `orion_trace_workers_active` | Gauge | — | Async trace workers currently executing a workflow. |
| `orion_trace_workers_total` | Gauge | — | Configured async trace worker pool size (`trace_queue.workers`). |
| `orion_trace_queue_rejected_total` | Counter | `reason` | `/async` submissions shed with `503` (`full` or `memory`). |
| `orion_trace_dropped_total` | Counter | `reason` | Synchronous traces not persisted (`overflow`, `sampled_out`, `errors_only`, `off`). |
| `orion_trace_persistence_queue_depth` | Gauge | — | Completed traces waiting to be flushed in `async` or `batch` storage mode. |
| `orion_trace_persistence_batch_size` | Histogram | — | Trace rows committed per batch transaction. |
| `orion_trace_persistence_failures_total` | Counter | — | Trace writes that failed to persist to the database. |
| `orion_trace_dlq_depth` | Gauge | — | Rows currently in `trace_dlq` (refreshed when `trace_queue.dlq_retry_enabled = true`). |
| `orion_trace_dlq_retries_total` | Counter | `outcome` | DLQ retry outcomes (`retried`, `exhausted`, `failed`). |
| `orion_admin_audit_events_total` | Counter | `action`, `resource_type` | Admin mutations recorded in `audit_logs`. |
| `orion_audit_queue_depth` | Gauge | — | Pending audit log rows waiting to be written. |
| `orion_audit_events_dropped_total` | Counter | `reason` | Audit log events dropped (`queue_full`, `write_failed`, `drain_timeout`, `writer_stopped`). |

## Next steps

- [Monitoring, metrics, and alerts](../operate/monitoring.md): Production alert rules and latency attribution queries.
- [Backups, upgrades, and troubleshooting](../operate/maintenance.md): Symptom-based troubleshooting for common error codes.
