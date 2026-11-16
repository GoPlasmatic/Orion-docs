<!-- description: Practical guide to authoring Orion workflows: reading payloads, mapping fields, branching, task groups, loops, channel composition, and shared fragments. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Author workflows

A workflow is a JSON document containing an ordered list of tasks that process an incoming request or event. This guide covers how to read and transform request data, write conditional branches, group tasks and exit early, iterate over arrays, compose channels in-process, handle errors, and eliminate duplication with shared definitions (`$from`, `use`, `$use`, `$each`, and `$sql`).

---

## 1. Read request data from `data`

When a request arrives, `data` starts as a copy of the incoming payload. If a caller sends `{"data": {"total": 120}}`, your tasks and conditions can read `data.total` immediately.

To keep incoming fields organized under a dedicated key, start your workflow with `parse_json`, which copies the raw payload into `data.<target>`:

```json
{
  "id": "parse",
  "name": "Parse payload",
  "function": {
    "name": "parse_json",
    "input": { "source": "payload", "target": "order" }
  }
}
```

Downstream tasks then read `data.order.*`.

> [!WARNING]
> Do not reference `payload.*` inside JSONLogic expressions (such as `{"var": "payload.total"}`). The raw payload lives outside the expression evaluation context, so `{"var": "payload.total"}` always evaluates to `null` without raising an error. Always read `data.total` (or `data.order.total` after `parse_json`).

### Use `temp_data` for intermediate state
The workflow context has three top-level namespaces:
- **`data`**: The working document. On synchronous channels, the final `data` object is returned to the caller as the HTTP response body.
- **`metadata`**: Read-only ingress context (`channel`, `http_method`, lowercase `headers`, `params`, `query`, `cookies`, `auth.claims`, `vars`, and `trigger`).
- **`temp_data`**: Scratch space that is never returned in the HTTP response. Store intermediate scores, loop counters, or sub-request payloads in `temp_data` so they do not clutter the response body:

```json
{ "path": "temp_data.raw_score", "logic": { "var": "data.risk.score" } }
```

---

## 2. Transform and enrich data with `map`

The `map` function evaluates an ordered list of mappings and writes each result to a dotted `path` in the context. Each mapping's `logic` can be a literal value or a [JSONLogic expression](../reference/expressions.md):

```json
{
  "id": "flag",
  "name": "Flag high-value order",
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
```

Mappings run in the order listed, so a later mapping can read a path written by an earlier mapping in the same task.

> [!WARNING]
> In JSONLogic, an unrecognized operator key is treated as a literal JSON object, not a syntax error. For example, writing `{"catt": ["a", "b"]}` writes that literal object to the target path (and inside a `condition`, a non-empty object is truthy!). Always run `orion-cli lint` and `orion-cli dry-run` to verify your expressions.

---

## 3. Branch with conditions

Conditions can be placed at two levels:

| Level | What It Controls | When to Use It |
| :--- | :--- | :--- |
| **Workflow `condition`** | Whether this workflow matches the incoming request at all | Selecting between two completely different workflow pipelines on the same channel |
| **Task `condition`** | Whether an individual task (or task group) executes | Branching (`if` / `else`) inside a single workflow pipeline |

Most workflows set `"condition": true` at the top level and place conditions on individual tasks.

### Writing mutually exclusive branches (`if` / `else`)
Tasks in a workflow execute independently in sequence. To write an `if / else` branch, make the conditions mutually exclusive so both tasks cannot fire on boundary values:

```json
{
  "condition": true,
  "tasks": [
    {
      "id": "parse",
      "function": { "name": "parse_json", "input": { "source": "payload", "target": "order" } }
    },
    {
      "id": "vip",
      "condition": { ">=": [{ "var": "data.order.amount" }, 500] },
      "function": {
        "name": "map",
        "input": { "mappings": [{ "path": "data.order.tier", "logic": "vip" }] }
      }
    },
    {
      "id": "standard",
      "condition": { "<": [{ "var": "data.order.amount" }, 500] },
      "function": {
        "name": "map",
        "input": { "mappings": [{ "path": "data.order.tier", "logic": "standard" }] }
      }
    }
  ]
}
```

If you wrote `>= 500` and `<= 500`, an order of exactly `500` would match both tasks and the second task would overwrite `data.order.tier`.

---

## 4. Group tasks and exit early (`terminal: true`)

An element in `tasks` that contains its own `tasks` array is a **task group**. Its `condition` is evaluated once on entry—if false, the entire group is skipped without evaluating the inner tasks' conditions.

