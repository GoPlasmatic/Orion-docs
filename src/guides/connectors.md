<!-- description: Connect Orion workflows to PostgreSQL, MySQL, SQLite, HTTP APIs, and caches using env:// secrets, operation gates, and the portable data dialect. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Connect databases, APIs, and caches

Connectors allow workflows to read and write SQL databases, call external HTTP APIs, and use caches while keeping credentials, connection pools, operation gates, and circuit breakers separate from workflow logic.

---

## 1. Write a connector file

Place your connector file (`connector.json`) in your definition set directory alongside the workflows that reference it:

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
    "operations": {
      "delete": false,
      "raw_write": false
    }
  }
}
```

When you run `orion-cli compile`, the CLI validates the connector, extracts `env://` and `vault://` references into `secret_locators`, and writes `connections/orders-db.json`. When you run `orion-cli deployments apply -d`, Orion creates an immutable connection revision and deploys it alongside your workflows.

---

## 2. Keep credentials in `env://` or `vault://` references

Never put plaintext passwords, API keys, or tokens in a connector file:
- **`env://VARIABLE_NAME`**: Reads the secret from the Orion server's environment when the connection loads.
- **`vault://secret/data/app#api_key`**: Reads the secret from HashiCorp Vault.

`orion-cli compile` rejects any connector that contains a literal credential in a secret-shaped field or inside a URL (`postgres://user:password@...`).

> [!TIP]
> If an external HTTP API requires an API key in the URL query string, do not put it in `url`. Place it in the connector's `query_params` map (`"query_params": {"api_key": "env://PARTNER_KEY"}`) so the resolved secret is never logged in URLs or traces.

---

## 3. Restrict blast radius with operation gates

Every connector supports an `operations` block that restricts what workflows can do through that connection. All gates default to allowed (`true`).

### SQL database gates (`db`)
```json
{
  "operations": {
    "read": true,
    "insert": true,
    "update": true,
    "delete": false,
    "upsert": true,
    "raw_write": false
  }
}
```
To make a database connector strictly append/update-only (preventing row deletion), set both `"delete": false` and `"raw_write": false` (since raw SQL statements in `db_write` can contain `DELETE`).

### HTTP method allow-lists (`http`)
Restrict an HTTP connector to specific HTTP verbs:
```json
{
  "operations": {
    "methods": ["GET", "POST"]
  }
}
```

### Private network egress (`allow_private_urls`)
To prevent Server-Side Request Forgery (SSRF), Orion blocks connections to loopback, link-local, and RFC 1918 private IP ranges by default. When connecting to a database or internal service on `localhost` or a private VPC subnet, set `"allow_private_urls": true` on the connector.

---

## 4. Query SQL databases (`data_query` and `data_write` vs. `db_read` and `db_write`)

Orion provides two pairs of task functions for SQL connectors (`postgresql`, `mysql`, `sqlite`):

| Feature | Portable Data Dialect (`data_query` / `data_write`) | Raw SQL (`db_read` / `db_write`) |
| :--- | :--- | :--- |
| **How you write it** | Declarative JSON envelope (`filter`, `select`, `include`, `write`) | Raw SQL string (inline or via `{"$sql": "query.sql"}`) |
| **Database portability** | Runs unchanged across PostgreSQL, MySQL, and SQLite | Tied to the specific SQL dialect you write |
| **SQL injection safety** | Injection-safe by construction; all values pass through `params` | Parameterized (`$1` / `?`), but you own the SQL text |
| **Schema enforcement** | Validated against the task's inline `schema` allow-list | Bounded by connector operation gates and DB role grants |

### Example: Inserting and querying with the portable data dialect
Use `data_write` and `data_query` by default:

