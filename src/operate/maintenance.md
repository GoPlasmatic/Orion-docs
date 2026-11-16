<!-- description: Back up and restore SQLite, PostgreSQL, or MySQL state databases, perform zero-downtime Orion upgrades, and diagnose common operational symptoms. -->
<!-- type: troubleshooting -->
<!-- last_verified: 2026-11-16 -->

# Backups, upgrades, and troubleshooting

This guide covers three core maintenance operations for a live Orion deployment: **backing up and restoring the state database**, **upgrading Orion safely**, and **diagnosing common production symptoms**.

---

## Part 1: Back up and restore state databases

Every deployed revision, connection, execution trace, cron occurrence, and audit row lives in Orion's state database (`[storage]`), while your source definitions (`workflow.json`, `channel.json`, `connector.json`) live in Git. Keep both backed up: Git restores your service definitions onto any fresh instance, and a database backup restores an instance's revision history, traces, and audit trail.

| Database backend | Backup mechanism | Restore procedure |
|---|---|---|
| **SQLite** (single-node only) | Online `VACUUM INTO` snapshot via `orion-cli backups create` (`POST /api/v2/admin/backups`) | Stop Orion, copy the backup `.db` file over `storage.url`, and start Orion. |
| **PostgreSQL** | `pg_dump`, `pg_basebackup`, or cloud-managed snapshots + PITR | Stop Orion replicas, restore snapshot/PITR, run `orion-server migrate`, and start replicas. |
| **MySQL** | `mysqldump`, binlog PITR, or cloud-managed snapshots | Stop Orion replicas, restore snapshot/PITR, run `orion-server migrate`, and start replicas. |

### Create and prune SQLite backups

Configure `backup_dir` and `backup_retention_count` in `orion.toml` so old snapshots are pruned automatically:

```toml
[storage]
backup_dir = "/var/lib/orion/backups"
backup_retention_count = 7
```

Trigger an online consistent backup (`VACUUM INTO`) without stopping the server:

<!-- tabs:start -->
#### **CLI**
```bash
orion-cli backups create
orion-cli backups list
```

#### **curl**
```bash
curl -s -X POST -H "Authorization: Bearer $ORION_ADMIN_KEY" \
  http://localhost:8080/api/v2/admin/backups

curl -s -H "Authorization: Bearer $ORION_ADMIN_KEY" \
  http://localhost:8080/api/v2/admin/backups
```
<!-- tabs:end -->

> [!NOTE]
> `POST /api/v2/admin/backups` and `GET /api/v2/admin/backups` return `400` in cluster mode because cluster deployments run on PostgreSQL or MySQL. Always copy SQLite backup files off the local host disk to object storage or a remote volume.

### Restore a SQLite backup

```bash
# 1. Stop Orion so in-flight requests drain cleanly
systemctl stop orion

# 2. Replace the active database file with the backup snapshot
cp /var/lib/orion/backups/orion_backup_20261116_101500_042.db /var/lib/orion/orion.db

# 3. Start Orion and verify health
systemctl start orion
curl -sf http://localhost:8080/readyz
curl -s -H "Authorization: Bearer $ORION_ADMIN_KEY" \
  http://localhost:8080/health | jq '{status, workflows_loaded}'
```

---

## Part 2: Upgrade Orion

### Upgrading within Orion 2.x

Within Orion 2.x, committed deployments live in the state database and remain compatible across minor and patch upgrades:

1. **Back up the state database** and read the target version's release notes in [Upgrade to Orion 2.0](../releases/upgrade-to-2.0.md).
2. **Validate definitions with the new `orion-cli` binary**:
   ```bash
   orion-cli lint ./definitions
   orion-cli clippy ./definitions
   ```
   If the new version flags any deprecated fields, deploy the updated definitions before upgrading the server binary.
3. **Validate server configuration with the new `orion-server` binary**:
   ```bash
   orion-server validate-config -c config.toml
   ```
