<!-- description: Complete reference for Orion server configuration (orion.toml), ORION_SECTION__KEY environment variables, secret locators, and setting resolution order. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# Server configuration and environment variables

Orion resolves server configuration across three layers in increasing order of precedence:
1. **Struct defaults** (documented in the tables on this page and verified against `orion-common` by `config_docs_drift_test`).
2. **The TOML configuration file** passed via `orion-server -c config.toml` (supports `${VAR}`, `${VAR:-default}`, and `${VAR:?message}` shell-style placeholders before parsing, plus whole-value `env://VAR` references after parsing).
3. **`ORION_SECTION__KEY` environment variables**, using a double underscore (`__`) between section levels (for example, `ORION_SERVER__PORT` or `ORION_ENGINE__CIRCUIT_BREAKER__ENABLED`). List-valued settings accept comma-separated values.

Run `orion-server validate-config -c config.toml` to inspect the merged configuration with secrets masked.

---

## Startup validation and typo detection

Unknown keys in `config.toml` and unrecognized `ORION_*` environment variables that match the `ORION_SECTION__KEY` override grammar fail startup immediately with a suggestion for the closest valid setting:

```text
Error: Configuration error: these ORION_* environment variables are not Orion
settings and would be silently ignored:
  ORION_SERVER__PORTT (did you mean ORION_SERVER__PORT?)
```

- **Top-level environment check**: `ORION_ENVIRONMENT` is the only top-level override without a `__` separator; near-misses such as `ORION_ENVIRONMEN` are also caught and refused at startup.
- **Single-underscore service links**: Variables without `__` (such as Kubernetes service links like `ORION_SERVICE_HOST` or `ORION_SERVER_PORT`, and CLI variables like `ORION_SERVER_URL` and `ORION_API_KEY`) are not treated as config overrides. Always use double underscores (`__`) between section and key names.
- **Custom secret variables**: Prefix custom environment variables that happen to contain `__` with `ORION_SECRET_*` so the startup config scanner leaves them alone.

---

## Where secret references (`env://`, `vault://`, `var://`) resolve

| Surface | `${VAR}` placeholder | `env://VAR` | `vault://path#field` | `var://name` | `{"secret": "name"}` |
|---|---|---|---|---|---|
| `orion.toml` (outside `[vars]` / `[secrets]`) | Yes (at boot) | Yes (at boot) | No (use `[secrets]`) | No | No |
| `orion.toml` `[vars]` | Yes (at boot) | No (values are public literals) | No | No | No |
| `orion.toml` `[secrets]` | Yes (at boot) | Yes (at boot) | Yes (at boot) | No | No |
| Connector `config` (`secret_locators`) | No | Yes (at connection load) | Yes (at connection load) | Yes (in `public_config`) | No |
| Channel `config.auth` & `oauth2_login` | No | Yes (at channel load) | Yes (at channel load) | Yes (in `oauth2_login`) | No |
| Workflow expressions & task inputs | No | Only in `crypto.key`, `jwt_sign.key`, and `jwt_verify` (`keys[].key`, `issuer`, `audience`) | Same 5 fields | Via `{"var": "metadata.vars.<name>"}` | Yes (any JSONLogic expression that is not recorded in `data` or `log`) |

---

## Core settings (`environment`, `[server]`, `[storage]`, `[cluster]`)

### Deployment environment

| Setting | Default | Env var | When to change |
|---|---|---|---|
| `environment` | `"development"` | `ORION_ENVIRONMENT` | Set to `"production"` before exposing an instance to external traffic. Enforces `admin_auth.enabled`, strong admin keys, non-wildcard CORS, `server.verbose_errors = false`, and `storage.auto_migrate = false` in cluster mode. |

### `[server]`

