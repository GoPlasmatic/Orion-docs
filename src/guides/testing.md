<!-- description: Test Orion workflows offline without a server or database: format definitions, run lint and clippy, dry-run with connector stubs, and run *.case.json suites. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Test and lint offline

`orion-cli` includes a complete offline toolchain that formats, validates, statically analyzes, and executes your workflow definitions in-process without needing a running Orion server, database, or network connection.

---

## 1. Format definition files (`orion-cli fmt`)

Format all JSON definition files in your directory to Orion's canonical key ordering and line width:

```bash
orion-cli fmt ./definitions          # Format files in place
orion-cli fmt --check ./definitions  # Check formatting in CI (exits 1 on diff)
```

`fmt` orders keys deterministically (for example, placing `id`, `name`, `condition`, and `function` in canonical order) and verifies that the formatted JSON parses to the exact same value before writing.

---

## 2. Validate schemas and cross-references (`orion-cli lint`)

Run `lint` against a single workflow file or an entire definition set directory:

```bash
orion-cli lint workflow.json                  # Validate a single workflow
orion-cli lint ./definitions --deny-warnings  # Validate a full definition set
```

Always point `lint` at the **directory** in CI. In directory mode, `lint` expands shared definitions (`$from`, `use`, `$each`, `$sql`), validates every workflow, channel, and connector against the server's exact schemas, and checks cross-file references:
- Rejects a channel whose `workflow_id` does not exist in the set.
- Rejects two channels that claim the same HTTP route and method.
- Rejects a task that references a missing connector or a connector of the wrong type.
- Rejects circular `channel_call` chains.

Pass `--capabilities target-capabilities.json` when your definition set uses Standard or Enterprise protocols (`kafka`, `graphql`, `sse`) or edition providers (`com.orion.*`).

---

## 3. Run static analysis (`orion-cli clippy`)

After `lint`, run `clippy` to catch logical defects, redundant conditions, and duplication across your workflows:

```bash
orion-cli clippy ./definitions
orion-cli clippy -c config.toml ./definitions  # Also check [vars] and [secrets] references
orion-cli clippy --fix ./definitions           # Automatically apply proven safe fixes
```

Every `clippy` rule is certainty-based—it only reports findings it can prove statically, such as:
- A workflow or task whose condition can never evaluate to true (`correctness.workflow_never_matches`, `correctness.task_never_runs`).
- Steps placed after an unconditional `terminal: true` step (`correctness.unreachable_step`).
- Expressions reading `payload.*` instead of `data.*` (`correctness.payload_var`).
- A `map` task that overwrites a path written earlier in the same task (`correctness.mapping_overwritten`).
- A `data_query` using pagination without a deterministic `sort` (`correctness.unordered_page`).

See [Static analysis rules (clippy)](../reference/clippy.md) for all 20 rules.

---

## 4. Execute workflows interactively (`orion-cli dry-run`)

Run a workflow in-process against a sample payload file:

```bash
orion-cli dry-run -w ./definitions/workflow.json -i payload.json
```

> [!NOTE]
> `-i` takes the bare payload JSON (such as `{"order_id": "ORD-1", "total": 25000}`), without the `{"data": ...}` HTTP wrapper.

`dry-run` outputs a single JSON document to stdout containing:

| Field | Contents |
| :--- | :--- |
| `matched` | `true` if the workflow's top-level condition matched and tasks ran. |
| `tasks` | Ordered list of task IDs that executed. |
| `trace` | Step-by-step execution trace (including skipped tasks and per-step context diffs). Omit with `--trace none`. |
| `data` / `output` | The final `data` context after workflow execution (`output` is an alias for `data`). |
| `metadata` | The final `metadata` context. |
| `temp_data` | The final `temp_data` scratch context. |
| `calls` | Recorded connector calls grouped by function name (`http_call`, `data_write`, etc.), with resolved inputs. |
| `audit_trail` | Per-task execution summary and field writes. |
| `errors` | Any task or validation errors recorded during the run. |