4. **Apply database migrations**: On single-node SQLite with `auto_migrate = true`, migrations run at boot. In cluster mode (`auto_migrate = false`), run `orion-server migrate --wait 60s -c config.toml` (or the Helm `pre-upgrade` Job) before starting new replicas.
5. **Roll replicas one at a time**: Wait for `GET /readyz` (`200`) and `GET /health` (`status: "ok"`, `applied_revision == head_revision`) on each replica before restarting the next.

> [!IMPORTANT]
> Keep `orion-cli` and `orion-server` / `orion-ext` on the same release version. `orion-cli` verifies `api_version` on `GET /api/v2/admin/runtime-capabilities` before running deployment, provider, or connection commands.

### Moving from Orion 1.x to Orion 2.0

Orion 2.0 uses a clean baseline state schema and does not upgrade an Orion 1.x state database in place. Point `storage.url` at a fresh database, compile your definitions with `orion-cli compile ./definitions -o dist/deployment`, apply them with `orion-cli deployments apply -d dist/deployment`, and update client URLs from `/api/v1/*` to `/api/v2/*`. See [Upgrade to Orion 2.0](../releases/upgrade-to-2.0.md) for the step-by-step migration guide.

---

## Part 3: Symptom-first troubleshooting

<div class="fold-sections" data-level="3" data-default="closed" data-skip="next-steps"></div>

### A channel returns `503` "failed to load and is not being served"

- **Cause**: The channel is **quarantined**. When the node built the active deployment revision, either the channel's configuration (such as an unresolved `env://` secret in `auth`) or its bound workflow failed to load. Orion quarantines the broken channel rather than serving it with a missing security guard.
- **Fix**: Inspect `channels.quarantined` on `/health`:
  ```bash
  curl -s -H "Authorization: Bearer $ORION_ADMIN_KEY" \
    http://localhost:8080/health | jq '.channels.quarantined'
  ```
  If an environment variable was missing on the node, set it and restart the node. If the definition itself has an error, fix the source file, run `orion-cli compile`, and apply a new deployment revision.

### Data endpoint returns `404 NOT_FOUND` after `deployments apply` succeeded

- **Cause**:
  1. The request path does not byte-match the channel's `route_pattern` (routes are case-sensitive: `/Orders` does not match `/orders`).
  2. The latest deployment directory omitted the channel, removing it from the active head.
  3. On a multi-node cluster, this replica has not yet converged or failed to reload (`components.engine_reload: "degraded"`).
- **Fix**: Check `head_revision` vs `applied_revision` and inspect the head manifest's members:
  ```bash
  curl -s -H "Authorization: Bearer $ORION_ADMIN_KEY" \
    http://localhost:8080/api/v2/admin/deployments | jq '{head_revision, applied_revision}'
  ```

### `deployments apply` removed channels or workflows belonging to another service

- **Cause**: A deployment manifest declares the **complete state** of an Orion instance. Applying a directory that contains only Service B removes Service A from the head revision.
- **Fix**: Keep all definitions served by the same Orion instance in one compiled directory, and always inspect `orion-cli deployments plan -d dist/deployment` before applying. Use `orion-cli deployments rollback` to restore the previous revision immediately.

### Plan or apply returns `400 DEPLOYMENT_WOULD_NOT_BUILD`

- **Cause**: The candidate manifest failed server-side build validation (for example, two channels claiming the same HTTP route, a workflow calling a missing plugin function, or a missing `requires` entry).
- **Fix**: Compile the definitions directory with `orion-cli compile ./definitions -o dist/deployment --capabilities target-capabilities.json` to catch and fix build errors offline before applying.

### Apply returns `409 DEPLOYMENT_HEAD_CONFLICT` or `409 REVIEWED_DIGEST_MISMATCH`

