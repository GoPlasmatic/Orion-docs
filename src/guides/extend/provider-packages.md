<!-- description: Publish a connector provider package: scaffold it, lay out its manifest and schemas, lint, pack, upload, admit, revoke, and bind its connection. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Publish a connector provider package

A provider package is how a connector is published as files: a manifest, its schemas, and the workflows or component that serve its operations. The package mechanism runs on any v2 node. A package that declares tier `standard` or `enterprise` is admitted only at that edition, so a Community node refuses it with `TIER_NOT_ENABLED`.

## Before you start

You need:

- A v2 node, Community or licensed, at `http://localhost:8080`.
- `orion-cli` at the same release as the node, and an admin API key.
- A publisher namespace you control, such as `com.example`.
- A provider id, a semantic version, and the operations the package will serve.

The example publishes `com.example/orders-api` version `1.0.0`, a `workflow` package with one operation, `item.get`. [Connector providers](../../reference/connectors/edition/index.md) is the reference for the seven packages Orion Ext ships; this page is how you publish your own.

## 1. Scaffold the package

Generate a deterministic package and write it to a file:

```bash
orion-cli provider scaffold \
  --publisher com.example \
  --provider-id orders-api \
  --version 1.0.0 \
  --kind workflow \
  --tier standard > scaffold.json
```

The answer is one JSON document with the package's files:

```json
{
  "publisher": "com.example",
  "provider_id": "orders-api",
  "version": "1.0.0",
  "implementation": "workflow",
  "package_digest": "sha256:…",
  "files": {
    "LICENSE": "Apache-2.0\n",
    "connector.json": "{…}",
    "distribution.json": "{…}"
  }
}
```

`files` is a map of relative path to contents. `package_digest` is what the scaffold's files pack to; it changes as soon as you edit one.

`--kind` chooses how the operations run:

| Kind | What it is | Where it runs |
|---|---|---|
| `rust` | A native provider compiled into the node. | Only on a node that registers a backend under the provider id. |
| `workflow` | One dataflow workflow per operation, held to the hermetic rules. | Every node once admitted. |
| `wasm` | A WebAssembly component in the declared world. | Admitted, but no Community node executes it yet. |

`--tier` writes the `tier` field of `distribution.json`. Omit it and the package declares `community`, which every node admits. `standard` needs a Standard or Enterprise node; `enterprise` needs Enterprise. The scaffold and lint run anywhere; the target's edition decides admission.

## 2. Know the layout

List the scaffold's paths:

```bash
jq -r '.files | keys[]' scaffold.json
```

```text
LICENSE
connector.json
distribution.json
schemas/config.public.json
schemas/config.secret.json
schemas/item.get.input.json
schemas/item.get.output.json
workflows/entry.json
```

| Path | Holds |
|---|---|
| `connector.json` | The manifest: identity, implementation, operations, schemas, dependencies and payload limits. |
| `distribution.json` | The publisher, policy requirements and the optional tier. |
| `LICENSE` | The package's licence. Required, whatever the licence is. |
| `schemas/config.public.json` | The connection's non-secret settings, as a JSON Schema. |
| `schemas/config.secret.json` | The credential fields, marked `x-orion-sensitive`. |
| `schemas/<operation>.input.json` | One operation's input schema. |
| `schemas/<operation>.output.json` | One operation's answer schema. |
| `workflows/<name>.json` | A workflow entry, for a `workflow` package. |
| `component.wasm` | The component, for a `wasm` package. |
| `inventory.json` | Added by `pack`; never authored. |

`connector.json` is the contract the node reads. Its scaffolded fields are:

```json
{
  "schema_version": 2,
  "kind": "connector-provider",
  "id": "orders-api",
  "version": "1.0.0",
  "contract": "orion:connector@1.0.0",
  "requires_orion": ">=2.0.0, <3.0.0",
  "schema_profile": "orion-json-schema/1",
  "implementation": {
    "kind": "workflow",
    "entries": { "item.get": "entry" },
    "workflows": ["workflows/entry.json"]
  },
  "configuration_schema": "schemas/config.public.json",
  "credential_schema": "schemas/config.secret.json",
  "operations": {
    "item.get": {
      "effect": "read",
      "retry": "safe",
      "input_schema": "schemas/item.get.input.json",
      "output_schema": "schemas/item.get.output.json"
    }
  },
  "dependencies": [],
  "capabilities": {},
  "payload_limits": {
    "configuration_bytes": 4096,
    "input_bytes": 4096,
    "output_bytes": 4096,
    "metadata_bytes": 1024
  }
}
```

A workflow package's `implementation` may hold only `kind`, `entries` and `workflows`. `entries` maps each operation to a workflow id, and `workflows` lists the files. Every workflow is an operation's entry: a helper workflow, or a workflow no operation enters, is refused. Each operation's `effect` and `retry` are contract; a retry-safe read may be retried, a write may not.

A workflow in a package may call the dataflow built-ins (`map`, `validation`, `filter`, `log`, `parse_json`, `parse_xml`, `publish_json`, `publish_xml`), `crypto`, `jwt_sign`, `jwt_verify` and plugin functions. It may also call `connector_call` on a dependency its manifest declares. It may not name `http_call`, `db_read`, `data_query`, `cache_*` or `channel_call`, and it may not carry a secret or an `env://` reference. Use `dependencies` for anything outside.

## 3. Edit, then lint

Turn the scaffold into a lint request, which takes only the files:

```bash
jq '{files}' scaffold.json > package.json
```

Edit the files in `package.json` until the manifest, schemas and workflows say what you mean. Then lint:

