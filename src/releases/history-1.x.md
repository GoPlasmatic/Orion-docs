<!-- description: Historical release and upgrade notes for Orion 1.x (1.12.0, 1.11.0, 1.10.0, 1.9.0, 1.8.0, 1.6.0, 1.2.0, 1.1.0, and 1.0.0) for teams migrating to Orion 2.0. -->
<!-- type: migration -->
<!-- last_verified: 2026-11-16 -->

# Orion 1.x release history

This page consolidates the behavioral changes, breaking renames, and migration notes across the Orion 1.x release series (`1.0.0` through `1.12.0`). If you are migrating an older 1.x or 0.3.x deployment to Orion 2.0, review the sections below alongside [Upgrade to Orion 2.0](./upgrade-to-2.0.md).

---

## Orion 1.12.0

Orion 1.12.0 moved the workflow engine to dataflow-rs 3.15, which changed how `engine.ops_budget` charges some operators and how `var` defaults resolve, and added plugin memory residency settings.

### 1. `engine.ops_budget` charges more for collection and string operators

`merge`, `in`, `keys`/`values`/`entries`, `sort`, `distinct`, `group_by`, `cat`, `split`, and deep equality now charge per item, or per 64 bytes of string, so a `merge` accumulator is priced for the quadratic work it does. If you set a positive `engine.ops_budget` sized tightly against an earlier release, and your rules use these operators, re-measure the ceiling before you upgrade. See [Server configuration and environment variables](../reference/configuration.md).

### 2. `var` with a computed path returns its default on a miss

In `{"var": [computed_path, default]}`, when the path was an expression, earlier releases read the default as a second path segment, so the `var` answered `null` whether or not the path existed. From 1.12.0 it returns the default when the path misses. Review any expression of this form whose callers expected `null` on a miss.

### 3. A saturated `forbid` cron channel records skips immediately

When a cron tick comes due while every slot of its `forbid` concurrency key is held, the reconciler writes the occurrence as `skipped_singleton` straight away instead of queuing it as `pending`. A row written already settled (this one, or a misfire summary) now carries `completed_at`, like one a worker settles. If dashboards or scripts track `pending` occurrences on busy `forbid` channels, expect `skipped_singleton` rows instead. See [Schedule jobs with cron](../guides/scheduled-jobs.md).

### 4. Plugin pooled instances keep memory resident (new defaults)

The `[plugins]` section gained `linear_memory_keep_resident_bytes` (default 1 MiB) and `table_keep_resident_bytes` (default 64 KiB). They set how much of a returned pooled instance stays mapped between invocations, so the next call on that slot does not fault its pages back in. Both apply on Linux only; elsewhere they are ignored. The resident cost is at most that much per pooled memory that has been used, so include it when you size the container's memory limit, or lower the values with `ORION_PLUGINS__LINEAR_MEMORY_KEEP_RESIDENT_BYTES` and `ORION_PLUGINS__TABLE_KEEP_RESIDENT_BYTES`. See [Server configuration and environment variables](../reference/configuration.md).

---

## Orion 1.11.0

Orion 1.11.0 added multi-provider OAuth2 sign-in channels, OIDC discovery from an `issuer`, and a normalized `metadata.identity`, and stopped long-lived processes from retaining resources across reload and delete.

### 1. `metadata.identity` is platform-reserved

On an OAuth2 sign-in channel, Orion stamps `metadata.identity` (`provider`, `kind`, `subject`, `login`, `name`, `email`, `picture`) from the verified `id_token` claims, or from a `userinfo_url` request for a provider that has no `id_token`. `subject` is always a string. The slot is platform-reserved, so a caller cannot pre-seed it; read identity fields only from what Orion stamps. See [Channel configuration reference](../reference/channels.md).

### 2. Outbound requests send a `User-Agent` (1.11.1+)

