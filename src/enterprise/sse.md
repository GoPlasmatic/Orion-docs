<!-- description: Stream real-time Server-Sent Events (SSE) to browsers and HTTP clients from an Orion channel with keep-alive heartbeats and replay IDs. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Server-Sent Events (SSE)

<span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

An `sse` channel serves a persistent `GET` HTTP route with `Content-Type: text/event-stream`. When a browser `EventSource` or streaming HTTP client connects, Orion runs the bound workflow immediately for the first event and re-runs it every `interval_ms` milliseconds, streaming each completed run's output as an SSE frame.

## Configure an `sse` channel

Create a channel with `"channel_type": "sync"`, `"protocol": "sse"`, and `"methods": ["GET"]`:

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
  "config": {
    "timeout_ms": 5000,
    "backpressure": {
      "max_concurrent_per_node": 100
    }
  }
}
```

### Channel and `transport_config` fields

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `channel_type` | string | yes | — | Must be `"sync"`. Requests to `/async` return `405`. |
| `methods` | array of strings | yes | — | Must be `["GET"]`. Any other HTTP method is refused at upload. |
| `route_pattern` | string | yes | — | HTTP route that opens the stream (for example, `"/prices/{symbol}"`). |
| `transport_config.interval_ms` | integer | no | `1000` | Cadence in milliseconds from the start of one workflow run to the start of the next (`50` to `3600000`). Runs for a single stream never overlap. |
| `transport_config.event` | string | no | — | Optional `event:` name written on every emitted frame (`1` to `64` characters of `[A-Za-z0-9_.-]`). |
| `transport_config.keepalive_ms` | integer | no | `15000` | Maximum silence in milliseconds before Orion writes a `: keepalive` comment frame (`1000` to `55000`). Cannot be disabled (`0` and `null` are refused). |
| `transport_config.retry_ms` | integer | no | — | Optional reconnection delay (`retry: <ms>`) sent when the stream opens (`100` to `600000`). |

---

## How workflow runs map to SSE frames

### Input passed to each run

On every tick, Orion populates `data` with the stream's tick counter, query string parameters, and route path parameters, and writes trigger metadata to `metadata.trigger`:

```json
{
  "data": {
    "tick": 1,
    "query": { "currency": "USD" },
    "params": { "symbol": "ACME" }
  },
  "metadata": {
    "trigger": {
      "tick": 1,
      "last_event_id": null
    }
  }
}
```

### Wire format of emitted frames

Orion sets `content-type: text/event-stream`, `cache-control: no-cache`, and `x-accel-buffering: no` on the response. Each completed workflow run emits one frame where `id:` is the run's `tick` number and `data:` is the workflow's output serialized as a single line of compact JSON:

```text
retry: 3000

id: 1
event: price
data: {"symbol":"ACME","price":42.5}

```

While the stream is silent for `keepalive_ms`, Orion writes `: keepalive` both to prevent reverse proxies from closing idle connections and to detect disconnected clients.

---

## Client reconnection and stream termination

### Resuming with `Last-Event-ID`

When a client reconnects and sends the `Last-Event-ID` request header with a numeric value `n`, Orion starts the resumed stream at `tick = n + 1` and passes `"last_event_id": "<n>"` in `metadata.trigger` so event IDs continue increasing monotonically.

Follow and resume an SSE stream from the terminal using `orion-cli streams tail` or `curl -N`:

<!-- tabs:start -->
#### **CLI**
```bash
orion-cli streams tail price-feed --last-event-id 41
```

#### **curl**
```bash
curl -N http://localhost:8080/api/v2/data/prices/ACME \
  -H "Accept: text/event-stream" \
  -H "Last-Event-ID: 41"
```
<!-- tabs:end -->

### Error handling and when a stream closes

- **Opening run failure**: The first workflow run executes before the HTTP response headers are committed. If the opening run is refused by `validation_logic` (`400 VALIDATION_ERROR`), exceeds `timeout_ms` (`504 TIMEOUT`), or fails (`500 ENGINE_ERROR`), Orion returns a standard HTTP error response rather than opening a broken stream.
- **Mid-stream transient failure**: If a later tick times out or fails in the workflow, Orion writes an `error` frame (`event: error` with `data: {"code":"TIMEOUT"}` or `{"code":"WORKFLOW_ERROR"}`) and keeps the stream open for the next tick.
- **When a stream ends**:
  1. The client disconnects (detected on the next event or `: keepalive` write).
  2. A tick is rejected by the channel's `validation_logic` guard (writes a final `error` event and closes the stream).
  3. The channel's own definition is changed by a new deployment (writes `Unavailable` and closes so the client reconnects; a deployment that updates only the bound workflow keeps the stream open and runs the new workflow on the next tick).
  4. The node enters graceful shutdown drain.

### Supported guards and metrics

- **Guards**: `auth`, `rate_limit`, and `origin_allow_list` run once when the stream opens. `backpressure` holds one concurrency permit for the entire lifetime of the open stream, so `backpressure.max_concurrent_per_node` bounds the number of simultaneous open SSE connections per node. `validation_logic` and `timeout_ms` run on every tick. (`deduplication`, `cache`, `request`, `response`, and `oauth2_login` are refused on `sse` channels.)
- **Metrics**: Prometheus tracks `orion_sse_frames_total{channel, kind}` where `kind` is `event`, `error`, or `keepalive`.

## Next steps

- [Configure channels](../guides/channels.md): Configure `backpressure`, `auth`, and `validation_logic` guards on your SSE channel.
- [Timeouts, retries, and circuit breakers](../operate/resilience.md): Bound concurrent streams and per-tick execution deadlines.
