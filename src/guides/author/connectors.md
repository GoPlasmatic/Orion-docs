<!-- description: Write Orion connector files for HTTP APIs, SQL databases and in-memory caches, with secrets held by reference and per-operation gates. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Connect a database or API

A connector is how a workflow reaches anything outside the process. You declare one beside your workflows, deploy it as a connection, and every workflow references it by name. This guide declares one, keeps its credential out of the deployment, bounds what it may do, and uses it from a task.

## Before you start

You need a definition set to add the connector to, a server on `http://localhost:8080`, and a reachable external system. The examples use a PostgreSQL database; [Add your first connector](../../get-started/tutorials/first-connector.md) ships one in Docker if you need it.

## Write the connector file

Save the connector as a file in your definition set, beside the workflows that use it:

```json
{
  "name": "orders-db",
  "connector_type": "db",
  "config": {
    "type": "db",
    "driver": "postgresql",
    "connection_string": "env://ORDERS_DB_URL",
    "max_connections": 5,
    "allow_private_urls": true,
    "operations": { "delete": false, "raw_write": false }
  }
}
```

[`orion-cli compile`](../../reference/cli/orion-cli/compile.md#deployment-directories) turns it into a connection revision, and `orion-cli deployments apply -d` deploys it with the workflows. A `db` connector must name its `driver`: `postgresql`, `mysql` or `sqlite`. A changed connector becomes a new connection revision, never an edit of the old one. [Connectors](../../concepts/connectors.md) explains the two forms.

## Never write a credential into one

Reference the server's environment instead:

```json
{ "connection_string": "env://ORDERS_DB_URL" }
```

The reference resolves from the server's environment when the connection is used, so the deployment holds a variable name. Three things follow:

- No deployment holds the credential, so a database dump is not a leak.
- The same JSON works in dev, QA and production; only the variable's value differs.
- The connector compiles into a deployment. `compile` moves each reference to the connection's `secret_locators`, and refuses a literal credential, so one cannot be deployed at all.

`vault://<api-path>#<field>` reads HashiCorp Vault. `${VAR}` and `${VAR:-default}` shell-style substitution also works.

If an API wants its credentials in the query string, do not put them in the connector `url`. Use [`query_params`](../../reference/connectors/request-layering.md#query-parameter-precedence), which keeps the resolved value out of the URL, and therefore out of traces, logs and error messages.

## Say what it may do

Every connector carries operation gates, all allowed by default. Turning one off makes the call a validation error regardless of what any workflow asks:

```json
{ "operations": { "read": true, "insert": true, "update": true,
                  "delete": false, "upsert": true, "raw_write": false } }
```

Both `delete` and `raw_write` must be off to make a SQL connector delete-proof. Raw SQL cannot be classified per operation, so `db_write` is gated as a whole.

An HTTP connector gates by method instead, as an allow-list. Empty means every method; naming even one makes the list exhaustive:

```json
{ "operations": { "methods": ["GET"] } }
```

This is the cheapest blast-radius control Orion offers. It sits below the logic, and it survives every workflow change.

## Reach a private address on purpose

Connections to private ranges are refused unless the connector says otherwise:

```json
{ "allow_private_urls": true }
```

Most databases *are* private, so most connectors set this. The point is that reaching an internal address becomes a stated decision. The unstated case, a workflow-authored connector reaching a metadata endpoint, stays refused by default.

## Use it from a workflow

Name the connector, pass request data through `params`, and declare the schema the task may touch:

```json
{ "id": "record", "name": "Insert the order",
  "function": { "name": "data_write", "input": {
    "connector": "orders-db",
    "params": { "total": { "var": "data.req.total" } },
    "write": { "op": "insert", "target": "orders",
               "values": { "total": { "param": "total" } }, "returning": ["id"] },
    "schema": { "entities": { "orders": { "columns": { "id": { "type": "int", "writable": false },
                                                       "total": { "type": "float" } } } } },
    "output": "data.created"
  }}}
```

Two things are doing real work there. `params` is the only door request data comes through, and every resolved value becomes a bound parameter, so the dialect is injection-safe by construction. `schema` declares what the task may touch; undeclared entities and columns are rejected, so a task without one reaches nothing.

## Choose between the two data APIs

| | `data_query` / `data_write` | `db_read` / `db_write` |
|---|---|---|
| **You write** | A backend-neutral envelope | Raw SQL |
| **Portable across** | PostgreSQL, MySQL and SQLite, unchanged | The SQL dialect you wrote |
| **Injection safety** | By construction; no query text exists | Parameterized, but the text is yours |
| **Bounded by** | The task's `schema`, plus connector gates | Connector gates and the database user |

Use the portable dialect by default. Reach for raw SQL only when the dialect cannot express the query, such as a window function or a recursive CTE. Know that you have given up the schema bound when you do.

## Reach an edition provider <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

A Community node serves only the three built-in types. Standard and Enterprise add seven connector providers: SMTP, Kafka, Redis, storage, MongoDB, Elasticsearch and ONNX models. They are available in Orion Standard and Enterprise only.

Name one with `provider` in place of `connector_type`, and keep its credential as a reference exactly as above:

```json
{
  "name": "mailer",
  "provider": "com.orion.smtp",
  "config": {
    "host": "smtp.example.com",
    "from": "Orders <orders@example.com>"
  }
}
```

`compile` derives the `provider_digest` from that id, so no package is uploaded first. A call on the connection names the same id in `target_id`, because the connection id is not a provider id.

Choose by what the workflow needs, not by the nearest built-in type:

| Need | Provider |
|---|---|
| Transactional email | `com.orion.smtp` |
| Record production to a broker | `com.orion.kafka` |
| A cache shared by every node | `com.orion.redis` |
| Object storage and presigned URLs | `com.orion.storage` |
| Document queries and writes | `com.orion.mongodb` |
| Full-text search and bulk indexing | `com.orion.elasticsearch` |
| ONNX inference | `com.orion.models` |

Check the target before compiling. The capabilities answer names every provider and the operations it declares, and `lint` and `compile` accept that file through `--capabilities`:

```bash
orion-cli deployments capabilities --output json > target-capabilities.json
orion-cli lint ./definitions --capabilities target-capabilities.json
orion-cli compile ./definitions -o dist/deployment --capabilities target-capabilities.json
```

A Community node has no such backend, so it refuses the definition or the connection that names one.

## Verify

After the apply, confirm the node holds the connection revision and loaded it:

```bash
orion-cli connections list
curl -s http://localhost:8080/api/v2/admin/engine/status | jq '.data.load_issues.connectors'
```

The list shows `orders-db` at revision 1, and the load issues list is `[]`. Then send one request through a channel whose workflow uses the connector. Only real traffic proves the credential, because the reference resolves on the node.

## Keep it healthy

Each connector has its own circuit breaker and, for HTTP, its own retry policy. Turn breakers on globally with `engine.circuit_breaker.enabled = true`; they are off by default. Inspect and reset them through the [circuit breaker endpoints](../../reference/admin-api/circuit-breakers.md). See [Timeouts, retries and circuit breakers](../../operate/run/failure-handling.md).

## Next steps

- [Connector types](../../reference/connectors/index.md): every field of every type.
- [Edition providers](../../reference/connectors/edition/index.md): the seven providers an edition adds, with their operations and limits.
- [Portable data dialect](../../reference/data-dialect.md): the query and write envelope in full.
- [Add your first connector](../../get-started/tutorials/first-connector.md): the same job as a walkthrough against PostgreSQL.
- [Keep credentials out of the database](../../operate/run/security.md#keep-credentials-out-of-the-database): secrets, masking and encryption at rest.
