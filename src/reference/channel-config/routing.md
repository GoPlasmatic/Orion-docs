<!-- description: The channel fields beside config that decide how requests arrive: channel_type, protocol, methods, route_pattern, topic, consumer_group and priority. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# Routing and protocol

These fields sit on the channel object itself, beside `config`. They decide how requests reach the channel; [Route Resolution](../data-api.md#route-resolution) in the Data API reference specifies how a request path resolves to a channel.

## Synopsis

```json
{{#include ../../../examples/packages/webhook-transform/channel.json}}
```

## Fields

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `channel_type` | string | yes | — | `sync` (the caller waits for the result) or `async` (queued; answers `202` with a trace id). Case-insensitive. A `cron` or `kafka` channel must be `async`. |
| `protocol` | string | yes | — | The channel driver: `rest`, `http` or `cron` on Community; <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> adds [`kafka`](./kafka.md), [`graphql`](./graphql.md) and [`sse`](./sse.md). Case-insensitive. A protocol no driver on the node serves is refused with `INVALID` on `protocol`, its `expected` listing the ones that are. |
| `methods` | array of strings | `rest`, `http`, `graphql`, `sse` | — | HTTP methods the route answers. Valid values: `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `HEAD`, `OPTIONS`. An unknown or duplicated method is refused. |
| `route_pattern` | string | `rest`, `http`, `graphql`, `sse` | — | Path pattern, for example `/orders/{id}`. Grammar below. At most 255 characters. |
| `topic` | string | no | — | Stored with the channel; core reads nothing from it. A `kafka` channel names its topics in `transport_config.topics`. At most 255 characters. Refused on a `cron` channel. |
| `consumer_group` | string | no | — | Stored with the channel; core reads nothing from it. A `kafka` channel names its group in `transport_config.group_id`. At most 255 characters. Refused on a `cron` channel. |
| `priority` | number | no | `0` | Route-match precedence. Routes match by priority descending, then segment count descending, then channel name — deterministic on every node. |

`rest` and `http` route identically: both declare `methods` and `route_pattern` and register in the route table. Both also stay reachable by name at `/api/v2/data/{name}`. An async channel's pattern serves at `/{pattern}/async`, whatever its `channel_type`.

A `graphql` channel takes `GET` and/or `POST`, each once. An `sse` channel takes `GET` only. Both declare one `route_pattern`, and their drivers write the answer.

A `cron` channel declares none of those four fields, and each is refused. It registers no HTTP route, and it is **not** reachable by name at `/api/v2/data/{name}` either. Its schedule is the only thing that starts it. See [Cron transport](./cron.md).

A `kafka` channel is `async` and declares no `methods` and no `route_pattern`. Its records arrive over the connection, not the data plane. Its topics and group live in `transport_config`, where the Kafka driver reads them; see [Kafka transport](./kafka.md).

**`route_pattern` grammar.** The pattern must start with `/`. It must not contain whitespace, `?`, `#`, or `%`. No segment may be empty (no `//`, no trailing `/`). A parameter is a whole segment written `{name}`; the name must match `[A-Za-z_][A-Za-z0-9_]*` and be unique within the pattern. Captured parameters reach the workflow as `metadata.params`. See the [Workflow Schema](../workflows.md).

A channel names its workflow with a top-level `workflow_id`; how conditions select a workflow is specified in the [Workflow Schema](../workflows.md). A deployment requires that workflow to be a member beside the channel.

Guard keys go in a `config` object beside these fields:

```json
{
  "name": "orders",
  "channel_type": "sync",
  "protocol": "rest",
  "methods": ["POST"],
  "route_pattern": "/orders",
  "workflow_id": "order-processing",
  "config": { "timeout_ms": 5000 }
}
```

## Related

- [Channels](../../concepts/channels.md): what a channel is.
- [Configure a channel](../../guides/author/channels.md): declaring a route in practice.
- [Data API › Route resolution](../data-api.md#route-resolution): how a request path resolves to a channel.
- [Channel configuration](./index.md): every key, with its page.
