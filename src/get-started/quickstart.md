<!-- description: Run Orion locally, write a workflow and channel, test offline, deploy with orion-cli, and call your first live endpoint in under five minutes. -->
<!-- type: quickstart -->
<!-- last_verified: 2026-11-16 -->

# Quickstart

In this quickstart, you start an Orion server, define an order-triage service using two JSON files (`workflow.json` and `channel.json`), test the logic offline, deploy it, and call the live HTTP endpoint. The service inspects incoming orders and flags any order over `$10,000`.

## Before you start

Tested with Orion 2.0.0. You need:

- **`orion-cli`** and either **Docker** or **`orion-server`** installed (see [Install and run Orion](./install.md) if you need to install them—via Homebrew: `brew install GoPlasmatic/tap/orion-server GoPlasmatic/tap/orion-cli`)
- `curl` and `jq`
- Port `8080` free on your machine

## 1. Start the Orion server

Start Orion locally. It uses an embedded SQLite database by default, so there is no external database to set up:

<div class="tabs">
<section data-tab="CLI">

```bash
orion-server
```

</section>
<section data-tab="Docker">

```bash
docker run --name orion-quickstart -d -p 8080:8080 \
  ghcr.io/goplasmatic/orion:latest
```

</section>
</div>

In a new terminal, configure `orion-cli` to talk to your local server and verify that it is healthy:

<div class="tabs">
<section data-tab="CLI">

```bash
orion-cli config set-server http://localhost:8080
orion-cli health
```

</section>
<section data-tab="curl">

```bash
curl --retry 10 --retry-delay 1 --retry-all-errors -fsS \
  http://localhost:8080/healthz
```

</section>
</div>

Expected output:

```json
{"status":"ok"}
```

## 2. Create the workflow and channel

Create a directory called `quickstart-orders` for your service definitions:

```bash
mkdir -p quickstart-orders
```

A **workflow** defines the business logic as an ordered list of tasks. Create `quickstart-orders/workflow.json` with two tasks: one to parse the incoming JSON payload into `data.order`, and a second to flag the order when `data.order.total` exceeds `10000`:

```bash
cat > quickstart-orders/workflow.json <<'JSON'
{
  "workflow_id": "quickstart-orders",
  "name": "High-value order",
  "condition": true,
  "tasks": [
    {
      "id": "parse",
      "name": "Parse payload",
      "function": {
        "name": "parse_json",
        "input": { "source": "payload", "target": "order" }
      }
    },
    {
      "id": "flag",
      "name": "Flag order",
      "condition": { ">": [{ "var": "data.order.total" }, 10000] },
      "function": {
        "name": "map",
        "input": {
          "mappings": [
            { "path": "data.order.flagged", "logic": true },
            {
              "path": "data.order.alert",
              "logic": { "cat": ["High-value order: $", { "var": "data.order.total" }] }
            }
          ]
        }
      }
    }
  ]
}
JSON
```

A **channel** exposes the workflow on an HTTP route or schedule. Create `quickstart-orders/channel.json` to bind `POST /orders` to the `quickstart-orders` workflow:

```bash
cat > quickstart-orders/channel.json <<'JSON'
{
  "channel_id": "orders",
  "name": "orders",
  "channel_type": "sync",
  "protocol": "rest",
  "route_pattern": "/orders",
  "methods": ["POST"],
  "workflow_id": "quickstart-orders"
}
JSON
```

## 3. Test the workflow offline

Before deploying anything to the server, validate the definition set and run the workflow locally in `orion-cli`:

```bash
orion-cli lint quickstart-orders
orion-cli dry-run -w quickstart-orders/workflow.json \
  -i <(printf '%s' '{"order_id": "ORD-9182", "total": 25000}') | jq '.output.order'
```

Expected output:

```json
{
  "order_id": "ORD-9182",
  "total": 25000,
  "flagged": true,
  "alert": "High-value order: $25000"
}
```

`lint` and `dry-run` execute the real Orion workflow engine in-process without needing a running server or network connection.

## 4. Compile and deploy

Compile the `quickstart-orders` directory into a deployment bundle and apply it to your local server:

<div class="tabs">
<section data-tab="CLI">

```bash
orion-cli compile quickstart-orders -o dist/deployment
orion-cli deployments apply -d dist/deployment --yes
```

</section>
<section data-tab="curl">