```json
{
  "id": "record_order",
  "name": "Insert order",
  "function": {
    "name": "data_write",
    "input": {
      "connector": "orders-db",
      "params": {
        "customer_id": { "var": "data.req.customer_id" },
        "total": { "var": "data.req.total" }
      },
      "write": {
        "op": "insert",
        "target": "orders",
        "values": {
          "customer_id": { "param": "customer_id" },
          "total": { "param": "total" }
        },
        "returning": ["id"]
      },
      "schema": {
        "entities": {
          "orders": {
            "columns": {
              "id": { "type": "int", "writable": false },
              "customer_id": { "type": "int" },
              "total": { "type": "float" }
            }
          }
        }
      },
      "output": "data.created"
    }
  }
}
```

Use `db_read` and `db_write` only when you need SQL features outside the portable dialect (such as window functions, CTEs, or vendor-specific extensions), and verify those statements in CI with `orion-cli sql check`.

---

## 5. Call external HTTP APIs (`http` connectors and `http_call`)

An `http` connector defines a base `url`, timeouts, optional `retry` policy, and authentication (`bearer`, `basic`, `api_key`, or managed `oauth2`):

```json
{
  "name": "crm",
  "connector_type": "http",
  "config": {
    "type": "http",
    "url": "https://api.crm.example.com/v1",
    "timeout_ms": 3000,
    "auth": {
      "type": "bearer",
      "token": "env://CRM_API_TOKEN"
    },
    "retry": {
      "max_attempts": 3,
      "initial_interval_ms": 100,
      "max_interval_ms": 2000
    }
  }
}
```

For upstream APIs that use OAuth2 (`client_credentials` or `refresh_token` grants), configure `"auth": {"type": "oauth2", ...}` on the connector—Orion automatically acquires, caches, and refreshes tokens in the background.

Invoke the connector from a workflow using `http_call`:

```json
{
  "id": "fetch_customer",
  "name": "Look up customer in CRM",
  "function": {
    "name": "http_call",
    "input": {
      "connector": "crm",
      "method": "GET",
      "path": { "cat": ["/customers/", { "var": "data.order.customer_id" }] },
      "output": "data.customer"
    }
  }
}
```

---

## 6. Use Standard & Enterprise providers (`connector_call`) <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

Available in Orion Standard and Enterprise (`orion-ext`). To connect to Kafka, Redis, S3 storage, MongoDB, Elasticsearch, SMTP, or ONNX models, declare `"provider"` instead of `"connector_type"` in your connector file:

```json
{
  "name": "mailer",
  "provider": "com.orion.smtp",
  "config": {
    "host": "smtp.example.com",
    "port": 587,
    "tls": "starttls",
    "auth": {
      "type": "basic",
      "username": "mailer",
      "password": "env://SMTP_PASSWORD"
    },
    "from": "Orders <orders@example.com>"
  }
}
```

Call the provider operation in your workflow with `connector_call`:

```json
{
  "id": "send_receipt",
  "name": "Send order confirmation email",
  "function": {
    "name": "connector_call",
    "input": {
      "connection": "mailer",
      "target_id": "com.orion.smtp",
      "operation": "send",
      "input": {
        "to": { "var": "data.order.email" },
        "subject": "Order confirmed",
        "text": { "cat": ["Your order ", { "var": "data.order.id" }, " is confirmed."] }
      },
      "output": "data.email_result"
    }
  }
}
```

See [Enterprise connector providers](../enterprise/providers.md) for all seven providers and their schemas.

---

## 7. Verify and probe connections

After deploying, verify that your connection revisions are stored and healthy, and probe live backend connectivity:

```bash
orion-cli connections list
orion-cli connections probe orders-db
```

`connections probe` runs a live health check against the target backend (`SELECT 1` for SQL databases, TCP connect for HTTP connectors, or the provider's health check) and exits with code `2` if the backend is unreachable or unhealthy.

## Next steps

- [Connector schemas and portable data dialect](../reference/connectors.md) — Complete field tables, auth options, header precedence, and portable query syntax.
- [Enterprise connector providers](../enterprise/providers.md) — Configure Redis, Kafka, S3, MongoDB, Elasticsearch, SMTP, and ONNX models.
- [Timeouts, retries, and circuit breakers](../operate/resilience.md) — Tune connector timeouts and circuit breaker recovery.