Adding `"terminal": true` to any task or task group stops the workflow as soon as that step completes. Together, a conditional task group with `"terminal": true` implements a clean **guard clause** (if a required record is missing, set a `404` response and exit immediately):

```json
{
  "id": "not_found",
  "condition": { "==": [{ "var": "data.user" }, null] },
  "terminal": true,
  "tasks": [
    {
      "id": "body",
      "name": "Set 404 body",
      "function": {
        "name": "map",
        "input": {
          "mappings": [
            { "path": "data.out", "logic": { "error": "User not found" } }
          ]
        }
      }
    },
    {
      "id": "status",
      "name": "Set 404 status",
      "function": {
        "name": "map",
        "input": {
          "mappings": [
            {
              "path": "data._orion.response",
              "logic": { "status": 404, "body_path": "data.out" }
            }
          ]
        }
      }
    }
  ]
}
```

Key rules for task groups and `terminal`:
- A skipped task or group (where `condition` evaluated to `false`) **does not** trigger `terminal: true`; the workflow continues to the next step.
- Task groups can nest up to 8 levels deep and share the same ID namespace as tasks.

---

## 5. Iterate over arrays with `loop`

While JSONLogic's `map` and `filter` operators can transform arrays in memory, they cannot invoke a connector (`http_call` or `data_write`) per element. To run a sequence of tasks once per item in an array, declare a workflow **`loop`**:

```json
{
  "workflow_id": "notify-each",
  "condition": true,
  "loop": { "counter": "i", "max": 500 },
  "tasks": [
    {
      "id": "parse",
      "name": "Parse",
      "function": { "name": "parse_json", "input": { "source": "payload", "target": "req" } }
    },
    {
      "id": "more",
      "name": "Stop when all recipients are processed",
      "function": {
        "name": "filter",
        "input": {
          "condition": { "<": [{ "var": "temp_data.i" }, { "var": "data.req.count" }] },
          "on_reject": "halt"
        }
      }
    },
    {
      "id": "send",
      "name": "Send notification",
      "function": {
        "name": "http_call",
        "input": {
          "connector": "notifier",
          "method": "POST",
          "path": "/send",
          "body": { "val": ["data", "req", "recipients", { "val": ["temp_data", "i"] }] }
        }
      }
    }
  ]
}
```

Two important rules when writing loops:
1. **Use `val` (not `var`) for dynamic array indexing**: In JSONLogic, the second element of `{"var": ["path", default]}` is a default value, not an array index. To index `data.req.recipients` by `temp_data.i`, use `{"val": ["data", "req", "recipients", {"val": ["temp_data", "i"]}]}`.
2. **Place the loop exit condition in a `filter` task (`on_reject: "halt"`)**: If your exit check reads fields written by `parse_json` (or move `parse_json` into `loop.setup`), keep the break in a `filter` task so sweep `0` parses the payload before checking the bound. `loop.max` is bounded by the server's `engine.max_loop_iterations` setting.

---

## 6. Compose services in-process with `channel_call`

When one Orion service needs to call another Orion service on the same node, use `channel_call` instead of `http_call`. It executes the target channel's workflow in-process with zero network or serialization overhead.

Here is the caller workflow from `examples/packages/channel-composition/`:

```json
{{#include ../../examples/packages/channel-composition/workflow.json}}
```

And the target `customer-lookup` workflow it calls:

```json
{{#include ../../examples/packages/channel-composition/workflow-lookup.json}}
```

How `channel_call` works:
- Assemble the sub-request object in `temp_data` using `map`, then pass it to `channel_call` via `"data": {"var": "temp_data.lookup_req"}`.
- The target channel's final `data` context is written directly to the caller's `output` path (`data.customer`).
- The target channel still enforces its applicable guards (`rate_limit`, `validation_logic`, `backpressure`) and its `timeout_ms`.

---

## 7. Handle errors and validation rejections

### Continuing past optional task failures (`on_failure: "continue"`)
By default, if a connector task (`http_call`, `data_query`) fails with a `5xx` or network error, the workflow halts immediately. For optional enrichment steps where a failure should not break the request, set `"on_failure": "continue"` on the task:

```json
{
  "id": "enrich",
  "name": "Optional CRM enrichment",
  "on_failure": "continue",
  "function": {
    "name": "http_call",
    "input": { "connector": "crm", "output": "data.crm" }
  }
}
```

