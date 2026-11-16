<!-- description: Follow a channel's server-sent event stream, resume from an event id, add request headers, and flush each chunk to standard output. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# `orion-cli streams`

Follows a channel's server-sent event stream. Available in Orion Standard and Enterprise only: the target must serve an `sse` channel, and Community does not register that driver.

## Synopsis

```bash
orion-cli streams tail <channel> [--header <name: value>]... [--last-event-id <id>]
```

## Tail a channel <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

`tail` sends a streaming `GET /api/v2/data/<channel>` with `Accept: text/event-stream`. It writes each chunk to standard output as it arrives, then flushes it. A pipe or a terminal sees an event when the server sends it.

There is no buffering and no reformatting: the bytes are the SSE frames, including their `event:`, `data:` and `id:` lines. The command runs until the server closes the response or you interrupt it. Ctrl-C closes the connection, and the process ends at the interrupt.

## Options

| Flag | Description |
|---|---|
| `<channel>` | Channel name to follow. The channel must be served by the `sse` driver. |
| `--header <name: value>` | Extra request header, split at the first colon. Repeatable. |
| `--last-event-id <id>` | Resume after this SSE event id, sent as the `Last-Event-ID` header. |

An extra header is added to the same request. The command sets `Accept: text/event-stream`, and a `--header` value without a colon is refused before the request, with a message naming the value.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | The server closed the stream. |
| `1` | A transport, authentication or server error before or during the stream, or a malformed `--header` value. |
| `2` | A usage error: a missing channel or an argument clap rejects. |

## Examples

Follow the orders channel:

```bash
orion-cli streams tail orders
```

Resume after the last event a previous process printed, and tag the request for tracing:

```bash
orion-cli streams tail orders --last-event-id 41 --header 'x-trace: 1'
```

Write the raw stream to a file for a later replay:

```bash
orion-cli streams tail orders > orders.events
```

## Caveats

- The output is raw SSE, not JSON. Do not pass it to `--output json` and expect a document.
- Each chunk is flushed as it arrives. A downstream reader sees partial frames, exactly as the server wrote them.
- The stream carries whatever the channel's driver emits. A channel on another protocol answers with an error before any event arrives.
- The `Last-Event-ID` header asks the driver to resume. The driver decides how much history it can replay.
- An interrupted stream is not an error the command reports. Reconnect with `--last-event-id` to continue.

## Related

- [Routing and protocol](../../channel-config/routing.md): the `sse` protocol and the fields a channel declares.
- [Data API](../../data-api.md): the request path and the envelopes beside the stream.
- [Editions](../../editions.md): the capability matrix that gates the `sse` driver.
- [`orion-cli send`](./send.md): sending a request to the same channel.
- [`orion-cli` commands](./index.md): every `orion-cli` subcommand.
