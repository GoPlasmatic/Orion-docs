<!-- description: Publish a custom connector provider package: scaffold its manifest and schemas, lint, pack, upload, admit, revoke, and bind connections. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Custom provider packages

In addition to the seven built-in [Enterprise connector providers](./providers.md), you can package and publish your own reusable connector providers using `orion-cli provider`. A provider package bundles a `connector.json` contract, configuration and credential JSON Schemas, per-operation input/output JSON Schemas, and the workflows or component that implement its operations.

## Package implementation kinds and edition tiers

When you scaffold a package with `orion-cli provider scaffold`, you choose an implementation `--kind` and an edition `--tier`:

| Implementation `--kind` | How operations execute | Runtime support |
|---|---|---|
| `workflow` | One hermetic dataflow workflow per operation (`workflows/<name>.json`). May call built-in dataflow tasks (`map`, `validation`, `filter`, `log`, `parse_json`, `parse_xml`, `publish_json`, `publish_xml`, `crypto`, `jwt_sign`, `jwt_verify`), plugins, and `connector_call` over declared `dependencies`. | Runs on all Orion nodes once admitted. |
| `rust` | Native Rust driver compiled into the server binary. | Runs on nodes that register a backend under the package's `driver_id`. |
| `wasm` | WebAssembly component in the declared WIT world. | Admitted and stored; execution requires a host runtime with the component ABI enabled. |

| Package `--tier` | Admitted on Community? | Admitted on Standard? | Admitted on Enterprise? |
|---|:---:|:---:|:---:|
| `community` (default) | Yes | Yes | Yes |
| `standard` <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | No (`403 TIER_NOT_ENABLED`) | Yes | Yes |
| `enterprise` <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> | No (`403 TIER_NOT_ENABLED`) | No (`403 TIER_NOT_ENABLED`) | Yes |

---

## 1. Scaffold the package

Generate a deterministic package skeleton for publisher `com.example` and provider `orders-api`:

```bash
orion-cli provider scaffold \
  --publisher com.example \
  --provider-id orders-api \
  --version 1.0.0 \
  --kind workflow \
  --tier standard > scaffold.json
```

Inspect the generated file map inside `scaffold.json`:

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

| File path | Purpose |
|---|---|
| `connector.json` | Package manifest declaring `id`, `version`, `implementation`, `configuration_schema`, `credential_schema`, `operations`, `dependencies`, and `payload_limits`. |
| `distribution.json` | Publisher namespace, admission policy requirements, and optional `tier` (`community`, `standard`, or `enterprise`). |
| `LICENSE` | Required package license text. |
| `schemas/config.public.json` | JSON Schema validating non-secret `public_config` fields on a connection revision. |
| `schemas/config.secret.json` | JSON Schema validating secret fields (`x-orion-sensitive`) supplied via `secret_locators`. |
| `schemas/<op>.input.json` | JSON Schema validating `connector_call` `input` before the operation runs. |
| `schemas/<op>.output.json` | JSON Schema validating the operation's answer before returning it to the caller workflow. |
| `workflows/<name>.json` | Entry workflow for each operation in a `workflow` package. |

---

## 2. Edit and lint the package bundle

Extract the `files` object into `package.json`, customize your schemas and operation workflows, and validate the bundle offline with `orion-cli provider lint`:

```bash
jq '{files}' scaffold.json > package.json
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

`orion-cli provider lint` exits with code `2` when `valid` is `false`, making it suitable as a fast CI gate.

---

## 3. Pack, upload, and admit the package

### Pack into a content-addressed bundle

`orion-cli provider pack` computes the deterministic `inventory.json` and `package_digest` (`sha256:...`):

```bash
orion-cli provider pack -f package.json > packed.json
jq -r '.package_digest' packed.json
```

```text
sha256:4f1c...e9a2
```

### Upload and admit on the target node

Upload the packed bundle (you can set `admit: true` to upload and admit in one step, or run `orion-cli provider admit` separately against a `provider.lock.json` file):

```bash
jq '{files, admit: true}' packed.json > upload.json
orion-cli provider upload -f upload.json | jq -r '.package.digest'

# Or admit explicitly by digest and verify against a lock file:
orion-cli provider admit sha256:4f1c...e9a2 --lock provider.lock.json
```

Inspect the admitted package state and operations:

```bash
orion-cli provider get sha256:4f1c...e9a2
```

```text
orders-api v1.0.0
  Publisher:    com.example
  Kind:         workflow
  Tier:         standard
  State:        admitted · artifact active
  Digest:       sha256:4f1c...e9a2
  Admission:    orion.ext.admission/0 (admitted)
  Fingerprint:  sha256:...
  Operations:   item.get
```

---

## 4. Create a connection revision and call the provider

### Create and probe a connection revision

Write `connection.json` referencing the admitted `provider_digest`, public configuration, and secret locators:

```json
{
  "revision": 1,
  "provider_digest": "sha256:4f1c...e9a2",
  "public_config": { "endpoint": "https://orders.example.com" },
  "secret_locators": { "api_key": "env://ORDERS_API_KEY" }
}
```

Create the connection revision (binding any declared dependencies with `--bind <alias>=<connection_id>`) and probe its health:

```bash
orion-cli connections create orders-api-prod -f connection.json
orion-cli connections probe orders-api-prod
```

### Invoke the operation with `connector_call`

In your workflow, call the operation using `connector_call` with `"kind": "workflow"`:

```json
{
  "id": "lookup_order",
  "name": "Look up order via custom provider package",
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

---

## 5. Emergency revocation

If a package version has a vulnerability or broken contract, revoke it by digest:

```bash
orion-cli provider revoke sha256:4f1c...e9a2 --reason "CVE-2026-4412"
```

Revocation is recorded in the audit log, blocks new deployment plans from referencing the digest, and causes any active deployment's next call to that package to fail closed with `CONNECTION_FORBIDDEN`. Replace the active deployment with one using a healthy package revision before or immediately after revoking.

## Next steps

- [Enterprise connector providers](./providers.md): Reference for the seven built-in `com.orion.*` providers.
- [CLI reference](../reference/cli.md): Full flag reference for `orion-cli provider` and `orion-cli connections`.
