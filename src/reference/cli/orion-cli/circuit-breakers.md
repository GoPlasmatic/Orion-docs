<!-- description: List this node's connector circuit breaker states and reset one key, which fans out to every node in a Standard cluster. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# `orion-cli circuit-breakers`

Reads this node's connector breaker map and resets one key. A reset fans out to every node in a cluster. Cluster mode is available in Orion Standard and Enterprise only <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>.

## Synopsis

```bash
orion-cli circuit-breakers list
orion-cli circuit-breakers reset <key>
```

## Description

A breaker guards one `(channel, connector)` pairing. Its key is `channel:connector`, the spelling `list` prints. The map is node-local, so `list` names its `scope` and the `instance_id` that answered. Another replica may hold a different state for the same key.

`reset <key>` confirms the action, then posts the key. In cluster mode the reset reaches every node, so one call clears the key on every replica. The breaker that needs clearing is often open on a node other than the one that answered, which is why the reset is not node-local. Every reset is audited as `reset` on `circuit_breaker`.

The command reads the server's admin API directly. It is a Community command, and only the cluster fan-out is gated.

## Subcommands

| Subcommand | Description |
|---|---|
| `list` | Prints this node's enabled state, scope, node id and one row per breaker. |
| `reset <key>` | Confirms and resets one breaker, on every node in cluster mode. |

`list` prints a header with the breaker state, scope and node, then a table with two columns: key and state. An empty map prints `No breaker states.` `--quiet` prints the keys alone.

`reset` prints the endpoint's answer: `reset`, the key and whether this node held it. `--quiet` prints the key alone.

## Options

`list` and `reset` take no command-specific flags. The global `--yes` skips the reset confirmation, and `--output json|yaml` prints the server's answer. `reset` takes one positional argument:

| Argument | Description |
|---|---|
| `<key>` | Breaker key as `list` prints it, in the form `channel:connector`. |

## Exit codes

| Code | Meaning |
|---|---|
| `0` | The list or reset succeeded, or the reset was cancelled at the prompt. |
| `1` | A network or server error, including a key this single node does not hold. |
| `2` | A usage error: a missing key or an argument clap rejects. |

## Examples

Read the breakers this node holds:

```bash
orion-cli circuit-breakers list
```

Reset one breaker without a prompt, as a pipeline would:

```bash
orion-cli circuit-breakers reset orders:orders-db --yes
```

## Caveats

- The read is node-local. Load-balanced requests can reach different replicas, and each prints its own map.
- In cluster mode a key this node does not hold is not a `404`. The reset fans out to the node that holds it.
- On a single node, a key the node does not hold answers `404`.
- A reset clears the breaker state. It does not retry the calls that failed, and it does not change the threshold that opened the breaker.
- The confirmation is the only guard. `--yes` is what a non-interactive job needs.

## Related

- [Circuit breaker endpoints](../../admin-api/circuit-breakers.md): the routes and the node-local scope.
- [Retries and circuit breakers](../../connectors/reliability.md): when a breaker opens and the settings behind it.
- [Engine settings](../../configuration/engine.md#circuit-breaker): the breaker thresholds and half-open behaviour.
- [`orion-cli` commands](./index.md): every `orion-cli` subcommand.
