<!-- description: Deploy a multi-node Orion Standard or Enterprise cluster over PostgreSQL or MySQL and shared Redis for distributed rate limits, cache, and rollouts. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Multi-node clustering and HA

<span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

In Orion Standard and Enterprise, multiple `orion-ext` replicas run as a stateless compute fleet over two shared backends:
1. **PostgreSQL or MySQL (`[storage]`)** for durable control-plane records: deployment history, config epochs, the cron occurrence ledger, job leases, circuit-breaker epochs, and dead-letter queue claims.
2. **Redis (`[extensions.redis]`)** for low-latency hot-path state: request deduplication claims, the channel response cache, and cluster-wide per-channel rate-limit windows.

## Cluster architecture and state separation

```mermaid
flowchart TB
    LB["Load Balancer (Data Plane :8080)"]
    subgraph Replicas["orion-ext Fleet (cluster.enabled = true)"]
        N1["node-a"]
        N2["node-b"]
        N3["node-c"]
    end
    DB[("PostgreSQL / MySQL\n(Deployments, Cron Ledger, Leases, DLQ)")]
    Redis[("Redis Cluster Store\n(Dedup Claims, Response Cache, Rate Windows)")]

    LB --> N1 & N2 & N3
    N1 & N2 & N3 <--> DB
    N1 & N2 & N3 <--> Redis
```

| State | Backing store in cluster mode | How replicas coordinate |
|---|---|---|
| **Active deployment & history** | PostgreSQL or MySQL | Reconciler on every replica polls the committed head every `cluster.epoch_poll_interval_ms` and hot-swaps in memory. |
| **Cron schedules & singletons** | PostgreSQL or MySQL | Database-backed cron ledger claims each `(channel, scheduled_time)` tick so a scheduled run executes once across the fleet. |
| **Background maintenance jobs** | PostgreSQL or MySQL | Job leases (held by `cluster.instance_id`) ensure trace cleanup, audit cleanup, and DLQ retries run on one replica at a time. |
| **Deduplication claims** | Redis (`[extensions.redis]`) | Conditional set (`SET NX`) under `{key_prefix}{key}` rejects duplicate requests across all replicas. |
| **Channel response cache** | Redis (`[extensions.redis]`) | Responses cached on one replica are served by every replica; namespace invalidation increments `{key_prefix}orion:rc:ns:{namespace}`. |
| **Per-channel rate limits** | Redis (`[extensions.redis]`) | One-second fixed windows counted at `{key_prefix}rl:{key}` enforce the channel's limit across the entire fleet. |

> [!IMPORTANT]
> Cluster mode requires `postgres://` or `mysql://` for `storage.url` (SQLite is single-host only and is refused at startup) and `storage.auto_migrate = false`. Run `orion-server migrate` (or `orion-ext migrate`) as a dedicated release step before starting replicas so booting pods do not race database migrations.

---

## Configure `orion.toml` for a multi-replica cluster

Configure `[storage]`, `[cluster]`, `[extensions.licence]`, and `[extensions.redis]` in each replica's `orion.toml`:

```toml
[storage]
url = "postgres://orion:secret@postgres.internal:5432/orion"
auto_migrate = false

[cluster]
enabled = true
epoch_poll_interval_ms = 2000
instance_id = "node-a"

[extensions.licence]
tier = "enterprise"

[extensions.redis]
host = "redis.internal"
port = 6379
db = 0
tls = true
username = "orion"
password = "env://REDIS_PASSWORD"
key_prefix = "orion:cluster:"
timeout_ms = 1000
```

### `[cluster]` settings

| Setting | Default | Environment variable | Description |
|---|---|---|---|
| `cluster.enabled` | `false` | `ORION_CLUSTER__ENABLED` | Enable multi-replica mode. Requires `[extensions.redis]` and PostgreSQL/MySQL storage. |
| `cluster.epoch_poll_interval_ms` | `2000` | `ORION_CLUSTER__EPOCH_POLL_INTERVAL_MS` | How often each replica polls the state database for new deployment revisions and circuit-breaker resets (`> 0`). |
| `cluster.instance_id` | `""` (UUID per boot) | `ORION_CLUSTER__INSTANCE_ID` | Stable replica identifier (`1` to `64` characters, such as the Kubernetes Pod name `HOSTNAME`) recorded on job leases and convergence reports. |

### `[extensions.redis]` settings

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `host` | string | yes | — | Redis server hostname or IP address (URLs are refused). |
| `port` | integer | no | `6379` | Redis TCP port (`> 0`). |
| `db` | integer | no | `0` | Logical database number (`>= 0`). |
| `tls` | boolean | no | `false` | Use TLS from the first byte, verified against the platform trust store. |
| `username` | string | no | default user | Redis ACL username. |
| `password` | string | no | — | Must be an `env://VAR` reference resolved at boot (`vault://` and plaintext literals are refused). |
| `key_prefix` | string | no | `"orion:cluster:"` | Prefix applied to all cluster store keys (max `256` bytes). Use distinct prefixes if two clusters share one Redis instance. |
| `timeout_ms` | integer | no | `1000` | Connect and per-command timeout in milliseconds (`> 0`). |

---

## Roll out a deployment and wait for cluster convergence

1. **Run database migrations once before starting or upgrading replicas**:

```bash
orion-ext migrate --config /etc/orion/orion.toml
```

2. **Apply a deployment and wait for all observed nodes to converge**:

<!-- tabs:start -->
### **CLI**
```bash
orion-cli deployments apply -d dist/deployment \
  --wait-ms 30000 \
  --wait-target observed
```

### **curl**
```bash
curl -s "http://localhost:8080/api/v2/admin/deployments/8/convergence?target=observed" \
  -H "X-Admin-Key: $ORION_ADMIN_KEY" | jq .
```
<!-- tabs:end -->

You can also poll convergence for any historical or current revision explicitly:

```bash
orion-cli deployments wait 8 --target observed --timeout-ms 120000
```

---

## Health probes and behavior during a Redis outage

- **Startup ping**: At boot, `orion-ext` pings the Redis cluster store within `engine.health_check_timeout_secs` (default `2s`). If Redis does not answer, startup stops immediately rather than admitting traffic with uncoordinated deduplication.
- **Readiness probe (`/readyz`)**: Inspect the cluster store component health via `/readyz`:

```bash
curl -s http://localhost:8080/readyz | jq '.components.cluster_store'
```

```json
{
  "name": "redis",
  "status": "ok"
}
```

- **Mid-run Redis degradation**: If Redis becomes unreachable after boot, `/readyz` marks the replica not ready so load balancers stop sending new connections. Active workflows continue running from memory, while deduplication and per-channel rate limits follow each channel's configured `on_backend_error` policy (`allow` by default, or `deny` to return `503`).

## Next steps

- [Deploy with Docker and Kubernetes](../operate/deploy.md): Production container manifests, liveness/readiness probes, and rolling update strategies.
- [Backups, upgrades, and troubleshooting](../operate/maintenance.md): Database backup strategies and zero-downtime cluster upgrades.
