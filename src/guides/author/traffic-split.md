<!-- description: Split a channel across two workflow revisions with a Standard+ canary: versioned definitions, bucket rollouts, plan marks, observation and rollback. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Split traffic across workflow revisions

Available in Orion Standard and Enterprise only. A <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> canary sends a share of a channel's traffic to another workflow revision. This guide splits `orders` 90/10, verifies it and rolls it back.

## Before you start

You need:

- An `orion-ext` node at `http://localhost:8080`, with the `orders` channel already serving at deployment revision 1.
- A definition set, `orion-cli`, and admin access to the node.
- Two workflows that answer the same request.

A traffic split is a property of a channel's deployment member, not a second channel and not a load balancer rule. The channel keeps one route, one name and one set of guards; only the workflow revision behind it varies.

## 1. Write two revisions of one workflow

Both documents carry the same `workflow_id` and a distinct, explicit `version`. The version is an authoring selector; the node still assigns deployment revisions. Save the first as `order-processing-v1.json`:

```json
{
  "workflow_id": "order-processing",
  "version": 1,
  "name": "Order processing (v1)",
  "condition": true,
  "tasks": [
    {
      "id": "parse",
      "name": "Parse the payload",
      "function": {
        "name": "parse_json",
        "input": { "source": "payload", "target": "order" }
      }
    },
    {
      "id": "route",
      "name": "Report the revision",
      "function": {
        "name": "map",
        "input": {
          "mappings": [
            { "path": "data.routing", "logic": "v1" },
            { "path": "data.order_id", "logic": { "var": "data.order.order_id" } }
          ]
        }
      }
    }
  ]
}
```

Save the second as `order-processing-v2.json`. It has the same `workflow_id`, `version: 2`, and a different definition:

```json
{
  "workflow_id": "order-processing",
  "version": 2,
  "name": "Order processing (v2)",
  "condition": true,
  "tasks": [
    {
      "id": "parse",
      "name": "Parse the payload",
      "function": {
        "name": "parse_json",
        "input": { "source": "payload", "target": "order" }
      }
    },
    {
      "id": "route",
      "name": "Report the revision",
      "function": {
        "name": "map",
        "input": {
          "mappings": [
            { "path": "data.routing", "logic": "v2" },
            { "path": "data.order_id", "logic": { "var": "data.order.order_id" } }
          ]
        }
      }
    }
  ]
}
```

The two definitions must differ. Two versions with byte-identical definitions are refused, because the target would assign them one revision and there would be nothing to split. A set where one revision omits `version` and another carries one is also refused: a split needs an explicit version on every variant.

The `routing` field is only there to make the result visible. A real canary would carry the new logic instead; the field is how the verification step tells the two apart.

## 2. Put the split on the channel

The channel names its workflow and a list of bucket intervals. Save this as `orders.json`:

```json
{
  "channel_id": "orders",
  "name": "orders",
  "channel_type": "sync",
  "protocol": "rest",
  "methods": ["POST"],
  "route_pattern": "/orders",
  "workflow_id": "order-processing",
  "rollouts": [
    { "version": 1, "start_bucket": 0, "end_bucket": 90 },
    { "version": 2, "start_bucket": 90, "end_bucket": 100 }
  ]
}
```

The intervals partition the hundred buckets `[0, 100)` with no gap and no overlap. This one sends 90% to version 1 and 10% to version 2. Each interval names a `version` that the set carries, and a version appears in one interval only. One bucket is one percent, so the finest split is 99/1.

A channel whose workflow has more than one version in the set must carry `rollouts`; without them the set is refused as ambiguous. A channel with a single workflow version needs no `rollouts`. A single interval covering every bucket is not a split, so a Community node accepts it.

## 3. Compile and read the plan

Compile the set, then plan the directory against the head. Planning commits nothing:

```bash
orion-cli deployments capabilities --output json > target-capabilities.json
orion-cli compile ./definitions -o dist/canary --capabilities target-capabilities.json
orion-cli deployments plan -d dist/canary
```

The plan prints one line per member, then the plan document. Read the mark at the start of each line:

| Mark | Meaning |
|---|---|
| `=` | The head already deploys this member at this revision. |
| `+` | The member is new, or its definition changed and it takes a new revision. |
| `-` | The head deploys the member and the candidate drops it. It stops serving. |

```text
Plan: dist/canary against head revision 1
  = workflow order-processing@1
  + workflow order-processing@2
  = channel orders@1
  1 to deploy, 2 unchanged, 0 removed; reviewed manifest digest sha256:…
```

Version 1 of the workflow is unchanged. Version 2 is a new member under the same workflow id, so it takes the next revision. The channel keeps its revision because its definition did not change: `rollouts` live on the deployment member, not in the channel document. The manifest digest still changes, so the apply below commits a revision.

