<!-- description: The [extensions.<id>] tables an edition reads: the licence label, and the Redis cluster store's host, keyspace and timeouts. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# Extension settings

Available in Orion Standard and Enterprise only. An `[extensions.<id>]` table carries settings an edition's own drivers and providers read: the edition label a node reports, and the Redis cluster store. Community stores each table and never interprets one.

## Synopsis

```toml
[extensions.licence]
tier = "standard"

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

## Description

An edition composes drivers and providers that Community does not have, and some of them need node-wide settings. Community cannot type those settings, and the rest of the configuration refuses unknown keys, so they get one free-form table keyed by the extension's id. Community stores every table and never reads one. The edition reads its own section before it composes the node, and holds the section to its own type.

The table sits outside the `ORION_SECTION__KEY` scheme, so it has no per-key environment override. Write `${VAR}` in a value instead. A value that must stay secret belongs behind an `env://` reference the edition resolves, or in `[secrets]`, never as a literal. `validate-config` prints this table as written.

Standard and Enterprise are one artifact and run the same extensions. `[extensions.licence]` only names the edition the node reports; the reported edition changes no capability.

## Fields

### `[extensions.licence]`

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `tier` | string | yes | — | The edition the node reports: `"standard"` or `"enterprise"`. Omitting the whole section reports `standard`. |

The section is optional; when present, `tier` is required. It is strict: `tier` is the only key it accepts, and a value other than the two names is refused when the node starts.

### `[extensions.redis]`

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `host` | string | yes | — | The server's host name or IP; a URL is refused. |
| `port` | integer | no | `6379` | The server's TCP port; zero is refused. |
| `db` | integer | no | `0` | The logical database, which must not be negative. |
| `tls` | boolean | no | `false` | Use TLS from the first byte, verified against the platform trust store. |
| `username` | string | no | the default user | The ACL user; a blank value is refused. |
| `password` | string | no | — | An `env://VAR` reference resolved at compose time; a literal is refused. |
| `key_prefix` | string | no | `orion:cluster:` | The prefix on every key the store writes, at most 256 bytes. |
| `timeout_ms` | integer | no | `1000` | Connect and per-command timeout in milliseconds; zero is refused. |

The section configures the cluster store only. A workflow-facing Redis cache is a `com.orion.redis` connection with its own settings, not this table.

The store is used only with `cluster.enabled = true`. With the section present and cluster mode off, the store is registered but unused, and the node logs that at boot. Without the section, no store is registered, and a node with cluster mode on does not boot.

### The keyspace

Every key is under `key_prefix`:

- Values are stored at `{key_prefix}{key}`.
- Rate windows are stored at `{key_prefix}rl:{key}` and expire two seconds after their second.
- A response-cache namespace's version counter is `{key_prefix}orion:rc:ns:{namespace}`.

Redis expires its own keys. The store reports that it needs no sweep, so no sweeper task is started.

## Errors

| Error or refusal | Raised when |
|---|---|
| Unknown `[extensions.licence]` field | The licence table names a key other than `tier`. |
| Unknown `[extensions.licence]` tier | `tier` is neither `standard` nor `enterprise`. |
| Invalid `[extensions.redis]` `host` | The host is empty or is written as a URL. |
| Invalid `[extensions.redis]` `port` | The port is zero. |
| Invalid `[extensions.redis]` `db` | The database number is negative. |
| Invalid `[extensions.redis]` `username` | The user name is blank. |
| Invalid `[extensions.redis]` `key_prefix` | The prefix exceeds 256 bytes. |
| Invalid `[extensions.redis]` `timeout_ms` | The timeout is zero. |
| `[extensions.redis] 'password' must be an env://VAR reference` | The password is a literal or names another scheme. |
| `[extensions.redis] 'password': the environment variable '<VAR>' is not set` | The referenced variable is unset at compose time. |
| Unknown `[extensions.redis]` field | A key is misspelled; the section is strict. |
| `NO_CLUSTER_STORE` | `cluster.enabled = true` and no `[extensions.redis]` section registered a store. |

## Examples

### Report the Enterprise edition and share one Redis store

```toml
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

### Register the store while the cluster is off

```toml
[cluster]
enabled = false

[extensions.redis]
host = "redis.internal"
```

The store is registered and never used. The node logs one warning naming it, and serves as a single node with deduplication, the response cache and rate limits in process.

## Caveats

- There is no per-key environment override. Mount the section as a config file, or write `${VAR}` inside a value.
- The password accepts `env://` only. It is resolved before a secret store exists, so `vault://` is refused.
- `validate-config` prints this table as written, which is why a credential belongs behind a reference.
- The licence is a label, not an entitlement check. Nothing is verified, bound or expires.
- Two clusters on one Redis server must use different `key_prefix` values. The prefix is the isolation boundary.
- A second cluster store registration is a composition error. An edition composes one store, or none.

## Related

- [Cluster settings](./cluster.md): `[cluster]`, the boot refusal and what stays on the state database.
- [Deploy a cluster](../../operate/deploy/cluster.md): the topology, Redis operations and readiness.
- [Editions](../../concepts/editions.md): the model and the support boundary.
- [Edition capability matrix](../editions.md): every capability Community does not serve.
- [Vars and secrets](./vars-and-secrets.md): the other free-form sections and how they differ.
