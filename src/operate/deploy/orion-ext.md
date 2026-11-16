<!-- description: Deploy the licensed orion-ext artifact: the extension config, migrations, the HA Compose topology, rolling drills, and the rules for pinning and upgrading. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Deploy Orion Ext

Available in Orion Standard and Enterprise only. `orion-ext` is the <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> node: Community plus the edition's drivers and providers. This guide takes the licensed image to a two-node cluster.

## Before you start

You need:

- Access to the private image registry that carries `orion-ext`, or a checkout of the Orion Ext repository to build it.
- Docker Engine or Docker Desktop, with Docker Compose for the HA topology.
- One PostgreSQL or MySQL database. Cluster mode refuses SQLite.
- One Redis server for the cluster store. It is not a persistence tier.
- `orion-cli` at the same release as the node.

The examples use `$ORION_ADMIN_KEY` for an admin API key, `$ORION_ADMIN_API_KEYS` for the server's comma-separated key list, and `registry.example.com/orion-ext:2.0.0` for the published image. Replace all three with your own.

Standard and Enterprise are one artifact. The edition is a label the node reports, not a different binary. [Editions](../../concepts/editions.md) explains the model, and [Run a cluster](./cluster.md) covers the cluster internals this page configures.

## 1. Get the image

Pull the version your licence names from the private registry:

```bash
docker pull registry.example.com/orion-ext:2.0.0
```

Or build it from a checkout. The build fetches Community at the revision `Cargo.toml` pins, never a local patch:

```bash
docker build -t orion-ext:local \
  --build-arg GIT_HASH="$(git rev-parse --short HEAD)" .
```

The image runs as UID 10001, exposes `8080`, and keeps `curl` for its health check. Its default `ORION_SERVER__HOST` is `0.0.0.0`, so the container is reachable from outside its loopback.

## 2. Write the config file

The extension sections have no environment-variable override, so they need a file. Everything else can come from `ORION_*` variables. Save this as `orion-ext.toml`:

```toml
[server]
port = 8080

[storage]
url = "postgres://orion:orion@postgres:5432/orion"
auto_migrate = false

[cluster]
enabled = true
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

`[extensions.licence]` names the edition the node reports: `standard` or `enterprise`. Without the section the node reports `standard`; a section that is present must set `tier`. The label changes no capability. Both editions compose every extension and serve every feature.

`[extensions.redis]` is the cluster store: deduplication claims, the response cache, fleet-wide rate windows and namespace counters. Without the section, no store is registered, and a node with `cluster.enabled = true` does not boot. It fails with the `NO_CLUSTER_STORE` message rather than starting a cluster that cannot coordinate.

`password` is an `env://VAR` reference, never a literal; a literal is refused. The section configures the cluster store only. A workflow-facing Redis cache is a `com.orion.redis` connection with its own settings.

> [!WARNING]
> Do not translate `[extensions.*]` into `ORION_EXTENSIONS__*` variables. That prefix is not an override, and the server refuses the shape it would produce. A deployment that renders config from the environment must still mount this file read-only.

Check the file before the first start:

```bash
docker run --rm -v "$PWD/orion-ext.toml:/app/config.toml:ro" \
  orion-ext:local -c /app/config.toml validate-config --format summary
```

The command prints the effective config with secrets masked. It fails on an unknown key, an unparseable section, or a `password` reference that does not resolve.

## 3. Migrate before the replicas start

Replicas never race migrations. Keep `auto_migrate = false` and run the migration once as a deploy step:

```bash
docker run --rm -v "$PWD/orion-ext.toml:/app/config.toml:ro" \
  -e REDIS_PASSWORD \
  orion-ext:local -c /app/config.toml migrate --dry-run
```

`--dry-run` lists the pending migrations and changes nothing. Apply them, waiting for a database that is still starting:

```bash
docker run --rm -v "$PWD/orion-ext.toml:/app/config.toml:ro" \
  -e REDIS_PASSWORD \
  orion-ext:local -c /app/config.toml migrate --wait 60s
```

`--wait` retries connection failures for the given time. A wrong password or a failed migration stops at once. A retry cannot fix it, and a half-migrated schema is worse than a failed deploy.

## 4. Start one node

Run the image with the config mounted. Pass `REDIS_PASSWORD` from your secret source, never in the file:

```bash
docker run -d --name orion-ext-a -p 8080:8080 \
  -v "$PWD/orion-ext.toml:/app/config.toml:ro" \
  -e REDIS_PASSWORD \
  -e ORION_ADMIN_AUTH__ENABLED=true \
  -e ORION_ADMIN_AUTH__API_KEYS="$ORION_ADMIN_API_KEYS" \
  orion-ext:local -c /app/config.toml
```

Confirm the node is ready and sees its store:

```bash
curl -s http://localhost:8080/readyz | jq '{status, components}'
```

```json
{
  "status": "ready",
  "components": {
    "database": "ok",
    "engine": "ok",
    "initialized": true,
    "background_tasks": "ok",
    "channel_drivers": { "rest": "ok", "http": "ok" },
    "cluster_store": { "name": "redis", "status": "ok" }
  }
}
```

A `cluster_store` other than `ok` means the node cannot use its shared state. `/readyz` then answers `503`, so the load balancer does not send it traffic.

