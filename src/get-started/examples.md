<!-- description: Explore and deploy the 11 ready-to-run Orion example packages, including REST APIs, webhooks, cron jobs, service composition, and WebAssembly plugins. -->
<!-- type: tutorial -->
<!-- last_verified: 2026-11-16 -->

# Run the example packages

The Orion repository includes 11 ready-to-deploy example packages under [`examples/packages/`](https://github.com/GoPlasmatic/Orion/tree/main/examples/packages). Each package directory is a complete definition set that you can lint, test offline, compile, and deploy to a running server in two commands.

## Before you start

Tested with Orion 2.0.0. You need:

- Git, `curl`, `orion-server`, and `orion-cli`
- A running Orion server on `http://localhost:8080`
- Docker with Compose (only for `postgres-orders`)

Clone the Orion repository to access the examples:

```bash
git clone https://github.com/GoPlasmatic/Orion.git
cd Orion/examples
```

## 1. Compile and deploy an example package

With `orion-server` running, compile any package directory and apply it:

```bash
orion-cli config set-server http://localhost:8080
orion-cli compile packages/high-value-order -o /tmp/high-value-order
orion-cli deployments apply -d /tmp/high-value-order --yes
```

`orion-cli compile` validates all workflows, channels, and connectors in the directory and writes a deployment bundle (automatically skipping `request.json`, which is a sample payload). `orion-cli deployments apply -d` uploads the definitions and commits the deployment revision.

Send the package's sample request:

<div class="tabs">
<section data-tab="CLI">

```bash
orion-cli send high-value-orders -d "$(jq -c '.data' packages/high-value-order/request.json)"
```

</section>
<section data-tab="curl">

```bash
curl -sS -X POST http://localhost:8080/api/v2/data/high-value-orders \
  -H 'Content-Type: application/json' \
  --data @packages/high-value-order/request.json | jq
```

</section>
</div>

> [!TIP]
> Because a deployment represents everything a node serves, applying a second package directory replaces the first. To deploy multiple example packages side by side in a single deployment, use the helper script included in `examples/`:
> ```bash
> ./deploy.sh order-classification webhook-transform
> ```

## 2. Available example packages

| Package | Endpoint / Trigger | What it demonstrates |
| :--- | :--- | :--- |
| [`order-summary`](https://github.com/GoPlasmatic/Orion/tree/main/examples/packages/order-summary) | `POST /order-summary` | Minimal two-task workflow (`parse_json` + `map`) from [Build your first service](./first-service.md). |
| [`high-value-order`](https://github.com/GoPlasmatic/Orion/tree/main/examples/packages/high-value-order) | `POST /high-value-orders` | Conditional task execution (`>` threshold) and string formatting with `cat`. |
| [`order-classification`](https://github.com/GoPlasmatic/Orion/tree/main/examples/packages/order-classification) | `POST /order-tiers` | Multi-tier classification using mutually exclusive task-level conditions. |
| [`iot-sensor-alert`](https://github.com/GoPlasmatic/Orion/tree/main/examples/packages/iot-sensor-alert) | `POST /sensors` | Range-based alerting using compound `and` / `or` JSONLogic expressions. |
| [`webhook-transform`](https://github.com/GoPlasmatic/Orion/tree/main/examples/packages/webhook-transform) | `POST /webhooks` | Normalizing external webhook payloads with null-safe `var` defaults. |
| [`notification-routing`](https://github.com/GoPlasmatic/Orion/tree/main/examples/packages/notification-routing) | `POST /notifications` | Progressive routing rules using the `in` set-membership operator. |
| [`postgres-orders`](https://github.com/GoPlasmatic/Orion/tree/main/examples/packages/postgres-orders) | `POST /record-order` | **Database connector:** `validation`, `data_write` insert, and `data_query` with relation hydration over PostgreSQL. |
| [`channel-composition`](https://github.com/GoPlasmatic/Orion/tree/main/examples/packages/channel-composition) | `POST /order-enrichment` | **Service composition:** Two channels where one invokes the other in-process via `channel_call`. |
| [`nightly-rollup`](https://github.com/GoPlasmatic/Orion/tree/main/examples/packages/nightly-rollup) | `0 15 2 * * *` (cron) | **Scheduled job:** A `protocol: "cron"` channel that executes on a schedule and records durable occurrences. |
| [`fixed-width-statement`](https://github.com/GoPlasmatic/Orion/tree/main/examples/packages/fixed-width-statement) | `POST /statements` | **WebAssembly plugin:** Calls a compiled Rust Wasm codec ([source](https://github.com/GoPlasmatic/Orion/tree/main/examples/plugins/fixed-width)) to parse fixed-width bank records. Requires `plugins.enabled = true`. |
| [`c4-referee`](https://github.com/GoPlasmatic/Orion/tree/main/examples/packages/c4-referee) | `POST /c4/move` | **WebAssembly plugin:** Evaluates Connect Four game rules inside a Wasm sandbox ([source](https://github.com/GoPlasmatic/Orion/tree/main/examples/plugins/c4-rules)). Requires `plugins.enabled = true`. |

## 3. What is inside a package directory

Each package directory follows a standard layout:

| File | Compiled Artifact | Purpose |
| :--- | :--- | :--- |
| `workflow.json` (and `workflow-*.json`) | Workflow definition (`definitions/<sha256>.json`) | Task pipeline and JSONLogic conditions. |
| `channel.json` (and `channel-*.json`) | Channel definition (`definitions/<sha256>.json`) | Route or schedule and ingress guards. |
| `connector.json` *(optional)* | Connection spec (`connections/<name>.json`) | External database, HTTP, or cache connection with secret references. |
| `plugin.toml` + `.wasm` *(optional)* | Plugin definition (`definitions/<sha256>.json`) | WebAssembly component and manifest, inlined as base64. |
| `request.json` | Ignored by `compile` | Sample request payload for testing the live endpoint. |

## 4. Run the offline regression suite

All example packages are tested offline in [`examples/workflow-tests/`](https://github.com/GoPlasmatic/Orion/tree/main/examples/workflow-tests) using `*.case.json` files. Run the full suite locally without a server or database:

```bash
orion-cli test workflow-tests \
  --plugin-dir packages/fixed-width-statement \
  --plugin-dir packages/c4-referee
```

The `--plugin-dir` flags load the compiled WebAssembly components so the plugin-backed test cases execute the real Wasm guest code inside the sandbox.

## Next steps

- [Author workflows](../guides/workflows.md) — Write your own workflows using conditions, task groups, loops, and shared fragments.
- [Test and lint offline](../guides/testing.md) — Write `*.case.json` regression suites and connector stubs for your own services.
- [Extend with WebAssembly plugins](../guides/plugins.md) — Build and sign your own Rust WebAssembly plugins with `orion-plugin-sdk`.
