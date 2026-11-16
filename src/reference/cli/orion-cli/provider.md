<!-- description: Author, lint, pack, upload, admit and revoke v2 connector provider packages, with scaffold flags for kind and edition tier. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# `orion-cli provider`

Authors, lints, packs, uploads, admits and revokes v2 connector provider packages. A `standard` or `enterprise` package tier is available in Orion Standard and Enterprise only <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>.

## Synopsis

```bash
orion-cli provider list [--limit <n>] [--cursor <c>]
orion-cli provider get <digest>
orion-cli provider upload -f <file>
orion-cli provider admit <digest> [--lock <file>]
orion-cli provider revoke <digest> [--reason <text>]
orion-cli provider scaffold --publisher <ns> --provider-id <id> [--version <semver>]
                                  [--kind rust|workflow|wasm] [--tier community|standard|enterprise]
orion-cli provider lint -f <file>
orion-cli provider pack -f <file>
```

## Description

A provider package is a connector backend a connection revision runs on. Its candidate identity is its publisher, provider id and version. Its content address is the `sha256:` digest over the packed bytes, and every command after packing takes that digest.

The package declares a tier in `distribution.json`. A node admits a package whose tier is at or below its own edition: Community admits `community`, Standard adds `standard`, and Enterprise adds `enterprise`. `provider scaffold --tier` writes the tier, and omitting the flag keeps today's `community` default.

`scaffold` answers the candidate's publisher, provider id, version, kind and the digest its files pack to, with the files themselves. `pack` answers the same digest for a bundle, and `upload` answers the digest the target stores.

Every command reads the server's v2 admin API revision first and refuses one this CLI does not speak.

## Subcommands

| Subcommand | Description |
|---|---|
| `list` | Lists uploaded and admitted packages with their tier, state and digest. |
| `get <digest>` | Shows one package with its admission and declared operations. |
| `upload -f <file>` | Uploads a verified package archive or bundle and answers its digest. |
| `admit <digest>` | Admits an uploaded package, optionally against a provider lock. |
| `revoke <digest>` | Revokes an admitted package, optionally recording a reason. |
| `scaffold` | Generates a deterministic package skeleton for a publisher, provider and kind. |
| `lint -f <file>` | Validates a package manifest and schema bundle without uploading it. |
| `pack -f <file>` | Packs a bundle into a deterministic content-addressed archive. |

`list` prints one row per package: publisher, provider id, version, kind, tier, state and digest. The state cell reads `not admitted`, `admitted` or `revoked`, and appends the artifact state when it is not `active`.

`get` prints the package's identity, kind, tier, state and digest, then its admission policy and fingerprint, its operations and its dependency names. Both views keep the server's answer under `--output json` or `--output yaml`.

## Options

| Flag | Used by | Description |
|---|---|---|
| `--publisher <namespace>` | `scaffold` | Publisher namespace, for example `org.acme`. Required. |
| `--provider-id <id>` | `scaffold` | Provider identifier within the publisher, for example `orders-api`. Required. |
| `--version <semver>` | `scaffold` | Package version written into the manifest. Default: `1.0.0`. |
| `--kind <kind>` | `scaffold` | Implementation kind: `rust`, `workflow` or `wasm`. Default: `rust`. |
| `--tier <tier>` | `scaffold` | Declared tier: `community`, `standard` or `enterprise`. A `standard` or `enterprise` value is <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>. Omit for `community`. |
| `--lock <file>` | `admit` | A provider lock the admission must verify against, or `-` for standard input. |
| `--reason <text>` | `revoke` | Reason recorded for the emergency revocation. |
| `-f, --file <path>` | `upload`, `lint`, `pack` | Package upload payload, bundle or archive JSON, or `-` for standard input. |
| `--limit <n>` | `list` | Page size. Default: `50`; the maximum is `1000`. |
| `--cursor <c>` | `list` | Resume after this position: the previous page's `next_cursor`. |

A `rust` package names a native provider compiled into the node, and runs only where a backend registers under its provider id. A `workflow` package runs on every node once admitted: one dataflow workflow per operation, held to the hermetic rules. A `wasm` package can be admitted, but no Community node executes it yet.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | The command succeeded. |
| `1` | A network, server or package error, including a refused admission or revocation. |
| `2` | `lint` found the bundle invalid, or an argument clap rejects. |

## Examples

Scaffold a Standard package, pack it and admit what the target stores:

```bash
orion-cli provider scaffold --publisher org.acme --provider-id orders-api \
  --version 1.4.0 --kind workflow --tier standard
orion-cli provider pack -f package-bundle.json
orion-cli provider upload -f package-upload.json
orion-cli provider admit sha256:8c1d… --lock provider.lock.json
```

Read the admitted packages and one package's operations:

```bash
orion-cli provider list
orion-cli provider get sha256:8c1d…
```

Revoke a package that must stop serving at once:

```bash
orion-cli provider revoke sha256:8c1d… --reason "CVE-2026-4412"
```

## Caveats

- `pack` is deterministic. The same bundle yields the same bytes and the same digest, so a digest is safe to pin in a lock.
- `lint` exits `2` on an invalid bundle and prints the server's findings. It does not upload anything.
- Admission is by digest, not by publisher and version. A repacked package with one changed byte is a different candidate.
- Revocation is immediate and audited. A deployment that requires the revoked provider is refused at its next plan.
- `--tier` is a declaration, not a grant. The node's admission policy decides whether the package is admitted.

## Related

- [Connector types](../../connectors/index.md): the built-in drivers a connection revision can run on.
- [Editions](../../editions.md): the package tier ladder and the refusal codes.
- [Deployment endpoints](../../admin-api/deployments.md#connections): the connection revisions that run on a provider.
- [Admin API](../../admin-api/index.md): the contract every provider command calls.
- [`orion-cli connections`](./connections.md): creating the revision that names a provider digest.
