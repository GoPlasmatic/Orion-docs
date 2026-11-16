<!-- description: Run Orion as a multi-replica cluster on Standard or Enterprise: the shared Redis store, topology, leases, readiness and the migration step. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Run a cluster

Available in Orion Standard and Enterprise only. *Cluster mode* makes N identical Orion replicas behave as one logical system, sharing deployment state and the request guards through the store the edition supplies. A Community node serves alone and keeps deduplication, the response cache and rate limits in process.

## Before you start

You need:

- The `orion-ext` image, which is the Standard and Enterprise artifact. It composes the Redis cluster store. Community's `orion-server` ships no cluster store and cannot run this page's configuration.
- One shared **PostgreSQL or MySQL** database. Startup refuses `sqlite:`, because a file is single-host by construction.
- One **Redis** server for the shared store. It is not a persistence tier; every key in it has a TTL.
- A config file, mounted into each node, that carries `[extensions.licence]` and `[extensions.redis]`. These sections have no environment-variable override.

| Backend | Single instance | Cluster mode | Notes |
|---------|:-:|:-:|-------|
| **SQLite** | Recommended | Refused at startup | WAL mode allows concurrent reads but one writer, and the file cannot be shared across hosts |
| **PostgreSQL** | Supported | Recommended | Use connection pooling (PgBouncer) when replica count × `storage.max_connections` approaches the server's limit |
| **MySQL** | Supported | Supported | Use `READ-COMMITTED` isolation for best concurrency |

## When to go cluster

Reach for cluster mode when you need one of these, in roughly this order:

- **Availability.** One node is one restart away from an outage. Two behind a load balancer survive a rolling deploy, a node failure and a kernel upgrade.
- **Correct guards across replicas.** The moment a second node exists, per-node dedup and rate limits stop meaning what they say. Cluster mode is what makes "100 requests per second" a fleet-wide number rather than a per-node one.
- **Deployments that every node serves.** Every replica follows the same deployment head, and convergence reports which nodes serve it.