## 5. Run the HA topology

The repository's `deploy/compose/ha.yml` is the production shape. It runs nginx in front of two `orion-ext` nodes in cluster mode, over one PostgreSQL and one Redis. A one-shot `migrate` service completes before either node starts.

```bash
export ORION_ADMIN_API_KEYS="$(openssl rand -hex 32)"
docker compose -f deploy/compose/ha.yml -f deploy/compose/ha.build.yml up -d --wait
curl -s http://localhost:8080/health | jq '{status, version}'
```

| Service | Role |
|---|---|
| `lb` | nginx, the only published port; round-robins to both nodes |
| `orion-a`, `orion-b` | The two nodes, each with its own `ORION_CLUSTER__INSTANCE_ID` |
| `postgres` | Deployment head and history, leases, cron ledger, breaker epochs |
| `redis` | The cluster store; no persistence, because every key has a TTL |
| `migrate` | One-shot migrations, completed before either node boots |

Each node mounts `deploy/compose/orion-ext.ha.toml` as `/app/config.toml`. That file holds only the extension sections; everything the environment can set is set in `ha.yml`. The stack boots with `ORION_ENVIRONMENT=production`, so admin auth is enforced and `ORION_ADMIN_API_KEYS` is required.

A third node is behind the `n3` profile, which also swaps the load balancer config:

```bash
HA_NGINX=nginx.n3.conf docker compose --profile n3 \
  -f deploy/compose/ha.yml -f deploy/compose/ha.build.yml up -d --wait
```

To run a registry image instead of a local build, set `ORION_EXT_IMAGE` and omit the build override:

```bash
export ORION_EXT_IMAGE="registry.example.com/orion-ext:2.0.0"
docker compose -f deploy/compose/ha.yml up -d --wait
```

## 6. Roll one node at a time

Give every node a grace period longer than its drain budget. The HA stack sets `stop_grace_period: 45s` against a 10-second drain and a 20-second force timeout. On `SIGTERM`, the node withdraws readiness, keeps serving through the drain window, then stops.

Roll a node and watch the load balancer:

```bash
docker compose -f deploy/compose/ha.yml stop orion-a
docker compose -f deploy/compose/ha.yml start orion-a
```

The rolling drill automates that under load and asserts zero non-2xx responses at the load balancer:

```bash
START_STACK=1 deploy/ha/rolling-drill.sh
```

The other drills prove the cluster's two coordination claims:

```bash
deploy/ha/plugin-drill.sh
deploy/ha/cron-drill.sh
```

`plugin-drill.sh` deploys a plugin through one node and watches it converge on every node, then deploys a new revision under load. `cron-drill.sh` proves one occurrence per scheduled instant across nodes. Both need `orion-cli`; the rolling drill does not.

## 7. Upgrade and pin

Pin the image by version tag or digest, never `latest`. The running version is the artifact; a config change does not upgrade it, and a new image does not redeploy your definitions.

- **Move the node and CLI together.** `orion-cli` speaks one v2 contract revision and refuses a server that reports another.
- **Keep the extension sections in the file.** `[extensions.licence]` and `[extensions.redis]` have no environment override, so a rendered config must still mount them.
- **Upgrade Redis before the nodes that depend on it.** The store is external state and is not migrated with the database. Its key prefix is stable across upgrades. A node pings it at boot and refuses to start when it does not answer.
- **Migrate once, then roll.** Run `migrate --dry-run` with the new image, apply migrations, then replace one node at a time.
- **Keep the previous image.** A rollback is the previous image against the same schema only when the release notes say the migration is compatible. Otherwise restore the database backup first.

The full order, including linting the definition set with the new binary, is in [Upgrade an instance](../maintain/upgrades.md#upgrade-an-edition). Back up the database before any of it; see [Back up and restore](../maintain/backup-restore.md).

## Verify

After the roll, confirm every node serves the head and reports the edition:

```bash
curl -s http://localhost:8080/api/v2/admin/deployments \
  -H "authorization: Bearer $ORION_ADMIN_KEY" \
  | jq '{head_revision, applied_revision}'
curl -s http://localhost:8080/api/v2/admin/runtime-capabilities \
  -H "authorization: Bearer $ORION_ADMIN_KEY" \
  | jq '{edition, admission_policy_id}'
```

`applied_revision` equals `head_revision` on every node, and the edition matches the label in your config file. Then confirm the drivers and providers the edition adds are registered:

```bash
orion-cli --server http://localhost:8080 --api-key $ORION_ADMIN_KEY \
  deployments capabilities
```

The answer lists the `kafka`, `graphql` and `sse` channel drivers and the seven edition provider backends. A Community node reports its own three drivers and built-in providers, but none of these. That is why an edition feature is refused there at upload rather than at run time.

## Next steps

- [Run a cluster](./cluster.md): convergence, leases, readiness and the store's role in each.
- [Extension settings](../../reference/configuration/extensions.md): every field of `[extensions.licence]` and `[extensions.redis]`.
- [Upgrade an instance](../maintain/upgrades.md): the order that works, including the edition's extra rules.
- [Deploy with Docker](./docker.md): the single-node shape and the settings this page assumes.
- [Edition capability matrix](../../reference/editions.md): every capability, with the edition that serves it.