| Setting | Default | Env var | When to change |
|---|---|---|---|
| `server.host` | `"127.0.0.1"` | `ORION_SERVER__HOST` | Set to `"0.0.0.0"` in containers to accept connections from other hosts. |
| `server.port` | `8080` | `ORION_SERVER__PORT` | HTTP listener port. |
| `server.shutdown_drain_secs` | `30` | `ORION_SERVER__SHUTDOWN_DRAIN_SECS` | Time to keep accepting requests after `/readyz` flips to `503` on `SIGTERM`. |
| `server.shutdown_force_timeout_secs` | `30` | `ORION_SERVER__SHUTDOWN_FORCE_TIMEOUT_SECS` | Maximum wait for in-flight requests after drain (`0` waits indefinitely). |
| `server.max_admin_body_size` | `8388608` | `ORION_SERVER__MAX_ADMIN_BODY_SIZE` | Maximum request body size (`8 MiB`) on `/api/v2/admin/*`. |
| `server.data_mounts` | `[]` | `ORION_SERVER__DATA_MOUNTS` | Extra path prefixes where the data plane is mounted alongside `/api/v2/data`. |
| `server.hsts` | `false` | `ORION_SERVER__HSTS` | Emit `Strict-Transport-Security` even when TLS terminates at an upstream proxy. |
| `server.verbose_errors` | — | `ORION_SERVER__VERBOSE_ERRORS` | Unset returns raw task error messages outside production and sanitizes them in production. Explicit `true` is refused in production. |
| `server.tls.enabled` | `false` | `ORION_SERVER__TLS__ENABLED` | Enable native TLS termination on the HTTP listener. |
| `server.tls.cert_path` | `""` | `ORION_SERVER__TLS__CERT_PATH` | Path to PEM certificate chain (required when `server.tls.enabled = true`). |
| `server.tls.key_path` | `""` | `ORION_SERVER__TLS__KEY_PATH` | Path to PEM private key (required when `server.tls.enabled = true`). |
| `server.compression.enabled` | `false` | `ORION_SERVER__COMPRESSION__ENABLED` | Enable HTTP response compression. |
| `server.docs.enabled` | — | `ORION_SERVER__DOCS__ENABLED` | Unset serves `/docs` and `/api/v2/openapi.json` outside production only. Set `false` to disable everywhere. |

### `[storage]`

| Setting | Default | Env var | When to change |
|---|---|---|---|
| `storage.url` | `"sqlite:orion.db"` | `ORION_STORAGE__URL` | State database URL (`sqlite:`, `postgres://`, or `mysql://`). |
| `storage.max_connections` | `50` | `ORION_STORAGE__MAX_CONNECTIONS` | Maximum connections in the state database pool. |
| `storage.min_connections` | `5` | `ORION_STORAGE__MIN_CONNECTIONS` | Minimum warm connections held in the pool. |
| `storage.busy_timeout_ms` | `5000` | `ORION_STORAGE__BUSY_TIMEOUT_MS` | SQLite busy wait timeout in milliseconds. |
| `storage.acquire_timeout_secs` | `3` | `ORION_STORAGE__ACQUIRE_TIMEOUT_SECS` | Maximum wait for a free connection from the pool. |
| `storage.idle_timeout_secs` | `300` | `ORION_STORAGE__IDLE_TIMEOUT_SECS` | Idle connection lifetime before closing (`0` disables). |
| `storage.connector_encryption_key` | `""` | `ORION_STORAGE__CONNECTOR_ENCRYPTION_KEY` | 64-char hex key (`openssl rand -hex 32`) encrypting `connector_oauth_state` tokens at rest with AES-256-GCM. |
| `storage.backup_dir` | `"./backups"` | `ORION_STORAGE__BACKUP_DIR` | Target directory for `POST /api/v2/admin/backups` (SQLite only). |
| `storage.backup_retention_count` | — | `ORION_STORAGE__BACKUP_RETENTION_COUNT` | Number of newest SQLite backups to retain (unset keeps all backups). |
| `storage.auto_migrate` | `true` | `ORION_STORAGE__AUTO_MIGRATE` | Set `false` in multi-replica clusters and run `orion-server migrate` as a release step. |
| `storage.connect_retry_secs` | `60` | `ORION_STORAGE__CONNECT_RETRY_SECS` | Startup retry window for an unreachable PostgreSQL/MySQL database (`0` fails fast). |
| `storage.artifact_gc_interval_secs` | `3600` | `ORION_STORAGE__ARTIFACT_GC_INTERVAL_SECS` | Sweep interval for unreferenced deployment artifacts (`0` disables). |
| `storage.artifact_gc_grace_secs` | `86400` | `ORION_STORAGE__ARTIFACT_GC_GRACE_SECS` | Minimum age in seconds before an unreferenced artifact can be collected. |
| `storage.deployment_retain_revisions` | `20` | `ORION_STORAGE__DEPLOYMENT_RETAIN_REVISIONS` | Number of newest deployment revisions retained uncompacted (`>= 1`). |

