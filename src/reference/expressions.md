<!-- description: Complete JSONLogic operator reference for Orion workflows and channel guards: core, dates, strings, arrays, math, encoding, randomness, and secrets. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# Expression language (JSONLogic)

Orion uses [JSONLogic](https://jsonlogic.com) (compiled and evaluated by `datalogic-rs`) for workflow `condition` blocks, task `condition` blocks, `map` mappings, `validation` rules, `filter` conditions, dynamic task inputs, and channel guards (`validation_logic`, `authorization_logic`, `rate_limit.key_logic`, and `cache.key_logic`).

> [!WARNING]
> Inside a `map` task's `logic` field, a misspelled operator name (for example, `{"uppr": [...]}` instead of `{"upper": [...]}`) is **not** an error—JSONLogic treats an unrecognized single-key object in a templating position as a literal JSON object and writes `{"uppr": ...}` to the target path. In `condition` blocks, unknown operators fail compilation at `lint` and upload time.

---

## Available operators

The tables in this section list every JSONLogic operator compiled by Orion.

### Core operators

| Operator | Example | Meaning |
|----------|---------|---------|
| `var` / `val` | `{ "var": "data.order.total" }` | Read a value from the context by dotted path |
| `val` (path chain) | `{ "val": ["data", "items", { "val": ["temp_data", "i"] }] }` | Read a path with computed segments (used to index an array by a dynamic integer) |
| `==` / `!=` | `{ "==": [{ "var": "data.type" }, "order"] }` | Equality / inequality (with type coercion) |
| `===` / `!==` | `{ "===": [{ "var": "data.qty" }, 1] }` | Strict equality / inequality (no type coercion) |
| `>` `>=` `<` `<=` | `{ ">": [{ "var": "data.order.total" }, 10000] }` | Numeric and lexicographic comparison (supports 3-argument range chains) |
| `and` / `or` / `!` | `{ "and": [a, b] }` | Boolean conjunction, disjunction, and negation |
| `!!` | `{ "!!": [{ "var": "data.order.id" }] }` | Coerce to boolean truthiness |
| `if` / `?:` | `{ "if": [cond, then_val, else_val] }` | Conditional branching |
| `+` `-` `*` `/` `%` | `{ "*": [{ "var": "data.qty" }, 1.1] }` | Arithmetic addition, subtraction, multiplication, division, and modulo |
| `max` / `min` | `{ "max": [1, 2, 3] }` | Maximum or minimum value |
| `cat` | `{ "cat": ["Order #", { "var": "data.order.id" }] }` | Concatenate values into a string |
| `substr` | `{ "substr": [{ "var": "data.code" }, 0, 3] }` | Extract substring by start index and optional length |
| `in` | `{ "in": [{ "var": "data.tier" }, ["vip", "premium"]] }` | Array membership or substring containment |
| `merge` | `{ "merge": [[1, 2], [3]] }` | Concatenate arrays one level deep |
| `map` / `filter` / `reduce` | `{ "map": [{ "var": "data.items" }, { "var": "price" }] }` | Per-element array transformation, filtering, and reduction |
| `all` / `some` / `none` | `{ "some": [{ "var": "data.items" }, { ">": [{ "var": "qty" }, 0] }] }` | Quantifiers over an array |
| `missing` / `missing_some` | `{ "missing": ["data.order.id"] }` | Return missing context paths |

### Dates and durations (`datetime`)

| Operator | Example | Meaning |
|----------|---------|---------|
| `now` | `{ "now": [] }` | Current UTC timestamp as an RFC 3339 string |
| `datetime` | `{ "datetime": ["2026-07-31T00:00:00Z"] }` | Parse an RFC 3339 timestamp |
| `parse_date` | `{ "parse_date": [{ "var": "data.when" }, "yyyy-MM-dd"] }` | Parse a date string using an explicit format (`yyyy`, `MM`, `dd`, `HH`, `mm`, `ss`) and optional IANA timezone |
| `format_date` | `{ "format_date": [{ "now": [] }, "yyyy-MM-dd", "Asia/Kolkata"] }` | Format a datetime, optionally converting to an IANA timezone |
| `date_diff` | `{ "date_diff": [a, b, "days"] }` | Whole units between two datetimes (`"days"`, `"hours"`, `"minutes"`, `"seconds"`, `"milliseconds"`) |
| `timestamp` | `{ "timestamp": ["1d"] }` | Parse a duration string (`"1d"`, `"2h30m"`) for date arithmetic |

### Strings (`ext-string` and Orion string operators)

| Operator | Example | Meaning |
|----------|---------|---------|
| `length` | `{ "length": [{ "var": "data.items" }] }` | Character length of a string or element count of an array |
| `upper` / `lower` | `{ "upper": [{ "var": "data.code" }] }` | Convert string to uppercase or lowercase |
| `trim` | `{ "trim": [{ "var": "data.name" }] }` | Strip leading and trailing whitespace |
| `split` | `{ "split": [{ "var": "data.csv" }, ","] }` | Split a string by delimiter into an array |
| `starts_with` / `ends_with` | `{ "starts_with": [{ "var": "data.sku" }, "AB-"] }` | Prefix or suffix match |
| `url_encode` | `{ "url_encode": [{ "var": "data.email" }] }` | RFC 3986 percent-encoding (spaces become `%20`) |
| `url_decode` | `{ "url_decode": [{ "var": "data.state" }] }` | Strict inverse of `url_encode` |
| `join` | `{ "join": [{ "var": "data.tags" }, ", "] }` | Join array elements with a required separator string |

### Arrays (`ext-array`) and math (`ext-math`)

| Operator | Example | Meaning |
|----------|---------|---------|
| `sort` | `{ "sort": [{ "var": "data.scores" }] }` | Sort array ascending |
| `slice` | `{ "slice": [{ "var": "data.items" }, 0, 2] }` | Extract sub-array by start and end index |
| `abs` | `{ "abs": [{ "var": "data.delta" }] }` | Absolute value |
| `ceil` / `floor` | `{ "ceil": [{ "var": "data.price" }] }` | Round up or down to the nearest integer |
| `group_by` | `{ "group_by": [{ "var": "data.meetings" }, { "var": "day" }] }` | Group array elements by key expression into `[{key, items}]` |
| `distinct` | `{ "distinct": [{ "var": "data.tags" }] }` | Deduplicate array elements (preserving first occurrence) |

### Objects (`ext-object`)

| Operator | Example | Meaning |
|----------|---------|---------|
| `keys` / `values` | `{ "keys": [{ "var": "data.totals" }] }` | Extract an object's keys or values as an array |
| `entries` | `{ "entries": [{ "var": "data.totals" }] }` | Convert an object into `[{"key": k, "value": v}]` pairs |

### Control flow and error handling

| Operator | Example | Meaning |
|----------|---------|---------|
| `??` | `{ "??": [{ "var": "data.nickname" }, "anonymous"] }` | Nullish coalescing (returns first non-null argument) |
| `type` | `{ "type": [{ "var": "data.price" }] }` | Runtime value type name as a string |
| `exists` | `{ "exists": ["data", "order", "id"] }` | Test whether a nested path of literal segments exists |
| `switch` / `match` | `{ "switch": [{ "var": "data.tier" }, [["gold", 20], ["silver", 10]], 0] }` | Multi-way branch over `[case, result]` pairs with a default value |
| `try` / `throw` | `{ "try": [expr, fallback] }` | Catch or raise an expression evaluation error |

### Encoding, randomness, and secrets (Orion operators)

| Operator | Example | Meaning |
|----------|---------|---------|
| `base64_encode` / `base64_decode` | `{ "base64_encode": [{ "var": "data.raw" }] }` | Standard RFC 4648 §4 Base64 encode/decode |
| `base64url_encode` / `base64url_decode` | `{ "base64url_encode": [{ "var": "data.claims" }] }` | Unpadded URL-safe Base64 encode/decode (JWS format) |
| `hex_encode` / `hex_decode` | `{ "hex_encode": [{ "var": "data.token" }] }` | Lowercase hexadecimal encode/decode |
| `random` | `{ "random": ["digits", 6] }` | CSPRNG generation: `["uuid"]`, `["uuid", "v7"]`, `["digits", n]`, `["int", min, max]`, `["string", len, alphabet?]`, `["bytes", n, encoding?]` |
| `secret` | `{ "secret": "partner_hmac" }` | Read a secret declared in `[secrets]` in `orion.toml` (refused in `map` and `log` where the output would be recorded) |

---

## Operator caveats and common pitfalls

- **`exists` requires an array of path segments, not a dotted string**: Write `{"exists": ["data", "order", "id"]}`. Writing `{"exists": ["data.order.id"]}` checks for a single top-level key literally named `"data.order.id"`.
- **`switch` takes an array of `[case, result]` pairs**: The second argument to `switch` must be `[[case1, val1], [case2, val2]]`, followed by the default fallback as the third argument.
- **`date_diff` units must be plural**: Use `"days"`, `"hours"`, `"minutes"`, `"seconds"`, or `"milliseconds"`. Singular names such as `"day"` raise an evaluation error.
- **Document-shaped connector fields fold `{"var": ...}` only**: In `cache_write.input.value`, `jwt_sign.input.claims`, and `db_read`/`db_write`/`data_query`/`data_write` `params`, Orion resolves `{"var": "..."}` references and leaves other single-key objects untouched as literal data. Compute complex expressions in a preceding `map` task into `temp_data.*` and reference the result with `{"var": "temp_data.*"}`.

## Next steps

- [Workflow schema and formatting](./workflows.md): How conditions and mappings fit into the workflow pipeline.
- [Task functions](./functions.md): Input schemas for `map`, `filter`, `validation`, and connector tasks.
