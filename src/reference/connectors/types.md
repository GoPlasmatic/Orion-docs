<!-- description: The three built-in connector types — http, db and cache — with the config each one takes, and the edition providers that extend them. -->
<!-- type: hub -->
<!-- last_verified: 2026-11-16 -->

# The three types

The three built-in types a connector may be, and the `config` each one takes. An edition adds seven providers that a set names by `provider` instead of `connector_type`; [Edition providers](./edition/index.md) specifies them.

| Type | Backs | Task functions |
|------|-------|----------------|
| [`http`](./http.md) | REST APIs and webhooks | `http_call` |
| [`db`](./db.md) | PostgreSQL, MySQL, SQLite | `data_query`, `data_write`, `db_read`, `db_write` |
| [`cache`](./cache.md) | Per-node in-process memory | `cache_read`, `cache_write` |

Type values match case-insensitively. Any other value is refused with the valid list. A stored connector whose config no longer parses fails to load and surfaces as a connector load issue.

**Required** `yes` on a type's field table means a connector without the field is refused. `—` in **Default** means the field is optional and unset until you set it.

## Related

- [Connector types](./index.md): every type, and the shared blocks all of them carry.
- [Task functions](../functions/index.md): the functions that call through a connector.
- [Shared connector blocks](./common.md): what every type carries whatever it is.
- [Edition providers](./edition/index.md): the seven providers an edition adds and the `provider` field that names them.
- [Portable data dialect](../data-dialect.md): the language `db` serves.