### `[cluster]` <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

| Setting | Default | Env var | When to change |
|---|---|---|---|
| `cluster.enabled` | `false` | `ORION_CLUSTER__ENABLED` | Enable multi-replica coordination (requires PostgreSQL/MySQL and `[extensions.redis]` on `orion-ext`). |
| `cluster.epoch_poll_interval_ms` | `2000` | `ORION_CLUSTER__EPOCH_POLL_INTERVAL_MS` | Poll interval for deployment head and circuit-breaker epoch changes. |
| `cluster.instance_id` | `""` | `ORION_CLUSTER__INSTANCE_ID` | Stable replica identifier (`<= 64` chars, such as pod `HOSTNAME`). |

---

## Runtime settings (`[engine]`, `[ingest]`, `[trace_queue]`, `[trace_storage]`, `[audit]`, `[query]`, `[write]`, `[cron]`)

### `[engine]`

| Setting | Default | Env var | When to change |
|---|---|---|---|
| `engine.health_check_timeout_secs` | `2` | `ORION_ENGINE__HEALTH_CHECK_TIMEOUT_SECS` | Per-component ping timeout for `/health` and `/readyz`. |
| `engine.max_channel_call_depth` | `10` | `ORION_ENGINE__MAX_CHANNEL_CALL_DEPTH` | Maximum nested `channel_call` depth. |
| `engine.default_channel_call_timeout_ms` | `30000` | `ORION_ENGINE__DEFAULT_CHANNEL_CALL_TIMEOUT_MS` | Default timeout for `channel_call` tasks when not set on the task or channel. |
| `engine.max_loop_iterations` | `10000` | `ORION_ENGINE__MAX_LOOP_ITERATIONS` | Maximum `loop.max` value permitted on a workflow (`0` removes the ceiling). |
| `engine.global_http_timeout_secs` | `30` | `ORION_ENGINE__GLOBAL_HTTP_TIMEOUT_SECS` | Fallback timeout for outbound HTTP requests. |
| `engine.max_pool_cache_entries` | `100` | `ORION_ENGINE__MAX_POOL_CACHE_ENTRIES` | Maximum cached connector pools before LRU eviction. |
| `engine.sqlite_connector_root` | `"./data/connectors/sqlite"` | `ORION_ENGINE__SQLITE_CONNECTOR_ROOT` | Filesystem jail directory for application SQLite `db` connectors. |
| `engine.cache_cleanup_interval_secs` | `60` | `ORION_ENGINE__CACHE_CLEANUP_INTERVAL_SECS` | Sweep interval for expired in-memory cache entries. |
| `engine.max_memory_cache_entries` | `100000` | `ORION_ENGINE__MAX_MEMORY_CACHE_ENTRIES` | Maximum entries per in-memory cache partition (`0` removes the bound). |
| `engine.rollout_sticky_header` | `""` | `ORION_ENGINE__ROLLOUT_STICKY_HEADER` | Optional custom request header name used for sticky canary routing. |
| `engine.fail_on_connector_load_error` | `false` | `ORION_ENGINE__FAIL_ON_CONNECTOR_LOAD_ERROR` | Set `true` in production to fail startup if any enabled connector fails to initialize. |
| `engine.ops_budget` | `0` | `ORION_ENGINE__OPS_BUDGET` | Maximum operations per single JSONLogic evaluation (`0` = unlimited). |
| `engine.max_run_tasks` | `0` | `ORION_ENGINE__MAX_RUN_TASKS` | Maximum task dispatches across one workflow run (`0` = unlimited). |
| `engine.max_run_expr_ops` | `0` | `ORION_ENGINE__MAX_RUN_EXPR_OPS` | Maximum total JSONLogic operations across one workflow run (`0` = unlimited). |
| `engine.max_run_custom_units` | `0` | `ORION_ENGINE__MAX_RUN_CUSTOM_UNITS` | Maximum custom host units charged across one run (`0` = unlimited). |
| `engine.max_run_in_flight` | `0` | `ORION_ENGINE__MAX_RUN_IN_FLIGHT` | Maximum concurrent tasks within one run (`0` = unlimited). |
| `engine.native_work_max_in_flight` | `4` | `ORION_ENGINE__NATIVE_WORK_MAX_IN_FLIGHT` | Maximum concurrent blocking native jobs (such as Wasm compilations). |
| `engine.native_work_timeout_ms` | `300000` | `ORION_ENGINE__NATIVE_WORK_TIMEOUT_MS` | Timeout in milliseconds when waiting for a native work job. |
| `engine.connector_budget.max_attempts` | `32` | `ORION_ENGINE__CONNECTOR_BUDGET__MAX_ATTEMPTS` | Maximum total connector attempts across a single run. |
| `engine.connector_budget.max_tasks` | `256` | `ORION_ENGINE__CONNECTOR_BUDGET__MAX_TASKS` | Maximum tasks charged by connector work across a run. |
| `engine.connector_budget.max_network_sends` | `64` | `ORION_ENGINE__CONNECTOR_BUDGET__MAX_NETWORK_SENDS` | Maximum network sends charged by connector work across a run. |
| `engine.connector_budget.max_bytes` | `16777216` | `ORION_ENGINE__CONNECTOR_BUDGET__MAX_BYTES` | Maximum bytes (`16 MiB`) charged by connector work across a run. |
| `engine.connector_budget.max_active_jobs` | `16` | `ORION_ENGINE__CONNECTOR_BUDGET__MAX_ACTIVE_JOBS` | Maximum concurrent connector jobs within a run. |
| `engine.connector_budget.max_depth` | `8` | `ORION_ENGINE__CONNECTOR_BUDGET__MAX_DEPTH` | Maximum nested provider dependency depth. |
| `engine.circuit_breaker.enabled` | `false` | `ORION_ENGINE__CIRCUIT_BREAKER__ENABLED` | Enable per-`(channel, connector)` circuit breakers. |
| `engine.circuit_breaker.failure_threshold` | `5` | `ORION_ENGINE__CIRCUIT_BREAKER__FAILURE_THRESHOLD` | Consecutive retryable failures before opening a circuit breaker. |
| `engine.circuit_breaker.recovery_timeout_secs` | `30` | `ORION_ENGINE__CIRCUIT_BREAKER__RECOVERY_TIMEOUT_SECS` | Cool-down window in seconds before probing a tripped breaker in `half_open`. |
| `engine.circuit_breaker.max_breakers` | `10000` | `ORION_ENGINE__CIRCUIT_BREAKER__MAX_BREAKERS` | Maximum tracked `(channel, connector)` circuit breaker entries. |

