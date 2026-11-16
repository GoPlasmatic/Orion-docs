<!-- description: Run Orion as a container: one image with every backend compiled in, a durable volume, drain settings, plus the Standard+ Orion Ext HA stack. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Deploy with Docker

Orion ships as a container image with everything compiled in: all three database backends, OTLP export, TLS. There is nothing to install alongside it. This guide is what to change before the container holds anything you care about.

## Before you start

You need Docker Engine or Docker Desktop, and for the HA topology, Docker Compose. `docker run -p 8080:8080 ghcr.io/goplasmatic/orion:latest` gives you a working instance; the rest of this page hardens it.

## Give SQLite a volume

> [!WARNING]
> The default SQLite database lives inside the container. Without a volume, every workflow, channel and connector you create is lost when the container is replaced, which includes every image upgrade.

Mount a volume and point the storage URL at it:

```bash
docker run -p 8080:8080 \
  -v orion-data:/app/data \
  -e ORION_STORAGE__URL=sqlite:/app/data/orion.db \
  ghcr.io/goplasmatic/orion:latest
```

Both halves are needed. The volume gives the file somewhere durable to live, and `ORION_STORAGE__URL` puts it there. As a compose file:

```yaml
services:
  orion:
    image: ghcr.io/goplasmatic/orion:1.12.0
    ports: ["8080:8080"]
    environment:
      ORION_STORAGE__URL: sqlite:/app/data/orion.db
      ORION_LOGGING__FORMAT: json
    volumes:
      - orion-data:/app/data
    stop_grace_period: 45s

volumes:
  orion-data:
```

Pin the tag. `latest` makes an upgrade something that happens to you rather than something you decide.

## Configure it

Every setting is an `ORION_SECTION__KEY` environment variable, which is what makes the image configurable without a config file:

```yaml
environment:
  ORION_STORAGE__URL: "postgres://user:pass@db:5432/orion"
  ORION_ADMIN_AUTH__ENABLED: "true"
  ORION_ADMIN_AUTH__API_KEYS: "${ORION_ADMIN_API_KEYS:?set this}"
  ORION_METRICS__ENABLED: "true"
  ORION_ENVIRONMENT: "production"
```

A name that is not a real setting is refused at startup with the nearest match, rather than silently ignored. A typo costs you a boot rather than a week. Mount a TOML file and pass `-c` if you prefer files; the environment still overrides it.

## Run the Orion Ext image <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

Available in Orion Standard and Enterprise only. `orion-ext` is one artifact that composes Community's node with every Orion Ext driver, provider and the Redis cluster store. It is a container image like Community's, built from the Orion Ext checkout:

```bash
docker build -t orion-ext:local .
docker run -p 8080:8080 \
  -v "$PWD/orion-ext.toml:/app/config.toml" \
  orion-ext:local
```

The image reads the same `ORION_*` environment and the same `-c` flag. Two sections are not available as environment variables: `[extensions.licence]` and `[extensions.redis]`. Mount them as the config file:

```toml
[extensions.licence]
tier = "enterprise"

[extensions.redis]
host = "redis.internal"
tls = true
password = "env://REDIS_PASSWORD"
```

`ORION_EXTENSIONS__` is not an override; a node given that shape refuses to start. No published tag is assumed, so pin or build the image yourself.

## Give the container time to drain

Orion's shutdown sequence takes `shutdown_drain_secs + shutdown_force_timeout_secs` to complete. If Docker's grace period is shorter, the process is killed mid-drain and in-flight requests die with it:

```yaml
stop_grace_period: 45s      # > ORION_SERVER__SHUTDOWN_DRAIN_SECS + FORCE_TIMEOUT_SECS
```

