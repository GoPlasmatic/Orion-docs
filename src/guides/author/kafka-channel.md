<!-- description: Consume two order topics into one workflow with an Orion kafka channel: the connection, consumer group, dead-letter topic, offsets and lag. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Consume Kafka messages

Available in Orion Standard and Enterprise only. A <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> `kafka` channel consumes order records into workflow runs. This guide builds one channel for two topics, with a dead-letter topic and lag checks.

## Before you start

You need:

- An `orion-ext` node, reachable at `http://localhost:8080`. A Community node refuses `kafka` at upload with `400 VALIDATION_ERROR` on `protocol`.
- A Kafka cluster, and credentials if it authenticates.
- A definition set, with admin access to the node.
- `orion-cli` on your `PATH`.

The workflow is `order-processing`, and the connection is `orders-events`. The channel is `orders-kafka`. [Configure a channel](./channels.md) covers the keys every channel shares.

## 1. Declare the connection

A definition set names an edition provider with `provider` in place of `connector_type`. Save this as `orders-events.json`:

```json
{
  "name": "orders-events",
  "provider": "com.orion.kafka",
  "config": {
    "brokers": ["kafka-1.example.com:9092", "kafka-2.example.com:9092"],
    "security_protocol": "sasl_ssl",
    "sasl_mechanism": "SCRAM-SHA-512",
    "sasl_username": "orion",
    "sasl_password": "env://KAFKA_PASSWORD",
    "allow_private_urls": true
  }
}
```

Each broker is `host:port`, with no scheme; a value containing `://` is refused. `security_protocol` is `plaintext`, `ssl`, `sasl_plaintext` or `sasl_ssl`. A SASL protocol needs `sasl_mechanism` (`PLAIN`, `SCRAM-SHA-256` or `SCRAM-SHA-512`) and both halves of the credential.

`sasl_password` is a secret locator, never a literal. `compile` moves the reference into the connection revision's `secret_locators`, and the node resolves it when the connection is used. `sasl_username` and `ssl_ca_pem` may also be locators when they are secret.

`allow_private_urls` is off by default. Turn it on when the bootstrap brokers resolve to an internal address. Only the bootstrap addresses are checked; the brokers the cluster advertises afterwards are reached as any Kafka client reaches them.

A `client` map carries further librdkafka properties. The keys Orion sets itself are refused, by full path, so the connection cannot disagree with the channel about the group or the commit mode.

## 2. Declare the channel

Save the channel beside the workflow and the connection:

```json
{
  "channel_id": "orders-kafka",
  "name": "orders-kafka",
  "channel_type": "async",
  "protocol": "kafka",
  "workflow_id": "order-processing",
  "connections": ["orders-events"],
  "transport_config": {
    "connection": "orders-events",
    "topics": ["orders.placed", "orders.cancelled"],
    "group_id": "orion-order-processing",
    "start": "earliest",
    "processing_timeout_ms": 60000,
    "dlq": { "topic": "orders.dlq" }
  }
}
```

A `kafka` channel is asynchronous by construction. It takes no `route_pattern` and no `methods`; a record, not an HTTP request, starts a run. `/api/v2/data/orders-kafka` answers `405`.

`connections` and `transport_config.connection` must name the same connection. The first makes the deployment member require it; the second is the cluster the driver consumes from. A channel whose `connections` omits it is refused with `REQUIRED`.

The `transport_config` fields are:

| Field | Meaning |
|---|---|
| `connection` | The Kafka connection to consume from. |
| `topics` | One to 100 topic names, each unique. |
| `group_id` | The consumer group to join, 1 to 255 characters. |
| `start` | `earliest` or `latest`, for a group with no committed offset. Default `earliest`. |
| `processing_timeout_ms` | Ceiling on one record's run, 1 to 300000. Default 60000. |
| `dlq.topic` | Where a record that cannot run is preserved. Must not be a consumed topic. |
| `client` | librdkafka consumer properties for this group. |

