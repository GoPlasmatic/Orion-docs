<!-- description: Complete CLI reference for orion-server and orion-cli: global flags, offline authoring (fmt, lint, clippy, compile, dry-run, test), and remote deployments. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# CLI reference

Orion provides two command-line binaries:
1. **`orion-server`** (and **`orion-ext`** on Standard and Enterprise): Runs the runtime server and provides offline configuration validation, database migration, connectivity probing, and OpenAPI spec generation.
2. **`orion-cli`**: Provides offline definition authoring (`fmt`, `lint`, `clippy`, `compile`, `dry-run`, `test`, `sql check`, `plugin`) and remote cluster administration (`deployments`, `provider`, `connections`, `send`, `traces`, `dlq`, `cron`, `circuit-breakers`, `cache`, `audit-logs`, `backups`, `health`, `engine`, `functions`, `metrics`, `streams`).

---

## Part 1: `orion-server` (and `orion-ext`)

```bash
orion-server [-c <config.toml>] [subcommand] [flags]
```

| Subcommand | Synopsis | Description |
|---|---|---|
| *(none)* | `orion-server [-c <config.toml>]` | Start the Orion HTTP data and admin server. |
| `validate-config` | `orion-server [-c <config.toml>] validate-config [--format toml\|json\|summary]` | Validate `config.toml` and `ORION_*` environment variables without starting the server, printing the merged config with secrets masked. |
| `migrate` | `orion-server [-c <config.toml>] migrate [--dry-run] [--wait <duration>]` | Apply pending state database migrations (`--dry-run` previews without applying; `--wait 60s` retries connection failures while the database boots). |
| `test-connectivity` | `orion-server [-c <config.toml>] test-connectivity [--wait <duration>]` | Probe the state database connection pool and report pending migrations. |
| `dump-openapi` | `orion-server dump-openapi > openapi.json` | Print the generated OpenAPI 3.1 JSON specification to standard output without starting the server. |

---

## Part 2: `orion-cli` global flags and configuration

Every remote `orion-cli` subcommand accepts these global flags and environment variables:

| Flag | Environment variable | Config file key (`~/.config/orion-cli/config.toml`) | Default | Description |
|---|---|---|---|---|
| `--server-url <url>` | `ORION_SERVER_URL` | `server_url` | `http://localhost:8080` | Base URL of the Orion server. |
| `--api-key <key>` | `ORION_API_KEY` | `api_key` | — | Admin API key sent on `/api/v2/admin/**` requests. |
| `--change-context <text>` | `ORION_CHANGE_CONTEXT` | — | — | Audit correlation label sent in `X-Orion-Change-Context`. |
| `-o, --output <fmt>` | — | `output` | `table` | Output format: `table`, `json`, or `yaml`. |
| `-q, --quiet` | — | — | `false` | Suppress non-essential headers and progress output. |
| `-y, --yes` | — | — | `false` | Skip interactive confirmation prompts on destructive commands. |
| `--timeout <secs>` | — | `timeout` | `30` | HTTP request timeout in seconds. |

Manage persistent CLI settings and shell tab-completion with `orion-cli config` and `orion-cli completions`:

```bash
orion-cli config set server_url http://localhost:8080
orion-cli config set api_key "$ORION_ADMIN_KEY"
orion-cli config show
orion-cli completions zsh > ~/.zfunc/_orion-cli
```

---

## Part 3: Offline authoring commands (`orion-cli`)

Offline authoring commands run locally with no server or database connection required.

### `orion-cli fmt`

Formats workflow, channel, connector, shared fragment, and test case JSON files in place using Orion's [canonical style](./workflows.md#canonical-formatting-orion-cli-fmt).

```bash
orion-cli fmt <dir | file>... [--check] [--stdin]
```

- `--check`: Print a diff and exit `1` if any file is not canonically formatted (without modifying files).
- `--stdin`: Read a JSON document or bare `tasks` array from standard input and write the formatted output to standard output.

### `orion-cli lint`

Validates the syntax, schemas, cross-file references (`workflow_id`, `connector`, `channel_call` targets), JSONLogic expressions, and secret references of a definition directory or file.

```bash
orion-cli lint <dir | file> [--deny-warnings] [--capabilities <file>] [--plugin-dir <dir>]
```

