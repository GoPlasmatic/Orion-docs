<!-- description: What an Orion edition is, how Community differs from Standard and Enterprise, and how the node reports which capabilities it serves. -->
<!-- type: concept -->
<!-- last_verified: 2026-11-16 -->

# Editions

An *edition* is the capability and support boundary around the same Orion runtime. It is not a different product, and it is not a build flag.

## The problem it solves

A runtime that serves a request has one behaviour. A commercial product needs a way to package that behaviour for different support terms without forking the code or making the open-source node mysteriously different. Orion solves that by keeping one runtime and one licensed artifact, and by making the edition a recorded fact the node reports.

The edition answers two questions:

- **What does this node serve?** Channel protocols, provider backends and feature gates.
- **What may it admit?** A provider package declares a tier, and the node admits a package at or below its own edition.

## How it works

Orion has three editions:

| Edition | What it is | How it is obtained |
|---|---|---|
| **Community** | The open-source node, `orion-server`. | Apache-2.0, from the public repository and public images. |
| **Standard** | Community's node plus every Orion Ext driver, provider and cluster store. | The licensed `orion-ext` artifact through the private image registry. |
| **Enterprise** | The same artifact as Standard. | The same artifact, with an Enterprise support agreement. |

Standard and Enterprise are one binary. Both compose Community's foundation, both register the same extensions, and both serve the same capabilities. They differ in support terms, not in software. `[extensions.licence]` only names the edition the node reports; absent, it reports `standard`.

The edition is a ladder. A node admits a provider package whose declared tier is at or below its own:

| Package tier | Community | Standard+ | Enterprise |
|---|:---:|:---:|:---:|
| `community` | Yes | Yes | Yes |
| `standard` | No | Yes | Yes |
| `enterprise` | No | No | Yes |

The shipped Orion Ext packages declare `standard`, so either licensed edition admits them. A first-party Enterprise-only feature does not ship today; the `enterprise` tier is the mechanism that would carry one.

### What changes by edition

 marks a feature that Community does not serve. The full list is in the [edition capability matrix](../reference/editions.md).

| Capability | Community | Standard+ |
|---|:---:|:---:|
| `rest`, `http`, `cron` channels | Yes | Yes |
| `kafka`, `graphql`, `sse` channels | No | Yes |
| `http`, `db`, `cache` connectors | Yes | Yes |
| SMTP, Kafka, Redis, storage, MongoDB, Elasticsearch and model providers | No | Yes |
| Cluster mode | No | Yes |
| Canary rollout across workflow revisions | No | Yes |

A Community node does not hide these features behind a disabled switch. It has no driver or store to run them, so it refuses the definition or the boot:

- a `kafka`, `graphql` or `sse` channel has no driver, so it is refused at upload;
- an edition provider has no backend, so a connection over it cannot be created;
- `cluster.enabled = true` fails boot with the `NO_CLUSTER_STORE` message;
- a split channel is refused at plan, apply and rollback with `TIER_NOT_ENABLED`;
- a package above the node's tier is refused with `TIER_NOT_ENABLED`.

### How the node reports it

Ask the node, do not infer it from a URL or an image tag:

```bash
orion-cli deployments capabilities
```

The answer names the edition, the admission policy, every channel driver with its capabilities, and every provider backend with its operations. The same answer is at `GET /api/v2/admin/runtime-capabilities`. An agent or a deployment pipeline should read it before authoring or applying an edition feature.

## Boundaries

- **The edition is not a per-request gate.** A node serves what its composition registered. There is no runtime toggle that turns a Standard feature on in Community.
- **The edition is not an entitlement check.** `orion-ext` ships no licence verification, expiry or binding. Access to the artifact is the commercial control.
- **The edition does not change the wire contract.** Community and the licensed editions speak the same v2 admin API and the same data plane.
- **A canary is not a second workflow.** It is a channel member's traffic split across workflow revisions, and it is the only deployment feature that is edition-gated.
- **Cluster mode is not a database setting alone.** Standard+ requires a shared store (`[extensions.redis]`); the state database continues to hold deployment history, leases, the cron ledger and breaker epochs.

## Next steps

- [Edition capability matrix](../reference/editions.md): every capability with its minimum edition and the page that specifies it.
- [Extension settings](../reference/configuration/extensions.md): the `[extensions.licence]` and `[extensions.redis]` sections.
- [Deploy Orion Ext](../operate/deploy/orion-ext.md): the licensed image and its reference HA topology.
- [Run a cluster](../operate/deploy/cluster.md): the shared store, convergence and migrations.
- [Connector providers](../reference/connectors/edition/index.md): the seven providers and how a set names them.
