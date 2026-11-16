<!-- description: Pre-production checklist for Orion: harden admin authentication, TLS, CORS, trusted proxies, secret locators, database pools, alerts, and backups. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Production checklist

Orion's default configuration is tuned for local development on a laptop: admin authentication is disabled, TLS is off, CORS allows all origins, and SQLite writes to a local file. Before exposing an Orion instance to real traffic, work through the checklist below.

## What `ORION_ENVIRONMENT=production` enforces automatically

Setting `environment = "production"` (or `ORION_ENVIRONMENT=production`) turns five unsafe defaults into immediate startup errors:
1. `admin_auth.enabled = false` or an empty `admin_auth.api_keys` list.
2. An admin API key shorter than 32 characters (when not hashed as `sha256:<digest>`).
3. A wildcard `cors.allowed_origins = ["*"]`.
4. `server.verbose_errors = true` (which would expose internal error details to callers).
5. `cluster.enabled = true` combined with `storage.auto_migrate = true` (which would cause booting replicas to race database migrations).

Every other item in the pre-launch checklist—including TLS termination, per-channel data-plane authentication, trusted proxy CIDRs, and retention bounds—remains your responsibility to configure explicitly.

---

## Pre-launch checklist

| Area | Production requirement | Guide |
|---|---|---|
| **Environment** | Set `ORION_ENVIRONMENT=production` so missing admin keys and wildcard CORS fail closed at startup. | [Server configuration](../reference/configuration.md) |
| **Admin authentication** | Set `admin_auth.enabled = true` with SHA-256 key digests (`sha256:<hex>`). Configure at least two keys for zero-downtime key rotation, and issue `read_only_api_keys` for dashboards and CI `plan`/`lint` jobs. | [Security and access control](./security.md#1-authenticate-the-admin-plane) |
| **Data-plane authentication** | Configure an `auth` guard (`api_key`, `hmac`, or `jwt`) on every public channel, or authenticate requests at an upstream API gateway. `admin_auth` never protects `/api/v2/data/**`. | [Security and access control](./security.md#2-authenticate-data-plane-channels) |
| **TLS** | Terminate TLS either at your load balancer/Ingress or directly on Orion (`[server.tls]`). | [Security and access control](./security.md#3-terminate-tls) |
| **CORS** | Replace `["*"]` with explicit browser origins (`cors.allowed_origins`), or leave the list empty if no browser calls Orion directly. | [Security and access control](./security.md#7-configure-cors-and-origin-allowlists) |
| **Trusted proxies** | Set `rate_limit.trusted_proxies` to your load balancer or pod CIDRs so rate limiting keys on client IPs rather than collapsing all traffic into the proxy's IP. | [Security and access control](./security.md#4-trust-reverse-proxy-cidrs) |
| **Secrets & encryption** | Reference all connector and channel secrets with `env://` or `vault://` locators. Set `storage.connector_encryption_key` to encrypt persisted OAuth2 tokens at rest with AES-256-GCM. | [Security and access control](./security.md#5-keep-secrets-out-of-definitions-and-databases) |
| **OpenAPI & Swagger UI** | Keep `server.docs.enabled = false` in production so `/docs` and `/api/v2/openapi.json` are not exposed anonymously. | [Security and access control](./security.md#9-close-unused-surfaces) |
| **Database backend** | Use PostgreSQL or MySQL for multi-replica deployments, or mount a persistent volume for single-node SQLite. Size `storage.max_connections` so `max_connections × replicas` stays within your database limit. | [Deploy with Docker and Kubernetes](./deploy.md) |
| **Cluster & extensions** | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> For multi-replica deployments, set `cluster.enabled = true` and `storage.auto_migrate = false`, mount `[extensions.licence]` and `[extensions.redis]` via config file, and run `orion-server migrate` as a release step. | [Multi-node clustering and HA](../enterprise/clustering.md) |
| **Rate limits & circuit breakers** | Enable `[rate_limit]` per channel and set `engine.circuit_breaker.enabled = true` when workflows call external services. | [Timeouts, retries, and circuit breakers](./resilience.md) |
| **Table retention** | Bound `trace_queue.retention_hours` and `audit.retention_days` so the `traces` and `audit_logs` tables do not grow without bound. | [Execution traces, DLQ, and audit logs](./traces-and-audit.md) |
| **Logs, metrics, & alerts** | Set `logging.format = "json"`, expose Prometheus metrics on an internal `metrics.bind_addr`, and configure alerts for background job staleness, dropped audit rows, and DLQ growth. | [Monitoring, metrics, and alerts](./monitoring.md) |
| **WebAssembly plugins** | Leave `plugins.enabled = false` unless your deployment uses plugins. When enabled, configure Ed25519 signing keys in `[plugins.trust]` and account for the 16 GiB virtual address space reservation in container memory limits. | [Security and access control](./security.md#8-sandbox-and-sign-webassembly-plugins) |
| **Graceful shutdown** | Ensure your container orchestrator's kill grace period (`stop_grace_period` or `terminationGracePeriodSeconds`) exceeds `server.shutdown_drain_secs + server.shutdown_force_timeout_secs`. | [Timeouts, retries, and circuit breakers](./resilience.md#5-shut-down-without-dropping-requests) |
| **Backups** | Configure off-host database backups and verify a full restore procedure before taking live traffic. | [Backups, upgrades, and troubleshooting](./maintenance.md#part-1-back-up-and-restore-state-databases) |

---

## Verify your configuration before launch

Run these three preflight commands against your production configuration file and definition set:

```bash
# 1. Validate config.toml and ORION_* environment variables
orion-server validate-config -c config.toml

# 2. Verify database connectivity and check for pending migrations
orion-server test-connectivity -c config.toml

# 3. Validate all workflow, channel, and connector definitions
orion-cli lint ./definitions
```

Once the server is running, inspect its component health using an admin credential:

```bash
curl -s -H "Authorization: Bearer $ORION_ADMIN_KEY" \
  http://localhost:8080/health | jq '{status, workflows_loaded, channels, connectors}'
```

Confirm that:
- `status` is `"ok"` (an HTTP `200` response with `"status": "degraded"` indicates a connector failed to load or a channel is quarantined).
- `channels.quarantined` and `connectors.failed_to_load` are both empty.

## Pre-deployment checklist for every release

1. **Back up the database** before applying schema migrations or upgrading binaries.
2. **Validate with the target version**: Run `orion-cli lint ./definitions` and `orion-cli clippy ./definitions` using the CLI release matching the target server.
3. **Preview the diff**: Run `orion-cli deployments plan -d dist/deployment` and verify that no shared channels or workflows are unintentionally removed.
4. **Run migrations as a dedicated step**: In cluster mode, run `orion-server migrate` before rolling new pods.
5. **Roll one replica at a time**: Wait for `/readyz` to return `200` on each replica before proceeding to the next.

## Next steps

- [Deploy with Docker and Kubernetes](./deploy.md): Container and Helm deployment guides.
- [Security and access control](./security.md): Deep dive into authentication, TLS, secrets, and SSRF controls.
- [Monitoring, metrics, and alerts](./monitoring.md): Production Prometheus alert rules and health probes.