```bash
orion-cli provider lint -f package.json
```

```json
{
  "valid": true,
  "provider_id": "orders-api",
  "version": "1.0.0",
  "implementation": "workflow",
  "errors": []
}
```

Lint exits `2` when `valid` is `false`, so it gates a pipeline. It checks the manifest and schema bundle, not the package's behaviour. A schema that admits nothing and a workflow that answers the wrong shape both pass lint.

## 4. Pack

Packing produces the inventory and the content-addressed digest:

```bash
orion-cli provider pack -f package.json > packed.json
jq -r '.package_digest' packed.json
```

```text
sha256:4f1c…e9a2
```

The pack is deterministic: the same files produce the same inventory and the same digest, whatever the order they were written in. The digest is the package's identity. A change to any file is a new digest, and a new upload.

## 5. Upload and admit

An upload takes the packed files and can admit them in the same request:

```bash
jq '{files, admit: true}' packed.json > upload.json
orion-cli provider upload -f upload.json | jq -r '.package.digest'
```

```text
sha256:4f1c…e9a2
```

Admission is a separate decision on a node that will serve the package. If the upload did not admit it, admit by digest:

```bash
orion-cli provider admit sha256:4f1c…e9a2
```

`admit --lock <FILE>` verifies a provider lock before admitting, which is how a pipeline pins the dependency set. Read the result:

```bash
orion-cli provider get sha256:4f1c…e9a2
```

```text
orders-api v1.0.0
  Publisher:    com.example
  Kind:         workflow
  Tier:         standard
  State:        admitted · artifact active
  Digest:       sha256:4f1c…e9a2
  Admission:    orion.ext.admission/0 (admitted)
  Fingerprint:  sha256:… 
  Operations:   item.get
```

`State` tells admitted from not admitted from revoked, and reports a quarantined artifact separately. The admission fingerprint covers the package digest, the policy, the runtime ABI and the schema profile, so it changes when any of them does.

A tier above the node's edition is refused with `403 TIER_NOT_ENABLED`, naming the package's declared tier and the node's edition. That is the only edition gate on a package: the mechanism itself is the same on every node.

## 6. Revoke a package

Revocation is the emergency stop:

```bash
orion-cli provider revoke sha256:4f1c…e9a2 --reason "compromised build"
```

A revoked package cannot appear in a new plan or apply, and an existing deployment's next call over it fails with `CONNECTION_FORBIDDEN` at the provider. It is not a rollback: the deployment still names the package, so replace the deployment with one that does not, then revoke. The reason is recorded in the audit log.

## 7. Connect and bind

A connection revision names the package by digest. Save this as `connection.json`:

```json
{
  "revision": 1,
  "provider_digest": "sha256:4f1c…e9a2",
  "public_config": { "endpoint": "https://orders.example.com" },
  "secret_locators": { "api_key": "env://ORDERS_API_KEY" }
}
```

```bash
orion-cli connections create orders-api-prod -f connection.json
```

`public_config` must satisfy the package's `configuration_schema`; `secret_locators` places each credential at its path and never carries the value. A changed endpoint or credential is the next revision, never an edit of the first.

A package that declares `dependencies` binds each alias to a connection of the deployment:

```bash
orion-cli connections create orders-api-prod -f connection.json \
  --bind cache=sessions-cache
```

The alias must be one the package declares. The deployment member for the connection must require the bound connection, at the revision the call should use. A build refuses a binding its member does not require. A dependency's own dependencies are not carried, so a binding cannot chain.

A definition set cannot carry a package connection: a connector file names a direct driver with `provider`. A package connection is created against its digest, and a workflow reaches it in a deployment manifest that lists the provider and carries both members. [Deployment endpoints](../../reference/admin-api/deployments.md) specifies that manifest.

A workflow calls the operation through `connector_call`. For a workflow package, `kind` is `workflow` and `target_id` may be omitted, because the connection's revision already names the package:

```json
{
  "id": "lookup",
  "name": "Look up the order",
  "function": {
    "name": "connector_call",
    "input": {
      "connection": "orders-api-prod",
      "kind": "workflow",
      "operation": "item.get",
      "input": { "id": { "var": "data.order_id" } },
      "output": "data.order"
    }
  }
}
```

## 8. Discover what the target serves

Ask the node, not the package's README:

```bash
orion-cli deployments capabilities --output json > target-capabilities.json
jq '.provider_backends[] | {driver_id, kind, package_digest, operations}' \
  target-capabilities.json
```

Each backend carries its operations and their schemas. `provider list` shows every uploaded package with its tier, state and digest; `provider get` adds the operation names and dependency aliases. A package the node has not admitted does not appear as a backend.

## Verify

Probe the connection, then make one call through a workflow:

```bash
orion-cli connections probe orders-api-prod
```

A probe opens the connection and reports the stage that failed; it exits `2` when the backend is unhealthy. Then deploy a workflow that calls the operation and send one request. The trace names the provider code on failure. The data plane answers by class: `502` provider failure, `503` unavailable or rate-limited, `500` contract violation, `504` deadline.

## Next steps

- [`orion-cli provider`](../../reference/cli/orion-cli/provider.md): scaffold, lint, pack, upload, admit and revoke in full.
- [Deployment endpoints](../../reference/admin-api/deployments.md): the manifest that carries a package connection and the workflow that calls it.
- [`connector_call`](../../reference/functions/connector_call.md): the task's input, the hermetic rules and the failure classes.
- [Editions](../../concepts/editions.md): the tier ladder a package is admitted against.
- [Edition capability matrix](../../reference/editions.md): every gate and its refusal code.