- `--capabilities <file>`: Validate channel protocols, transport schemas, and provider operations against a `target-capabilities.json` snapshot captured from `orion-cli deployments capabilities --output json`.
- `--plugin-dir <dir>`: Load `plugin.toml` manifests from `<dir>` to validate plugin task inputs.
- `--deny-warnings`: Exit with code `2` on advisory warnings as well as errors.

### `orion-cli clippy`

Runs `lint` followed by Orion's 20 [static analysis rules](./clippy.md).

```bash
orion-cli clippy <dir | file> [--deny-warnings] [--format text|json] [--fix [--check]] [-c <config.toml>]
orion-cli clippy --list
orion-cli clippy --explain <rule_id>
```

- `--fix`: Automatically apply proven structural fixes (such as wrapping `perf.redundant_step_condition` into a task group). Combine with `--check` to preview the diff and exit `1` without writing.
- `-c, --server-config <file>`: Check `metadata.vars.*` and `{"secret": "..."}` references against the `[vars]` and `[secrets]` tables in your `orion.toml`.

### `orion-cli compile`

Expands shared definitions (`$from`, `use`, `$use`, `$each`, `$sql`), validates the definition set, extracts connector credentials into `secret_locators`, and writes a content-addressed deployment directory (`deployment.json`, `definitions/`, `connections/`, `target.json`).

```bash
orion-cli compile <dir> -o <out-dir> [--capabilities <file>] [--require-edition community|standard|enterprise] [--plugin-dir <dir>]
```

### `orion-cli dry-run`

Executes a workflow offline against a sample input payload using mocked connector responses (`--stubs`) and prints the resulting context and step trace.

```bash
orion-cli dry-run -w <workflow.json> [-i <input.json>] [--metadata <meta.json>] [--stubs <stubs.json>] [--trace]
```

### `orion-cli test`

Runs declarative `*.case.json` test suites offline against your workflows and asserts on output paths (`expect`), task errors (`expect_errors`), outbound connector calls (`expect_calls`), and task execution statuses (`expect_tasks`).

```bash
orion-cli test <dir | case.json> [--filter <pattern>] [--plugin-dir <dir>]
```

### `orion-cli sql check`

Prepares every `db_read` and `db_write` SQL statement in a definition set against a live PostgreSQL, MySQL, or SQLite database (without executing writes) to verify table/column names, parameter types, and role permissions before deployment.

```bash
orion-cli sql check <dir> [--connector <name>] [--url <db-url>]
```

### `orion-cli plugin`

Generates Ed25519 signing keys, signs WebAssembly plugin digests for `[plugins.trust]`, and inspects `plugin.toml` + `.wasm` components.

```bash
orion-cli plugin keygen -o signer.pem
orion-cli plugin sign plugin.toml --key signer.pem
orion-cli plugin verify plugin.toml --public-key <base64-key>
```

---

## Part 4: Remote deployment and operational commands (`orion-cli`)

### `orion-cli deployments`

Plans, applies, inspects, waits on, and rolls back immutable v2 deployments.

```bash
orion-cli deployments capabilities [--output json|yaml] [--quiet]
orion-cli deployments plan (-d <compiled-dir> | -f <manifest.json>) [--base-revision <n>]
orion-cli deployments apply (-d <compiled-dir> | -f <manifest.json>) [--base-revision <n>] [--reviewed-digest <sha256>] [--idempotency-key <key>] [--reason <text>] [--wait-ms <ms>] [--wait-target this-node|observed]
orion-cli deployments list [--limit <n>] [--cursor <c>]
orion-cli deployments get <revision>
orion-cli deployments wait <revision> [--target this-node|observed] [--timeout-ms <ms>]
orion-cli deployments rollback --target-revision <n> --expected-head-revision <head> --idempotency-key <key> [--reason <text>]
```

### `orion-cli provider`

Scaffolds, lints, packs, uploads, admits, inspects, and revokes custom [connector provider packages](../enterprise/provider-packages.md).

```bash
orion-cli provider scaffold --publisher <ns> --provider-id <id> [--version <semver>] [--kind rust|workflow|wasm] [--tier community|standard|enterprise]
orion-cli provider lint -f <package.json>
orion-cli provider pack -f <package.json>
orion-cli provider upload -f <upload.json>
orion-cli provider admit <digest> [--lock <provider.lock.json>]
orion-cli provider list [--limit <n>] [--cursor <c>]
orion-cli provider get <digest>
orion-cli provider revoke <digest> [--reason <text>]
```