### `[ingest]`

| Setting | Default | Env var | When to change |
|---|---|---|---|
| `ingest.max_payload_size` | `1048576` | `ORION_INGEST__MAX_PAYLOAD_SIZE` | Maximum data-plane HTTP request body size in bytes (`1 MiB`). |

### `[trace_queue]` and `[trace_storage]`

| Setting | Default | Env var | When to change |
|---|---|---|---|
| `trace_queue.workers` | `4` | `ORION_TRACE_QUEUE__WORKERS` | Concurrent worker count for `/async` channel executions. |
| `trace_queue.buffer_size` | `1000` | `ORION_TRACE_QUEUE__BUFFER_SIZE` | Maximum queued `/async` submissions before returning `503`. |
| `trace_queue.shutdown_timeout_secs` | `30` | `ORION_TRACE_QUEUE__SHUTDOWN_TIMEOUT_SECS` | Shutdown drain timeout for in-flight `/async` traces. |
| `trace_queue.retention_hours` | `72` | `ORION_TRACE_QUEUE__RETENTION_HOURS` | Trace retention window in hours (`0` keeps traces forever). |
| `trace_queue.cleanup_interval_secs` | `3600` | `ORION_TRACE_QUEUE__CLEANUP_INTERVAL_SECS` | Background trace retention sweep interval in seconds. |
| `trace_queue.processing_timeout_ms` | `60000` | `ORION_TRACE_QUEUE__PROCESSING_TIMEOUT_MS` | Per-trace execution timeout ceiling on `/async`. |
| `trace_queue.max_result_size_bytes` | `1048576` | `ORION_TRACE_QUEUE__MAX_RESULT_SIZE_BYTES` | Maximum workflow result size in bytes (`1 MiB`). |
| `trace_queue.max_queue_memory_bytes` | `104857600` | `ORION_TRACE_QUEUE__MAX_QUEUE_MEMORY_BYTES` | Maximum total memory (`100 MiB`) held by queued `/async` payloads before returning `503`. |
| `trace_queue.dlq_retry_enabled` | `true` | `ORION_TRACE_QUEUE__DLQ_RETRY_ENABLED` | Enable automatic background retries for failed `/async` runs in `trace_dlq`. |
| `trace_queue.dlq_max_retries` | `5` | `ORION_TRACE_QUEUE__DLQ_MAX_RETRIES` | Maximum DLQ retry attempts (`1` to `16`, exponential backoff `2^retries` seconds). |
| `trace_queue.dlq_poll_interval_secs` | `30` | `ORION_TRACE_QUEUE__DLQ_POLL_INTERVAL_SECS` | Poll interval for the DLQ retry worker. |
| `trace_queue.dlq_batch_size` | `20` | `ORION_TRACE_QUEUE__DLQ_BATCH_SIZE` | Number of DLQ rows claimed per retry tick. |
| `trace_queue.dlq_lease_secs` | `60` | `ORION_TRACE_QUEUE__DLQ_LEASE_SECS` | Lease duration in seconds for a claimed DLQ row. |
| `trace_storage.mode` | `"sync"` | `ORION_TRACE_STORAGE__MODE` | Default synchronous trace persistence mode: `"sync"`, `"async"`, `"batch"`, or `"off"`. |
| `trace_storage.sample_rate` | `1.0` | `ORION_TRACE_STORAGE__SAMPLE_RATE` | Fraction of synchronous traces persisted (`0.0` to `1.0`). |
| `trace_storage.errors_only` | `false` | `ORION_TRACE_STORAGE__ERRORS_ONLY` | Persist traces only when the workflow run ends with an error. |
| `trace_storage.max_pending` | `10000` | `ORION_TRACE_STORAGE__MAX_PENDING` | Pending write queue capacity in `async` and `batch` modes. |
| `trace_storage.async_on_overflow` | `"drop"` | `ORION_TRACE_STORAGE__ASYNC_ON_OVERFLOW` | Queue overflow policy in `async` and `batch` modes: `"drop"` or `"block"`. |
| `trace_storage.overflow_block_timeout_ms` | `100` | `ORION_TRACE_STORAGE__OVERFLOW_BLOCK_TIMEOUT_MS` | Maximum time `"block"` waits for queue space before dropping. |
| `trace_storage.async_workers` | `4` | `ORION_TRACE_STORAGE__ASYNC_WORKERS` | Persistence worker count in `"async"` mode. |
| `trace_storage.batch_size` | `1000` | `ORION_TRACE_STORAGE__BATCH_SIZE` | Maximum rows committed per transaction in `"batch"` mode (`<= 1000`). |
| `trace_storage.batch_flush_interval_ms` | `100` | `ORION_TRACE_STORAGE__BATCH_FLUSH_INTERVAL_MS` | Maximum wait before flushing a partial batch in `"batch"` mode. |
| `trace_storage.batch_workers` | `4` | `ORION_TRACE_STORAGE__BATCH_WORKERS` | Persistence worker count in `"batch"` mode. |

