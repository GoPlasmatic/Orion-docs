<!-- description: List, read, create and probe immutable v2 connection revisions, with repeatable binding and secret-locator flags. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# `orion-cli connections`

Manages immutable v2 connection revisions and runs audited health probes against them. A connection revision is the configuration a provider runs with, and a change is always a new revision.

## Synopsis

```bash
orion-cli connections list [--id <id>] [--limit <n>] [--cursor <c>]
orion-cli connections get <id> [--revision <n>]
orion-cli connections create <id> -f <file> [--bind <alias=connection>]... [--secret <path=locator>]...
orion-cli connections probe <id> [--revision <n>]
```

## Description

A revision holds five fields: `revision`, `provider_digest`, `public_config`, `secret_locators` and `bindings`. The pair of connection id and revision is immutable. Creating the same pair with the same content answers the stored revision, and different content is refused with `409 IMMUTABLE_CONNECTION_CONFLICT`.

`public_config` holds settings that are safe to read. A secret-shaped key, an `Authorization` value or credentials in a URL is refused there. `secret_locators` maps a dotted config path to a reference such as `env://ORDERS_DB_URL` or `vault://secret/data/db#url`. The reference is placed at its path when the connection is used, so a locator is never the secret itself.

`bindings` maps a dependency alias the provider declares to another connection in the deployment. A workflow provider package reaches only the connections its own bindings name. A binding change creates a new revision, because the configuration digest covers it.

Every command reads the server's v2 admin API revision before it runs and refuses one this CLI does not speak.

## Subcommands

| Subcommand | Description |
|---|---|
| `list` | Lists every connection's revisions, or one connection's with `--id`. |
| `get <id>` | Shows one revision, the latest when `--revision` is omitted. |
| `create <id> -f <file>` | Creates a new immutable revision from a request document. |
| `probe <id>` | Connects to the revision's backend and reports whether it answers. |

`list` prints one row per revision: connection, revision, provider digest, bindings and configuration digest. Digests are truncated to a readable prefix in the table and printed in full by `get` and under `--output json`.

`get` prints the connection and revision, the provider and configuration digests, the bindings, each secret path with its locator, and the public configuration. `--quiet` prints the configuration digest alone.

## Options

| Flag | Used by | Description |
|---|---|---|
| `--id <id>` | `list` | Only this connection's revisions. Omit it to list every connection. |
| `--revision <n>` | `get`, `probe` | Revision to read or probe. Defaults to the highest revision the connection has. |
| `-f, --file <path>` | `create` | Revision JSON, or `-` for standard input. Required. |
| `--bind <alias=connection>` | `create` | Bind a dependency alias to a connection of the deployment. Repeatable. |
| `--secret <path=locator>` | `create` | Place a credential reference at a config path. Repeatable. |
| `--limit <n>` | `list` | Page size. Default: `50`; the maximum is `1000`. |
| `--cursor <c>` | `list` | Resume after this position: the previous page's `next_cursor`. |

`--bind` and `--secret` merge into the request the file holds. Repeating a flag adds another pair. A pair whose key the file already sets is accepted at the same value. A different value is refused, so a conflict is never silently replaced. Each value needs `=` with a non-empty key and value on either side.

The revision JSON takes the shape the endpoint reads:

```json
{
  "revision": 2,
  "provider_digest": "sha256:…",
  "public_config": { "base_url": "https://orders.example.com" },
  "secret_locators": { "auth.token": "env://ORDERS_TOKEN" },
  "bindings": { "artifact_store": "store" }
}
```

## Exit codes

| Code | Meaning |
|---|---|
| `0` | The command succeeded, or the probe reported the backend healthy. |
| `1` | A network, server or validation error, including a refused revision. |
| `2` | The probe reported the backend unhealthy, or an argument clap rejects. |

## Examples

Create a revision with a binding and a secret locator on the command line:

```bash
orion-cli connections create orders-db -f revision.json \
  --secret auth.token=env://ORDERS_TOKEN \
  --bind dead_letter=dlq
```

Read the latest revision and probe it before a deployment names it:

```bash
orion-cli connections get orders-db
orion-cli connections probe orders-db --revision 2
```

## Caveats

- A probe is audited. It connects with the revision's real settings and locators, so it reaches the live backend.
- An unhealthy probe exits `2` and, outside `--quiet`, prints the driver, latency, probe, error class and error detail. A transport failure before the probe answers exits `1`.
- The latest revision is the highest number, not the newest creation time.
- `public_config` is readable by anyone with the admin credential. Keep every credential in `secret_locators`.
- A provider an edition compiles in requires that edition. The target refuses a driver it does not have, and names the drivers it serves.

## Related

- [Deployment endpoints › Connections](../../admin-api/deployments.md#connections): the revision fields and immutability rules.
- [Deployment endpoints › Bindings](../../admin-api/deployments.md#bindings): what a binding grants a workflow provider.
- [Connector types](../../connectors/index.md): the built-in drivers a revision can name.
- [`orion-cli provider`](./provider.md): admitting the package a provider digest names.
- [`orion-cli` commands](./index.md): every `orion-cli` subcommand.
