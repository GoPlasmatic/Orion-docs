<!-- description: Deploy Orion in production with Docker Compose or the official Kubernetes Helm chart: volumes, environment settings, health probes, and migrations. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Deploy with Docker and Kubernetes

Orion ships as a self-contained container image (`ghcr.io/goplasmatic/orion`) with SQLite, PostgreSQL, MySQL, TLS, and OpenTelemetry support compiled in, alongside an official Kubernetes Helm chart (`oci://ghcr.io/goplasmatic/charts/orion`).

---

## Part 1: Deploy with Docker and Docker Compose

### Persist SQLite on a volume

> [!WARNING]
> By default, SQLite writes inside the container's ephemeral filesystem. Always mount a named volume or host directory and set `ORION_STORAGE__URL` so your deployments, traces, and audit logs survive container restarts and image upgrades.

```bash
docker run -d --name orion \
  -p 8080:8080 \
  -v orion-data:/app/data \
  -e ORION_STORAGE__URL="sqlite:/app/data/orion.db" \
  ghcr.io/goplasmatic/orion:2.0.0-rc.1
```

### Single-node production `docker-compose.yml`

Every Community setting maps to an `ORION_SECTION__KEY` environment variable. Unknown or misspelled `ORION_*` variables fail startup immediately with a suggestion for the closest valid key.

```yaml
services:
  orion:
    image: ghcr.io/goplasmatic/orion:2.0.0-rc.1
    ports:
      - "8080:8080"
    environment:
      ORION_ENVIRONMENT: "production"
      ORION_STORAGE__URL: "sqlite:/app/data/orion.db"
      ORION_ADMIN_AUTH__ENABLED: "true"
      ORION_ADMIN_AUTH__API_KEYS: "${ORION_ADMIN_API_KEYS:?set ORION_ADMIN_API_KEYS}"
      ORION_LOGGING__FORMAT: "json"
      ORION_METRICS__ENABLED: "true"
      ORION_SERVER__SHUTDOWN_DRAIN_SECS: "15"
      ORION_SERVER__SHUTDOWN_FORCE_TIMEOUT_SECS: "20"
    volumes:
      - orion-data:/app/data
    stop_grace_period: 45s
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/healthz"]
      interval: 5s
      timeout: 3s
      start_period: 10s
      retries: 3

volumes:
  orion-data:
```

> [!IMPORTANT]
> Always set `stop_grace_period` higher than `ORION_SERVER__SHUTDOWN_DRAIN_SECS + ORION_SERVER__SHUTDOWN_FORCE_TIMEOUT_SECS` (`15s + 20s = 35s` above) so Docker does not `SIGKILL` the container while in-flight requests are draining.

### Multi-node HA Docker Compose (`orion-ext`) <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

For a multi-replica Standard or Enterprise cluster, mount a TOML config file carrying `[extensions.licence]` and `[extensions.redis]` (which do not accept `ORION_EXTENSIONS__` environment overrides), run database migrations in a one-shot `migrate` service before starting replicas, and place a load balancer probing `/readyz` in front:

```bash
export ORION_ADMIN_API_KEYS="$(openssl rand -hex 32)"
docker compose -f deploy/compose/ha.yml -f deploy/compose/ha.build.yml up -d --wait
curl -s http://localhost:8080/health | jq '{status, workflows_loaded}'
```

See [Multi-node clustering and HA](../enterprise/clustering.md) for the complete cluster configuration reference.

---

## Part 2: Deploy on Kubernetes with Helm

The official Helm chart is published as an OCI artifact at `oci://ghcr.io/goplasmatic/charts/orion`. It deploys:
- `N` stateless Orion replicas behind a `Service` with `RollingUpdate` (`maxUnavailable: 0`, `maxSurge: 1`).
- A `pre-install` / `pre-upgrade` migration `Job` (`orion-server migrate`) that completes before new pods start.
- A dedicated Prometheus metrics port (`9090`) separated from the public HTTP port (`8080`).
- Restricted pod security defaults (non-root UID `10001`, read-only root filesystem, dropped capabilities, `RuntimeDefault` seccomp profile).

### Quick evaluation with `devStack`

For a disposable local or staging test in Kubernetes, `devStack.enabled=true` deploys an in-namespace PostgreSQL container and sets `ORION_ENVIRONMENT=development`:

```bash
helm install orion oci://ghcr.io/goplasmatic/charts/orion \
  --version 1.0.0 \
  --set devStack.enabled=true
kubectl port-forward svc/orion 8080:8080
```