### `[audit]`

| Setting | Default | Env var | When to change |
|---|---|---|---|
| `audit.retention_days` | `90` | `ORION_AUDIT__RETENTION_DAYS` | Audit log retention window in days (`0` keeps audit rows forever). |
| `audit.cleanup_interval_secs` | `3600` | `ORION_AUDIT__CLEANUP_INTERVAL_SECS` | Background audit log cleanup interval in seconds. |
| `audit.max_pending` | `1000` | `ORION_AUDIT__MAX_PENDING` | Capacity of the async audit writer queue. |
| `audit.drain_timeout_secs` | `5` | `ORION_AUDIT__DRAIN_TIMEOUT_SECS` | Shutdown drain timeout in seconds for pending audit rows (`> 0`). |

### `[query]` and `[write]`

| Setting | Default | Env var | When to change |
|---|---|---|---|
| `query.default_limit` | `100` | `ORION_QUERY__DEFAULT_LIMIT` | Default page size when `data_query` omits `limit`. |
| `query.max_limit` | `1000` | `ORION_QUERY__MAX_LIMIT` | Hard cap on `data_query` `limit`, `data_write` `returning` rows, and `db_read` row count (`>= default_limit`). |
| `query.max_skip` | `10000` | `ORION_QUERY__MAX_SKIP` | Hard cap on `data_query` `skip` offset. |
| `query.max_result_bytes` | `16777216` | `ORION_QUERY__MAX_RESULT_BYTES` | Maximum decoded byte size (`16 MiB`) of a single `db_read` or `data_query` result (`0` disables). |
| `write.max_rows` | `1000` | `ORION_WRITE__MAX_ROWS` | Maximum rows in a single bulk `data_write` `insert` or `upsert`. |
| `write.allow_unfiltered` | `false` | `ORION_WRITE__ALLOW_UNFILTERED` | Allow unfiltered `data_write` `update`/`delete` calls that also set `"all": true`. |