Topic names use letters, digits, `.`, `_` and `-`, up to 249 bytes. A repeated topic, or a dead-letter topic that is also consumed, is refused at upload.

`start` applies only to a new group. Once the group has committed an offset, it resumes there; changing `start` later does not rewind it. Use a new `group_id` to read a topic again.

`processing_timeout_ms` is the deadline for one run. The channel's own `timeout_ms` may shorten it, never lengthen it, and the shorter value wins.

## 3. Read the record

The record's payload is parsed as JSON and becomes the run's `data`. The key, topic, partition, offset and headers arrive at `metadata.trigger`. Save the workflow as `order-processing.json`:

```json
{
  "workflow_id": "order-processing",
  "name": "Order processing",
  "condition": true,
  "tasks": [
    {
      "id": "validate",
      "name": "Validate the record",
      "on_rejection": "halt",
      "function": {
        "name": "validation",
        "input": {
          "rules": [
            {
              "logic": { "!!": [{ "var": "data.order_id" }] },
              "message": "order_id is required"
            }
          ]
        }
      }
    },
    {
      "id": "placed",
      "name": "Record the placement",
      "condition": {
        "==": [{ "var": "metadata.trigger.topic" }, "orders.placed"]
      },
      "function": {
        "name": "map",
        "input": {
          "mappings": [
            { "path": "data.event", "logic": "placed" },
            { "path": "data.partition", "logic": { "var": "metadata.trigger.partition" } }
          ]
        }
      }
    },
    {
      "id": "cancelled",
      "name": "Record the cancellation",
      "condition": {
        "==": [{ "var": "metadata.trigger.topic" }, "orders.cancelled"]
      },
      "function": {
        "name": "map",
        "input": {
          "mappings": [
            { "path": "data.event", "logic": "cancelled" },
            { "path": "data.partition", "logic": { "var": "metadata.trigger.partition" } }
          ]
        }
      }
    }
  ]
}
```

One workflow serves both topics because the run tells it which topic delivered the record:

| Trigger key | Meaning |
|---|---|
| `topic`, `partition`, `offset` | Where the record sits in the log. |
| `group_id` | The consumer group that delivered it. |
| `key` | The record key, when the record has one. |
| `timestamp_ms` | The broker timestamp, when the record has one. |
| `headers` | The record's UTF-8 headers, as an object of strings. |

The payload is already `data`; do not run `parse_json` over it. A record that is not JSON never reaches the workflow.

## 4. Know what delivery guarantees

Delivery is at least once. An offset is committed only after its run completed, the host recognised a settled duplicate, or a dead-letter write was confirmed. A lost commit causes a redelivery, never a lost record.

One group processes records strictly in order: one in flight per partition, and one at a time across the group. A group scales by spreading its partitions across the nodes in it. Ten partitions across two nodes means five consumers each, not ten concurrent runs per node.

| Outcome | What the driver does |
|---|---|
| Completed, duplicate | Commits the offset. |
| Deferred | Releases the claim and retries the record in place with backoff. |
| Rejected, failed | Writes the dead-letter record, then commits only once that write is confirmed. |
| Unavailable | Releases the claim and pauses the partition until the next reconcile. |

Retries happen in place, from one second doubling to sixty. A record that exhausts 80% of `max.poll.interval.ms` is rewound and redelivered from the poll loop, so the group stays a member.

At least once is not exactly once. A crash between the run and the commit runs the record again, so a workflow that writes must make the write idempotent. `data.order_id` or `metadata.trigger.key` is the natural key for an upsert. The record key is also what the host uses to recognise a redelivery.

> [!WARNING]
> If the channel sets `deduplication`, the record key becomes the idempotency key. Two different records that share a key inside the window run once, and the second is committed without a run. Leave deduplication off on a consume channel unless the key identifies the work, not the entity.

