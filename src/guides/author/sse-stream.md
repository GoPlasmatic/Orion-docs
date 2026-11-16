<!-- description: Stream workflow output over server-sent events: the sse channel, tick cadence, event names, keepalive, Last-Event-ID resumption and stream limits. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Stream events over SSE

Available in Orion Standard and Enterprise only. An <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> `sse` channel answers a GET route as an event stream. This guide ticks a workflow, names its events, and tests resumption.

## Before you start

You need:

- An `orion-ext` node at `http://localhost:8080`. A Community node refuses `sse` at upload with `400 VALIDATION_ERROR` on `protocol`.
- A definition set, and admin access to the node.
- `orion-cli` on your `PATH`, and `curl` for the raw stream.

The channel is `order-stream`, its route is `/order-stream`, and its workflow is `order-stream`. The example uses the `orders-db` connector from [Connect a database or API](./connectors.md) for its second version.

## 1. Write the channel

Save this as `order-stream.json` beside the workflow:

```json
{
  "channel_id": "order-stream",
  "name": "order-stream",
  "channel_type": "sync",
  "protocol": "sse",
  "methods": ["GET"],
  "route_pattern": "/order-stream",
  "workflow_id": "order-stream",
  "transport_config": {
    "interval_ms": 1000,
    "event": "order",
    "keepalive_ms": 15000,
    "retry_ms": 3000
  },
  "config": {
    "backpressure": { "max_concurrent_per_node": 100 }
  }
}
```

An `sse` channel takes `["GET"]` and nothing else. The route pattern opens the stream. `/async` answers `405`, because the driver owns the request and the answer.

The `transport_config` fields are:

| Field | Default | Range | Meaning |
|---|---|---|---|
| `interval_ms` | 1000 | 50 to 3600000 | Time from one run's start to the next. |
| `event` | unset | 1 to 64 bytes | The `event:` name on every frame. Unnamed when absent. |
| `keepalive_ms` | 15000 | 1000 to 55000 | Longest silence before a keepalive comment. Cannot be off. |
| `retry_ms` | unset | 100 to 600000 | The reconnection delay sent to the client when the stream opens. |

The interval measures from one run's start, not from its end, so a slow workflow stretches the cadence. Runs of one stream never overlap.

The run's `data` carries the subscription's query and route parameters, so one channel serves many subscriptions. Add `validation_logic` when the stream needs a required parameter; it judges each run's `data` before the run starts.

## 2. Write the workflow

Every run's `data` holds the subscription and the stream's position:

| Key | Meaning |
|---|---|
| `tick` | The run's number, which is also the event id. |
| `query` | The request's query string, as an object of strings. |
| `params` | The route parameters captured by the pattern, as an object of strings. |

The run's `metadata.trigger` carries `tick` and, on a resumed stream, `last_event_id`. Save this as `order-stream-workflow.json`:

```json
{
  "workflow_id": "order-stream",
  "name": "Order stream",
  "condition": true,
  "tasks": [
    {
      "id": "frame",
      "name": "Frame the tick",
      "function": {
        "name": "map",
        "input": {
          "mappings": [
            { "path": "data.orders", "logic": [] },
            { "path": "data.cursor", "logic": { "var": "data.tick" } }
          ]
        }
      }
    }
  ]
}
```

The event carries the run's whole `data`, not a `result` field. That is why `tick`, `query` and `params` appear in every frame alongside what the workflow writes. Every key is visible to the client, so keep secrets out of `data`.

## 3. Read the frames

A completed run is one frame:

```text
id: 1
event: order
data: {"tick":1,"query":{},"params":{},"orders":[],"cursor":1}
```

The `id` is the run's tick. The `event:` line appears only when `event` is set. The `data:` line is one line of compact JSON, which escapes any line break in the output.

When the stream has been silent for `keepalive_ms`, the server writes a comment:

```text
: keepalive
```

Every client ignores the comment, and the write is what tells the server the client is still there. An idle stream without it would hold its backpressure permit forever, so `keepalive_ms` cannot be turned off. Keep it below the idle timeout of any proxy between the client and the node.

When `retry_ms` is set, the first bytes are `retry: 3000`, which a browser's `EventSource` reads as its reconnection delay.

## 4. Resume with Last-Event-ID

A client that reconnects sends `Last-Event-ID`. A numeric value `n` starts the stream at tick `n + 1`, so ids keep increasing across a reconnect. An absent or non-numeric value starts at tick `1`.

The event ids are positions, not a replay log. A stream resumed after 41 starts at 42 and does not replay the events between the disconnect and the reconnect. A client that needs every event stores the ones it received, or reads them from the source by their own ids.