From 1.11.1, every request on the shared HTTP client sends `User-Agent: orion/<version>`: `http_call`, the OAuth2 token exchange, OIDC discovery, the JWKS fetch, and the userinfo fetch. `orion-client`, the transport behind `orion-cli` and `orion-server package plan/apply`, sends `User-Agent: orion-client/<version>`. Earlier releases sent no `User-Agent`, which made APIs that require one (such as GitHub's REST API) refuse the request. If an upstream filters on `User-Agent`, allow these values, or set `user-agent` in an `http_call`'s `headers` or on its connector, which still takes precedence.

---

## Orion 1.10.0

Orion 1.10.0 brought the dataflow-rs 3.14 loop and fan-out syntax into Orion's validation and tooling, added response-cache invalidation by namespace and cron occurrence cancellation, shortened cron claim leases, and closed two metadata spoofing paths at ingress.

### 1. Callers can no longer supply `metadata.auth` or `metadata.trigger`

The HTTP ingress used to copy the caller's `metadata` object and overwrite `auth` only when a `jwt` token verified. On a channel with no `auth`, an `api_key` or `hmac` channel, or a `jwt` channel with `required: false` called without a token, a workflow could read a caller-written `metadata.auth.claims` as verified claims. Both `metadata.auth` and `metadata.trigger` are now stripped at ingress, like `oauth`, `cookies`, and `vars`. A client that relied on sending its own `metadata.auth` loses it; move such values into the request body, or configure channel authentication so Orion fills `metadata.auth` from a verified token.

### 2. Response-cache `key_logic` reads verified claims

A channel's response-cache `key_logic` was evaluated before the claims merge, so a key on `metadata.auth.claims.sub` read the request envelope, and a caller with a valid token could store a response under another user's key. It now reads the same metadata as `validation_logic`. A request without a verified token resolves such a key to `null` and bypasses the cache, so expect those requests to run the workflow every time.

### 3. `cache.key_logic` takes precedence over `cache_key_fields`

A channel that declared both used to key on `cache_key_fields` and never evaluate `cache.key_logic`. From 1.10.0 the expression wins, as documented, which changes which requests share a cache entry. Run `orion-server preflight` before upgrading: it reports such channels as `cache-key-precedence`. Remove whichever of the two you do not intend to use.

### 4. Cron claim leases no longer stretch to the channel timeout

A running cron attempt now holds its claim for `cron.claim_lease_secs`, renewed every heartbeat, whatever its timeout. Previously the lease was `max(claim_lease_secs, timeout + heartbeat)`, so a node that died held its singleton slots for the channel's whole timeout. An attempt that cannot renew stops itself before its lease runs out. As a result, a state-database outage longer than `claim_lease_secs` minus one heartbeat (45 seconds at the defaults) now cancels running cron attempts, which are retried once the database is back. Raise `cron.claim_lease_secs` to ride out longer outages. Holds taken by older binaries keep their long leases until those nodes are replaced. See [Server configuration and environment variables](../reference/configuration.md).

### 5. Stricter `loop` and `for_each` validation

- A key that `loop` or `for_each` does not define is refused as `UNKNOWN_FIELD`, with a suggestion. Previously a typo such as `max_concurency` or `steup` was silently ignored.
- A loop the engine refuses (a duplicate step ID across `loop.setup` and `tasks`, or a loop `as` that is not a plain key) is now refused at create and update, and a stored one quarantines only its own channel. Earlier releases accepted such a workflow with a `201`, after which every reload failed with a `500` and a node stopped at boot. `orion-server preflight` checks stored loops.
- `PUT {"loop": null}` now clears a workflow's loop instead of leaving it in place.

Lint your definitions and run `preflight` before upgrading. See [Workflow definition reference](../reference/workflows.md).

### 6. Test and dry-run output changes

- `dry-run` reports `matched` as "any task ran". Earlier releases reported `matched: true` for a workflow whose condition was false; update scripts that read it.
- `dry-run --trace` accepts `full` (the default, each step snapshotting only its own audit entry), `steps`, or `none`, and the output gains `tasks`.
- `orion-server test` runs a case as a node runs an untraced message, unless an `expect` path is rooted at `audit_trail`.
- `POST /api/v1/admin/workflows/{id}/test` bounds its trace like a `task_details` trace: each step snapshots only its own audit entry, within `trace_queue.max_result_size_bytes`.

See [Test and debug locally](../guides/testing.md).

---

## Orion 1.9.0

Orion 1.9.0 added `.sql` statement files, value fragments and `$each`, `package apply --prune`, boot-time package apply, `sql check`, `clippy --fix`, and bounded cron concurrency (`concurrency.slots`), and tightened config placeholder parsing and package apply failure reporting.

### 1. Placeholders in config-file comments are no longer substituted

Substitution used to run over the raw config file, so a `#` comment mentioning `${R2_ENDPOINT}` made that variable required at boot, and the shipped `config.toml.example` wrote its comments with `$${…}` to survive it. From 1.9.0 the file's `#` comments are skipped (a `#` inside a quoted value or a placeholder's default is still text), and `$$` in a comment stays as written. Connector `config_json` and connection strings still treat `#` as data.

