<!-- description: Set up CI/CD pipelines for Orion: offline pull-request validation gates, capability-aware compilation, staged plan and apply, rollbacks, and Orion Console. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# CI/CD and deployment workflows

An Orion delivery pipeline has three stages:
1. **Validate offline on pull requests** (`fmt`, `lint`, `clippy`, `test`, `compile`) with zero server or database dependencies.
2. **Plan and apply to staging** against the target node's runtime capabilities.
3. **Promote to production** with compare-and-swap protection, cluster convergence verification, and one-command rollback.

## Before you start

Tested with Orion 2.0.0. You need Git, `orion-cli`, a repository holding your service definitions, and an admin API key (`ORION_API_KEY`) for your target Orion instances.

---

## 1. Repository layout

Keep your human-authored definition files in Git and exclude the compiled output directory (`dist/`) in `.gitignore`:

```text
services/
├── package.json                 # Optional: { "package": { "name": "core", "requires": { "orion": ">=2.0.0, <3" } } }
├── shared.json                  # Optional: shared constants, errors, and fragments
├── orders/
│   ├── workflow.json
│   ├── channel.json
│   ├── connector.json
│   └── tests/
│       └── valid-order.case.json
└── notifications/
    ├── workflow.json
    └── channel.json
```

Compile the entire `services/` tree into a single deployment directory during CI:

```bash
orion-cli compile services -o dist/deployment
```

---

## 2. Pull request validation gate (offline)

Because `fmt`, `lint`, `clippy`, `test`, and `compile` run entirely offline, your PR workflow requires no secrets or network access to an Orion server:

```yaml
name: Validate Definitions

on:
  pull_request:
    paths:
      - 'services/**'

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install orion-cli
        run: |
          curl --proto '=https' --tlsv1.2 -LsSf \
            https://github.com/GoPlasmatic/Orion-cli/releases/latest/download/orion-cli-installer.sh | sh
          echo "$HOME/.cargo/bin" >> "$GITHUB_PATH"

      - name: Check formatting
        run: orion-cli fmt --check services

      - name: Lint definition set
        run: orion-cli lint services --deny-warnings

      - name: Run static analysis
        run: orion-cli clippy services

      - name: Run offline regression tests
        run: orion-cli test services

      - name: Compile deployment bundle
        run: orion-cli compile services -o dist/deployment
```

---

## 3. Deploy to staging and promote to production

When deploying to a live server—especially a Standard or Enterprise (`orion-ext`) instance—capture the target node's capabilities first (`orion-cli deployments capabilities`) so `lint`, `clippy`, and `compile` validate against the exact channel drivers and providers registered on that target:

```yaml
name: Deploy to Production

on:
  push:
    tags:
      - 'release-v*'

jobs:
  deploy:
    runs-on: ubuntu-latest
    environment: production
    env:
      ORION_SERVER_URL: https://orion.internal.example.com
      ORION_API_KEY: ${{ secrets.ORION_PROD_API_KEY }}
      ORION_CHANGE_CONTEXT: "workflow=${{ github.workflow }},sha=${{ github.sha }}"
    steps:
      - uses: actions/checkout@v4

      - name: Install orion-cli
        run: |
          curl --proto '=https' --tlsv1.2 -LsSf \
            https://github.com/GoPlasmatic/Orion-cli/releases/latest/download/orion-cli-installer.sh | sh
          echo "$HOME/.cargo/bin" >> "$GITHUB_PATH"

      - name: Capture target capabilities
        run: orion-cli deployments capabilities --output json > target-capabilities.json

      - name: Validate and compile for target
        run: |
          orion-cli lint services --deny-warnings --capabilities target-capabilities.json
          orion-cli clippy services --capabilities target-capabilities.json
          orion-cli compile services -o dist/deployment --capabilities target-capabilities.json

      - name: Plan deployment
        run: orion-cli deployments plan -d dist/deployment

      - name: Apply deployment and wait for cluster convergence
        run: |
          orion-cli deployments apply -d dist/deployment \
            --yes \
            --reason "Deploy ${{ github.ref_name }}" \
            --wait-ms 30000 \
            --wait-target observed
```

### Why this pipeline is safe
- **Idempotent retries**: `deployments apply -d` derives an idempotency key from the current head and manifest digest. If a network blip causes the CI step to retry, the server replays the receipt without committing a duplicate revision.
- **No-op on unchanged definitions**: If the server's head revision already serves the exact definitions in `dist/deployment`, `apply -d` reports that the head is up to date and makes no changes.
- **Audit context (`ORION_CHANGE_CONTEXT`)**: Setting `ORION_CHANGE_CONTEXT` (or passing `--change-context`) attaches your CI run or ticket identifier to every admin audit log row written during the deployment.
- **Edition enforcement (`--require-edition`)**: Add `--require-edition standard` (or `enterprise`) to fail immediately before uploading if the target server is running a lower edition than your services require.

---

## 4. Roll back a deployment

If metrics or traces show an issue after deploying revision `8`, roll back to revision `7`:

<div class="tabs">
<section data-tab="CLI">

```bash
orion-cli deployments rollback \
  --target-revision 7 \
  --expected-head-revision 8 \
  --idempotency-key rollback-rev-8
```

</section>
<section data-tab="curl">

```bash
curl -sS -X POST http://localhost:8080/api/v2/admin/deployments \
  -H "Authorization: Bearer $ORION_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"idempotency_key": "rollback-rev-8", "base_revision": 8, "rollback_to": 7}'
```

</section>
</div>

Rolling back commits revision `7`'s manifest as revision `9` and triggers an atomic hot reload across all nodes.

---

## 5. Inspecting operations from the CLI and Orion Console

On Orion 2.0 nodes, `orion-cli` provides terminal views for every operational surface:

| Operational View | CLI Command |
| :--- | :--- |
| **Instance & component health** | `orion-cli health` |
| **Loaded workflows, channels & quarantine issues** | `orion-cli engine status` |
| **Deployment history & convergence** | `orion-cli deployments list` · `orion-cli deployments get <REV>` |
| **Send test requests & profile execution** | `orion-cli send <channel> -d '<json>' --profile` |
| **Search & inspect execution traces** | `orion-cli traces list` · `orion-cli traces get <TRACE_ID>` |
| **Scheduled cron occurrences** | `orion-cli cron status` · `orion-cli cron list` |
| **Circuit breaker states** | `orion-cli circuit-breakers list` |
| **Admin audit trail** | `orion-cli audit-logs list` |

> [!NOTE]
> The browser-based [Orion UI Console](https://github.com/GoPlasmatic/Orion-ui) (`ghcr.io/goplasmatic/orion-ui`) currently targets Orion 1.x (`/api/v1/*`). For Orion 2.0 instances, use the `orion-cli` commands above and your Prometheus/Grafana dashboards against `GET /metrics`.

## Next steps

- [Canary rollouts and traffic splitting](../enterprise/traffic-splitting.md) — Gradually shift traffic between two workflow revisions.
- [Production checklist](../operate/production-checklist.md) — Harden your Orion instances before accepting production traffic.
- [CLI reference](../reference/cli.md) — Complete reference for `orion-cli deployments`.
