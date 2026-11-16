<!-- description: Build a PostgreSQL-backed orders API in Orion with input validation, injection-safe queries, offline stub tests, execution traces, and safe updates. -->
<!-- type: tutorial -->
<!-- last_verified: 2026-11-16 -->

# Build a Postgres-backed orders API

Real services validate inputs, read and write databases, and run automated tests in CI before deployment. In this tutorial, you will build and deploy the `postgres-orders` service—an HTTP endpoint that validates an incoming order, inserts it into PostgreSQL through a restricted connector, and returns the customer's updated order history.

## What you will learn

- How a **connector** separates database credentials, connection pooling, and operation gates (`delete: false`) from workflow logic.
- How `validation` with `on_rejection: "halt"` prevents invalid requests from reaching database writes.
- How the portable data dialect (`data_write` and `data_query`) runs parameterized, injection-safe queries with relations across PostgreSQL, MySQL, and SQLite.
- How to test connector-backed workflows offline using **connector stubs** (`--stubs`) and regression test suites (`*.case.json`).

## Before you start

Tested with Orion 2.0.0-rc.1. You need:

- Git, Docker, and Docker Compose
- `orion-server`, `orion-cli`, `curl`, and `jq`
- Ports `8080` and `5432` free on your machine

All files for this tutorial ship in [`examples/packages/postgres-orders/`](https://github.com/GoPlasmatic/Orion/tree/main/examples/packages/postgres-orders). Clone the repository to follow along:

```bash
git clone https://github.com/GoPlasmatic/Orion.git
cd Orion
```

## 1. Inspect the service definitions

The `examples/packages/postgres-orders/` directory contains three definitions (`connector.json`, `workflow.json`, and `channel.json`), a `docker-compose.yml` for local PostgreSQL, a sample `request.json`, and an offline test case in `tests/valid-order.case.json`.

### The PostgreSQL connector (`connector.json`)

```json
{{#include ../../examples/packages/postgres-orders/connector.json}}
```

Three settings on this connector protect the database:
- **`"connection_string": "env://ORDERS_DB_URL"`**: The connector stores a reference to an environment variable, never a plaintext password. `orion-cli compile` moves this reference into `secret_locators`, and `orion-server` resolves it at runtime.
- **`"operations": { "delete": false }`**: Enforces an operation gate at the connection layer. Even if a workflow task attempts a delete, the connector rejects it.
- **`"allow_private_urls": true`**: Orion blocks private/internal IP ranges by default to prevent SSRF. Setting this explicitly allows connecting to `localhost` or an internal VPC database.

### The workflow (`workflow.json`)

```json
{{#include ../../examples/packages/postgres-orders/workflow.json}}
```

How the workflow tasks operate:
1. **`parse` (`parse_json`)**: Parses the request body into `data.req`.
2. **`validate` (`validation`)**: Checks that `customer_id`, `item`, and a positive `total` are present. Setting `"on_rejection": "halt"` stops the workflow immediately with a `4xx` validation error if any rule fails, ensuring invalid data never reaches the database.
3. **`insert_order` (`data_write`)**: Inserts the new order into `orders` and returns the generated `id`. Request values enter only through the `params` map (`{"param": "total"}`), which binds them as prepared statement parameters and prevents SQL injection by construction.
4. **`load_customer` (`data_query`)**: Reads the customer record and hydrates their 10 most recent orders using `"include": { "orders": ... }`, governed by the inline `schema` declaration.

### The channel (`channel.json`)

```json
{{#include ../../examples/packages/postgres-orders/channel.json}}
```

This channel binds `POST /record-order` to the `record-order` workflow.

## 2. Lint and test offline (without a database)

You can validate the entire package and run its tests without starting PostgreSQL or `orion-server`.

First, lint the definition set to verify all schemas and cross-file references (`channel → workflow → connector`):

```bash
orion-cli lint examples/packages/postgres-orders
```

Next, run the offline regression test suite:

```bash
orion-cli test examples/packages/postgres-orders/tests
```

Expected output:

```text
  ok    valid order is recorded and history is returned

1 passed, 0 failed (1 case(s))
```

### How offline stubs and test cases work

When running offline, `orion-cli` uses a **stub file** to supply canned responses for external connectors (`data_write`, `data_query`, `http_call`). If a workflow calls a connector that has no matching stub, the run fails immediately and names the missing stub key.

You can also run `orion-cli dry-run` directly with inline stubs or test that invalid input halts before touching the database:

```bash
orion-cli dry-run \
  -w examples/packages/postgres-orders/workflow.json \
  -i <(printf '%s' '{"customer_id": 1, "item": "Invalid", "total": -1}') \
  | jq '{errors, calls, steps: [.trace.steps[].task_id]}'
```

In the output, `errors` contains `"total must be positive"`, `calls` is empty (`[]`), and `steps` lists only `["parse", "validate"]`—proving that `insert_order` was never called.

## 3. Start PostgreSQL and Orion

Start the seeded PostgreSQL container from the package's Compose file:

```bash
docker compose -f examples/packages/postgres-orders/docker-compose.yml up -d postgres
```

In a second terminal, start `orion-server` with `ORDERS_DB_URL` set for the connector's `env://ORDERS_DB_URL` reference:

```bash
ORDERS_DB_URL=postgres://orion:orion@localhost:5432/orion_orders orion-server
```

Verify that both the database container and the Orion server are ready:

```bash
docker compose -f examples/packages/postgres-orders/docker-compose.yml ps
curl --retry 10 --retry-delay 1 --retry-all-errors -fsS \
  http://localhost:8080/healthz
```

## 4. Compile, deploy, and call the API

Compile the `postgres-orders` package into a deployment directory and apply it:

```bash
orion-cli config set-server http://localhost:8080
orion-cli compile examples/packages/postgres-orders -o /tmp/postgres-orders
orion-cli deployments apply -d /tmp/postgres-orders --yes
```

The deployment commits three members as one atomic revision: `channel record-order@1`, `connection orders-db@1`, and `workflow record-order@1`.

Now send the sample order request:

<div class="tabs">
<section data-tab="CLI">

```bash
orion-cli send record-order -d '{"customer_id": 1, "item": "Difference Engine Blueprint", "total": 4200.0}'
```

</section>
<section data-tab="curl">

```bash
curl -sS -X POST http://localhost:8080/api/v2/data/record-order \
  -H 'Content-Type: application/json' \
  --data @examples/packages/postgres-orders/request.json | jq
```

</section>
</div>

Expected response:

```json
{
  "status": "ok",
  "data": {
    "created": {
      "rows_affected": 1,
      "returning": [{ "id": 4 }],
      "status": "ok"
    },
    "customer": [
      {
        "id": 1,
        "name": "Ada Lovelace",
        "email": "ada@example.com",
        "orders": [
          { "id": 4, "item": "Difference Engine Blueprint", "total": 4200.0 },
          { "id": 2, "item": "Punch Card Set", "total": 35.5 },
          { "id": 1, "item": "Analytical Engine Manual", "total": 120.0 }
        ]
      }
    ]
  },
  "errors": []
}
```

## 5. Verify validation errors and inspect traces

Send an invalid order (`total: -1`) to the live endpoint:

```bash
curl -sS -X POST http://localhost:8080/api/v2/data/record-order \
  -H 'Content-Type: application/json' \
  -d '{"data": {"customer_id": 1, "item": "Invalid", "total": -1}}' | jq
```

The response returns the validation error `"total must be positive"` and does not insert a row.

Inspect the execution traces recorded by the server:

```bash
orion-cli traces list --channel record-order --limit 2
orion-cli traces get <TRACE_ID>
```

## 6. Ship updates safely

Deployed definitions are immutable. When you modify a workflow, channel, or connector, follow the same pipeline:

```bash
orion-cli lint examples/packages/postgres-orders
orion-cli test examples/packages/postgres-orders/tests
orion-cli compile examples/packages/postgres-orders -o /tmp/postgres-orders
orion-cli deployments plan -d /tmp/postgres-orders
orion-cli deployments apply -d /tmp/postgres-orders --yes
```

`deployments plan` shows which definitions changed before committing anything. `deployments apply` commits the next revision and swaps the engine without dropping in-flight requests.

## Clean up

Stop `orion-server` with `Ctrl+C`, then stop and remove the PostgreSQL container and its volume:

```bash
docker compose -f examples/packages/postgres-orders/docker-compose.yml down -v
rm -rf /tmp/postgres-orders
```

## Next steps

- [Connect databases, APIs, and caches](../guides/connectors.md) — Configure SQL, HTTP, and cache connectors, OAuth2 credentials, and operation gates.
- [Connector schemas and portable data dialect](../reference/connectors.md) — Full reference for `data_query`, `data_write`, filter operators, and relation schemas.
- [Test and lint offline](../guides/testing.md) — Deep dive into `fmt`, `lint`, `clippy`, stub files, `*.case.json` assertions, and `sql check`.
- [CI/CD and deployment workflows](../guides/ci-cd.md) — Automate offline checks and staged deployments in GitHub Actions.