- Every substitution error names the file, line, and column instead of a byte offset.
- Unsupported shell forms (`${VAR:+x}`, `${VAR-x}`) are refused with an error that lists the supported forms.
- `${VAR:?message}` and nested defaults (`${A:-${B:-c}}`) are new; `:-` still falls back only when the variable is unset.

See [Server configuration and environment variables](../reference/configuration.md).

### 2. Newly reserved keys: `$sql`, `$use`, `$each`, and `package`

- An object with a string `$sql` anywhere in a workflow's tasks, condition, or loop, or in a connector's or channel's config, is refused at create and update with `UNCOMPILED_SOURCE`, as `$from` is. Stored rows keep running, but `preflight` reports a stored workflow holding one as `source.sql_key`, because its next update would be refused.
- A string-valued `$use` and an object-valued `$each` are newly reserved at the admin API.
- `package` is a newly reserved shared-document key.

Run `preflight` and rename any such keys that are meant as literal data. Source-form syntax must be compiled away with `compile` before it reaches the admin API. See [CLI reference](../reference/cli.md).

### 3. Fragments may include fragments, and `{{name}}` interpolates

`shared.fragment_nested` is retired: a `use` inside a fragment, at any depth, now expands. Inside a task fragment, text `{{x}}` where `x` is one of its parameters is now interpolated, which can move a recompiled package's content hash. Check fragments for literal `{{…}}` text that matches a parameter name, and expect a new content hash the first time you recompile.

### 4. `clippy` may newly fail a set that passed

Two new deny rules exit non-zero where they fire: `correctness.mapping_always_null` (a `map` mapping whose `logic` folds to `null`, so its target is never written) and `correctness.sql_bind_count` (a literal PostgreSQL `db_read`/`db_write` statement whose `$n` placeholders do not match its literal `params`). Each fires only on a definition that fails or does nothing on every execution, so fix the definition rather than suppressing the rule. See [Clippy rules](../reference/clippy.md).

### 5. `package` and `compile` behavior changes

- **`package apply` fails when the reload quarantines what it carries.** It used to print `applied` and flip the receipt while the node served the package without a quarantined member. Apply now fails naming each quarantined member before the receipt flip, so the receipt stays `staged` and a re-run after the fix completes it. Re-applying the version a node already runs reads `GET /engine/status` and fails the same way. An older server that cannot report load issues produces a warning, not a failure.
- **Re-applying a superseded version rolls back again.** Earlier releases printed `already applied with identical content — nothing to do` even when a later version had been applied since. Now the current version is still a no-op, and a superseded one is staged, activated, and reloaded.
- **An import whose signature differs is no longer `unchanged`.** Under `on_conflict=new_version`, `POST /plugins/import` and `POST /models/import` now write a differing signature (`updated_draft`, or `new_version` over an active row).
- **`compile` and `package export` refuse names and versions the target would refuse.** A version outside letters, digits, `.`, `_`, and `-`, or longer than 64 characters, is refused before an artifact is written, and `content` is a reserved `--version` value.