When `enrich` fails, Orion records the error in the response envelope's `errors` array, continues running subsequent tasks, and returns HTTP `200` with `"status": "ok"`. Clients that depend on optional steps should inspect `errors[]`.

### Halting on validation failures (`on_rejection: "halt"`)
The `validation` function evaluates payload rules and records `4xx` violations in `errors[]`. By default, `validation` **continues** to the next task so multiple validation steps can accumulate all field errors in one pass.

To stop the workflow when a validation check fails, add `"on_rejection": "halt"` to the task:

```json
{
  "id": "validate",
  "name": "Validate order",
  "on_rejection": "halt",
  "function": {
    "name": "validation",
    "input": {
      "rules": [
        { "logic": { ">": [{ "var": "data.order.total" }, 0] }, "message": "total must be positive" }
      ]
    }
  }
}
```

---

## 8. Reuse logic with shared definitions (`$from`, `use`, `$use`, `$each`, `$sql`)

In a multi-workflow definition set, you can define shared constants, error templates, and parameterized task fragments in any JSON file that contains `constants`, `errors`, or `fragments`. `orion-cli compile` (as well as `lint`, `clippy`, `dry-run`, and `test`) expands these references before validation.

### Splice shared constants (`$from`)
Declare shared objects under `constants` or `errors`:

```json
{
  "constants": {
    "db_defaults": { "connector": "orders-db", "numeric_as": "string" }
  }
}
```

Reference them in any workflow with `$from`. Sibling keys in the target object override keys from the constant:

```json
{ "input": { "$from": "constants.db_defaults", "query": { "source": "users" } } }
```

### Expand task fragments (`use`) and value fragments (`$use`)
Define reusable task sequences or values under `fragments`:

```json
{
  "fragments": {
    "require-session": {
      "params": { "deny_message": { "default": "Session expired." } },
      "tasks": [
        {
          "id": "check",
          "name": "Check session",
          "function": {
            "name": "map",
            "input": {
              "mappings": [
                { "path": "data.msg", "logic": { "$param": "deny_message" } }
              ]
            }
          }
        }
      ]
    },
    "wrote_rows": {
      "params": { "slot": {} },
      "value": { ">": [{ "var": "{{slot}}.rows_affected" }, 0] }
    }
  }
}
```

Invoke a task fragment in a `tasks` array with `use` (its inner task IDs are automatically prefixed with the call site's `id`, such as `_session.check`), and splice a value fragment anywhere with `$use`:

```json
{ "id": "_session", "use": "require-session", "with": { "deny_message": "Please sign in." } }
```

```json
{ "condition": { "$use": "wrote_rows", "with": { "slot": "temp_data.insert_result" } } }
```

### Unroll repeated steps (`$each`)
Use `$each` inside any array to repeat its `do` template once per item in a list:

```json
{
  "$each": { "seat": [0, 1, 2, 3] },
  "do": {
    "id": "score_{{seat}}",
    "name": "Score seat {{seat}}",
    "function": {
      "name": "map",
      "input": {
        "mappings": [
          { "path": "data.out_{{seat}}", "logic": { "val": ["data", "seats", { "$param": "seat" }] } }
        ]
      }
    }
  }
}
```

### Load external SQL files (`$sql`)
For `db_read` and `db_write` tasks, you can keep raw SQL queries in `.sql` files inside your definition set and reference them with `$sql`:

```json
{ "query": { "$sql": "../sql/settle_account.sql" } }
```

`orion-cli compile` normalizes whitespace, strips comments, checks for cross-dialect quoting traps, and inlines the SQL statement into the compiled definition.

### Pin the required Orion version (`package`)
Add a `package.json` file to your definition set to enforce a minimum Orion version across all CLI tools:

```json
{ "package": { "name": "orders-estate", "requires": { "orion": ">=2.0.0, <3.0.0" } } }
```

## Verify your workflow offline

Always run the four offline checks before deploying:

```bash
orion-cli fmt ./definitions
orion-cli lint ./definitions --deny-warnings
orion-cli clippy ./definitions
orion-cli dry-run -w ./definitions/workflow.json -i sample-payload.json
```

## Next steps

- [Configure channels and guards](./channels.md) — Expose your workflow over REST/HTTP and attach authentication, rate limits, and caching.
- [Connect databases, APIs, and caches](./connectors.md) — Use `data_query`, `data_write`, and `http_call` with external systems.
- [Workflow schema and formatting](../reference/workflows.md) — Complete JSON schema for workflows, tasks, and groups.
- [Task functions](../reference/functions.md) — Detailed reference for all 27 built-in functions.