---

## 5. Stub external connector calls (`--stubs`)

When a workflow calls an external system (`http_call`, `data_query`, `data_write`, `db_read`, `db_write`, `connector_call`, or `channel_call`), supply a **stub file** so the workflow can run offline:

```json
{
  "http_call": {
    "crm": { "id": 1, "name": "Ada Lovelace", "tier": "vip" }
  },
  "data_query": {
    "orders-db": [{ "id": 101, "total": 420.0 }]
  },
  "data_write": {
    "orders-db": { "status": "ok", "rows_affected": 1, "returning": [{ "id": 4 }] }
  },
  "channel_call": {
    "inventory-check": { "in_stock": true }
  }
}
```

Pass the stub file to `dry-run`:

```bash
orion-cli dry-run -w ./definitions/workflow.json -i payload.json --stubs stubs.json
```

The outer key is the task function name, and the inner key is the connector name (or channel name for `channel_call`, or `"*"` to match any target). If a workflow executes a connector task that has no matching entry in `--stubs`, `dry-run` fails immediately and prints the exact stub key required.

*(Self-contained functions like `crypto`, `jwt_sign`, `jwt_verify`, and WebAssembly plugins with `--plugin-dir` execute for real offline and do not use stubs.)*

---

## 6. Build regression test suites (`*.case.json` and `orion-cli test`)

Turn your dry-runs into automated regression tests by creating `*.case.json` files:

```json
{
  "name": "flags high-value orders and enriches customer",
  "workflow": "../workflow.json",
  "input": { "order_id": "ORD-1", "total": 25000 },
  "metadata": {
    "headers": { "x-tenant-id": "acme" },
    "auth": { "claims": { "sub": "user-42" } },
    "params": { "id": "ORD-1" }
  },
  "stubs": {
    "http_call": { "crm": { "name": "Ada Lovelace" } }
  },
  "expect": {
    "data.order.flagged": true,
    "data.customer.name": "Ada Lovelace",
    "calls.http_call[0].task_id": "fetch_customer"
  },
  "expect_calls": {
    "http_call": [
      { "connector": "crm", "method": "GET" }
    ]
  },
  "expect_tasks": ["parse", "flag", "fetch_customer"],
  "expect_errors": []
}
```

### Key case file features
- **Rooted `expect` paths**: Every key in `expect` must start with `data.`, `metadata.`, `temp_data.`, `calls.`, or `audit_trail.`.
- **Asserting connector writes (`expect_calls`)**: Verify the exact resolved payload a task sent to `data_write` or `http_call` (matching as a deep subset in execution order), or assert `"http_call": []` to prove no external call was made.
- **Testing expected refusals (`expect_errors`)**: By default, `expect_errors` is `[]`, so any unexpected task error fails the test. To test that invalid input is rejected, set `"expect_errors": ["VALIDATION_ERROR"]`.
- **Simulating request metadata (`metadata`)**: Supply `headers` (automatically lowercased), `params`, `query`, `cookies`, `vars`, and `auth.claims` just as the HTTP ingress builds them.

Run all `*.case.json` files in a directory with `orion-cli test`:

```bash
orion-cli test ./definitions
```

---

## 7. Verify raw SQL statements (`orion-cli sql check`)

If your workflows use raw SQL (`db_read` or `db_write`), `lint` cannot verify table or column names without a database schema. Run `orion-cli sql check` in CI against a disposable database to prepare every SQL statement as the connector's database role without executing any writes:

```bash
orion-cli sql check ./definitions --schema ./migrations --database "$TEST_DATABASE_URL"
```

## Next steps

- [CI/CD and deployment workflows](./ci-cd.md) — Run these offline checks as pull-request gates in GitHub Actions.
- [Static analysis rules (clippy)](../reference/clippy.md) — Reference for all 20 `clippy` rules.
- [CLI reference](../reference/cli.md) — Complete flag documentation for `fmt`, `lint`, `clippy`, `dry-run`, `test`, and `sql check`.
