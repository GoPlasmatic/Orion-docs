<!-- description: Every Orion capability with the edition that serves it, the package tier ladder, and the refusals a Community node returns for gated features. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# Edition capability matrix

Every capability in Orion, the edition that serves it, and the page that specifies it. Community is the unmarked default. A cell marked is served by Standard and Enterprise and refused by Community.

## Synopsis

```bash
orion-cli deployments capabilities
orion-cli deployments capabilities --output json > target-capabilities.json
```

```json
GET /api/v2/admin/runtime-capabilities
```

## Description

The edition is a property of the composition, not of a request. Community composes the open-source foundation. Standard and Enterprise compose that foundation plus every Orion Ext registration under one binary. The node reports the edition, its drivers and its provider backends in one answer.

Read the answer before authoring a channel protocol, a provider connection or a deployment that splits traffic. A deployment pipeline that skips the read can build a definition the target will refuse.

## Capability matrix

| Capability | Community | Standard+ | Specified by |
|---|:---:|:---:|---|
| `rest` channel driver | Yes | Yes | [Routing and protocol](./channel-config/routing.md) |
| `http` channel driver | Yes | Yes | [Routing and protocol](./channel-config/routing.md) |
| `cron` channel driver | Yes | Yes | [Cron transport](./channel-config/cron.md) |
| `kafka` channel driver | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Kafka transport](./channel-config/kafka.md) |
| `graphql` channel driver | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [GraphQL transport](./channel-config/graphql.md) |
| `sse` channel driver | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [SSE transport](./channel-config/sse.md) |
| `http` connector over `org.orion.http` | Yes | Yes | [`http` connectors](./connectors/http.md) |
| `db` connector over `org.orion.postgresql`, `org.orion.mysql` or `org.orion.sqlite` | Yes | Yes | [`db` connectors](./connectors/db.md) |
| `cache` connector over `org.orion.memory-cache` | Yes | Yes | [`cache` connectors](./connectors/cache.md) |
| `com.orion.smtp` provider | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [SMTP provider](./connectors/edition/smtp.md) |
| `com.orion.kafka` provider | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Kafka provider](./connectors/edition/kafka.md) |
| `com.orion.redis` provider | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Redis provider](./connectors/edition/redis.md) |
| `com.orion.storage` provider | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Storage provider](./connectors/edition/storage.md) |
| `com.orion.mongodb` provider | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [MongoDB provider](./connectors/edition/mongodb.md) |
| `com.orion.elasticsearch` provider | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Elasticsearch provider](./connectors/edition/elasticsearch.md) |
| `com.orion.models` provider | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Models provider](./connectors/edition/models.md) |
| Cluster mode | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Cluster settings](./configuration/cluster.md) |
| Redis cluster store | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Extension settings](./configuration/extensions.md) |
| Canary rollout across workflow revisions | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Deployment endpoints](./admin-api/deployments.md) |
| WebAssembly plugins | Yes | Yes | [Plugin manifest](./plugin-manifest.md) |
| Cron scheduling | Yes | Yes | [Cron transport](./channel-config/cron.md) |

## Package tiers

A provider package declares a tier in `distribution.json`. A node admits a package whose tier is at or below its own edition.

| Tier | Community | Standard | Enterprise |
|---|:---:|:---:|:---:|
| `community` | Yes | Yes | Yes |
| `standard` | No | Yes | Yes |
| `enterprise` | No | No | Yes |

A first-party Enterprise-only feature does not ship today. The `enterprise` tier lets an organization build and admit a package that only an Enterprise node accepts. The shipped Orion Ext packages declare `standard`.

`orion-cli provider scaffold --tier` writes the tier. Omit the flag and the package declares `community`.

## Refusals

| Situation | Answer |
|---|---|
| A channel names `kafka`, `graphql` or `sse` on Community | `400 VALIDATION_ERROR` from the upload, naming `protocol` and the drivers the node serves |
| A connection names an edition provider on Community | `400 VALIDATION_ERROR` or `404 PROVIDER_NOT_FOUND`, depending on where the refusal is found |
| `cluster.enabled = true` with no composed store | Boot fails with the `NO_CLUSTER_STORE` message |
| A channel member has two or more `rollouts` intervals on Community | `403 TIER_NOT_ENABLED` at plan, apply and rollback |
| A package declares a tier above the node's edition | `403 TIER_NOT_ENABLED` |
| A `distribution.json` names an unknown tier | `400 PACKAGE_INVALID_TIER` |

## Examples

Read the target before authoring a definition set:

```bash
orion-cli deployments capabilities --output json > target-capabilities.json
orion-cli lint ./definitions --capabilities target-capabilities.json
orion-cli compile ./definitions -o dist/deployment --capabilities target-capabilities.json
```

The rendered answer starts with the edition and the admission policy:

```text
Orion Standard · v2 admin API 2 · admission policy orion.ext.admission/0
  Head:        revision 8 (epoch 12) · this node serves 8
  Node:        orion-1 (boot 3f2a)
  Reconciler:  desired revision 8 · configuration ok
```

## Caveats

- Community has no cluster store. A single Community node keeps deduplication, response cache and rate limits in process.
- A Standard or Enterprise node reports `standard` unless `[extensions.licence]` names `enterprise`. The reported edition does not change the capabilities it serves.
- Edition facts belong in the capability answer. Do not infer them from the image tag, the hostname or the presence of a config file.
- A Community refusal is deliberate. A definition that needs an edition feature has no Community workaround that preserves its behaviour.

## Related

- [Editions](../concepts/editions.md): the model and the support boundary.
- [Extension settings](./configuration/extensions.md): the licence and Redis cluster store sections.
- [Runtime capabilities](./admin-api/deployments.md#the-contract-and-its-version): the endpoint behind the CLI command.
- [Connector providers](./connectors/edition/index.md): the seven providers and their operations.
- [Deploy Orion Ext](../operate/deploy/orion-ext.md): the licensed image and topology.
