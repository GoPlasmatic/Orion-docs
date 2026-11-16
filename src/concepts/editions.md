<!-- description: Compare Orion Community, Standard, and Enterprise editions: the capability matrix, package admission tiers, refusal codes, and runtime capability checks. -->
<!-- type: concept -->
<!-- last_verified: 2026-11-16 -->

# Editions and capabilities

Orion is available in three editions built on the same core runtime and v2 API contract: **Community**, **Standard**, and **Enterprise**.

---

## How editions are structured

| Edition | Binary / Artifact | Distribution | What It Includes |
| :--- | :--- | :--- | :--- |
| **Community** | `orion-server` | Open source (Apache-2.0) on GitHub, Crates.io, Homebrew, and GHCR | Single-node runtime, REST/HTTP/Cron channels, HTTP/SQL/Memory-cache connectors, all 27 built-in task functions, and WebAssembly plugins. |
| **Standard** | `orion-ext` | Commercial image via private registry | Everything in Community, plus `kafka`, `graphql`, and `sse` channels, 7 enterprise connector providers, canary traffic splitting, and multi-node Redis clustering. |
| **Enterprise** | `orion-ext` | Commercial image via private registry + Enterprise SLA | Identical binary and capabilities to Standard, plus admission support for `enterprise`-tier custom provider packages and Enterprise support terms. |

Standard and Enterprise ship as **one binary (`orion-ext`)**. There are no feature flags or runtime entitlement servers that phone home. The `[extensions.licence]` section in `orion.toml` sets the edition label the node reports (`standard` by default, or `enterprise`):

```toml
[extensions.licence]
tier = "enterprise"   # or "standard"
```

---

## Edition capability matrix

Community is the unmarked default throughout the documentation. Capabilities marked with <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> require `orion-ext`:

| Capability | Community (`orion-server`) | Standard & Enterprise (`orion-ext`) | Documentation |
| :--- | :---: | :---: | :--- |
| **`rest` and `http` channels** | Yes | Yes | [Configure channels and guards](../guides/channels.md) |
| **`cron` scheduled channels** | Yes | Yes | [Schedule jobs with cron](../guides/scheduled-jobs.md) |
| **`kafka` consumer channels** | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Kafka channels and producer](../enterprise/kafka.md) |
| **`graphql` API channels** | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [GraphQL APIs](../enterprise/graphql.md) |
| **`sse` streaming channels** | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Server-Sent Events (SSE)](../enterprise/sse.md) |
| **`http` connector (`org.orion.http`)** | Yes | Yes | [Connect databases, APIs, and caches](../guides/connectors.md) |
| **`db` connector (`postgresql`, `mysql`, `sqlite`)** | Yes | Yes | [Connect databases, APIs, and caches](../guides/connectors.md) |
| **`cache` connector (`org.orion.memory-cache`)** | Yes | Yes | [Connect databases, APIs, and caches](../guides/connectors.md) |
| **SMTP provider (`com.orion.smtp`)** | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Enterprise connector providers](../enterprise/providers.md) |
| **Kafka producer provider (`com.orion.kafka`)** | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Kafka channels and producer](../enterprise/kafka.md) |
| **Redis provider (`com.orion.redis`)** | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Enterprise connector providers](../enterprise/providers.md) |
| **S3 storage provider (`com.orion.storage`)** | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Enterprise connector providers](../enterprise/providers.md) |
| **MongoDB provider (`com.orion.mongodb`)** | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Enterprise connector providers](../enterprise/providers.md) |
| **Elasticsearch provider (`com.orion.elasticsearch`)** | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Enterprise connector providers](../enterprise/providers.md) |
| **ONNX models provider (`com.orion.models`)** | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Enterprise connector providers](../enterprise/providers.md) |
| **Canary traffic splitting (`rollouts`)** | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Canary rollouts and traffic splitting](../enterprise/traffic-splitting.md) |
| **Multi-node clustering & Redis cluster store** | No | <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | [Multi-node clustering and HA](../enterprise/clustering.md) |
| **WebAssembly plugins (`orion-plugin-sdk`)** | Yes | Yes | [Extend with WebAssembly plugins](../guides/plugins.md) |
| **Custom workflow provider packages** | `community` tier | Up to node's tier | [Custom provider packages](../enterprise/provider-packages.md) |

---

## Package admission tiers

When you publish a custom connector provider package (`orion-cli provider`), its `distribution.json` declares a `tier`. A node only admits packages whose tier is at or below the node's own edition:

| Package Tier | Admitted by Community | Admitted by Standard | Admitted by Enterprise |
| :--- | :---: | :---: | :---: |
| **`community`** | Yes | Yes | Yes |
| **`standard`** | No | Yes | Yes |
| **`enterprise`** | No | No | Yes |

All seven built-in Orion Ext providers declare `standard`, so both Standard and Enterprise nodes admit them.

---

## How Community rejects Standard+ features

A Community node never silently ignores an edition setting—it fails fast with an explicit error code:

| Action on a Community Node | Refusal Behavior |
| :--- | :--- |
| Uploading a `kafka`, `graphql`, or `sse` channel | Returns `400 VALIDATION_ERROR` on `protocol`, listing the supported drivers (`rest`, `http`, `cron`). |
| Creating a connection for an edition provider | Returns `400 VALIDATION_ERROR` or `404 PROVIDER_NOT_FOUND`. |
| Booting with `cluster.enabled = true` | Startup fails immediately with `NO_CLUSTER_STORE`. |
| Planning or applying a channel with `rollouts` (2+ buckets) | Returns `403 TIER_NOT_ENABLED`. |
| Admitting a `standard` or `enterprise` provider package | Returns `403 TIER_NOT_ENABLED`. |

---

## Inspecting a node's capabilities

Never guess which edition or drivers a target server supports from its hostname or image tag. Query the server directly with `orion-cli`:

<div class="tabs">
<section data-tab="CLI">

```bash
orion-cli deployments capabilities
```

</section>
<section data-tab="curl">

```bash
curl -s http://localhost:8080/api/v2/admin/runtime-capabilities | jq
```

</section>
</div>

Example output on an Orion Standard node:

```text
Orion Standard · v2 admin API 2 · admission policy orion.ext.admission/0
  Head:        revision 8 (epoch 12) · this node serves 8
  Node:        orion-1 (boot 3f2a)
  Reconciler:  desired revision 8 · configuration ok
```

When compiling definition sets that use `kafka`, `graphql`, `sse`, or edition providers, export the target node's capabilities to a JSON file and pass it to `lint`, `clippy`, and `compile`:

```bash
orion-cli deployments capabilities --output json > target-capabilities.json
orion-cli lint ./definitions --capabilities target-capabilities.json
orion-cli compile ./definitions -o dist/deployment --capabilities target-capabilities.json
```

## Next steps

- [Standard & Enterprise overview](../enterprise/overview.md) — Set up `orion-ext` and compile edition-aware deployments.
- [Multi-node clustering and HA](../enterprise/clustering.md) — Deploy a high-availability cluster with PostgreSQL and Redis.
- [Enterprise connector providers](../enterprise/providers.md) — Reference for all seven `com.orion.*` providers.