Update deployment pipelines that treated `applied` as success regardless of load issues.

### 6. Connector endpoints are scheme-checked after references resolve

An `env://` URL that resolved to a non-HTTP scheme such as `ftp://…` used to reach the client unjudged and fail late. It is now an `endpoint` load issue naming the scheme (never the value). Any connector this refuses was already non-functional. After upgrading, check the load issues that `GET /engine/status` and `/health` report.

### 7. Upgrade every node before using `concurrency.slots`

`concurrency.slots` lets a `forbid` cron channel run up to that many instances of its key at once (default `1`, the single-run `forbid` behavior). Slots 1 and up live in a new table (migration `cron_singleton_slots`, expand-only). An older node refuses the field and quarantines the channel, so upgrade every node before activating a channel that sets `slots`, and remove `slots` first if you roll back. The `package apply --prune` inventory is also a new expand-only migration (`package_receipt_inventory`); a receipt written before it prunes nothing. See [Schedule jobs with cron](../guides/scheduled-jobs.md).

### 8. dataflow-rs 3.14 syntax is unsupported in 1.9.1

1.9.1 moved to dataflow-rs 3.14. Its new syntax (`loop.setup`, `loop.over`, `loop.as`, `loop.scratch`, a task's `for_each`, and a `map` mapping's `unset` and `on_null`) passes `orion-server lint` and runs as authored, but Orion's loop checks, `clippy`, and `fmt` do not cover it in 1.9.1, so treat it as unsupported there. Orion 1.10.0 adopted it across validation and tooling.

---

## Orion 1.8.0

Orion 1.8.0 enabled the 20 tensor operators in the JSONLogic expression engine and added the `engine.ops_budget` evaluation ceiling.

### 1. Twenty tensor operator names became live in templates

Enabling the `tensor` feature in `dataflow-rs` made 20 operator names active in template positions (`map` mapping `logic` and expression-typed function inputs): `tensor`, `zeros`, `full`, `scatter`, `rle_expand`, `one_hot`, `stack`, `concat`, `unstack`, `reshape`, `transpose`, `pad`, `crop`, `cast`, `normalize`, `argmax`, `gather`, `to_list`, `shape`, and `dtype`.

In template positions, a single-key object whose key matches an active operator is evaluated as an operator call rather than emitted as a literal object. If a workflow previously emitted a single-key literal object using one of those names, escape the key with a leading `$`:

```json
{ "path": "data.board", "logic": { "$shape": [6, 7] } }
```

See [Expression language (JSONLogic)](../reference/expressions.md) for the full operator table and `$` escaping rules.

### 2. `engine.ops_budget` added (default `0`)

The `[engine]` section gained `ops_budget = 0` (`0` disables the ceiling). When set to a positive integer, expressions that exceed the node budget fail their task with `BUDGET_EXCEEDED` (or evaluate to `false` in routing/task conditions). See [Server configuration and environment variables](../reference/configuration.md).

---

## Orion 1.6.0

Orion 1.6.0 introduced the WebAssembly plugin sandbox (`[plugins]`, off by default), scheduled cron channels (`[cron]`, on by default), stricter `db_read` statement validation, and concrete SQL driver decoding.

### 1. `db_read` enforces read-only SQL statements

Previously, `db_read` checked the connector's `read` permission gate and passed the raw SQL string to the database driver, which allowed `DELETE`, `UPDATE`, or `INSERT` statements (including `DELETE ... RETURNING`) to execute. From 1.6.0 onward, `db_read` statements must begin with `SELECT`, `WITH` (without data-modifying CTEs), `VALUES`, or `TABLE`. Move any mutating SQL statements to `db_write` or `data_write`.