```bash
WF=$(curl -fsS -X POST http://localhost:8080/api/v2/admin/definitions \
  -H 'Content-Type: application/json' \
  -d '{"kind": "workflow", "definition": {
    "name": "High-value order",
    "tasks": [
      { "id": "parse", "name": "Parse payload",
        "function": { "name": "parse_json",
                      "input": { "source": "payload", "target": "order" } } },
      { "id": "flag", "name": "Flag order",
        "condition": { ">": [{ "var": "data.order.total" }, 10000] },
        "function": { "name": "map", "input": { "mappings": [
          { "path": "data.order.flagged", "logic": true },
          { "path": "data.order.alert",
            "logic": { "cat": ["High-value order: $",
                               { "var": "data.order.total" }] } }
        ] } } }
    ]
  }}' | jq -r .digest)

CH=$(curl -fsS -X POST http://localhost:8080/api/v2/admin/definitions \
  -H 'Content-Type: application/json' \
  -d '{"kind": "channel", "definition": {
    "name": "orders", "channel_type": "sync", "protocol": "rest",
    "route_pattern": "/orders", "methods": ["POST"] }}' | jq -r .digest)

curl -fsS -X POST http://localhost:8080/api/v2/admin/deployments \
  -H 'Content-Type: application/json' \
  -d '{"idempotency_key": "quickstart-1", "base_revision": 0, "reason": "quickstart",
    "manifest": { "schema_version": 2, "providers": [], "members": [
      { "entity": { "kind": "workflow", "id": "quickstart-orders", "revision": 1 },
        "definition_digest": "'"$WF"'" },
      { "entity": { "kind": "channel", "id": "orders", "revision": 1 },
        "definition_digest": "'"$CH"'",
        "requires": [{ "kind": "workflow", "id": "quickstart-orders", "revision": 1 }] }
    ] } }'
```

</section>
</div>

When you apply the deployment, Orion uploads the content-addressed definitions, commits revision `1` as the server's active deployment head, compiles a new runtime generation, and swaps it in atomically without restarting or dropping requests.

## 5. Call the live endpoint

Send a high-value order (`total: 25000`) to your new service:

<div class="tabs">
<section data-tab="CLI">

```bash
orion-cli send orders -d '{"order_id": "ORD-9182", "total": 25000}'
```

</section>
<section data-tab="curl">

```bash
curl -fsS -X POST http://localhost:8080/api/v2/data/orders \
  -H 'Content-Type: application/json' \
  -d '{ "data": { "order_id": "ORD-9182", "total": 25000 } }'
```

</section>
</div>

Expected response (the `id` is the unique execution trace ID for this call):

```json
{
  "id": "01a1738f-4070-70ba-abbb-e76b63dd57e7",
  "status": "ok",
  "data": {
    "order_id": "ORD-9182",
    "total": 25000,
    "order": {
      "order_id": "ORD-9182",
      "total": 25000,
      "flagged": true,
      "alert": "High-value order: $25000"
    }
  },
  "errors": []
}
```

## Verify the condition

Send an order below the threshold (`total: 50`) to confirm that the `flag` task only runs when its condition matches:

<div class="tabs">
<section data-tab="CLI">

```bash
orion-cli send orders -d '{"order_id": "ORD-0001", "total": 50}'
```

</section>
<section data-tab="curl">

```bash
curl -fsS -X POST http://localhost:8080/api/v2/data/orders \
  -H 'Content-Type: application/json' \
  -d '{ "data": { "order_id": "ORD-0001", "total": 50 } }'
```

</section>
</div>

This time `data.order` returns only `order_id` and `total`. Because `total > 10000` evaluated to `false`, Orion skipped the `flag` task.

## Clean up

If you ran Orion in Docker, stop and remove the container:

```bash
docker stop orion-quickstart && docker rm orion-quickstart
```

Remove the temporary files if you no longer need them:

```bash
rm -rf quickstart-orders dist
```

## Next steps

- [Build your first service](./first-service.md) — Walk through request envelopes, execution traces, and rolling back deployments.
- [Build a Postgres-backed orders API](./postgres-orders-api.md) — Connect a workflow to PostgreSQL, validate inputs, and run offline regression tests with connector stubs.
- [Architecture and request lifecycle](../concepts/architecture.md) — Understand how Orion routes requests, runs ingress guards, and swaps deployments at runtime.
- [When to use Orion](./when-to-use-orion.md) — See where Orion fits best and how it compares to API gateways, Temporal, and rule engines.