The channel's guards are `rate_limit`, `validation_logic`, `deduplication` and `backpressure`. The upload refuses `auth`, `origin_allow_list`, `cache`, `request`, `response` and `oauth2_login`: the connection authenticates the delivery, and there is no HTTP caller or response.

## 5. Preserve what cannot run

`dlq.topic` keeps a record the workflow could not process. The driver writes one JSON envelope to that topic:

```json
{
  "channel": "orders-kafka",
  "source_topic": "orders.placed",
  "partition": 3,
  "offset": 1042,
  "key": "ord_1001",
  "reason": "workflow",
  "error": "the warehouse rejected the order",
  "original_payload": "{\"order_id\":\"ord_1001\",\"total\":42.5}",
  "dead_lettered_at_ms": 1763337600000
}
```

`reason` is `empty`, `decode` or `parse` for a record that never reached the host, or `rejected` for a run refused by the channel. A failed run writes `timeout`, `workflow`, `engine` or `failed`. The record's key is reused as the dead-letter record's key, so a DLQ consumer keeps the original partitioning.

Without a dead-letter topic, a record that cannot run blocks its partition. That is visible as lag, not as an error, so set `dlq` before production.

## 6. Deploy

Compile against the target's capabilities, then plan the directory:

```bash
orion-cli deployments capabilities --output json > target-capabilities.json
orion-cli compile ./definitions -o dist/deployment --capabilities target-capabilities.json
orion-cli deployments plan -d dist/deployment
```

`--capabilities` is what tells `compile` that the target serves `kafka`. Without it, the protocol is unknown and the set is refused.

Apply the reviewed directory and read the receipt:

```bash
orion-cli deployments apply -d dist/deployment | jq '{committed_revision, converged: .evaluation.converged}'
```

```json
{
  "committed_revision": 2,
  "converged": true
}
```

## Verify

Confirm the node loaded the channel and the group is consuming:

```bash
orion-cli engine status
curl -s http://localhost:8080/health | jq '.components.kafka'
```

`engine status` lists `orders-kafka` under `Channels` and reports no load issue for it. The health component answers `"ok"` while every group consumes, `"degraded"` with a reason when one cannot, and `"disabled"` when no Kafka channel is deployed. A `degraded` driver also removes the node from `/readyz`.

Produce one record to `orders.placed` with any Kafka client, then read the trace. Its mode is `kafka`:

```bash
orion-cli traces list --channel orders-kafka --mode kafka --limit 5
```

Open the newest one to see the task steps and the final `data`:

```bash
orion-cli traces get <TRACE_ID>
```

Read lag and dead letters from the node's metrics:

```bash
curl -s http://localhost:8080/metrics | grep -E 'orion_kafka_(consumer_lag|dlq|rebalances|commit_errors)'
```

`orion_kafka_consumer_lag_messages{channel,topic,partition}` is the distance from the committed offset to the high-water mark, reported every 30 seconds. A lag that climbs while the workflow succeeds means the group is not keeping up. A lag pinned on one partition usually means a record that cannot run.

| Metric | Type | Meaning |
|---|---|---|
| `orion_kafka_consumer_lag_messages` | gauge | Records behind the high-water mark, per partition. |
| `orion_kafka_dlq_total` | counter | Records written to a dead-letter topic, by `channel` and `reason`. |
| `orion_kafka_rebalances_total` | counter | Completed partition assignments; a sustained rate means an unstable group. |
| `orion_kafka_commit_errors_total` | counter | Failed offset commits; records may be redelivered. |

## Next steps

- [Kafka transport](../../reference/channel-config/kafka.md): every field, the delivery table and the exact refusals.
- [`com.orion.kafka`](../../reference/connectors/edition/kafka.md): the same connection's `publish` operation and its error codes.
- [Monitor and alert](../../operate/run/monitoring.md): the metrics worth an alert, and the thresholds that belong on them.
- [Configure a channel](./channels.md): the guards this page leaves at their defaults.