### Production Helm installation

In production (`env: production`, the chart default), provide your PostgreSQL/MySQL connection string and admin API keys via Kubernetes Secrets so credentials never appear in Helm release history:

```bash
kubectl create secret generic orion-storage \
  --from-literal=storage-url="postgres://orion:secret@postgres.internal:5432/orion"

kubectl create secret generic orion-admin-auth \
  --from-literal=api-keys="$(openssl rand -hex 32)"
```

Create `values.yaml`:

```yaml
replicaCount: 2

storage:
  existingSecret: orion-storage      # Reads key: storage-url
  autoMigrate: false

adminAuth:
  existingSecret: orion-admin-auth   # Reads key: api-keys (comma-separated)

cors:
  allowedOrigins:
    - https://console.example.com

metrics:
  enabled: true
  port: 9090
  serviceMonitor:
    enabled: true

ingress:
  enabled: true
  className: nginx
  hosts:
    - host: orion.example.com
      paths:
        - path: /
          pathType: Prefix
  tls:
    - secretName: orion-tls
      hosts:
        - orion.example.com
```

Install and verify the release using the built-in `helm test` hooks:

```bash
helm install orion oci://ghcr.io/goplasmatic/charts/orion \
  --version 1.0.0 \
  -f values.yaml

helm test orion
```

### Key Helm chart values

| Value | Default | Description |
|---|---|---|
| `replicaCount` | `2` | Number of replicas (ignored when `autoscaling.enabled: true`). |
| `image.repository` / `image.tag` | `ghcr.io/goplasmatic/orion` / `appVersion` | Container image repository and tag. |
| `env` | `"production"` | Sets `ORION_ENVIRONMENT`. Enforces admin auth and rejects wildcard CORS. |
| `storage.url` / `storage.existingSecret` | — | Database URL or Secret name containing `storage-url`. |
| `storage.autoMigrate` | `false` | Keep `false` when using `migrateJob.enabled: true`. |
| `migrateJob.enabled` | `true` | Runs `orion-server migrate` as a `pre-install`/`pre-upgrade` hook Job. |
| `cluster.enabled` | `false` | Enables cluster mode (`Standard+`; requires mounting `[extensions.redis]` in a config file). |
| `adminAuth.apiKeys` / `adminAuth.existingSecret` | — | Admin keys or Secret name containing comma-separated `api-keys`. |
| `server.shutdownDrainSecs` | `15` | Readiness drain window on `SIGTERM` before stopping listener accept. |
| `server.shutdownForceTimeoutSecs` | `20` | Maximum wait for in-flight requests after drain (`terminationGracePeriodSeconds` is set to `drain + force + 10`). |
| `metrics.enabled` / `metrics.port` | `true` / `9090` | Exposes unauthenticated `/metrics` on a cluster-internal port separate from `8080`. |
| `podDisruptionBudget.enabled` | `true` | Enforces `maxUnavailable: 1` during voluntary node drains. |
| `networkPolicy.enabled` | `false` | Restricts pod ingress and egress to declared rules. |
| `persistence.enabled` | `false` | Mounts a `ReadWriteOnce` PVC at `/app/data` for single-replica SQLite installs. |
| `extraEnv` | `[]` | Additional `ORION_*` environment variables (such as `ORION_PLUGINS__ENABLED`). |

### Single-replica SQLite on Kubernetes

If you run a single-node SQLite instance on Kubernetes with a `ReadWriteOnce` PersistentVolumeClaim, disable the separate migration Job (which cannot mount the RWO volume concurrently) and use the `Recreate` deployment strategy:

```bash
helm install orion oci://ghcr.io/goplasmatic/charts/orion --version 1.0.0 \
  --set storage.url="sqlite:/app/data/orion.db" \
  --set persistence.enabled=true \
  --set cluster.enabled=false \
  --set replicaCount=1 \
  --set strategy.type=Recreate \
  --set migrateJob.enabled=false \
  --set storage.autoMigrate=true \
  --set adminAuth.existingSecret=orion-admin-auth
```

## Next steps

- [Production checklist](./production-checklist.md): Verify all security, resilience, and observability settings before go-live.
- [Multi-node clustering and HA](../enterprise/clustering.md): Configure `[extensions.redis]` and `[extensions.licence]` for Standard+ clusters.
- [Backups, upgrades, and troubleshooting](./maintenance.md): Upgrade procedures and symptom troubleshooting.
