<!-- description: Choose and use an Orion Standard and Enterprise connector provider: Kafka, SMTP, Redis, object storage, MongoDB, Elasticsearch or ONNX models. -->
<!-- type: hub -->
<!-- last_verified: 2026-11-16 -->

# Use an edition provider

Available in Orion Standard and Enterprise only. These <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> providers connect a workflow to Kafka, SMTP, Redis, storage, MongoDB, Elasticsearch or a model. Each is a connection plus operations, reached through [`connector_call`](../../../reference/functions/connector_call.md).

## Choose a provider

- [SMTP provider](../../../reference/connectors/edition/smtp.md) sends transactional email through a relay, with `send` as its one operation.
- [Kafka provider](../../../reference/connectors/edition/kafka.md) produces records with `publish`, on the same connection a `kafka` channel consumes from.
- [Redis provider](../../../reference/connectors/edition/redis.md) is a cache shared by every node, with `get`, `set`, `delete`, `incr`, `get_many` and `delete_many`.
- [Storage provider](../../../reference/connectors/edition/storage.md) reads, writes and presigns objects in one S3-compatible bucket with `presign`, `head`, `get` and `put`.
- [MongoDB provider](../../../reference/connectors/edition/mongodb.md) runs `find`, `aggregate` and `write` against one cluster, with Extended JSON documents.
- [Elasticsearch provider](../../../reference/connectors/edition/elasticsearch.md) serves `search`, `count`, `get`, `index`, `update`, `delete` and `bulk`, bounded by an index allow-list.
- [Models provider](../../../reference/connectors/edition/models.md) admits an ONNX model as a connection and runs `infer` on it.

The four steps are the same for all seven:

| Step | What you write |
|---|---|
| **Connection** | A connector file with `provider` in place of `connector_type`. Every credential is an `env://` or `vault://` locator, and `compile` turns the file into an immutable revision. |
| **Call** | A `connector_call` task naming the connection, the provider id as `target_id`, the operation, its `input`, and an `output` path. |
| **Discovery** | `orion-cli deployments capabilities` lists each backend with its digest and operations; the operation's schemas are the input and answer contract. |
| **Failure** | The data plane answers by class, and `error.connector.provider_code` carries the provider's code. The provider's message reaches the trace, never the caller. |

A provider that needs another connection declares a dependency alias, and the connection binds it to a connection of the deployment. The Models provider is the one shipped example: its artifact is an object of a `com.orion.storage` connection bound as `artifact_store`. The deployment must require the bound connection too, or the build refuses the binding by name.

The seven reference pages hold each provider's connection fields, operation inputs and answers, and error codes. This group does not repeat them.

## Next steps

- [Connector providers](../../../reference/connectors/edition/index.md): the same seven, as a contract reference with a provider table and the naming rules.
- [Connect a database or API](../connectors.md): the connector file, secret locators and operation gates every provider shares.
- [Configure a channel](../channels.md): the route and guards for a channel whose workflow calls a provider.
- [Edition capability matrix](../../../reference/editions.md): the edition that serves each provider, and the refusal a Community node returns.