### 2. Concrete SQL driver decoding (`numeric_as` and `binary_as`)

SQL connectors switched from `sqlx::AnyPool` to concrete PostgreSQL, MySQL, and SQLite drivers so `uuid`, `numeric`, `timestamptz`, `jsonb`, arrays, and `inet` columns decode cleanly.

- **`numeric` / `decimal`**: Decodes to a JSON number (`f64`) by default. Set `numeric_as: "string"` on `db_read` or `data_query` to preserve exact decimal digits on currency columns.
- **Binary columns (`bytea`, `blob`, `varbinary`)**: Set `binary_as` (`"hex"` or `"base64"`) explicitly rather than relying on UTF-8 fallback detection.

### 3. Trace token query parameter (`?token=`) deprecated

Passing an async trace capability token in the URL query string (`?token=...`) was deprecated in favor of the `x-trace-token` HTTP header so capability tokens do not leak into proxy access logs or browser history. Trace responses also added `Cache-Control: no-store`.

---

## Orion 1.2.0

Orion 1.2.0 added recursive task groups (`children`, `mode`, `terminal`), shared definition fragments, and directory-level static analysis (`lint`), while tightening test case paths and removing the legacy CLI MCP server.

### 1. Test case `expect` paths must include a root segment

In `*.case.json` files used by `orion-cli test`, every key in `expect` must start with a valid message root (`data.`, `metadata.`, `temp_data.`, `calls`, or `audit_trail`). Previously, unrooted paths implicitly defaulted to `data.`, which caused `"metadata.foo": null` to read `data.metadata.foo` and pass silently. See [Test and debug locally](../guides/testing.md).

### 2. Empty workflow task lists (`"tasks": []`) rejected

Workflows with an empty `tasks` array are rejected at validation and compile time (`tasks` must contain at least one step).

### 3. CLI MCP server (`orion-cli mcp serve`) removed

The unauthenticated `orion-cli mcp serve` subcommand was removed in 1.2.0. To use AI coding assistants with Orion, install the agent skill or prompt pack so the assistant runs `orion-cli` directly under your shell credentials. See [AI-assisted authoring](../guides/ai-authoring.md).

### 4. Fragment task IDs inside task groups namespaced (1.2.1+)

When a shared fragment contains a nested task group, every task ID inside the group is prefixed flatly with `{call-site}.{id}` so multiple inclusions cannot collide with `DUPLICATE_TASK_ID`.

---

## Orion 1.1.0

Orion 1.1.0 added JWT channel authentication, managed OAuth2 client credentials, and specific task error codes.

### 1. Unresolved `rate_limit.key_logic` headers return `429`

If a channel's `rate_limit.key_logic` evaluates to `null` or `""` (for example, because it reads a custom request header not listed in `rate_limit.key_headers`), the request is rejected with `429 RATE_LIMITED` and increments `orion_rate_limit_key_unavailable_total` instead of silently bucketing all callers under the literal string `"null"`. Eight headers are always available in the rate-limit key context (`authorization`, `x-api-key`, `x-forwarded-for`, `x-real-ip`, `user-agent`, `content-type`, `origin`, and `x-tenant-id`); list any additional headers in `rate_limit.key_headers`.

### 2. Specific `errors[].code` values replace generic `TASK_ERROR`

When a task fails on a channel with `continue_on_error: true`, `errors[].code` and `metadata._orion_errors` report the specific failure category (`IO_ERROR`, `TIMEOUT_ERROR`, `FUNCTION_ERROR`, or `circuit_open`) instead of always reporting `TASK_ERROR`.

### 3. Connector header values masked and error URLs redacted

Admin API reads mask all values inside `headers`, `query_params`, and `extra_params` regardless of key name, and `http_call` error messages redact connector URLs and truncate upstream error bodies to 512 bytes.