- **Cause**: Another deployment or rollback advanced `head_revision` after your plan was created. Compare-and-swap prevented your apply from silently overwriting the concurrent change.
- **Fix**: Re-run `orion-cli deployments plan -d dist/deployment` against the new head revision and apply again.

### `/health` reports `status: "degraded"` or `components.engine_reload: "degraded"` at HTTP `200`

- **Cause**: The node is alive and serving traffic, but either a connector failed to initialize, a channel is quarantined, or the latest deployment revision failed to build on this node (so it continues serving the previous revision).
- **Fix**: Query `GET /health` with an admin key and inspect `channels.quarantined`, `connectors.failed_to_load`, and `GET /api/v2/admin/engine/status` (`.data.load_issues`).

### Clients receive `429 Too Many Requests` far below the configured rate limit

- **Cause**: Orion is running behind a reverse proxy or load balancer without `rate_limit.trusted_proxies` configured, causing every caller to share the proxy's TCP peer IP bucket.
- **Fix**: Add your proxy or pod network CIDRs to `rate_limit.trusted_proxies` in `orion.toml`.

### Calls through a connector fail with `503 CIRCUIT_OPEN`

- **Cause**: Repeated failures against that upstream backend tripped the `(channel, connector)` circuit breaker.
- **Fix**: Verify that the upstream service has recovered, then either wait for the half-open probe window to close the breaker automatically or reset it via `orion-cli circuit-breakers reset <key>`.

### Async submissions stay `pending` or `trace_dlq` keeps growing

- **Cause**: Async workers (`trace_queue.workers`) are saturated, or downstream failures are routing async runs into `trace_dlq`.
- **Fix**: Inspect recent DLQ errors with `orion-cli dlq list --limit 20`. Increase `trace_queue.workers` or `trace_queue.dlq_batch_size`, and purge old exhausted entries with `orion-cli dlq purge --older-than-hours 168`.

### A deployed `cron` channel never fires

- **Cause**: `cron.enabled = false` on the node (which quarantines deployed cron channels and marks `components.cron: "degraded"`), or a previous run under `concurrency: "forbid"` is still holding the lock.
- **Fix**: Check `orion-cli cron status` and `orion-cli cron list --channel-id <id>`. Ensure `ORION_CRON__ENABLED=true` across all replicas.

### Apply or upload returns `403 TIER_NOT_ENABLED`, `PROVIDER_NOT_FOUND`, or refuses `kafka`/`graphql`/`sse`

- **Cause**: You are applying a Standard+ feature (canary traffic splitting, `kafka`/`graphql`/`sse` channels, or `com.orion.*` providers) to an open-source Community node (`orion-server`).
- **Fix**: Check `orion-cli deployments capabilities` to confirm the target node runs Orion Standard or Enterprise (`orion-ext`).

### Server refuses to start at boot

| Startup error | Cause & fix |
|---|---|
| Unknown config key or `ORION_*` variable | Fix the typo named in the error message and verify with `orion-server validate-config -c config.toml`. |
| `sqlite:` with `cluster.enabled = true` | Cluster mode requires PostgreSQL (`postgres://`) or MySQL (`mysql://`). |
| `NO_CLUSTER_STORE` | `cluster.enabled = true` requires `orion-ext` with `[extensions.redis]` mounted in `config.toml`. |
| `auto_migrate` in a production cluster | Set `storage.auto_migrate = false` and run `orion-server migrate` before starting replicas. |
| Pending database migrations | Run `orion-server migrate -c config.toml` before starting the server. |
| Missing or weak admin keys in `production` | Set `admin_auth.enabled = true` and configure at least one `sha256:<hex>` key in `admin_auth.api_keys`. |

## Next steps

- [Production checklist](./production-checklist.md): Pre-launch configuration checklist.
- [Monitoring, metrics, and alerts](./monitoring.md): Health endpoints and Prometheus alerts.
- [Timeouts, retries, and circuit breakers](./resilience.md): Configure timeouts, retries, and graceful drain settings.