Throughput is usually the weakest reason. In the historical Orion 1.0.0 baseline test, a single instance sustained 5.1K to 5.7K workflow requests per second with single-digit-millisecond average latency. The [benchmark record](https://github.com/GoPlasmatic/Orion/blob/main/crates/orion-server/tests/benchmark/results/v1.0.0/SUMMARY.md) ran on an Apple M2 Pro Mac Mini with a release build and 50 concurrent connections. Treat that as a historical, versioned benchmark result, not a capacity promise for 2.x, another machine or another workflow. Most estates hit an availability requirement long before they hit a throughput ceiling.

> [!NOTE]
> The published record measures a single instance. The repository ships a cluster load scenario (`crates/orion-server/tests/benchmark/bench.sh cluster`), but its numbers are not part of the 1.0.0 record. Size a fleet against your own measurements rather than assuming replicas multiply the figure above.

## Turn it on

Point every replica at the shared database, and give it the edition's store and label:

```toml
[storage]
url = "postgres://orion:orion@postgres:5432/orion"
auto_migrate = false               # run `orion-server migrate` as a deploy step

[cluster]
enabled = true
epoch_poll_interval_ms = 2000      # how often each node's reconciler polls the head
instance_id = "node-a"             # stable per replica; max 64 chars

# The edition the node reports. Absent, it reports `standard`.
[extensions.licence]
tier = "enterprise"

# The cluster store. Without this section, the node does not boot.
[extensions.redis]
host = "redis.internal"
tls = true
username = "orion"
password = "env://REDIS_PASSWORD"  # an env:// reference, never a literal
key_prefix = "orion:cluster:"
```

Give each replica its own `instance_id`. It names the node as a lease holder and in log lines. An empty value generates a UUID per boot, which is fine for serving but makes those records harder to follow across a restart.

Do not translate the extension sections into environment variables. `ORION_EXTENSIONS__` is not an override, and the server refuses the shape it would produce. The two edition sections reach each node through its config file, mounted read-only.

## The topology

The reference shape is one load balancer in front of two or three identical nodes, one PostgreSQL primary, and one Redis server:

| Component | Role |
|---|---|
| Load balancer | Terminates TLS, spreads requests, removes a node whose `/readyz` fails |
| Two or three `orion-ext` nodes | Serve every channel, run the reconciler, hold job leases |
| PostgreSQL or MySQL | Deployment head and history, config epoch, job leases, cron ledger, breaker epochs, DLQ claims |
| Redis | Dedup claims, response cache, fleet-wide rate-limit windows, namespace counters |

A third node is what lets you take one out for maintenance without dropping below two. All nodes are identical: no leader is elected and no node is special.

## What the cluster shares

| Concern | Where it lives | How it works |
|---|---|---|
| **Deployments** | State database | A committed deployment is a row. Each replica's reconciler polls the head every `epoch_poll_interval_ms` and builds the revision it should serve. See [How a deployment reaches every node](#how-a-deployment-reaches-every-node). |
| **Deduplication** | Shared store | A claim made on one node stops the same key on every node. The window runs on the store's clock. |
| **Response caching** | Shared store | A response cached on one node is served by every node. |
| **Per-channel rate limits** | Shared store | A one-second fixed window counted in the store; the configured rate holds across all replicas combined. |
| **Background jobs** | State database | Trace cleanup, audit cleanup, DLQ retry and artifact collection take a per-tick lease, so one node runs each job. DLQ rows are leased too. |
| **Circuit-breaker resets** | State database | `POST /api/v2/admin/connectors/circuit-breakers/{key}` records an epoch, and every node's reconciler applies it. Breakers themselves trip per node. |
| **Scheduled runs** | State database | Occurrence identity, claim leases and singletons are rows. No leader is elected. |

Redis expires its own keys. Nothing sweeps them, and a node that stops does not leave rows behind.

### How a deployment reaches every node

An apply commits a new head to the shared database and builds it on the node that took the request. Nothing pushes it to the other nodes. Each one finds it through its own reconcile loop, the one loop that changes what a node serves.

Each tick, the loop reads the committed head. When the head has moved, it re-reads the deployment history and builds the revision it should serve. It also applies circuit-breaker resets requested on other nodes, and writes a heartbeat naming the revision it serves.

A build that fails leaves the node serving what it had, and its `/health` reports `engine_reload` as degraded. The loop retries on the next tick. `GET /api/v2/admin/deployments/{revision}/convergence?target=observed` reports which nodes serve a revision, from those heartbeats. [Deployments](../../concepts/deployments.md#convergence) explains the model.

### A node that has recently started

A booting node is at most one `epoch_poll_interval_ms` behind, like every other node. `/healthz` and `/readyz` are green once the first generation is published, so a load balancer sends it traffic at once. A deployment committed during that window can answer `404` there and `200` on its peers until the tick lands. That is normal eventual consistency, not a fault.

### Scheduled runs

Cron channels coordinate entirely through three shared tables and need no leader. An occurrence's identity is `(channel_id, scheduled_for)`, so two reconcilers racing the same pass produce one row. Claims are leased against the database clock, and a running attempt renews its claim every heartbeat. When a node dies, a peer recovers its work and its singleton slots after one `cron.claim_lease_secs`. A `forbid` singleton is a row exactly one occurrence holds at a time, acquired in the same transaction that marks it running.

Every node should agree on `cron.enabled`. A node with it off quarantines the deployed cron channels rather than ignoring them. A mixed cluster is visible on `/health` rather than silently half-scheduling.

## What stays per node

These are per node by design, and each has ×N semantics you should size for:

| Component | Semantics |
|---|---|
| **Circuit breakers** | Trip independently; each node stops calling after its own failures. Resets fan out. |
| **Backpressure** | `max_concurrent_per_node` is per node, as the name says: N replicas admit up to N× that many in flight. |
| **Platform rate limits** | `[rate_limit]` IP limits are per node: N× the configured value fleet-wide. |
| **In-memory cache connectors** | A `cache` connector stays inside its node. A dedup or response-cache guard that names one refuses to load in cluster mode. |
| **Plugins** | The WebAssembly sandbox, its ceilings and its loaded components are per node. The deployment and its convergence are shared. |
| **`/metrics`** | Scraped per node. Point Prometheus at every replica, or let it discover pods. |

> [!WARNING]
> A channel whose dedup or cache connector is missing, broken, or explicitly in-memory refuses to load in cluster mode. The deployment commits; the channel is then quarantined at load, refused at every ingress with a `503`, absent from the route table, and listed under `/health`'s `channels.quarantined` with `components.channels: "degraded"`. The node boots and every other channel keeps serving. Silently degrading to per-node state would leave a channel advertising a guarantee it no longer keeps.

## Operate the Redis store

The store is one Redis server, or a primary behind one address. The client reconnects on its own after a dropped connection and does not need a node restart.

| Setting | Default | What it controls |
|---|---|---|
| `host` | required | The server's host name or IP, not a URL |
| `port` | `6379` | The server's port |
| `db` | `0` | The logical database |
| `tls` | `false` | TLS from the first byte, verified against the platform trust store |
| `username` | the default user | The ACL user |
| `password` | unset | An `env://VAR` reference; a literal is refused |
| `key_prefix` | `orion:cluster:` | The prefix on every key the store writes |
| `timeout_ms` | `1000` | Connect and per-command timeout |

The keyspace is flat and entirely under `key_prefix`:

- Values such as dedup claims and cached responses are stored at `{key_prefix}{key}`.
- Rate windows live at `{key_prefix}rl:{key}`, and each expires two seconds after its second.
- A response-cache namespace's version counter is `{key_prefix}orion:rc:ns:{namespace}`. An operator's own tooling must include the prefix:

```bash
redis-cli -h redis.internal INCR orion:cluster:orion:rc:ns:catalog
```

Do not enable persistence for this store. Everything in it is hot-path state with a TTL, and a restart only forgets dedup windows and cached responses. A restarted Redis costs cache warmth and one dedup window, not correctness of a committed deployment.

`db` isolates the keyspace if you share a server. `key_prefix` is the stronger isolation: two Orion clusters on one Redis must not share a prefix. Size memory for the cached response bodies, which dominate; dedup keys and rate windows are small and expire on their own.

## Boot and readiness

A node pings its store at boot, bounded by `engine.health_check_timeout_secs` (default 2 seconds). A store that does not answer fails the boot, naming it, rather than letting every dedup claim fail open.

`/readyz` reports the store alongside the database:

```json
{
  "status": "ready",
  "components": {
    "database": "ok",
    "cluster_store": { "name": "redis", "status": "ok" }
  }
}
```

An `error` status makes the node not ready, so the load balancer removes it. The component is absent on a single node, which keeps all three guards in process.

After boot, a store failure does not stop the node. Deduplication and per-channel rate limits resolve it through their `on_backend_error` policy: `allow` fails open, `deny` refuses with `503`. Both default to `allow`. The store client keeps reconnecting.

## Migrate as a deploy step, not at boot

Run `orion-server migrate` before new replicas start:

```toml
[storage]
auto_migrate = false
```

A replica that boots against a pending migration fails fast. A production cluster left on `auto_migrate = true` is refused at startup rather than allowed to race. Write migrations expand and contract style. First ship one that only *adds* columns, tables and indexes, beside code that works with both shapes; remove the old shape in a later release. During a rolling deploy, old and new binaries briefly share one database.

The Orion Ext HA compose stack has a one-shot `migrate` service that completes before either node boots. The Helm chart runs a `pre-install` and `pre-upgrade` Job. That chart is Community's and has no mechanism for `[extensions.*]`; see [Deploy on Kubernetes](./kubernetes.md).

## Sizing a cluster

- **Database connections.** `storage.max_connections` is per process. N × that value must stay below the server's own limit. Three replicas against a stock PostgreSQL want roughly 25 each, less whatever else connects.
- **Redis memory.** Budget the cached response bodies at peak, plus a small margin for dedup keys and two seconds of rate windows. Every key expires.
- **Epoch polling.** `epoch_poll_interval_ms = 2000` means one head read every two seconds per node. Lowering it speeds propagation and adds polling load.
- **Node count.** Two is the minimum for availability. A third lets you drain one node without dropping to one.

## Verify

Confirm that a deployment applied through one node reaches the others:

```bash
curl -s http://node-a:8080/api/v2/admin/deployments | jq '.head_revision'
curl -s "http://node-b:8080/api/v2/admin/deployments/1/convergence?target=observed"
```

Every node appears under `converged_nodes` within one `epoch_poll_interval_ms`. Then confirm the store is ready on each node:

```bash
curl -s http://node-b:8080/readyz | jq '.components.cluster_store'
```

`deploy/ha/rolling-drill.sh` drives a zero-5xx rolling deploy against the compose topology.

## Backups change shape in a cluster

`POST /api/v2/admin/backups` returns `400` in cluster mode. The file would land on one arbitrary replica, and cluster storage is PostgreSQL or MySQL, which the SQLite backup mechanism cannot copy anyway. Use your database's own tooling: automated snapshots plus point-in-time recovery on a managed service, or `pg_dump` and `mysqldump` self-managed.

The Redis store needs no backup. Everything in it expires, and an empty Redis is a cold cache, not a lost deployment.

## Shard channels across pools

Cluster mode scales one estate. If you want dedicated capacity for a group of channels, run separate instance pools against the same database and store, and filter what each loads:

```toml
# Pool A: order processing
[channel_filter]
include = ["orders.*", "payments.*"]

# Pool B: analytics
[channel_filter]
include = ["analytics.*", "reports.*"]
```

This is a refinement layered on top of cluster mode, not an alternative to it. Each pool is still a cluster if it has more than one node.

## Next steps

- [Deploy Orion Ext](./orion-ext.md): the licensed image, its Compose stack and the mounted edition config.
- [Deploy with Docker](./docker.md): the HA Compose topology, with nginx, PostgreSQL and Redis.
- [Extension settings](../../reference/configuration/extensions.md): every `[extensions.licence]` and `[extensions.redis]` field.
- [Monitor and alert](../run/monitoring.md): cluster store readiness, edition drivers and the per-node metrics that need aggregating.
- [Cluster settings](../../reference/configuration/cluster.md): every `[cluster]` key with its default.