The sequence itself is in [Shut down without dropping requests](../run/failure-handling.md#shut-down-without-dropping-requests).

## Verify

The image needs no extra configuration for health checks:

```yaml
healthcheck:
  test: ["CMD", "curl", "-f", "http://localhost:8080/healthz"]
  interval: 5s
  timeout: 3s
  start_period: 10s
  retries: 3
```

Use `/healthz` for "is the process alive" and `/readyz` for "should it get traffic". A load balancer wants the second one. Then confirm the estate built:

```bash
curl -s http://localhost:8080/health | jq '{status, workflows_loaded}'
```

## The reference HA topology

`deploy/compose/ha.yml` in the Orion Ext repository is the production shape for a Standard or Enterprise cluster. It runs nginx, two `orion-ext` nodes in cluster mode, one shared PostgreSQL, one Redis store and a one-shot `migrate` service. The migration completes before either node boots:

```bash
export ORION_ADMIN_API_KEYS="$(openssl rand -hex 32)"
docker compose -f deploy/compose/ha.yml -f deploy/compose/ha.build.yml up -d --wait
curl -s http://localhost:8080/health
```

The nodes read `deploy/compose/orion-ext.ha.toml`, mounted at `/app/config.toml`, for `[extensions.redis]` and `[extensions.licence]`. Everything the environment can set is in the compose file; the extension sections are the part it cannot set. Cluster mode is Standard+ because Community ships no cluster store.

The stack boots as `ORION_ENVIRONMENT=production`, so admin auth is enforced and `ORION_ADMIN_API_KEYS` is required. Set `ORION_CORS_ALLOWED_ORIGINS` if a browser dashboard needs it.

`ha.build.yml` builds the checkout as `orion-ext:local`. No published tag is assumed. To run a registry image, set `ORION_EXT_IMAGE` and leave the build override out.

A third node sits behind the `n3` profile with a matching nginx config:

```bash
HA_NGINX=nginx.n3.conf docker compose --profile n3 \
  -f deploy/compose/ha.yml -f deploy/compose/ha.build.yml up -d --wait
```

What it demonstrates, and what to copy into your own topology:

- **`auto_migrate = false` on the replicas**, with migrations applied once by a separate service before either node starts.
- **Cluster mode on**, so config changes made through either node reach both. Dedup claims, the response cache and rate limits are shared through Redis; job leases and the cron ledger stay on PostgreSQL.
- **Readiness that includes the store**, so a node whose Redis is unreachable leaves the load balancer before dedup fails open.
- **`stop_grace_period` above the drain budget**, so `SIGTERM` runs the graceful sequence instead of being cut short.

`deploy/ha/rolling-drill.sh` drives traffic through the load balancer while one node is `SIGTERM`ed, and asserts every response was a 2xx. Run it to prove the drain settings on your hardware, not only on the reference one.

## Upgrade a compose deployment

1. **Back up the database**: a SQLite volume snapshot, or your database's own tooling. See [Back up and restore](../maintain/backup-restore.md).
2. **Read the version's upgrade guide** and run `orion-cli lint` with the new image over the definition set you deploy. See [Upgrade an instance](../maintain/upgrades.md).
3. **Bump the pinned tag** and, in a cluster, let the `migrate` service apply migrations before the replicas restart.
4. **Restart one node at a time** so the load balancer always has a node to send to. With a single node there is a gap, which is the argument for the second one.

## Run without a container

Orion is also a plain binary, and nothing about it requires Docker. Install it from a release or Homebrew, put the config somewhere readable, and run `orion-server -c /etc/orion/config.toml` under whatever supervises processes on that host.

Orion ships container images and a Helm chart, not a systemd unit. If you run it under systemd, you write the unit file. Give it the same two properties this page is about: a durable path for the database, and a stop timeout longer than the drain budget.

## Next steps

- [Deploy Orion Ext](./orion-ext.md): the licensed artifact, the edition config and the HA stack in full.
- [Run a cluster](./cluster.md): what the HA compose file is configuring, and why.
- [Deploy on Kubernetes](./kubernetes.md): the same shape, on Kubernetes.
- [Production checklist](../production-checklist.md): before this becomes production.
- [Server configuration](../../reference/configuration/index.md): every environment variable named here.
