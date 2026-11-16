<!-- description: Reference for Orion workflow definitions and orion-cli fmt style rules: top-level fields, tasks, task groups, fan-out, loops, data context, and key order. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# Workflow schema and formatting

A workflow is a JSON document that defines an ordered pipeline of tasks. A [channel](./channels.md) binds to a workflow by `workflow_id`. When a request or event arrives, Orion evaluates the workflow's `condition`, runs its `tasks` in sequence against a shared JSON context (`data`, `metadata`, `temp_data`), and returns the resulting `data` object.

For a practical authoring walkthrough, see [Author workflows](../guides/workflows.md). For per-task `function.input` schemas, see [Task functions](./functions.md).

---

## Top-level workflow fields

Write a workflow file in your definition set and compile it with [`orion-cli compile`](./cli.md). During compilation, `workflow_id` moves onto the deployment manifest member so the content-addressed definition holds only the behavior.

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `workflow_id` | string | no | auto (UUID v4) | Stable identifier (`1` to `128` characters of `[A-Za-z0-9._-]`, starting with an alphanumeric character). |
| `name` | string | **yes** | — | Human-readable workflow name (`1` to `255` characters). |
| `description` | string | no | — | Optional summary (`<= 2048` characters). |
| `priority` | integer | no | `0` | Evaluation order when multiple workflows are bound to a channel; higher numbers run first. |
| `condition` | JSONLogic | no | `true` | Guard expression deciding whether this workflow runs for the incoming message. |
| `tasks` | array | **yes** | — | Non-empty ordered array of [task objects](#tasks) and [task groups](#task-groups). |
| `tags` | string[] | no | `[]` | Free-form labels for filtering and organization. |
| `loop` | object | no | — | Repeats the `tasks` list across bounded sweeps or array elements (see [Workflow loops](#workflow-loops)). |
| `continue_on_error` | bool | no | `false` | When `true`, an uncaught task failure (`5xx` or handler error) ends this workflow without failing the request so the next matching workflow can run. |

---

## Tasks

Each element in `tasks` is either a single task step or a [task group](#task-groups):

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `id` | string | **yes** | — | Unique step identifier within the workflow (shared across tasks, groups, and `loop.setup` steps). |
| `name` | string | no | — | Human-readable label displayed in execution traces. |
| `function` | object | **yes** | — | The task function to execute (`name` and `input`). |
| `condition` | JSONLogic | no | — | Optional JSONLogic expression; when falsy, this task is skipped. |
| `on_failure` | string | no | `"fail"` | Behavior when this task raises a handler error or `5xx`: `"fail"` propagates to the workflow, `"halt"` stops the workflow cleanly, and `"continue"` records the error in `metadata._orion_errors` and continues to the next task. |
| `terminal` | bool | no | `false` | Ends the workflow immediately after this step executes (position-based exit; see [Terminal steps](#terminal-steps)). |
| `on_rejection` | string | no | `"continue"` | Behavior when this task records a `4xx` rejection (such as a failed `validation` rule): `"continue"` records the error and continues, `"halt"` stops the workflow immediately. |
| `for_each` | object | no | — | Executes the task's function once per element of an array (see [Per-task fan-out (`for_each`)](#per-task-fan-out-for_each)). |

The `function` object names a [built-in or plugin function](./functions.md) and supplies its `input`:

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `name` | string | **yes** | — | One of the [built-in functions](./functions.md) or a qualified `<plugin>.<function>` name. |
| `input` | object | depends | — | Function-specific input parameters validated against the function's schema. |

```json
{
  "id": "flag_high_value",
  "name": "Flag high-value order",
  "condition": { ">": [{ "var": "data.order.total" }, 10000] },
  "function": {
    "name": "map",
    "input": {
      "mappings": [{ "path": "data.order.flagged", "logic": true }]
    }
  }
}
```

### Task groups

An element in `tasks` that contains a `tasks` array instead of a `function` object is a **task group**. A task group applies a single `condition` and optional `terminal` flag to a contiguous sequence of tasks (nested up to 8 levels deep):

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `id` | string | **yes** | — | Unique identifier within the workflow (shares the ID namespace with tasks). |
| `tasks` | array | **yes** | — | Non-empty list of member tasks or nested task groups. |
| `condition` | JSONLogic | no | `true` | Evaluated once on group entry; when falsy, skips the entire group. |
| `terminal` | bool | no | `false` | Ends the workflow after the group finishes executing. |
| `name` | string | no | — | Optional human-readable label. |
| `description` | string | no | — | Optional summary of what the group covers. |

```json
{
  "id": "not_found_guard",
  "condition": { "==": [{ "var": "data.user" }, null] },
  "terminal": true,
  "tasks": [
    {
      "id": "set_404_body",
      "name": "Write 404 error body",
      "function": {
        "name": "map",
        "input": {
          "mappings": [{ "path": "data.out", "logic": { "error": "User Not Found" } }]
        }
      }
    },
    {
      "id": "set_404_status",
      "name": "Write 404 response envelope",
      "function": {
        "name": "map",
        "input": {
          "mappings": [
            { "path": "data._orion.response", "logic": { "status": 404, "body_path": "data.out" } }
          ]
        }
      }
    }
  ]
}
```

### Terminal steps

Setting `"terminal": true` on a task or task group stops the workflow after that step or group executes. It is a **position-based** exit:
- If the step's `condition` evaluates to `false`, the step is skipped and `terminal` does not trigger.
- If the step runs (even if it fails under `"on_failure": "continue"`), the workflow stops after that step completes.

### Halting on a rejection (`on_rejection`)

Setting `"on_rejection": "halt"` on a task stops the workflow whenever the task records a `4xx` status (most commonly when a [`validation`](./functions.md) rule fails). Because `on_rejection` defaults to `"continue"` so multiple validation tasks can accumulate field errors, always set `"on_rejection": "halt"` when a `validation` task guards subsequent database writes or external calls:

```json
{
  "id": "check_state",
  "name": "Verify OAuth state token",
  "on_rejection": "halt",
  "function": {
    "name": "validation",
    "input": {
      "rules": [
        {
          "logic": { "==": [{ "var": "metadata.query.state" }, { "var": "metadata.cookies.oauth_state" }] },
          "message": "state mismatch"
        }
      ]
    }
  }
}
```

### Per-task fan-out (`for_each`)

A task with a `for_each` block runs its connector or composition function once per element of an array and collects the results in element order:

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `over` | JSONLogic | **yes** | — | Expression yielding the input array (non-array values fail the task; `[]` runs zero calls). |
| `as` | string | **yes** | — | Binding name under `temp_data.<as>` for each element (with its index at `temp_data.<as>_index`). |
| `max_concurrency` | integer | no | `1` | Maximum concurrent calls in flight (`>= 1`). Results are always folded back in element order. |
| `collect` | string | no | — | Context path where each call writes its result (requires `into`). |
| `into` | string | no | — | Context path where the collected results array is written (requires `collect`). |

---

## Workflow loops

A top-level `loop` object executes the workflow's `tasks` list repeatedly across bounded sweeps or over an array:

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `max` | integer | **yes** | — | Upper bound on sweeps (`1` to `engine.max_loop_iterations`, default `10000`). |
| `counter` | string | no | — | `temp_data` path holding the current sweep index (for example, `"i"` writes `temp_data.i`). |
| `init` | integer | no | `0` | Starting counter value (`>= 0` when `over` is used). |
| `increment` | integer | no | `1` | Step added to the counter after each sweep (`>= 1`). |
| `over` | JSONLogic | no | — | Array expression evaluated once after `setup`; sweep `k` binds `over[k]` to `temp_data.<as>`. |
| `as` | string | no | — | `temp_data` field holding the current array element (requires `over`). |
| `scratch` | string | no | — | `temp_data` field reset to `{}` at the start of every sweep so per-iteration scratch data does not leak. |
| `setup` | array of steps | no | — | Tasks or task groups executed once before sweep `0`. |

To break out of a loop early once a condition is satisfied, place a [`filter`](./functions.md) task inside `tasks` with `"on_reject": "halt"`.

---

## Data context and request metadata

Every workflow executes against a single JSON document with three top-level namespaces:

| Namespace | Initial value | Purpose |
|---|---|---|
| `data` | Copy of the request payload | Working document read and written by tasks (`data.*`). Returned as the synchronous response body unless shaped via `data._orion.response`. |
| `metadata` | Stamped by the channel ingress | Request context (`channel`, `http_method`, `params`, `query`, `headers`, `cookies`, `vars`, `auth`, `oauth`, `trigger`). |
| `temp_data` | `{}` | Scratch space for intermediate values, loop counters, and `for_each` bindings. Stripped before responding. |

### Reserved `_orion` keys and error branching

| Path | Scope | Description |
|---|---|---|
| `data._orion.response` | Context | Used by response-shaped HTTP channels to set custom `status`, `headers`, `cookies`, or `body` / `body_path`. |
| `metadata._orion_call_depth` | Context | Nesting depth across `channel_call` invocations. |
| `metadata._orion_call_chain` | Context | Array of channel names traversed by nested `channel_call` steps. |
| `metadata._orion_errors` | Context | Array of `{task_id, workflow_id, code, status}` objects recorded when a task fails under `"on_failure": "continue"`. |
| `_orion.profile` | HTTP response | Per-task timing breakdown included when request profiling is enabled. |

When a database task (`db_read`, `db_write`, `data_query`, `data_write`) violates a schema constraint under `"on_failure": "continue"`, `metadata._orion_errors[].code` records one of four portable integrity codes across SQLite, PostgreSQL, and MySQL:
- `integrity_unique` (`409 CONFLICT` if uncaught)
- `integrity_foreign_key` (`409 CONFLICT` if uncaught)
- `integrity_not_null` (`400 VALIDATION_ERROR` if uncaught)
- `integrity_check` (`400 VALIDATION_ERROR` if uncaught)

---

## Canonical formatting (`orion-cli fmt`)

`orion-cli fmt` formats workflow, channel, connector, shared fragment, and test case files into a single canonical layout with zero configuration options.

### Formatting parameters

| Setting | Value |
|---|---|
| Line width | 100 columns — a node prints on one line when it fits |
| Indent | 2 spaces |
| Scalar array inline cap | 8 elements — a longer array of scalars breaks one per line |

### JSONLogic expression layout

| Expression shape | Definition | Formatter layout |
|---|---|---|
| **Unary** | Argument is a scalar, a variable read (`var`, `val`, `secret`, `missing`), or a single-element array of either | Always formatted on a single line (`{ "var": "data.x" }`). |
| **Leaf** | Every argument is a scalar, short scalar array, unary node, or single-key atom (`{ "field": "id" }`) | Formatted on one line when it fits within 100 columns (`{ ">=": [{ "var": "data.order.amount" }, 500] }`). |
| **Compound** | Nested boolean or multi-branch expressions (`and`, `or`, `if`) | Broken across lines with one argument per line. |

### Canonical key order by document shape

`orion-cli fmt` orders known keys in each recognized object shape according to the table below (preserving user key order inside free-form `config` and payload objects):

| Shape | Canonical key order |
|---|---|
| Workflow | `workflow_id`, `name`, `description`, `tags`, `priority`, `condition`, `loop`, `continue_on_error`, `activate`, `tasks` |
| Task | `id`, `name`, `description`, `condition`, `terminal`, `on_rejection`, `on_failure`, `for_each`, `function` |
| Task group | `id`, `name`, `description`, `condition`, `terminal`, `tasks` |
| Fragment call site | `id`, `use`, `with` |
| `function` | `name`, `input` |
| `function.input` | Matches the field table order for each function in [Task functions](./functions.md) |
| Mapping | `path`, `logic` |
| Validation rule | `logic`, `message` |
| `loop` | `counter`, `init`, `max`, `increment`, `over`, `as`, `scratch`, `setup` |
| `for_each` | `over`, `as`, `max_concurrency`, `collect`, `into` |
| Channel | `channel_id`, `name`, `description`, `tags`, `channel_type`, `protocol`, `methods`, `route_pattern`, `priority`, `workflow_id`, `connections`, `activate`, `transport_config`, `config` |
| Connector | `id`, `name`, `connector_type`, `enabled`, `tags`, `config` |
| Shared document | `package`, `constants`, `errors`, `fragments` |
| Package declaration | `name`, `requires` |
| Fragment | `params`, `tasks`, `value` |
| `$each` element | `$each`, `do` |
| Case file | `name`, `workflow`, `input`, `metadata`, `secrets`, `stubs`, `stubs_file`, `expect`, `expect_errors`, `expect_calls`, `expect_tasks` |

## Next steps

- [Task functions](./functions.md): Complete `function.input` reference for all 27 built-in functions.
- [Expression language (JSONLogic)](./expressions.md): All supported JSONLogic operators.
- [Static analysis rules (clippy)](./clippy.md): Compile-time lints checked by `orion-cli clippy`.
