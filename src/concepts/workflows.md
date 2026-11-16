<!-- description: Understand how Orion workflows execute: ordered tasks, the shared JSON data context, JSONLogic conditions, task groups, loops, and error handling. -->
<!-- type: concept -->
<!-- last_verified: 2026-11-16 -->

# Workflows and data context

A **workflow** is an ordered pipeline of tasks that implements a service's business logic. Written as a JSON document and tracked in source control, a workflow is compiled directly into Orion's in-memory execution engine (`dataflow-rs`) without generating or compiling application binaries.

```orion-diagram
{
  "direction": "LR",
  "groups": [ { "id": "wf", "label": "order-processing" } ],
  "nodes": [
    { "id": "p", "label": "parse_json", "sublabel": "payload → data.order", "type": "service", "group": "wf" },
    { "id": "v", "label": "validation", "sublabel": "check required fields", "type": "service", "group": "wf" },
    { "id": "e", "label": "http_call", "sublabel": "enrich from CRM", "type": "service", "group": "wf" },
    { "id": "m", "label": "map", "sublabel": "compute risk tier", "type": "service", "group": "wf" },
    { "id": "out", "label": "response", "type": "channel" }
  ],
  "edges": [
    { "from": "p", "to": "v" },
    { "from": "v", "to": "e" },
    { "from": "e", "to": "m" },
    { "from": "m", "to": "out" }
  ]
}
```

---

## Tasks and built-in functions

Each step in a workflow's `tasks` array declares an `id`, a human-readable `name`, an optional JSONLogic `condition`, and a `function` block with its `input`:

```json
{
  "id": "flag",
  "name": "Flag high-value order",
  "condition": { ">": [{ "var": "data.order.total" }, 10000] },
  "function": {
    "name": "map",
    "input": {
      "mappings": [
        { "path": "data.order.flagged", "logic": true }
      ]
    }
  }
}
```

Orion provides 27 built-in task functions across six categories:
- **Data & Transformation**: `parse_json`, `parse_xml`, `map`, `filter`, `validation`, `log`, `publish_json`, `publish_xml`
- **Database & Portable Dialect**: `data_query`, `data_write`, `db_read`, `db_write`
- **HTTP & External Providers**: `http_call`, `connector_call`
- **Caching**: `cache_read`, `cache_write`, `cache_delete`, `cache_incr`, `cache_invalidate`
- **Security & Cryptography**: `crypto`, `jwt_sign`, `jwt_verify`
- **Service Composition**: `channel_call`

You can also register custom pure JSON-to-JSON functions using sandboxed [WebAssembly plugins](../guides/plugins.md). Every function's input schema is validated when you run `orion-cli lint` or upload the definition to the server. See [Task functions](../reference/functions.md) for the complete catalog.

---

## The shared data context

Tasks do not pass arguments directly to one another. Instead, every task reads from and writes to a single per-request JSON document called the **data context**, which has three top-level namespaces:

| Namespace | Purpose | Returned to Caller? |
| :--- | :--- | :--- |
| **`data`** | The working document. Starts as a copy of the incoming request payload (`{"data": ...}`). Tasks write their outputs here (for example, `data.order` or `data.customer`). | **Yes** — on synchronous channels, the final `data` object is the default HTTP response body. |
| **`metadata`** | Request context populated by the ingress: channel name, HTTP method, headers, cookies, path parameters, verified JWT claims, or Kafka/cron trigger fields. | **No** |
| **`temp_data`** | Scratch space for intermediate calculations, temporary scores, or internal tokens that tasks need during execution but should not expose to the caller. | **No** |

Because `data` starts as a copy of the request payload, expressions can read incoming fields immediately as `data.<field>`. Placing a `parse_json` task at the start of a workflow (`"source": "payload", "target": "order"`) copies the raw payload under `data.order` so downstream tasks have a dedicated namespace.

---

## Conditions, task groups, and early exits

Conditions are [JSONLogic](../reference/expressions.md) expressions that are compiled once when the runtime generation is built.

### Workflow-level vs. task-level conditions
- **Workflow `condition`**: Evaluated first to decide whether the workflow should run at all for the incoming request. Most workflows set `"condition": true`.
- **Task `condition`**: Evaluated before that specific task runs. If it evaluates to falsy, Orion skips the task and moves to the next step.

### Task groups and `terminal: true`
An entry in `tasks` that contains its own nested `tasks` array is a **task group**. Its `condition` is evaluated once upon entry—if false, the entire group is skipped.

Setting `"terminal": true` on any task or task group stops the workflow immediately after that step finishes executing. Combining a task group with `terminal: true` gives you a clean guard-clause pattern (for example, returning a `404` response and stopping early when a database lookup returns `null`).

---

## How a channel selects a workflow revision

1. **Immutable deployments**: A channel only runs workflow revisions included in the server's active [deployment](./deployments.md).
2. **Single revision (default)**: When compiled from a directory with `orion-cli compile`, a channel requires one specific revision of its workflow.
3. **Canary traffic splitting** <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>: On Orion Standard and Enterprise, a deployment manifest can attach a `rollouts` array to a channel member to split traffic across two or more revisions of the same workflow across 100 hash buckets (`[0, 100)`). Orion hashes the caller identity so repeat requests from the same caller land on the same revision. See [Canary rollouts and traffic splitting](../enterprise/traffic-splitting.md).

---

## Error handling (`on_failure` vs. `on_rejection`)

Orion distinguishes between two kinds of task problems:

1. **Execution failures (`5xx` / handler errors)**: By default, if a connector call or function fails, the workflow halts immediately and returns an error envelope (or routes an async run to the dead-letter queue). Setting `"on_failure": "continue"` on a task records the error in the `errors` array and continues executing subsequent tasks.
2. **Validation rejections (`4xx` rule failures)**: The [`validation`](../reference/functions.md) function evaluates business rules against the payload and appends any failures to `errors[]`. By default, `validation` **does not halt** the pipeline so multiple validation tasks can collect all errors at once. To stop the workflow immediately when a validation rule fails, set **`"on_rejection": "halt"`** on the validation task (`orion-cli lint` warns with `engine.unguarded_validation` if you omit it before a side-effecting task).

## Next steps

- [Author workflows](../guides/workflows.md) — Practical patterns for mapping data, branching, task groups, loops, and shared fragments.
- [Workflow schema and formatting](../reference/workflows.md) — Complete JSON field reference for workflows, tasks, and groups.
- [Task functions](../reference/functions.md) — Input and output contracts for all 27 built-in functions.
- [Expression language (JSONLogic)](../reference/expressions.md) — Operator catalog and evaluation rules.