## 5. Know what ends a stream

A stream ends in one of four ways:

- **The client goes away.** The server notices on the next write, which is why the keepalive exists.
- **The channel is redeployed in a way that changes it.** The next run answers `Unavailable` and the stream ends. The client reconnects to the channel as it now is. A redeploy that changes only the workflow keeps the stream, and later runs execute the new workflow.
- **A run is refused by `validation_logic`.** The driver writes an `error` event and ends the stream, because the same input would be refused again.
- **The node drains.** The stream ends once the drain gate resolves, so a rolling deploy does not wait for a force timeout.

A failed run writes an `error` event and the stream continues, because the next tick may succeed:

```text
event: error
data: {"code":"TIMEOUT"}
```

The code is `TIMEOUT`, `WORKFLOW_ERROR` or `ENGINE_ERROR`, and the event never carries the engine's own message. A deferred run or a duplicate writes nothing for that tick, and the keepalive covers the silence.

## 6. Deploy

Compile against the target's capabilities, then apply:

```bash
orion-cli deployments capabilities --output json > target-capabilities.json
orion-cli compile ./definitions -o dist/deployment --capabilities target-capabilities.json
orion-cli deployments apply -d dist/deployment
```

Without `--capabilities`, `compile` does not know the target serves `sse` and refuses the channel as an unknown protocol.

> [!WARNING]
> An open stream holds one backpressure permit for its whole life. `max_concurrent_per_node` is per node, so two replicas admit up to twice the configured number of open streams. Size it against the connections your source can carry, not against your request rate.

## Verify

Follow the stream from the start:

```bash
orion-cli streams tail order-stream
```

The command writes each frame as it arrives:

```text
retry: 3000

id: 1
event: order
data: {"tick":1,"query":{},"params":{},"orders":[],"cursor":1}

id: 2
event: order
data: {"tick":2,"query":{},"params":{},"orders":[],"cursor":2}
```

Read the raw stream with `curl`, which needs `-N` so it does not buffer:

```bash
curl -N 'http://localhost:8080/api/v2/data/order-stream'
```

Resume after event 41, which starts at tick 42:

```bash
orion-cli streams tail order-stream --last-event-id 41
```

```text
retry: 3000

id: 42
event: order
data: {"tick":42,"query":{},"params":{},"orders":[],"cursor":42}
```

The query string reaches the workflow at `data.query`, so the same channel serves a filtered subscription:

```bash
curl -N 'http://localhost:8080/api/v2/data/order-stream?status=open'
```

```text
retry: 3000

id: 1
event: order
data: {"tick":1,"query":{"status":"open"},"params":{},"orders":[],"cursor":1}
```

Count the frames the node wrote:

```bash
curl -s http://localhost:8080/metrics | grep orion_sse_frames_total
```

`orion_sse_frames_total{channel,kind}` counts `event`, `error` and `keepalive` frames separately, so a stream that only sends keepalives is visible. Each run is a trace with mode `sse`; `orion-cli traces list --channel order-stream --mode sse` finds them.

### Feed it from a table

The first workflow ticks without reading anything. Replace its task with a query against `orders-db` to stream real rows:

```json
{
  "workflow_id": "order-stream",
  "name": "Order stream",
  "condition": true,
  "tasks": [
    {
      "id": "orders",
      "name": "Read the open orders",
      "function": {
        "name": "data_query",
        "input": {
          "connector": "orders-db",
          "query": {
            "source": "orders",
            "filter": { "==": [{ "field": "status" }, { "param": "status" }] },
            "sort": [{ "id": "desc" }],
            "limit": 20
          },
          "params": { "status": { "var": "data.query.status" } },
          "schema": {
            "entities": {
              "orders": {
                "columns": {
                  "id": { "type": "int" },
                  "status": { "type": "text" },
                  "total": { "type": "float" }
                }
              }
            }
          },
          "output": "data.orders"
        }
      }
    }
  ]
}
```

Each tick re-reads the newest twenty matching rows, so this variant needs `?status=open` on the request. It is a polling feed, not a change log. The client keeps the ids it has seen, or the workflow stores a watermark in Redis and filters on it. See [`com.orion.redis`](../../reference/connectors/edition/redis.md) for a shared cursor.

## Next steps

- [SSE transport](../../reference/channel-config/sse.md): every field, the frame format and the complete end-of-stream table.
- [`orion-cli streams`](../../reference/cli/orion-cli/streams.md): `tail`, headers and resumption from the CLI.
- [Configure a channel](./channels.md): auth, rate limits and the guards an SSE route shares with REST.
- [Monitor and alert](../../operate/run/monitoring.md): the counters to watch when a stream is the product.
