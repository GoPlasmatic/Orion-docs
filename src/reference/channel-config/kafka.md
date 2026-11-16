<!-- description: Reference for a Kafka channel: the connection, topics, consumer group, start offset, processing deadline, dead-letter topic and client settings it declares. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# Kafka transport

Available in Orion Standard and Enterprise only: a Community node refuses `kafka` at upload with `400 VALIDATION_ERROR` on `protocol`. A Kafka channel consumes topics and runs the workflow once per record, using a connection the deployment requires. See [Channels](../../concepts/channels.md) for the model and [Configure a channel](../../guides/author/channels.md) for the surrounding keys.

## Synopsis

A complete channel with the defaults left unset:

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
    "group_id": "orion-orders"
  }
}
```

## Description

A Kafka channel is a driver ingress: records arrive over the broker connection, not over the data plane. The driver serves `channel_type: "async"` only and refuses `methods` and `route_pattern`. The channel's top-level `connections` must name the same connection as `transport_config.connection`, because the deployment member requires it.

Each record's payload is parsed as JSON and becomes the run's `data`. The record's key, topic, partition, offset and headers become its trigger.

## Fields <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `connection` | string | yes | — | The Kafka connection the channel consumes from; it must also appear in the channel's `connections`. |
| `topics` | array of strings | yes | — | One to 100 topic names, each unique and following Kafka's naming rules. |
| `group_id` | string | yes | — | The consumer group to join, 1 to 255 characters. |
| `start` | string | no | `earliest` | Where a new consumer group begins: `earliest` or `latest`. |
| `processing_timeout_ms` | integer | no | `60000` | Ceiling on one record's run, 1 to 300000; the channel's `timeout_ms` may only shorten it. |
| `dlq.topic` | string | no | — | Topic for records that cannot run; it must not be a topic the channel consumes. |
| `client` | object | no | `{}` | librdkafka consumer properties for the group; a key Orion owns is refused by name. |

Topic names are 1 to 249 bytes of `[A-Za-z0-9._-]`, and not `.` or `..`. `earliest` reads a new group from the beginning of its topics. `latest` reads only records that arrive after the group joins.

A `client` map narrows the consumer for this group. Orion sets the brokers, security settings, group identity, commit mode and offset reset itself. A key in that set is refused, and the refusal names its full path, such as `channel.transport_config.client.enable.auto.commit`.

<details><summary>Client keys Orion owns</summary>

`bootstrap.servers`, `metadata.broker.list`, `security.protocol`, `sasl.mechanism`, `sasl.mechanisms`, `sasl.username`, `sasl.password`, `ssl.ca.pem`, `ssl.ca.location`, `group.id`, `group.instance.id`, `enable.auto.commit`, `enable.auto.offset.store`, `enable.partition.eof`, `enable.idempotence`, `auto.offset.reset`, `acks`.

</details>

## Delivery

Delivery is at least once. An offset is committed only after its run completed, the host recognised a settled duplicate, or a dead-letter write was confirmed. A lost commit causes a redelivery, never a lost record.

A group processes records strictly in order: one in flight per partition, and one at a time across the group. A group scales by spreading its partitions across the nodes in it.

| Outcome | Action |
|---|---|
| Completed, duplicate | Commit the offset. |
| Deferred | Release the claim and retry the record in place with backoff. |
| Rejected, failed | Write the record to the dead-letter topic; commit only once the write is confirmed. |
| Unavailable | Release the claim and pause the partition until the next reconcile. |

Retries happen in place, from 1 second doubling to 60 seconds. A record that exhausts 80% of `max.poll.interval.ms` is rewound and redelivered from the poll loop, so the group stays a member.

A record that is empty, not UTF-8 or not JSON never reaches the host and takes the dead-letter path. Without `dlq`, such a record blocks its partition and appears as lag.

### What the workflow receives

`data` is the record payload parsed as JSON. `metadata.trigger` carries the transport facts:

| Key | Meaning |
|---|---|
| `topic`, `partition`, `offset` | Where the record sits in the log. |
| `group_id` | The consumer group that delivered it. |
| `key` | The record key, when the record has one. |
| `timestamp_ms` | The broker timestamp, when the record has one. |
| `headers` | The record's UTF-8 headers, as an object of name and value strings. |

## Guards <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

A `kafka` channel passes four core guards, and the upload refuses every HTTP-only guard on it.

- `rate_limit`: the same token bucket as every ingress, keyed on the topic unless `key_logic` returns something else.
- `validation_logic`: judges the parsed payload before the run starts.
- `deduplication`: the record key is the idempotency key; a record without a key runs without a deduplication check.
- `backpressure`: one per-node concurrency permit per delivery.

The upload refuses `auth`, `principal_rate_limit`, `origin_allow_list`, `cache`, `request`, `response` and `oauth2_login` on a `kafka` channel. Deliveries are authenticated by the connection, not per record. See [Guards by ingress](./guards-by-ingress.md) for the full matrix.

## Health and metrics

`components.kafka` in `GET /health` reports `disabled` when no Kafka channel is deployed. It reports `ok` when every group consumes, and `degraded` with a reason when a channel or group cannot. A `degraded` component also keeps the node out of `/readyz`.

| Metric | Type | Labels | Meaning |
|---|---|---|---|
| `orion_kafka_consumer_lag_messages` | gauge | `channel`, `topic`, `partition` | Distance from the committed offset to the high-water mark, reported every 30 seconds. |
| `orion_kafka_dlq_total` | counter | `channel`, `reason` | Records written to a dead-letter topic. |
| `orion_kafka_rebalances_total` | counter | `group` | Completed partition assignments; a sustained rate means an unstable group. |
| `orion_kafka_commit_errors_total` | counter | `group` | Offset commits that failed; records may be redelivered. |

For the dead-letter counter, an undecodable record reports `empty`, `decode` or `parse`, and a refused run reports `rejected`. A failed run reports `timeout`, `workflow`, `engine` or `failed`.

## Errors

These codes come from the Kafka extension's connection and producer paths. A channel's `transport_config` problems are field errors at upload, not runtime codes.

| Code | Class | When |
|---|---|---|
| `KAFKA_PUBLISH_FAILED` | unavailable | The broker did not acknowledge a `publish`; never retried, because the record may have landed. |
| `KAFKA_CONNECTION_INVALID` | provider_failure | The connection's settings build no producer. |
| `KAFKA_CONNECTION_INCOMPLETE` | provider_failure | A SASL connection lacks its username or password. |
| `CONNECTOR_INVALID_INPUT` | invalid_input | A connection field is invalid, with a JSON pointer in the details. |
| `CONNECTOR_EGRESS_DENIED` | forbidden | A bootstrap broker resolves to a private address and `allow_private_urls` is off. |
| `CONNECTOR_UNAVAILABLE` | unavailable | A producer cannot reach the cluster. |

The dead-letter write reuses the same producer, so a failing producer leaves the record uncommitted and the channel retries it.

## Examples

Preserve failed records and narrow the consumer:

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
  "config": { "timeout_ms": 30000 }
}
```

