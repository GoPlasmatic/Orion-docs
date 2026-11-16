<!-- description: Build your first Orion service from scratch: write a workflow and channel, test offline, deploy a revision, inspect traces, and roll back safely. -->
<!-- type: tutorial -->
<!-- last_verified: 2026-11-16 -->

# Build your first service

An Orion service consists of a **workflow** (which defines the processing steps) and a **channel** (which exposes the endpoint and enforces ingress policies). In this tutorial, you write both files from scratch, test them offline, compile and deploy them to a local server, inspect the execution trace, and practice updating and rolling back a revision.

## What you will learn

- How a workflow and a channel fit together as separate JSON definitions.
- How request payloads enter the `data` context and how tasks read and write fields.
- How `orion-cli lint`, `dry-run`, `compile`, `deployments plan`, and `deployments apply` work together.
- How to inspect execution traces and roll back to a previous deployment revision.

## Before you start

Tested with Orion 2.0.0-rc.1. You need:

- `orion-server` running on `http://localhost:8080` (see [Install and run Orion](./install.md))
- `orion-cli`, `curl`, and `jq` installed on your `PATH`

Create a directory for your definition set:

```bash
mkdir -p order-summary
```

The files you create below match the [`examples/packages/order-summary/`](https://github.com/GoPlasmatic/Orion/tree/main/examples/packages/order-summary) package in the Orion repository.

## 1. Write the workflow

Create `order-summary/workflow.json`. This workflow runs two tasks in sequence:
1. `parse`: Uses `parse_json` to parse the incoming request payload and place it at `data.req`.
2. `summarize`: Uses `map` and the JSONLogic `cat` operator to build a formatted summary string at `data.req.summary`.

```bash
cat > order-summary/workflow.json <<'JSON'
{
  "workflow_id": "order-summary",
  "name": "Order Summary",
  "condition": true,
  "tasks": [
    {
      "id": "parse",
      "name": "Parse",
      "function": {
        "name": "parse_json",
        "input": { "source": "payload", "target": "req" }
      }
    },
    {
      "id": "summarize",
      "name": "Summarize",
      "function": {
        "name": "map",
        "input": {
          "mappings": [
            {
              "path": "data.req.summary",
              "logic": {
                "cat": [
                  "Order ",
                  { "var": "data.req.order_id" },
                  ": $",
                  { "var": "data.req.total" }
                ]
              }
            }
          ]
        }
      }
    }
  ]
}
JSON
```

## 2. Write the channel

Create `order-summary/channel.json`. This channel exposes a synchronous REST endpoint at `POST /order-summary` and routes incoming requests to the `order-summary` workflow:

```bash
cat > order-summary/channel.json <<'JSON'
{
  "channel_id": "order-summary",
  "name": "order-summary",
  "channel_type": "sync",
  "protocol": "rest",
  "route_pattern": "/order-summary",
  "methods": ["POST"],
  "workflow_id": "order-summary"
}
JSON
```

 workflows and channels are separate files because a single workflow can be triggered by multiple channels—for example, both a synchronous REST endpoint and a nightly cron schedule.

## 3. Lint and dry-run offline

Before touching the server, check your directory for syntax, schema, or cross-reference errors:

```bash
orion-cli fmt order-summary
orion-cli lint order-summary
orion-cli clippy order-summary
```

Next, run the workflow offline against a sample input using `orion-cli dry-run`:

```bash
orion-cli dry-run -w order-summary/workflow.json \
  -i <(printf '%s' '{"order_id": "ORD-42", "total": 125}') | jq '{matched, output}'
```

Expected output:

```json
{
  "matched": true,
  "output": {
    "order_id": "ORD-42",
    "total": 125,
    "req": {
      "order_id": "ORD-42",
      "total": 125,
      "summary": "Order ORD-42: $125"
    }
  }
}
```

## 4. Compile, plan, and deploy

`orion-cli compile` validates the definition set and writes a content-addressed deployment bundle to `dist/deployment`:

```bash
orion-cli compile order-summary -o dist/deployment
```

Preview what will change on the server with `deployments plan`, then commit the deployment with `deployments apply`:

```bash
orion-cli deployments plan -d dist/deployment
orion-cli deployments apply -d dist/deployment --yes
```

`plan` prints the diff against the server's current head revision (`+ channel order-summary@1` and `+ workflow order-summary@1`) without making changes. `apply` commits those changes as the next immutable revision and hot-reloads the engine with zero downtime.

> [!IMPORTANT]
> A deployment defines the **entire set** of services a node serves. Any channel or workflow from a previous revision that is not included in the new deployment directory is shown as removed (`-`) in the plan and stops serving when applied.

## 5. Call the endpoint and inspect the trace

Send a request to your live endpoint:

<div class="tabs">
<section data-tab="CLI">

```bash
orion-cli send order-summary -d '{"order_id": "ORD-42", "total": 125}'
```

</section>
<section data-tab="curl">

```bash
curl -s -X POST http://localhost:8080/api/v2/data/order-summary \
  -H "Content-Type: application/json" \
  -d '{ "data": { "order_id": "ORD-42", "total": 125 } }'
```

</section>
</div>

Expected response:

```json
{
  "id": "019febae-d01f-7c31-b6f3-671a42a4a74e",
  "status": "ok",
  "data": {
    "order_id": "ORD-42",
    "total": 125,
    "req": {
      "order_id": "ORD-42",
      "total": 125,
      "summary": "Order ORD-42: $125"
    }
  },
  "errors": []
}
```

Notice how the data plane works:
- Raw HTTP requests send the payload inside `{"data": ...}`. (`orion-cli send` wraps your JSON in `{"data": ...}` automatically.)
- `data` starts as a copy of the incoming payload. `parse_json` parses the raw payload into `data.req`, and `map` writes `data.req.summary`.
- On a synchronous channel, the final `data` object is returned as the response body alongside the execution trace `id`.

Use that `id` (or list the latest traces) to inspect the recorded execution:

```bash
orion-cli traces list --channel order-summary --limit 1
orion-cli traces get <TRACE_ID>
```

## 6. Verify deployment state and roll back

Check the server's loaded workflow count and active deployment revision:

<div class="tabs">
<section data-tab="CLI">

```bash
orion-cli engine status
orion-cli deployments list
```

</section>
<section data-tab="curl">

```bash
curl -s http://localhost:8080/health | jq '.workflows_loaded'
curl -s http://localhost:8080/api/v2/admin/deployments | jq '{head_revision, applied_revision}'
```

</section>
</div>

Re-running `orion-cli deployments apply -d dist/deployment` when nothing has changed is a safe no-op: the CLI detects that the server's head revision already matches your compiled directory.

If you deploy a second revision later and need to revert to revision `1`, run `deployments rollback`:

<div class="tabs">
<section data-tab="CLI">

```bash
orion-cli deployments rollback \
  --target-revision 1 \
  --expected-head-revision 2 \
  --idempotency-key rollback-to-1
```

</section>
<section data-tab="curl">

```bash
curl -s -X POST http://localhost:8080/api/v2/admin/deployments \
  -H 'Content-Type: application/json' \
  -d '{"idempotency_key": "rollback-to-1", "base_revision": 2, "rollback_to": 1}'
```

</section>
</div>

Rolling back commits revision `1`'s manifest again as a new revision (`3`), keeping the deployment history append-only and auditable.

## Next steps

- [Build a Postgres-backed orders API](./postgres-orders-api.md) — Add a PostgreSQL connector, input validation, and offline regression tests with stubs.
- [Author workflows](../guides/workflows.md) — Learn branching, task groups, early exits (`terminal: true`), loops, and shared fragments.
- [Configure channels and guards](../guides/channels.md) — Add API key, HMAC, or JWT authentication, rate limiting, and response caching.
- [Deployments and revisions](../concepts/deployments.md) — Understand content-addressed definitions, connection revisions, and cluster convergence.
