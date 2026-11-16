<!-- description: Consume records from Apache Kafka topics into Orion channels, configure delivery guarantees and dead-letter topics, and publish events. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Kafka channels and producer

<span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

Orion Standard and Enterprise integrate with Apache Kafka through two components that share a single connection definition:
1. **`kafka` channels** join a consumer group, subscribe to one or more topics, and run a workflow once per record with at-least-once offset commits and optional dead-letter routing.
2. **The `com.orion.kafka` provider** exposes the `publish` operation (alias `publish_kafka`) so workflows can produce JSON records to any topic using [`connector_call`](../reference/functions.md).

## 1. Define a `com.orion.kafka` connection

Declare a Kafka connector in your definition set using `"provider": "com.orion.kafka"` in place of `"connector_type"`. Keep SASL credentials in `env://` or `vault://` secret locators.

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

### Connection `config` fields

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `brokers` | array of strings | yes | — | Bootstrap broker addresses, each written as `host:port`. |
| `security_protocol` | string | no | — | Wire security mode: `"plaintext"`, `"ssl"`, `"sasl_plaintext"`, or `"sasl_ssl"`. |
| `sasl_mechanism` | string | no | — | SASL mechanism (`"PLAIN"`, `"SCRAM-SHA-256"`, or `"SCRAM-SHA-512"`). Required when `security_protocol` is `"sasl_plaintext"` or `"sasl_ssl"`. |
| `sasl_username` | string | no | — | SASL username. Required when using a SASL protocol. |
| `sasl_password` | string | no | — | SASL password secret locator (`env://` or `vault://`) at `sasl_password`. Required when using a SASL protocol. |
| `ssl_ca_pem` | string | no | system trust store | PEM CA bundle used to verify broker TLS certificates. |
| `allow_private_urls` | boolean | no | `false` | Allow bootstrap brokers to resolve to private or internal IP addresses. |
| `client` | object | no | `{}` | Additional `librdkafka` producer/client properties. Keys managed by Orion (`bootstrap.servers`, `security.protocol`, `sasl.*`, `ssl.ca.*`, `enable.idempotence`, `acks`) are refused. |

---

## 2. Consume records with a `kafka` channel

A `kafka` channel is an `"async"` driver ingress. It references the Kafka connection in both top-level `connections` and `transport_config.connection`, and does not set `methods` or `route_pattern`.

```json
{
  "channel_id": "order-events",
  "name": "order-events",
  "channel_type": "async",
  "protocol": "kafka",
  "workflow_id": "order-processing",
  "connections": ["events"],
  "transport_config": {
    "connection": "events",
    "topics": ["orders.placed", "orders.cancelled"],
    "group_id": "orion-orders",
    "start": "earliest",
    "processing_timeout_ms": 60000,
    "dlq": { "topic": "orders.dlq" },
    "client": { "fetch.wait.max.ms": "500" }
  },
  "config": {
    "timeout_ms": 30000
  }
}
```

### Channel `transport_config` fields

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `connection` | string | yes | — | Name of the `com.orion.kafka` connection to consume from. Must also be listed in the channel's top-level `connections` array. |
| `topics` | array of strings | yes | — | 1 to 100 unique Kafka topic names (`1` to `249` bytes of `[A-Za-z0-9._-]`, excluding `.` and `..`). |
| `group_id` | string | yes | — | Kafka consumer group ID to join (`1` to `255` characters). |
| `start` | string | no | `"earliest"` | Initial offset reset when the consumer group has no committed offset: `"earliest"` or `"latest"`. |
| `processing_timeout_ms` | integer | no | `60000` | Maximum duration in milliseconds for processing one record (`1` to `300000`). If `config.timeout_ms` is shorter, the shorter deadline applies. |
| `dlq.topic` | string | no | — | Dead-letter topic for records that fail or cannot be parsed. Must not be one of the consumed `topics`. |
| `client` | object | no | `{}` | Additional `librdkafka` consumer properties for this group. Keys owned by Orion (`group.id`, `enable.auto.commit`, `auto.offset.reset`, broker/security keys) are refused. |

### What the workflow receives

Each record's UTF-8 payload is parsed as JSON and becomes the workflow's initial `data`. Broker metadata is placed under `metadata.trigger`:

| `metadata.trigger` key | Description |
|---|---|
| `topic`, `partition`, `offset` | Log coordinates of the consumed record. |
| `group_id` | Consumer group ID that delivered the record. |
| `key` | Record key as a string (when present on the record). |
| `timestamp_ms` | Broker timestamp in milliseconds (when present on the record). |
| `headers` | Object mapping UTF-8 record header names to string values. |

### Delivery guarantees, retries, and dead-letter handling

Delivery is **at-least-once** and strictly ordered within each partition (one record in flight per partition):

| Run outcome | Consumer action |
|---|---|
| **Completed or settled duplicate** | Commits the record offset. |
| **Deferred** | Releases the claim and retries the record in place with exponential backoff (`1s` doubling up to `60s`). |
| **Rejected, failed, or non-JSON payload** | Writes the record to `dlq.topic` (reusing the connection's producer) and commits the offset only after the DLQ write is acknowledged. |
| **Channel unavailable** | Releases the claim and pauses the partition until the next deployment reconcile. |

> [!WARNING]
> Always configure `dlq.topic` on production Kafka channels. Without a dead-letter topic, an empty, non-UTF-8, or non-JSON record (or a workflow failure) cannot be committed and blocks its partition, appearing as growing consumer lag (`orion_kafka_consumer_lag_messages`).

### Supported channel guards and metrics

- **Supported guards**: `rate_limit` (keyed by topic unless `key_logic` overrides it), `validation_logic`, `deduplication` (uses the record's `key` as the idempotency key), and `backpressure`. HTTP-only guards (`auth`, `cache`, `request`, `response`, `origin_allow_list`, `oauth2_login`) are refused at upload.
- **Health and metrics**: `GET /health` reports `components.kafka` (`disabled`, `ok`, or `degraded`). Prometheus exposes `orion_kafka_consumer_lag_messages`, `orion_kafka_dlq_total`, `orion_kafka_rebalances_total`, and `orion_kafka_commit_errors_total`.

---

## 3. Publish records from a workflow

Call the `publish` operation (alias `publish_kafka`) on your `com.orion.kafka` connection using `connector_call`. The producer enables `enable.idempotence` and `acks=all` automatically.

```json
{
  "id": "publish_order_event",
  "name": "Publish order event to Kafka",
  "function": {
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
}
```

### `publish` input fields

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `topic` | string | yes | — | Destination topic name (`1` to `249` bytes of `[A-Za-z0-9._-]`). |
| `key` | string | no | — | Record key used by the broker to assign a partition. |
| `value` | any JSON | yes | — | Record payload, serialized as JSON. |
| `headers` | object | no | `{}` | Map of string header names to string values. If the `connector_call` sets `idempotency_key`, Orion also sends it as the `x-idempotency-key` header. |

### `publish` output (`data.published`)

```json
{
  "topic": "orders",
  "partition": 3,
  "offset": 1042
}
```

### Error codes

| Code | Class | Condition |
|---|---|---|
| `KAFKA_PUBLISH_FAILED` | `unavailable` | Broker did not acknowledge the record. Never retried automatically because the record may already have landed. |
| `KAFKA_CONNECTION_INVALID` | `provider_failure` | Connection settings could not build a Kafka client/producer. |
| `KAFKA_CONNECTION_INCOMPLETE` | `provider_failure` | SASL protocol is configured without `sasl_username` or `sasl_password`. |
| `CONNECTOR_INVALID_INPUT` | `invalid_input` | Invalid topic name, unknown input field, or non-string header value. |
| `CONNECTOR_EGRESS_DENIED` | `forbidden` | Bootstrap broker resolves to a private address while `allow_private_urls` is `false`. |
| `CONNECTOR_UNAVAILABLE` | `unavailable` | Producer cannot reach the Kafka cluster. |
| `CONNECTOR_DEADLINE_EXCEEDED` | `deadline` | Workflow deadline elapsed while awaiting broker acknowledgment. |

## Next steps

- [Enterprise connector providers](./providers.md): Reference for the other six Standard+ connector providers.
- [Configure channels](../guides/channels.md): Configure `validation_logic`, `deduplication`, and `rate_limit` guards.