A `=` beside the channel is therefore not "nothing changes". Read the reviewed manifest digest, not only the marks. The channel's `requires` names both workflow revisions; a rollout interval that names a revision the member does not require is refused.

## 4. Apply

Apply the reviewed directory. The CLI plans it again, then commits under compare-and-swap on the head:

```bash
orion-cli deployments apply -d dist/canary | jq '{committed_revision, converged: .evaluation.converged}'
```

```json
{
  "committed_revision": 2,
  "converged": true
}
```

The channel now serves both revisions. Nothing restarted, and no request was dropped: a request in flight finishes on the generation it started with.

## 5. Know which revision answers

The hundred buckets are assigned by hashing a caller identity, not by counting requests. For a synchronous REST channel, the identity is `engine.rollout_sticky_header` when that setting names a request header and the request carries it. Otherwise it is the forwarded client address, from `x-forwarded-for` first and `x-real-ip` second. A request with neither gets a random bucket.

That makes the split sticky by caller: the same header value or address lands in the same bucket on every request. Two consequences follow. The observed ratio approaches 90/10 over many callers, not over the first ten requests from one caller. And a caller behind a shared address is stuck with whatever that address drew.

A Kafka record hashes its topic, partition and offset, so a redelivery stays on the revision that first ran it. A GraphQL request hashes its request id, so every field of one document routes alike. A stream's runs hash its request id, so one open stream stays on one revision while it lasts.

Set the header when you want a person or a tenant to stay in one bucket. It is a node setting, read at boot, so restart the node after changing it:

```toml
[engine]
rollout_sticky_header = "x-user-id"
```

Without a header value the caller's address decides the bucket. Behind a proxy, make sure `x-forwarded-for` reaches the node unchanged, or every caller shares the proxy's bucket.

## Verify

Send a batch and count the revisions that answered:

```bash
for i in $(seq 1 100); do
  curl -s -X POST http://localhost:8080/orders \
    -H 'content-type: application/json' \
    -H "x-user-id: user-$i" \
    -d '{"order_id":"ord_'"$i"'","total":42.5}' | jq -r '.data.routing'
done | sort | uniq -c
```

```text
   89 v1
   11 v2
```

The exact counts vary with the buckets the identities hash to. What matters is that both values appear, and that the shares are close to the intervals you declared.

Read a trace from the new revision. Its task output carries `routing: "v2"`:

```bash
orion-cli traces list --channel orders --limit 20
orion-cli traces get <TRACE_ID>
```

The workflow metric `orion_workflow_duration_seconds{workflow}` carries the workflow id, which both revisions share, so it does not split the canary for you. Use the trace, or a counter your workflow increments on its own path. Compare error rates by listing only failed traces, then opening them:

```bash
orion-cli traces list --channel orders --status failed --limit 20
```

## 6. Move the weight, then roll back

To widen the canary, change the intervals and apply again:

```json
"rollouts": [
  { "version": 1, "start_bucket": 0, "end_bucket": 50 },
  { "version": 2, "start_bucket": 50, "end_bucket": 100 }
]
```

```bash
orion-cli compile ./definitions -o dist/canary --capabilities target-capabilities.json
orion-cli deployments plan -d dist/canary
orion-cli deployments apply -d dist/canary
```

To end the canary on the old revision, give version 1 every bucket:

```json
"rollouts": [
  { "version": 1, "start_bucket": 0, "end_bucket": 100 }
]
```

A single interval covering every bucket is not a split, and a Community node accepts it. The second workflow may stay in the set; the channel simply routes nothing to it.

To undo the whole deployment rather than re-weight it, commit a new revision from the history:

```bash
orion-cli deployments rollback \
  --target-revision 1 \
  --expected-head-revision 2 \
  --idempotency-key rollback-orders-canary \
  --reason "canary failed its error budget"
```

`--expected-head-revision` refuses the rollback if someone deployed in the meantime. The definitions it restores are stored by digest, so they are exactly what last served. Rollback is a forward revision, not an edit of history.

### On a Community node

A Community node refuses a split channel at plan, apply and rollback with `403 TIER_NOT_ENABLED`, naming the channels that carry more than one interval. The refusal is the server's; `orion-cli deployments plan -f` also checks the edition before sending a hand-written manifest, and fails with:

```text
canary rollout (a channel's `rollouts`) requires edition 'standard' or higher; this node reports 'community'
```

A compiled directory (`-d`) is checked by the server. Either way, nothing is committed. A channel with a single interval is not a split and is accepted. The [edition capability matrix](../../reference/editions.md) lists the gate.

## Next steps

- [Deployment endpoints](../../reference/admin-api/deployments.md): the plan, apply and rollback contract, and the deployment directory.
- [Deployments](../../concepts/deployments.md): how a manifest becomes the head every node serves.
- [Traces](../../operate/run/traces.md): reading the run that a bucket selected.
- [Edition capability matrix](../../reference/editions.md): every gate, with the refusal each returns.