### `[cron]`

| Setting | Default | Env var | When to change |
|---|---|---|---|
| `cron.enabled` | `true` | `ORION_CRON__ENABLED` | Enable the cron scheduler on this node. |
| `cron.poll_interval_ms` | `1000` | `ORION_CRON__POLL_INTERVAL_MS` | Scheduler poll interval in milliseconds (`>= 100`). |
| `cron.workers` | `4` | `ORION_CRON__WORKERS` | Maximum concurrent cron occurrences executed on this node. |
| `cron.claim_batch_size` | `20` | `ORION_CRON__CLAIM_BATCH_SIZE` | Maximum due occurrences claimed per poll tick. |
| `cron.claim_lease_secs` | `60` | `ORION_CRON__CLAIM_LEASE_SECS` | Database lease duration for a running cron occurrence. |
| `cron.heartbeat_interval_secs` | `15` | `ORION_CRON__HEARTBEAT_INTERVAL_SECS` | Lease renewal heartbeat interval (must be less than `cron.claim_lease_secs`). |
| `cron.misfire_grace_secs` | `5` | `ORION_CRON__MISFIRE_GRACE_SECS` | Grace window in seconds before a late tick is classified as a misfire. |
| `cron.max_catch_up` | `100` | `ORION_CRON__MAX_CATCH_UP` | Server-wide ceiling on `catch_up` replay occurrences (`<= 1000`). |
| `cron.default_timeout_ms` | `3600000` | `ORION_CRON__DEFAULT_TIMEOUT_MS` | Default execution timeout (`1 hour`) when a `cron` channel omits `timeout_ms`. |
| `cron.shutdown_timeout_secs` | `30` | `ORION_CRON__SHUTDOWN_TIMEOUT_SECS` | Shutdown wait for in-flight cron occurrences. |

---

## Plugins, security, and observability settings

### `[plugins]`

