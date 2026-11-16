<!-- description: The seven connector providers Orion Ext adds, the Standard+ tier model, how a set names a provider, and the immutable revision model. -->
<!-- type: hub -->
<!-- last_verified: 2026-11-16 -->

# Edition providers

The seven connector providers Orion Ext adds to a licensed node, and the connection rules they share. Available in Orion Standard and Enterprise only.

A Community node registers the three built-in connector types and no provider. The editions form a ladder: Standard and Enterprise register these seven beside the built-ins, from the same composition and with the same behaviour. The [edition capability matrix](../../editions.md) is the single list of what each edition serves.

A definition set names a provider by driver id, with `provider` in place of `connector_type`:

```json
{
  "name": "mailer",
  "provider": "com.orion.smtp",
  "config": {
    "host": "smtp.example.com",
    "from": "Orders <orders@example.com>"
  }
}
```

`orion-cli compile` derives the connection revision's `provider_digest` from that id, so nothing is uploaded first. A connector that names both `provider` and `connector_type`, or neither, is refused. A workflow reaches the provider through [`connector_call`](../../functions/connector_call.md) by naming the connection and the same id as `target_id`. The connection id is not a provider id.

A connection revision is immutable. A changed provider configuration is the next revision, never an edit of the current one. A call uses exactly the revision its deployment manifest member names. Secret references stay references in the set and resolve on the node when the connection is used. [Definition and identity](../identity.md) specifies the revision model the built-in types share.

Ask the target node which providers and operations it serves:

```bash
orion-cli deployments capabilities --output json > target-capabilities.json
```

Each provider appears under `provider_backends` with its `driver_id`, its `package_digest` and its `operations`. An operation carries its aliases and the schemas its input and answer are held to:

```json
{
  "provider_backends": [
    {
      "kind": "rust",
      "driver_id": "com.orion.smtp",
      "package_digest": "sha256:…",
      "operations": {
        "send": { "aliases": ["send_email"] }
      }
    }
  ]
}
```

The excerpt omits the edition header and the other backends. `orion-cli lint` and `orion-cli compile` take the same file with `--capabilities`, so a set is checked against the node that will serve it.

## Providers

| Provider | Serves | Operations |
|---|---|---|
| [SMTP provider](./smtp.md) | Transactional email through an SMTP relay, with STARTTLS or implicit TLS. | `send` |
| [Kafka provider](./kafka.md) | Record production to a Kafka cluster, on the producer the Kafka channel driver also uses. | `publish` |
| [Redis provider](./redis.md) | A Redis cache shared by every node, with per-connection key prefixes and counters. | `get`, `set`, `delete`, `incr`, `get_many`, `delete_many` |
| [Storage provider](./storage.md) | One bucket of an S3-compatible object store, with presigned URLs. | `presign`, `head`, `get`, `put` |
| [MongoDB provider](./mongodb.md) | Document queries and writes on one MongoDB cluster, under connection caps. | `find`, `aggregate`, `write` |
| [Elasticsearch provider](./elasticsearch.md) | Elasticsearch search, document and bulk APIs, bounded by an index allow-list. | `search`, `count`, `get`, `index`, `update`, `delete`, `bulk` |
| [Models provider](./models.md) | ONNX inference, admitted per connection revision and run on tract. | `infer` |

## Related

- [Edition capability matrix](../../editions.md): every capability with the edition that serves it.
- [`connector_call`](../../functions/connector_call.md): the task that invokes a provider operation.
- [Shared connector blocks](../common.md): secrets, gates and the revision model every connector shares.
- [Editions](../../../concepts/editions.md): why the two licensed editions serve one feature set.
