<!-- description: Install Orion Standard or Enterprise, configure edition settings, inspect runtime capabilities, and compile definitions for licensed features. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Overview and setup

<span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

Orion Standard and Enterprise (`orion-ext`) extend the open-source Community runtime (`orion-server`) with three event and API channel protocols (`kafka`, `graphql`, `sse`), seven first-party connector providers (`com.orion.*`), multi-replica Redis cluster coordination, and canary traffic splitting across workflow revisions. Both commercial tiers ship as a single `orion-ext` binary and unlock the same technical feature set.

## Capability matrix

| Capability | Community (`orion-server`) | Standard & Enterprise (`orion-ext`) | Documentation |
|---|:---:|:---:|---|
| `rest`, `http`, and `cron` channels | Yes | Yes | [Configure channels](../guides/channels.md), [Scheduled jobs](../guides/scheduled-jobs.md) |
| `kafka` channel driver | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Kafka channels and producer](./kafka.md) |
| `graphql` channel driver | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [GraphQL APIs](./graphql.md) |
| `sse` channel driver | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Server-Sent Events (SSE)](./sse.md) |
| Built-in connectors (`http`, `db`, `cache`) | Yes | Yes | [Connect databases, APIs, and caches](../guides/connectors.md) |
| `com.orion.smtp`, `com.orion.kafka`, `com.orion.redis`, `com.orion.storage`, `com.orion.mongodb`, `com.orion.elasticsearch`, `com.orion.models` | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Enterprise connector providers](./providers.md) |
| Canary traffic splitting (`rollouts`) | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Canary rollouts and traffic splitting](./traffic-splitting.md) |
| Multi-node cluster mode (`[cluster]` + `[extensions.redis]`) | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Multi-node clustering and HA](./clustering.md) |
| Custom provider packages (`community` tier) | Yes | Yes | [Custom provider packages](./provider-packages.md) |
| Custom provider packages (`standard` / `enterprise` tiers) | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Custom provider packages](./provider-packages.md) |

## Configure the edition tier

An `orion-ext` node reports `"standard"` by default in `GET /api/v2/admin/runtime-capabilities`. To report `"enterprise"` (required to admit custom provider packages declared with `"tier": "enterprise"`), set `[extensions.licence]` in `orion.toml`:

> [!NOTE]
> The configuration table uses the spelling `[extensions.licence]` (with a `c`). Extension tables sit outside the `ORION_SECTION__KEY` environment variable scheme, so configure them in `orion.toml`.

```toml
[extensions.licence]
tier = "enterprise"
```

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `tier` | string | yes | `"standard"` (when section is omitted) | The edition reported by the node: `"standard"` or `"enterprise"`. Both tiers serve the same built-in protocols and providers. |

## Run `orion-ext`

Run the `orion-ext` binary or container image with your `orion.toml` configuration file:

<!-- tabs:start -->
### **Docker**
```bash
docker run -d --name orion-ext \
  -p 8080:8080 \
  -v "$(pwd)/orion.toml:/etc/orion/orion.toml:ro" \
  -e ORION_ADMIN_AUTH__API_KEYS="replace-with-admin-key" \
  orion-ext:2.0.0-rc.1 \
  --config /etc/orion/orion.toml
```

### **Binary**
```bash
ORION_ADMIN_AUTH__API_KEYS="replace-with-admin-key" \
  orion-ext --config ./orion.toml
```
<!-- tabs:end -->

## Inspect runtime capabilities

Before authoring definitions or compiling a deployment in CI, capture the target node's capabilities. The response lists the node's `edition`, `admission_policy_id`, registered `channel_drivers`, and `provider_backends` (including each provider's `package_digest` and operation schemas).

<!-- tabs:start -->
### **CLI**
```bash
orion-cli deployments capabilities
orion-cli deployments capabilities --output json > target-capabilities.json
```

### **curl**
```bash
curl -s http://localhost:8080/api/v2/admin/runtime-capabilities \
  -H "X-Admin-Key: $ORION_ADMIN_KEY" | jq .
```
<!-- tabs:end -->

Human-readable CLI output summarizes the edition, active revision, channel drivers, and provider backends:

```text
Orion Standard · v2 admin API 2 · admission policy orion.ext.admission/0
  Head:        revision 8 (epoch 12) · this node serves 8
  Node:        orion-1 (boot 3f2a)
  Reconciler:  desired revision 8 · configuration ok
```

## Validate, compile, and deploy against target capabilities

Pass `target-capabilities.json` to `orion-cli lint`, `orion-cli clippy`, and `orion-cli compile` so offline builds verify channel protocols, transport schemas, and provider operations against the target node:

```bash
# 1. Validate schemas and provider operations against the target node
orion-cli lint ./definitions --capabilities target-capabilities.json

# 2. Run static analysis checks
orion-cli clippy ./definitions --capabilities target-capabilities.json

# 3. Compile a deployment directory
orion-cli compile ./definitions \
  -o dist/deployment \
  --capabilities target-capabilities.json

# 4. Plan and apply the compiled directory
orion-cli deployments plan -d dist/deployment
orion-cli deployments apply -d dist/deployment --wait-ms 30000
```

You can also enforce a minimum edition during compilation with `--require-edition standard` or `--require-edition enterprise` so a pipeline fails immediately if handed a capabilities file from a Community node.

## Community node refusal codes

When a definition or deployment using Standard+ features is sent to an open-source Community node, the server rejects it before activation:

| Situation on a Community node | Refusal status and code |
|---|---|
| Uploading a channel with `"protocol": "kafka"`, `"graphql"`, or `"sse"` | `400 VALIDATION_ERROR` (lists the protocols served by the node) |
| Creating a connection naming a `com.orion.*` provider | `400 VALIDATION_ERROR` or `404 PROVIDER_NOT_FOUND` |
| Planning or applying a channel member with two or more `rollouts` intervals | `403 TIER_NOT_ENABLED` |
| Admitting a provider package that declares `"tier": "standard"` or `"enterprise"` | `403 TIER_NOT_ENABLED` |
| Starting with `cluster.enabled = true` and no registered cluster store | Boot fails with `NO_CLUSTER_STORE` |

## Next steps

- [Kafka channels and producer](./kafka.md): Consume from and publish to Apache Kafka topics.
- [GraphQL APIs](./graphql.md): Serve GraphQL queries and mutations backed by Orion workflows.
- [Server-Sent Events (SSE)](./sse.md): Stream live events to browsers and HTTP clients.
- [Enterprise connector providers](./providers.md): Connect workflows to SMTP, Redis, S3 storage, MongoDB, Elasticsearch, and ONNX models.
- [Multi-node clustering and HA](./clustering.md): Run multiple replicas over PostgreSQL or MySQL and a shared Redis store.
