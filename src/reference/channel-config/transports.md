<!-- description: How a request reaches an Orion channel: the guard-by-ingress matrix, routing fields, the cron schedule, the edition transports and OAuth2 sign-in. -->
<!-- type: hub -->
<!-- last_verified: 2026-11-16 -->

# Routing and transports

Which guard runs on which ingress, and how a request reaches a channel. Also the cron schedule a clockwork channel declares, the edition transports, and the OAuth2 sign-in flow.

| Page | Holds |
|---|---|
| [Guards by ingress](./guards-by-ingress.md) | Which channel guard runs on which of the four ingresses (HTTP sync, /async, channel_call, cron), and the fixed order the guards apply in. |
| [Routing and protocol](./routing.md) | channel_type, protocol, methods, route_pattern, topic, consumer_group and priority. |
| [Cron transport](./cron.md) | the six-field schedule, time zone and DST rules, payload, misfire policies, concurrency, and what it may not declare. |
| [`oauth2_login`](./oauth2_login.md) | a channel as the relying party in a browser OAuth2 authorization-code grant, with PKCE, the state cookie, id_token checks and return_to. |
| [Kafka transport](./kafka.md) | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> a channel that consumes topics through a required Kafka connection, with at-least-once delivery and a dead-letter topic. |
| [GraphQL transport](./graphql.md) | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> one GET and POST route where each root field is a workflow run under the channel's document limits. |
| [SSE transport](./sse.md) | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> a GET route answered as a `text/event-stream`, with a timer, keepalives and resumable event ids. |

## Related

- [Channel configuration](./index.md): every key, with its page.
- [Guards by ingress](./guards-by-ingress.md): which guard runs where, and in what order.
- [Configure a channel](../../guides/author/channels.md): the same keys, as a walkthrough.