| Setting | Default | Env var | When to change |
|---|---|---|---|
| `plugins.enabled` | `false` | `ORION_PLUGINS__ENABLED` | Enable loading and executing WebAssembly plugins on this node. |
| `plugins.cache_dir` | `""` | `ORION_PLUGINS__CACHE_DIR` | Reserved for on-disk compilation cache (must be `""`). |
| `plugins.max_component_bytes` | `16777216` | `ORION_PLUGINS__MAX_COMPONENT_BYTES` | Maximum uploaded `.wasm` component size (`16 MiB`). |
| `plugins.max_memory_bytes` | `67108864` | `ORION_PLUGINS__MAX_MEMORY_BYTES` | Maximum linear memory per plugin instance (`64 MiB`). |
| `plugins.max_request_bytes` | `1048576` | `ORION_PLUGINS__MAX_REQUEST_BYTES` | Maximum serialized `function.input` passed to a plugin (`1 MiB`). |
| `plugins.max_response_bytes` | `1048576` | `ORION_PLUGINS__MAX_RESPONSE_BYTES` | Maximum JSON output returned by a plugin (`1 MiB`). |
| `plugins.max_timeout_ms` | `5000` | `ORION_PLUGINS__MAX_TIMEOUT_MS` | Wall-clock execution ceiling per plugin invocation. |
| `plugins.max_concurrency_per_function` | `64` | `ORION_PLUGINS__MAX_CONCURRENCY_PER_FUNCTION` | Maximum concurrent invocations per plugin function. |
| `plugins.max_live_instances` | `256` | `ORION_PLUGINS__MAX_LIVE_INSTANCES` | Pooled instance capacity across all plugins (`>= max_concurrency_per_function`). |
| `plugins.linear_memory_keep_resident_bytes` | `1048576` | `ORION_PLUGINS__LINEAR_MEMORY_KEEP_RESIDENT_BYTES` | Linear memory kept resident between pooled invocations on Linux (`<= max_memory_bytes`). |
| `plugins.table_keep_resident_bytes` | `65536` | `ORION_PLUGINS__TABLE_KEEP_RESIDENT_BYTES` | Table memory kept resident between pooled invocations. |
| `plugins.fuel_backstop` | `100000000000` | `ORION_PLUGINS__FUEL_BACKSTOP` | Wasmtime instruction fuel backstop per invocation. |
| `plugins.trust.public_keys` | `[]` | `ORION_PLUGINS__TRUST__PUBLIC_KEYS` | Base64 Ed25519 public keys required to verify plugin digest signatures. |
| `plugins.overrides` | `[]` | — | Array of `[[plugins.overrides]]` tables (`id`, `timeout_ms`, `max_memory_bytes`, `max_concurrency`, `max_request_bytes`, `max_response_bytes`) narrowing ceilings per plugin. |

### `[admin_auth]`, `[jwt]`, `[oauth2_login]`, `[cors]`, `[rate_limit]`, and `[channel_filter]`

