<!-- description: Reference for an SSE channel: the GET event stream, its interval and keepalive settings, the frames written, resumption and the ways a stream ends. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# SSE transport

Available in Orion Standard and Enterprise only: a Community node refuses `sse` at upload with `400 VALIDATION_ERROR` on `protocol`. A channel of the server-sent events (SSE) protocol answers `GET` with a `text/event-stream` and writes one event per completed run. See [Channels](../../concepts/channels.md) for the model and [Configure a channel](../../guides/author/channels.md) for the surrounding keys.

## Synopsis

A complete channel with the default interval and keepalive:

```json
{
  "channel_id": "price-feed",
  "name": "price-feed",
  "channel_type": "sync",
  "protocol": "sse",
  "methods": ["GET"],
  "route_pattern": "/prices/{symbol}",
  "workflow_id": "price-snapshot",
  "transport_config": {
    "interval_ms": 1000,
    "event": "price",
    "keepalive_ms": 15000,
    "retry_ms": 3000
  },
  "config": { "backpressure": { "max_concurrent_per_node": 100 } }
}
```

## Description

An SSE channel is one `GET` route whose answer stays open. The request opens the stream, and the first run happens on that request. Every `interval_ms` after that the workflow runs again, and each completed run becomes one event.

The channel is a hook: its driver writes the answer itself, so the stream carries the driver's status, headers and frames. Runs of one stream never overlap, and a run that takes longer than the interval delays the next.

## Fields <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `channel_type` | string | yes | — | `sync`; the driver answers the request itself, and `/async` answers `405`. |
| `methods` | array of strings | yes | — | `["GET"]`; any other method, or an extra method, is refused at upload. |
| `route_pattern` | string | yes | — | The route that opens the stream, for example `/prices/{symbol}`. |
| `transport_config.interval_ms` | integer | no | `1000` | Time from one run's start to the next, 50 to 3600000. |
| `transport_config.event` | string | no | — | The `event:` name on every frame, at most 64 bytes of letters, digits, `_`, `-` or `.`. |
| `transport_config.keepalive_ms` | integer | no | `15000` | Longest silence before a keepalive comment, 1000 to 55000; it cannot be turned off. |
| `transport_config.retry_ms` | integer | no | — | Reconnection delay sent to the client when the stream opens, 100 to 600000. |

The interval measures from one run's start, not from its end, so a slow workflow stretches the cadence. An absent `transport_config` is the same as every default.

## Stream frames

The response carries `content-type: text/event-stream`, `cache-control: no-cache` and `x-accel-buffering: no`. A completed run is one frame:

```text
id: 7
event: price
data: {"symbol":"ACME","price":42.5}
```

The `id` is the run's tick. The `event` line appears only when the channel sets `event`. The `data` line is one line of compact JSON, which escapes any line break in the output.

A stream silent for `keepalive_ms` writes a comment line, `: keepalive`. The comment is ignored by every client, and the write is what tells the server that the client has gone.

When `retry_ms` is set, the first bytes are a `retry:` line. A browser's `EventSource` reads it as the reconnection delay.

## Workflow runs

Each run's `data` holds the subscription and the stream's position:

| Key | Meaning |
|---|---|
| `tick` | The run's number, which is also the event id. |
| `query` | The request's query string, as an object of strings. |
| `params` | The route parameters captured by the pattern, as an object of strings. |

The run's `metadata.trigger` carries `tick` and, on a resumed stream, `last_event_id`.

Every run passes the channel's validation guard against this `data`, so one channel can serve many subscriptions and judge each one. Each run is traced and counted as one message with mode `sse`.

## Resuming and ending

A client resumes with `Last-Event-ID`. A numeric value `n` starts the stream at tick `n + 1`, so ids keep increasing across a reconnect. An absent or non-numeric value starts at tick `1`.

A stream ends in one of four ways:

