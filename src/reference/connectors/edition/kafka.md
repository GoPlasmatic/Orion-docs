<!-- description: The com.orion.kafka provider: broker and SASL connection fields, the publish operation, its answer and producer error codes. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# Kafka provider

Available in Orion Standard and Enterprise only. The connection fields and the `publish` operation of `com.orion.kafka`, which produces records to a Kafka cluster.

## Synopsis

A definition set declares the connection with `provider` in place of `connector_type`:

```json
{
  "name": "events",
  "provider": "com.orion.kafka",
  "config": {
    "brokers": ["kafka-1.example.com:9092", "kafka-2.example.com:9092"],
    "security_protocol": "sasl_ssl",
    "sasl_mechanism": "SCRAM-SHA-512",
    "sasl_username": "orion",
    "sasl_password": "env://KAFKA_PASSWORD"
  }
}
```

## Description <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

`publish` produces one record and waits for the broker's acknowledgement. `publish_kafka` is a compatibility alias. The record value is any JSON, published as its serialisation.

One connection serves both the provider and a `kafka` channel. The channel driver's dead-letter writes reuse the producer that this provider's calls use, so the two share one connection revision and one producer pool.

The producer enables idempotence and `acks=all`, so a retried produce neither duplicates nor reorders a record. A failure reported by the provider is still never retryable, because the record may have landed before the broker's answer was lost.

## Fields

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `brokers` | array of strings | yes | — | The bootstrap brokers, each written as a `host:port` pair. |
| `security_protocol` | string | no | — | The wire security: `plaintext`, `ssl`, `sasl_plaintext` or `sasl_ssl`, unset for the client default. |
| `sasl_mechanism` | string | no | — | The SASL mechanism: `PLAIN`, `SCRAM-SHA-256` or `SCRAM-SHA-512`, required with a SASL protocol. |
| `sasl_username` | string | no | — | The SASL login name, which a SASL protocol requires. |
| `sasl_password` | string | no | — | The SASL password, always a secret locator at `sasl_password`. |
| `ssl_ca_pem` | string | no | — | The PEM CA bundle the brokers chain to; unset uses the system trust store. |
| `allow_private_urls` | boolean | no | `false` | Whether a bootstrap broker may be a private or internal address. |
| `client` | object | no | `{}` | Further librdkafka properties; the keys Orion sets are refused. |

A SASL protocol requires `sasl_mechanism`, `sasl_username` and `sasl_password`. Setting `sasl_mechanism` or `sasl_username` without a SASL protocol is refused.

## Operations

| Operation | Aliases | Answer |
|-----------|---------|--------|
| `publish` | `publish_kafka` | `{topic, partition, offset}` |

### `publish`

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `topic` | string | yes | — | The topic to produce to, at most 249 bytes. |
| `key` | string | no | — | The record key, which the broker uses to choose a partition. |
| `value` | any JSON | yes | — | The record payload, published as its JSON serialisation in the record. |
| `headers` | object | no | `{}` | Record headers, each a string value that travels beside the payload. |

A topic name holds letters, digits, `.`, `_` and `-`, and is neither `.` nor `..`. An `idempotency_key` on the call travels as the `x-idempotency-key` record header.

## Returns

| Field | Type | Description |
|-------|------|-------------|
| `topic` | string | The topic the record landed on. |
| `partition` | integer | The partition the broker assigned. |
| `offset` | integer | The record's offset in that partition. |

## Errors

| Code | Class | Condition |
|------|-------|-----------|
| `KAFKA_PUBLISH_FAILED` | unavailable | The broker did not acknowledge the record; it may have landed, so it is never retryable. |
| `KAFKA_CONNECTION_INVALID` | provider_failure | The connection settings build no producer. |
| `KAFKA_CONNECTION_INCOMPLETE` | provider_failure | SASL lacks its username or its password. |
| `CONNECTOR_INVALID_INPUT` | invalid_input | The input names an unknown field, an invalid topic or a non-string key or header. |
| `CONNECTOR_EGRESS_DENIED` | forbidden | A bootstrap broker is private and the connection does not allow it. |
| `CONNECTOR_UNAVAILABLE` | unavailable | The producer cannot reach the cluster. |
| `CONNECTOR_DEADLINE_EXCEEDED` | deadline | The run's deadline passed while the record awaited its acknowledgement. |

## Examples

The task publishes an order event through the `events` connection:

```json
{
  "name": "connector_call",
  "input": {
    "connection": "events",
    "target_id": "com.orion.kafka",
    "operation": "publish",
    "input": {
      "topic": "orders",
      "key": { "var": "data.order_id" },
      "value": { "var": "data.order" },
      "headers": { "source": "orion" }
    },
    "output": "data.published"
  }
}
```

The answer names where the record landed:

```json
{
  "topic": "orders",
  "partition": 3,
  "offset": 1042
}
```

## Caveats

- Only the bootstrap brokers are egress-checked. The brokers the cluster advertises afterwards are reached as any Kafka client reaches them.
- A publish that fails after the record reaches the producer may still have landed, so the provider marks every such failure never retryable.
- A string value is published with its JSON quotes. Send an object, or a pre-serialised string, according to what the consumers expect.
- The `client` map may not set the keys Orion owns: brokers, security, SASL, group and commit settings, `enable.idempotence`, `auto.offset.reset` and `acks`.
- The same connection is what a `kafka` channel names, and its dead-letter writes reuse this producer.

## Related

- [Edition providers](./index.md): the seven providers and the rules they share.
- [`connector_call`](../../functions/connector_call.md): the task that invokes `publish`.
- [Kafka transport](../../channel-config/kafka.md): the channel driver that consumes through the same connection.
- [Edition capability matrix](../../editions.md): the seven providers with their minimum edition.
