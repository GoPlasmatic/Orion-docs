<!-- description: Build services on the Orion Standard and Enterprise extensions: Kafka, GraphQL and SSE channels, and the seven connector providers. -->
<!-- type: hub -->
<!-- last_verified: 2026-11-16 -->

# Edition integrations

This group covers the capabilities Orion Standard and Enterprise add to Community. Read the target's edition before starting: an agent or a pipeline captures `orion-cli deployments capabilities --output json` and passes it to `lint`, `clippy` and `compile` with `--capabilities`.

<span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

A Community node refuses every feature in this group. It has no driver for a Kafka, GraphQL or SSE channel, no backend for an edition provider, and no shared store for cluster mode. [Editions](../../concepts/editions.md) explains the boundary; [the capability matrix](../../reference/editions.md) lists every gate.

| If you need to… | Start here |
|---|---|
| Consume a topic into a channel | [Consume Kafka messages](../author/kafka-channel.md) |
| Serve a GraphQL endpoint | [Serve a GraphQL API](../author/graphql-api.md) |
| Push events to a browser or client | [Stream events over SSE](../author/sse-stream.md) |
| Send email, share a cache, store objects, query MongoDB or Elasticsearch, or run a model | [Use an edition provider](../author/providers/index.md) |

## Related

- [Editions](../../concepts/editions.md): the three editions and the support boundary.
- [Edition capability matrix](../../reference/editions.md): every gated capability.
- [Deploy Orion Ext](../../operate/deploy/orion-ext.md): the licensed image and topology.
- [Extension settings](../../reference/configuration/extensions.md): the licence and Redis cluster store sections.