- The client goes away. The server notices on the next write, which is why the keepalive cannot be off.
- The channel is redeployed in a way that changes it. The next run answers `Unavailable`, the stream ends, and the client reconnects to the channel as it now is. A redeploy that changes only the workflow keeps the stream, and later runs execute the new workflow.
- A run is refused by `validation_logic`. The driver writes an `error` event and then ends the stream.
- The node drains. The stream ends once the drain gate resolves.

A failed run writes an `error` event and the stream continues, because the next tick may succeed:

```text
event: error
data: {"code":"TIMEOUT"}
```

The code is `TIMEOUT`, `WORKFLOW_ERROR` or `ENGINE_ERROR`, and the event never carries the engine's own message. A deferred run or a duplicate writes nothing for that tick, and the keepalive covers the silence.

## Guards <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

- `validation_logic`: judges each run's `data`, that is the tick, the query and the route parameters.
- `backpressure`: one permit per open stream, held until the stream ends or its client disconnects. `backpressure.max_concurrent_per_node` therefore caps a node's open streams.
- `rate_limit`, `auth` and `origin_allow_list`: run once per request, when the stream opens.
- `timeout_ms`: bounds each run.

A stream does not deduplicate, because a reconnect replays the same request by design, and the upload refuses `deduplication` on an `sse` channel. The upload also refuses `cache`, `request`, `response` and `oauth2_login`, because the driver owns the request and the answer. See [Guards by ingress](./guards-by-ingress.md) for the full matrix.

## Metrics

| Metric | Type | Labels | Meaning |
|---|---|---|---|
| `orion_sse_frames_total` | counter | `channel`, `kind` | Frames written; `kind` is `event`, `error` or `keepalive`. |

## Errors

Before any stream opens, the first run is part of the request and its refusal is an HTTP status in the data plane envelope.

| Code | Status | When |
|---|---|---|
| `VALIDATION_ERROR` | 400 | The opening run was refused by `validation_logic`. |
| `SERVICE_UNAVAILABLE` | 503 | The opening run was deferred, or the channel is unavailable. |
| `CONFLICT` | 409 | The opening run was recognised as a duplicate. |
| `TIMEOUT` | 504 | The opening run exceeded its deadline. |
| `ENGINE_ERROR` | 500 | The opening run failed for any other reason. |
| `METHOD_NOT_ALLOWED` | 405 | The request method is not `GET`. |

During the stream, an `error` event carries the code and the stream continues, except for a validation refusal.

| Code | When |
|---|---|
| `TIMEOUT` | The run exceeded its deadline. |
| `WORKFLOW_ERROR` | The run's workflow failed. |
| `ENGINE_ERROR` | The run failed for any other reason. |
| the refusal's code | `validation_logic` refused the run; the event ends the stream. |

## Example

Follow the stream from the start, then resume after event 41:

```bash
orion-cli streams tail price-feed --last-event-id 41
```

The command writes each frame as it arrives:

```text
id: 42
event: price
data: {"symbol":"ACME","price":42.5}
```

## Caveats

- `keepalive_ms` cannot be off: `0` and `null` are both refused at upload.
- `Last-Event-ID` sets the next tick. It does not replay the events between the disconnect and the reconnect.
- An open stream holds a backpressure permit for its whole life, however many runs it makes.
- A validation refusal ends the stream rather than skipping one event, because the same input would be refused again.
- The first run happens before the response opens, so a bad subscription gets a status rather than a stream that ends at once.
- A redeploy that changes the channel's own definition ends the stream with `Unavailable`; a workflow-only change does not.

## Related

- [Channels](../../concepts/channels.md): the channel model, its ingresses and its guards.
- [Configure a channel](../../guides/author/channels.md): the guards as a walkthrough.
- [Guards by ingress](./guards-by-ingress.md): the hook rows and the per-stream permit.
- [`orion-cli streams`](../cli/orion-cli/streams.md): following and resuming a stream from the CLI.
- [Edition capability matrix](../editions.md): the gate and its refusal.
