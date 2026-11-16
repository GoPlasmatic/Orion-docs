<!-- description: Reference for the twenty static analysis rules in orion-cli clippy: rule levels, scopes, certainty guarantees, automatic fixes, and per-rule explanations. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# Static analysis rules (clippy)

`orion-cli clippy` runs `orion-cli lint` over a definition set and then evaluates 20 deterministic static-analysis rules across your workflows, channels, and shared fragments. Because `clippy` has no per-rule suppression flags, every rule is backed by a strict compile-time proof (constant folding, ingress context invariants, control-flow reachability, or structural identity) and stays silent whenever a finding is uncertain.

Run `orion-cli clippy --explain <rule_id>` in your terminal to print any rule's explanation and remediation offline.

---

## Rule summary

| Rule | Level | Scope | Summary |
|---|---|---|---|
| [`correctness.workflow_never_matches`](#correctnessworkflow_never_matches) | deny | workflow | the workflow-level condition is false for every request, so the workflow never runs |
| [`correctness.task_never_runs`](#correctnesstask_never_runs) | warn | workflow | a step's condition folds to a constant false, so the step never runs |
| [`correctness.unreachable_step`](#correctnessunreachable_step) | deny | workflow | steps after an unconditional terminal step can never run |
| [`correctness.unconditional_call_cycle`](#correctnessunconditional_call_cycle) | deny | set | channel_call edges that are all unconditional form a cycle, so every request into it fails at the depth limit |
| [`correctness.payload_var`](#correctnesspayload_var) | deny | workflow | a read of `payload` — which is not in the data context — is always null |
| [`correctness.mapping_overwritten`](#correctnessmapping_overwritten) | warn | workflow | two mappings in one map write the same path with nothing reading it in between |
| [`correctness.mapping_always_null`](#correctnessmapping_always_null) | deny | workflow | a `map` mapping whose `logic` is always null, so it never writes |
| [`correctness.metadata_var_undeclared`](#correctnessmetadata_var_undeclared) | deny | workflow | a read of `metadata.vars.<name>` that the config given with -c does not declare |
| [`correctness.secret_undeclared`](#correctnesssecret_undeclared) | deny | set | a {"secret": name} that the config given with -c does not declare |
| [`correctness.response_cookie_type`](#correctnessresponse_cookie_type) | warn | workflow | a response cookie attribute is a literal of the wrong type, so the cookie is always dropped |
| [`correctness.unknown_input_key`](#correctnessunknown_input_key) | deny | workflow | a task input key the function does not declare, which is silently ignored |
| [`correctness.unordered_page`](#correctnessunordered_page) | deny | workflow | a read that skips rows without ordering them, so the page it skips is undefined |
| [`correctness.sql_bind_count`](#correctnesssql_bind_count) | deny | workflow | a PostgreSQL statement's `$n` placeholders and its literal `params` differ in number |
| [`perf.parse_result_overwritten`](#perfparse_result_overwritten) | warn | workflow | a parse/publish target is overwritten by a later unconditional task before anything reads it |
| [`perf.redundant_step_condition`](#perfredundant_step_condition) | warn | workflow | consecutive steps repeat one condition that none of them can change; a task group evaluates it once |
| [`perf.group_condition_repeated`](#perfgroup_condition_repeated) | warn | workflow | a group member repeats the group's own condition, which was already true on entry |
| [`duplication.fragment_available`](#duplicationfragment_available) | warn | set | a run of steps is exactly what an existing fragment expands to; a `use` would say it once |
| [`duplication.repeated_task_sequence`](#duplicationrepeated_task_sequence) | warn | set | the same run of two or more steps appears three or more times across the set |
| [`duplication.repeated_value`](#duplicationrepeated_value) | warn | set | the same object literal appears three or more times across the set |
| [`style.terminal_on_last_step`](#styleterminal_on_last_step) | warn | workflow | terminal: true on the last top-level step is a no-op |

### Rule levels and exit codes

- **`deny`**: Causes `orion-cli clippy` to exit with code `2`.
- **`warn`**: Printed to standard error without failing the command unless `--deny-warnings` is passed.
- **Automatic fixes (`--fix`)**: `orion-cli clippy --fix` automatically rewrites `perf.redundant_step_condition` into a task group (`when_<first_step_id>`) and verifies that the compiled output is semantically equivalent.

---

## Correctness rules

### `correctness.workflow_never_matches`

- **Level**: `deny` · **Scope**: `workflow`
- **What it detects**: A top-level workflow `condition` that constant-folds to falsy, or reads `temp_data.*` (which is always `{}` when workflow matching runs before any task executes).
- **How to fix**: Move checks that depend on parsed or computed task state into a task `condition` or `filter` step inside `tasks`.

### `correctness.task_never_runs`

- **Level**: `warn` · **Scope**: `workflow`
- **What it detects**: A task or task group whose `condition` folds to a constant falsy value (`false`, `null`, `0`, `""`, `[]`).
- **How to fix**: Remove the dead step or fix the constant expression in its `condition`.

### `correctness.unreachable_step`

- **Level**: `deny` · **Scope**: `workflow`
- **What it detects**: Any step placed after an unconditional step (or task group) that sets `"terminal": true`.
- **How to fix**: Add a `condition` to the terminal step, remove `"terminal": true`, or delete the unreachable steps that follow it.

### `correctness.unconditional_call_cycle`

- **Level**: `deny` · **Scope**: `set`
- **What it detects**: A cycle of `channel_call` tasks across workflows where every step along the cycle is unconditional, guaranteeing that every request entering the cycle fails at `engine.max_channel_call_depth`.
- **How to fix**: Guard recursive `channel_call` steps with a base-case `condition` or break the circular dependency between channels.

### `correctness.payload_var`

- **Level**: `deny` · **Scope**: `workflow`
- **What it detects**: A JSONLogic variable read of `{"var": "payload"}` or `{"var": "payload.*"}`. The raw payload is not a key in the JSONLogic context (`data` starts as a copy of the payload).
- **How to fix**: Read `{"var": "data.<field>"}` directly, or run `parse_json` with `"source": "payload", "target": "req"` and read `{"var": "data.req.<field>"}`.

### `correctness.mapping_overwritten`

- **Level**: `warn` · **Scope**: `workflow`
- **What it detects**: Two mappings in the same `map` task that write to the exact same `path` (under `mode: "set"`) without any intervening mapping reading that path.
- **How to fix**: Remove the earlier overwritten mapping or correct the target `path`.

### `correctness.mapping_always_null`

- **Level**: `deny` · **Scope**: `workflow`
- **What it detects**: A `map` mapping with `on_null: "skip"` (the default) whose `logic` folds to the constant `null`. Because `on_null: "skip"` ignores `null` results, the mapping never modifies `path`.
- **How to fix**: Use `"unset": true` (or `"on_null": "unset"`) to delete a key, instead of `"logic": null`.

### `correctness.metadata_var_undeclared`

- **Level**: `deny` · **Scope**: `workflow`
- **What it detects**: A read of `metadata.vars.<name>` (or `var://<name>`) that is not declared under `[vars]` in the server config file passed via `orion-cli clippy -c config.toml`.
- **How to fix**: Declare the variable under `[vars]` in `orion.toml` or fix the variable name in the workflow.

### `correctness.secret_undeclared`

- **Level**: `deny` · **Scope**: `set`
- **What it detects**: A `{"secret": "<name>"}` reference that is not declared under `[secrets]` in the server config file passed via `orion-cli clippy -c config.toml`.
- **How to fix**: Add the secret locator under `[secrets]` in `orion.toml` or fix the secret name.

### `correctness.response_cookie_type`

- **Level**: `warn` · **Scope**: `workflow`
- **What it detects**: A literal cookie attribute in `data._orion.response.cookies` with the wrong JSON type (for example, `"secure": "true"` as a string or `"max_age": "3600"` as a string), which causes Orion to drop the cookie at runtime.
- **How to fix**: Use booleans for `secure` and `http_only`, a number for `max_age`, and strings for `name`, `value`, `path`, `domain`, `expires`, and `same_site`.

### `correctness.unknown_input_key`

- **Level**: `deny` · **Scope**: `workflow`
- **What it detects**: An unrecognized key inside `function.input` on an engine built-in task (`map`, `filter`, `validation`, `log`, `parse_json`, `parse_xml`, `publish_json`, `publish_xml`), which the engine would otherwise ignore silently.
- **How to fix**: Check the function's table in [Task functions](./functions.md) and fix the misspelled input key.

### `correctness.unordered_page`

- **Level**: `deny` · **Scope**: `workflow`
- **What it detects**: A `data_query` task that sets `skip` (`> 0`) without a `sort` array, or a `db_read` query that contains `OFFSET` without `ORDER BY`.
- **How to fix**: Add a deterministic `sort` / `ORDER BY` clause so pagination order is well-defined.

### `correctness.sql_bind_count`

- **Level**: `deny` · **Scope**: `workflow`
- **What it detects**: A PostgreSQL `db_read` or `db_write` statement whose highest `$n` placeholder index does not match the length of its literal `params` array.
- **How to fix**: Ensure `params` supplies exactly one entry per `$1..$n` placeholder.

---

## Performance, duplication, and style rules

### `perf.parse_result_overwritten`

- **Level**: `warn` · **Scope**: `workflow`
- **What it detects**: A `parse_json`, `parse_xml`, `publish_json`, or `publish_xml` target that is unconditionally overwritten by a later task before any step reads it.
- **How to fix**: Remove the unused parse/serialize task or write the later task's output to a distinct path.

### `perf.redundant_step_condition`

- **Level**: `warn` · **Scope**: `workflow` (auto-fixable with `orion-cli clippy --fix`)
- **What it detects**: Consecutive steps in a task list that repeat the exact same `condition` when none of the steps modify any path read by that condition.
- **How to fix**: Run `orion-cli clippy --fix ./definitions` to wrap the consecutive steps in a single [task group](./workflows.md#task-groups) that evaluates the condition once.

### `perf.group_condition_repeated`

- **Level**: `warn` · **Scope**: `workflow`
- **What it detects**: A step inside a task group that repeats the enclosing group's `condition` when no prior step in the group modified any path read by that condition.
- **How to fix**: Remove the redundant `condition` from the inner step.

### `duplication.fragment_available`

- **Level**: `warn` · **Scope**: `set`
- **What it detects**: A contiguous sequence of tasks in a workflow that is structurally identical to an existing shared fragment declared in the definition set.
- **How to fix**: Replace the inline task sequence with `{"id": "...", "use": "<fragment_name>"}`.

### `duplication.repeated_task_sequence`

- **Level**: `warn` · **Scope**: `set`
- **What it detects**: A sequence of two or more tasks that appears three or more times across the definition set.
- **How to fix**: Extract the repeated tasks into a shared `fragments` definition and invoke it with `use`.

### `duplication.repeated_value`

- **Level**: `warn` · **Scope**: `set`
- **What it detects**: A multi-field object literal that is duplicated three or more times across the definition set.
- **How to fix**: Move the shared object into `constants` or `errors` in a shared definition file and reference it with `{"$from": "constants.<name>"}`.

### `style.terminal_on_last_step`

- **Level**: `warn` · **Scope**: `workflow`
- **What it detects**: `"terminal": true` on the final top-level step of a workflow (where the workflow already ends).
- **How to fix**: Remove `"terminal": true` from the last step.

## Next steps

- [Test and lint offline](../guides/testing.md): Combine `fmt`, `lint`, `clippy`, `dry-run`, and `test` in local development and CI.
- [CLI reference](./cli.md): Flags and exit codes for `orion-cli clippy`.