| Setting | Default | Env var | When to change |
|---|---|---|---|
| `admin_auth.enabled` | `false` | `ORION_ADMIN_AUTH__ENABLED` | Require API key authentication on `/api/v2/admin/*` (required in production). |
| `admin_auth.api_keys` | `[]` | `ORION_ADMIN_AUTH__API_KEYS` | Accepted full-access admin API keys or `sha256:<64-hex>` digests. |
| `admin_auth.read_only_api_keys` | `[]` | `ORION_ADMIN_AUTH__READ_ONLY_API_KEYS` | Read-only admin API keys (`GET`/`HEAD` and dry-run plan/lint routes only). |
| `admin_auth.header` | `"Authorization"` | `ORION_ADMIN_AUTH__HEADER` | Header carrying the admin key (`"Authorization"` expects `Bearer <key>`). |
| `jwt.allow_private_jwks_urls` | `false` | `ORION_JWT__ALLOW_PRIVATE_JWKS_URLS` | Allow fetching JWKS documents from private or internal IP addresses. |
| `oauth2_login.allow_private_token_urls` | `false` | `ORION_OAUTH2_LOGIN__ALLOW_PRIVATE_TOKEN_URLS` | Allow OAuth2 token and userinfo fetches to private or internal IP addresses. |
| `cors.allowed_origins` | `["*"]` | `ORION_CORS__ALLOWED_ORIGINS` | Allowed browser origins (`"*"` is refused in production and when `allow_credentials = true`). |
| `cors.additional_allowed_headers` | `[]` | `ORION_CORS__ADDITIONAL_ALLOWED_HEADERS` | Extra request headers permitted in CORS preflights. |
| `cors.additional_exposed_headers` | `[]` | `ORION_CORS__ADDITIONAL_EXPOSED_HEADERS` | Extra response headers exposed to browser scripts. |
| `cors.allow_credentials` | `false` | `ORION_CORS__ALLOW_CREDENTIALS` | Emit `Access-Control-Allow-Credentials: true` (requires explicit `allowed_origins`). |
| `cors.max_age_secs` | — | `ORION_CORS__MAX_AGE_SECS` | `Access-Control-Max-Age` in seconds (`<= 86400`; unset omits the header). |
| `rate_limit.enabled` | `false` | `ORION_RATE_LIMIT__ENABLED` | Enable the platform-wide per-IP rate limiter. |
| `rate_limit.default_rps` | `100` | `ORION_RATE_LIMIT__DEFAULT_RPS` | Default sustained requests per second per client IP. |
| `rate_limit.default_burst` | `50` | `ORION_RATE_LIMIT__DEFAULT_BURST` | Burst capacity above `default_rps`. |
| `rate_limit.trusted_proxies` | `[]` | `ORION_RATE_LIMIT__TRUSTED_PROXIES` | Proxy/load-balancer CIDRs trusted to supply `X-Forwarded-For` / `Forwarded`. |
| `rate_limit.endpoints.admin_rps` | `20` | `ORION_RATE_LIMIT__ENDPOINTS__ADMIN_RPS` | Per-IP rate limit for `/api/v2/admin/*`. |
| `rate_limit.endpoints.data_rps` | — | `ORION_RATE_LIMIT__ENDPOINTS__DATA_RPS` | Optional separate per-IP rate limit for `/api/v2/data/*`. |
| `channel_filter.include` | `[]` | `ORION_CHANNEL_FILTER__INCLUDE` | Glob patterns of channels to load on this node (empty loads all channels). |
| `channel_filter.exclude` | `[]` | `ORION_CHANNEL_FILTER__EXCLUDE` | Glob patterns of channels to exclude after `include`. |

### `[logging]`, `[metrics]`, and `[tracing]`

| Setting | Default | Env var | When to change |
|---|---|---|---|
| `logging.level` | `"info"` | `ORION_LOGGING__LEVEL` | Log verbosity: `"trace"`, `"debug"`, `"info"`, `"warn"`, or `"error"`. |
| `logging.format` | `"pretty"` | `ORION_LOGGING__FORMAT` | `"json"` for production log aggregators; `"pretty"` for local terminals. |
| `logging.access_log` | `true` | `ORION_LOGGING__ACCESS_LOG` | Emit one `INFO` access log line per HTTP request (requires `metrics.enabled = true`). |
| `metrics.enabled` | `false` | `ORION_METRICS__ENABLED` | Enable Prometheus metrics collection and `GET /metrics`. |
| `metrics.bind_addr` | — | `ORION_METRICS__BIND_ADDR` | Dedicated `host:port` listener serving unauthenticated `GET /metrics`. |
| `tracing.enabled` | `false` | `ORION_TRACING__ENABLED` | Enable OpenTelemetry OTLP gRPC span export. |
| `tracing.otlp_endpoint` | `"http://localhost:4317"` | `ORION_TRACING__OTLP_ENDPOINT` | OTLP gRPC collector endpoint URL. |
| `tracing.service_name` | `"orion"` | `ORION_TRACING__SERVICE_NAME` | OpenTelemetry `service.name` resource attribute. |
| `tracing.sample_rate` | `1.0` | `ORION_TRACING__SAMPLE_RATE` | OTLP span sampling ratio (`0.0` to `1.0`). |
| `tracing.debug_profile_enabled` | `false` | `ORION_TRACING__DEBUG_PROFILE_ENABLED` | Allow per-request `_orion.profile` timing breakdowns. |

## Next steps

- [Production checklist](../operate/production-checklist.md): Recommended production settings across all sections.
- [Multi-node clustering and HA](../enterprise/clustering.md): Reference for `[extensions.licence]` and `[extensions.redis]` on `orion-ext`.