---

## Orion 1.0.0

Orion 1.0.0 hardened configuration validation, SSRF protection, channel configuration schemas, and the portable data dialect over the 0.3.0 release.

### 1. Renamed and removed configuration keys

Four TOML sections and one environment variable were renamed in 1.0.0, and two obsolete settings were removed:

| Pre-1.0 setting | 1.0+ replacement |
|---|---|
| `[queue]` (`ORION_QUEUE__WORKERS`, etc.) | `[trace_queue]` (`ORION_TRACE_QUEUE__WORKERS`, etc.) |
| `queue.trace_retention_hours` | `trace_queue.retention_hours` |
| `queue.trace_cleanup_interval_secs` | `trace_queue.cleanup_interval_secs` |
| `queue.audit_retention_days` | `audit.retention_days` (plus `audit.cleanup_interval_secs`) |
| `[channels]` | `[channel_filter]` |
| `[tracing.storage]` | `[trace_storage]` |
| `ORION_ENV` | `ORION_ENVIRONMENT` |
| `kafka.max_inflight` (`ORION_KAFKA__MAX_INFLIGHT`) | *Removed* — reports `removed in 1.0 (K4)` because Kafka messages are processed sequentially per consumer |
| `engine.reload_timeout_secs` (`ORION_ENGINE__RELOAD_TIMEOUT_SECS`) | *Removed* — lock-free atomic engine swap eliminated the reload lock wait |

### 2. Misspelled `ORION_*__` environment overrides fail startup

Any `ORION_*` environment variable containing the `__` section separator that does not match a known server setting (or a near-miss of `ORION_ENVIRONMENT`) stops the server at startup instead of being silently ignored:

- `ORION_SERVER__PORTT` fails startup with `did you mean ORION_SERVER__PORT?`.
- `ORION_ADMIN_AUTH__API_KEY` (singular) fails startup; the valid setting is `ORION_ADMIN_AUTH__API_KEYS` (plural).
- Single-underscore names like `ORION_SERVER_PORT` or Kubernetes service links (`ORION_SERVICE_HOST`) are ignored by the config loader because they do not contain `__`.
- If you store connector secrets in environment variables referenced via `env://`, do not name them with a double underscore under `ORION_` (such as `ORION_DB__PASSWORD`), or the config loader rejects them as unknown server settings. Use `ORION_SECRET_*` (for example, `ORION_SECRET_DB_PASSWORD`), which is reserved for secrets and skipped by the config check.

### 3. Private network connectors require `allow_private_urls: true`

SSRF protection applies to all connector types. Any connector pointing to a loopback, link-local, or RFC 1918 private address (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`) must explicitly set `"allow_private_urls": true` in its connection configuration.

### 4. Channel and workflow field renames

- **`data_write` envelope nested under `write`**: The mutation fields (`op`, `target`, `values`, `set`, `filter`, `on_conflict`, `returning`, `all`) must be nested inside an `"input": { "write": { ... } }` object, matching `data_query`'s `"query"` object.
- **Channel `cors` renamed to `origin_allow_list`**: Per-channel origin filtering uses `"origin_allow_list": ["https://app.example.com"]`.
- **Channel `backpressure` fields**: `backpressure.max_concurrent` was renamed to `backpressure.max_concurrent_per_node`, and the unused `backpressure.queue_depth` field was removed.
- **Trace read endpoints moved to Admin API**: `GET /api/v2/admin/traces` and `GET /api/v2/admin/traces/{id}` replaced the pre-1.0 `/data/traces` paths.

## Related pages

- [Upgrade to Orion 2.0](./upgrade-to-2.0.md) — current major-version migration guide
- [Versioning and support policy](./versioning-policy.md) — support window and compatibility contract
- [Server configuration and environment variables](../reference/configuration.md) — current server settings and environment variable rules
