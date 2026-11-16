<!-- description: The [cluster] settings for multi-replica Orion: enabled, the epoch poll interval, instance_id, and the shared store an edition supplies. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# Cluster settings

Available in Orion Standard and Enterprise only. The `[cluster]` section turns on multi-replica mode: replicas over PostgreSQL or MySQL, sharing dedup, cache and rate limits through the edition's store. A Community node serves alone and keeps all three in process.

## Synopsis

```toml
[cluster]
enabled = false
epoch_poll_interval_ms = 2000
instance_id = ""

# The edition's store. Without this section, a node with `enabled = true`
# does not boot.
[extensions.redis]
host = "redis.internal"
port = 6379
db = 0
tls = false
username = "orion"
password = "env://REDIS_PASSWORD"
key_prefix = "orion:cluster:"
timeout_ms = 1000
```

## Description <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

With `cluster.enabled = false`, the default, there is no shared store. A single node keeps its dedup claims, response cache and rate limits in process. The deployment reconciler still runs, so a failed build recovers.

With it enabled, the cluster's hot-path state lives in the store the edition provides. Orion Ext composes a Redis store from `[extensions.redis]` ([extension settings](./extensions.md)); Community ships none. A Community node with `cluster.enabled = true` fails boot with the `NO_CLUSTER_STORE` message.

### What the shared store holds

- **Deduplication claims.** A key claimed on one node is a duplicate on every node.
- **The response cache.** A response cached on one node is served by every node.
- **Per-channel rate-limit windows.** A one-second fixed window counted in the shared store, so a channel's limit is the fleet's limit.
- **Namespace invalidation counters.** A response-cache namespace's counter lives at `{key_prefix}orion:rc:ns:{namespace}`.

Each operation maps onto one atomic store primitive. A claim is a conditional set; a rate window is a counted increment named by the store's own clock.

### What stays on the state database

The state database still owns everything a cluster coordinates on record:

- The deployment head and history, and the config epoch that announces a change.
- Job leases, so trace cleanup, audit cleanup, DLQ retry and artifact collection run on one node.
- The cron ledger, including occurrence identity, claims and singletons.
- Circuit-breaker epochs, so a reset requested on one node reaches every node.
- Dead-letter queue claims, so each entry is retried once.

Every node runs the reconciler, which polls the committed head every `epoch_poll_interval_ms`. A node that sees the head move builds the new revision. The same loop applies a circuit-breaker reset requested elsewhere.

Cluster mode requires a `postgres://` or `mysql://` storage URL. SQLite is single-host by construction and is refused at startup. Set `storage.auto_migrate = false` and run `orion-server migrate` as a deploy step; a production node refuses the combination of cluster mode and boot-time migration.

`instance_id` is capped at 64 characters because it is written as the holder of every job lease it takes.

## Options

| Setting | Default | Env var | When to change |
|---|---|---|---|
| `cluster.enabled` | `false` | `ORION_CLUSTER__ENABLED` | Turn on whenever more than one replica serves the same database. The node needs the edition's shared store, or it fails boot with `NO_CLUSTER_STORE`. <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> |
| `cluster.epoch_poll_interval_ms` | `2000` | `ORION_CLUSTER__EPOCH_POLL_INTERVAL_MS` | Lower it for faster deployment propagation, at the cost of more database polling. |
| `cluster.instance_id` | `""` | `ORION_CLUSTER__INSTANCE_ID` | Set a stable per-replica value, such as the pod name. Empty generates a UUID per boot. At most 64 characters. |

## Errors

| Error | Raised when |
|---|---|
| `NO_CLUSTER_STORE` | `cluster.enabled = true` and the composition registered no store, as on every Community node. |
| `cluster store '<name>' did not answer at boot` | The edition's store failed its startup ping. The boot stops rather than letting every dedup claim fail open. |
| `cluster store '<name>' did not answer at boot within <n>s` | The ping exceeded `engine.health_check_timeout_secs`, which defaults to 2. |
| `cluster.epoch_poll_interval_ms must be > 0` | The value is zero. |
| `cluster.instance_id must be at most 64 characters` | The configured identity is longer than the job-lease holder column. |
| `cluster mode requires PostgreSQL or MySQL` | The storage URL names SQLite. The binary's message is `cluster.enabled = true requires postgres:// or mysql:// storage`. |
| `cluster.enabled = true with storage.auto_migrate = true in production` | Every replica would migrate at boot and race the others. |

## Examples

### Run three replicas over one database and one Redis store

```toml
[storage]
url = "postgres://orion:secret@postgres:5432/orion"
auto_migrate = false

[cluster]
enabled = true
epoch_poll_interval_ms = 2000
instance_id = "node-a"

[extensions.licence]
tier = "enterprise"

[extensions.redis]
host = "redis.internal"
tls = true
username = "orion"
password = "env://REDIS_PASSWORD"
key_prefix = "orion:cluster:"
```

The other replicas carry the same file with their own `instance_id`.

### Read the cluster store's health

```bash
curl -s http://node-a:8080/readyz | jq '.components.cluster_store'
```

```json
{
  "name": "redis",
  "status": "ok"
}
```

An `error` status makes the node not ready.

## Caveats

- A store failure after boot does not stop the node. Deduplication and per-channel rate limits resolve it through `on_backend_error`, failing open by default; `deny` refuses with `503`. The store client reconnects on its own.
- Redis expires its own keys, so the shared store needs no sweeper. A store that does ask for one gets a sweeping task; Redis does not.
- A store registered while `cluster.enabled = false` is unused. The node logs it and keeps all three guards in process.
- A booting node is at most one `epoch_poll_interval_ms` behind its peers, as every node is. That is normal eventual consistency.
- `POST /api/v2/admin/backups` answers `400` in cluster mode. Use the database's own snapshot and point-in-time recovery tooling.
- [Extension settings](./extensions.md) have no environment-variable override. The section reaches a node through its config file.
- Platform rate limits, circuit breakers and backpressure stay per node. Only the per-channel rate window moves into the shared store.

## Related

- [Extension settings](./extensions.md): the `[extensions.redis]` fields and the `[extensions.licence]` label.
- [Deploy a cluster](../../operate/deploy/cluster.md): the topology, Redis operations and the migration step.
- [Storage settings](./storage.md): the PostgreSQL or MySQL requirement, pool sizing and `auto_migrate`.
- [Architectural characteristics](../../concepts/architectural-characteristics.md): what cluster mode guarantees, and what stays per node.
- [Edition capability matrix](../editions.md): every capability Community does not serve.