Read only new records under a shorter deadline:

```json
{
  "channel_id": "order-audit",
  "name": "order-audit",
  "channel_type": "async",
  "protocol": "kafka",
  "workflow_id": "order-audit",
  "connections": ["events"],
  "transport_config": {
    "connection": "events",
    "topics": ["orders.audit"],
    "group_id": "orion-order-audit",
    "start": "latest",
    "processing_timeout_ms": 5000
  },
  "config": { "timeout_ms": 2000 }
}
```

## Caveats

- A record that cannot run and has no `dlq` blocks its partition, and the block shows as lag rather than as an error.
- One group has one consumer: channels sharing a `group_id` on one connection must agree on `start` and `client`.
- A topic may be consumed once per group. A second channel naming it in the same group is refused with `CONFLICT`.
- A channel whose `connections` omits the connection named in `transport_config` is refused with `REQUIRED`.
- A `timeout_ms` cannot lengthen `processing_timeout_ms`; the shorter value is the deadline.
- A `kafka` channel registers no route, so a request to `/api/v2/data/{name}` answers `405`.

The payload must be JSON. A record that is not JSON is dead-lettered, not repaired, and its headers stay at `metadata.trigger.headers`.

## Related

- [Channels](../../concepts/channels.md): the channel model, its ingresses and its guards.
- [Configure a channel](../../guides/author/channels.md): the guards as a walkthrough.
- [Guards by ingress](./guards-by-ingress.md): which guard runs on a driver delivery, and in what order.
- [Metrics](../metrics.md): every metric, with its labels.
- [Edition capability matrix](../editions.md): the gate and its refusal.