### `orion-cli connections`

Manages and probes immutable connection revisions on the server.

```bash
orion-cli connections list [<connection-id>]
orion-cli connections get <connection-id> [--revision <n>]
orion-cli connections create <connection-id> -f <connection.json> [--bind <alias=target_id>]
orion-cli connections probe <connection-id> [--revision <n>]
```

### Data, traces, DLQ, cron, and operational commands

| Command | Synopsis | Purpose |
|---|---|---|
| `send` | `orion-cli send <channel> (-d <json> \| -f <file>) [--raw] [--async-mode [--wait]] [--profile]` | Send a test request to a deployed channel (`--raw` sends the body without wrapping in `{"data": ...}`). |
| `streams` | `orion-cli streams tail <channel> [--last-event-id <id>]` | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> Follow an [SSE channel stream](../enterprise/sse.md) from the terminal. |
| `traces` | `orion-cli traces <list\|get\|wait>` | List traces (`--channel`, `--status`, `--mode`), fetch a trace by ID (`--token`), or poll until completion. |
| `dlq` | `orion-cli dlq <list\|get\|requeue\|purge>` | Inspect failed `/async` traces in `trace_dlq`, requeue an entry, or purge exhausted entries (`--older-than-hours`). |
| `cron` | `orion-cli cron <status\|list\|get\|trigger\|retry\|cancel>` | Inspect cron channel schedules, list/get occurrences, manually trigger a cron channel, or retry/cancel an occurrence. |
| `circuit-breakers` | `orion-cli circuit-breakers <list\|reset <key>>` | List `(channel:connector)` circuit breaker states or reset a tripped breaker across the cluster. |
| `cache` | `orion-cli cache invalidate <namespace>` | Increment a channel response-cache invalidation namespace counter. |
| `audit-logs` | `orion-cli audit-logs list [--action <a>] [--resource-type <t>] [--resource-id <id>] [--principal <p>] [--start-time <ts>] [--end-time <ts>]` | Query the admin audit trail. |
| `backups` | `orion-cli backups <create\|list>` | Trigger or list online SQLite backups (`VACUUM INTO`). |
| `health` | `orion-cli health [--ready]` | Check `/health` (or `/readyz` with `--ready`). |
| `engine` | `orion-cli engine status` | Inspect the active engine generation and any `load_issues` (`channels`, `plugins`, `connectors`). |
| `functions` | `orion-cli functions <list\|get <name>>` | List all registered task functions and inspect their input schemas. |
| `metrics` | `orion-cli metrics` | Fetch Prometheus metrics from `/metrics`. |

---

## Part 5: Shared definition constructs (`shared.json`)

When `orion-cli compile` compiles a definition directory, it resolves five compile-time authoring constructs before emitting canonical JSON definitions:

| Construct | Where used | What `orion-cli compile` resolves it to |
|---|---|---|
| `{"$from": "constants.<name>"}` or `{"$from": "errors.<name>"}` | Any JSON object or value | Copies the named constant or error object from a shared document (`constants` / `errors`), merging any sibling keys as overrides. |
| `{"id": "step_1", "use": "<fragment>", "with": {...}}` | Inside `tasks` arrays | Expands a parameterized task-list fragment declared under `fragments.<fragment>.tasks`, prefixing inner task IDs with `step_1.`. |
| `{"$use": "<fragment>", "with": {...}}` | Any JSON value position | Expands a parameterized value fragment declared under `fragments.<fragment>.value`. |
| `{"$each": [...], "as": "item", "do": {...}}` | Inside JSON arrays | Repeats the `do` template once per element of `$each` (up to `4096` elements). Inside `do`, `{"$param": "item"}` substitutes the value and `{{item}}` interpolates scalars into strings. |
| `{"$sql": "sql/query.sql"}` | `db_read.input.query` and `db_write.input.query` | Inlines the SQL statement from the relative `.sql` file. |

## Next steps

- [Test and lint offline](../guides/testing.md): Walkthrough of `fmt`, `lint`, `clippy`, `dry-run`, and `test`.
- [CI/CD and deployment workflows](../guides/ci-cd.md): Automate `compile`, `deployments plan`, and `deployments apply` in CI.
