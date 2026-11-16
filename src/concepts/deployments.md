<!-- description: Understand how Orion v2 deployments work: content-addressed definitions, immutable revisions, plan and apply, atomic reload, rollback, and convergence. -->
<!-- type: concept -->
<!-- last_verified: 2026-11-16 -->

# Deployments and revisions

In Orion v2, you never mutate individual channels, workflows, or connectors in place on a running server. Instead, an Orion node serves a **deployment**: a single manifest listing every channel, workflow, connection, and plugin that should be active, committed as an immutable, numbered revision.

---

## Why Orion uses whole-estate deployments

When a runtime updates endpoints and workflows one at a time, configuration drifts:
- Updating a workflow before its connector is ready causes runtime errors.
- Rolling back a broken release requires remembering which of twenty individual entities changed.
- Replicas in a cluster can end up serving different combinations of routes and logic.

Orion solves this by treating the entire set of services on a node like a Git commit:
- **One active head**: The latest committed deployment revision is the **head**. Every node in a cluster reconciles to that exact head revision.
- **Full replacement, never partial merge**: Applying a deployment replaces the previous head completely. Any channel or workflow omitted from the new deployment stops serving.
- **Instant, append-only rollback**: Rolling back commits an earlier revision's manifest again as the next numbered revision.

---

## How a deployment is built and applied

```orion-diagram
{
  "direction": "LR",
  "nodes": [
    { "id": "set", "label": "Definition set", "sublabel": "files in Git", "type": "service" },
    { "id": "dir", "label": "Deployment directory", "sublabel": "orion-cli compile", "type": "ci" },
    { "id": "head", "label": "Head revision", "sublabel": "deployments apply -d", "type": "datastore" },
    { "id": "gen", "label": "Serving generation", "sublabel": "atomic ArcSwap", "type": "channel" }
  ],
  "edges": [
    { "from": "set", "to": "dir" },
    { "from": "dir", "to": "head" },
    { "from": "head", "to": "gen" }
  ]
}
```

### 1. Content-addressed definitions
A **definition** is the canonical JSON document of a workflow, channel, or WebAssembly plugin (`POST /api/v2/admin/definitions`). When uploaded, Orion validates the document, serializes it to canonical bytes, and stores it by its `sha256:` digest.
- A definition has no ID, version number, or active status on its own—uploading a definition does not expose any route.
- Two identical definitions always produce the same `sha256:` digest, making uploads idempotent.

### 2. Immutable connection revisions
A **connection revision** (`POST /api/v2/admin/connections/{id}/revisions`) stores the driver, `public_config`, `secret_locators`, and optional dependency `bindings` for an external system. Each `(connection_id, revision)` pair is immutable and identified in the manifest by its `configuration_digest`.

### 3. The deployment manifest
A **manifest** binds definitions and connection revisions to named entities (`kind`, `id`, `revision`) and declares the dependency graph between them via `requires`:

```json
{
  "schema_version": 2,
  "providers": [],
  "members": [
    {
      "entity": { "kind": "workflow", "id": "order-summary", "revision": 1 },
      "definition_digest": "sha256:87bf52da…"
    },
    {
      "entity": { "kind": "channel", "id": "order-summary", "revision": 1 },
      "definition_digest": "sha256:e627767c…",
      "requires": [
        { "kind": "workflow", "id": "order-summary", "revision": 1 }
      ]
    }
  ]
}
```

Every dependency in `requires` must be present in the same manifest:
- A **channel** requires the workflow(s) it routes to, plus any connection its transport driver uses (such as a Kafka connection).
- A **workflow** requires every connection its tasks reference (`http_call`, `data_query`, `connector_call`, etc.).
- A **connection** over a provider with dependencies (such as an ONNX model connection bound to an S3 `artifact_store`) requires the target connection.

---

## Plan, apply, and rollback

Three operations manage the deployment head:

| Operation | CLI Command | HTTP Endpoint | What It Does |
| :--- | :--- | :--- | :--- |
| **Plan** | `orion-cli deployments plan -d <dir>` | `POST /api/v2/admin/deployment-plans` | Compiles the candidate manifest against `base_revision`, returns the added/updated/removed diff and `reviewed_manifest_digest`, and commits nothing. |
| **Apply** | `orion-cli deployments apply -d <dir>` | `POST /api/v2/admin/deployments` | Verifies that the server head still matches `base_revision` and `reviewed_manifest_digest`, commits the next revision, and triggers a hot reload. |
| **Rollback** | `orion-cli deployments rollback` | `POST /api/v2/admin/deployments` (`rollback_to`) | Re-validates an earlier revision's manifest and commits it as the next head revision. |

### Pre-commit build check
Both `plan` and `apply` compile the candidate manifest the same way a serving node builds it. If two channels claim the same route, a workflow calls a missing plugin function, or a dependency is missing from `requires`, Orion rejects the plan/apply with `400 DEPLOYMENT_WOULD_NOT_BUILD` before anything is committed.

### Compare-and-swap protection
`apply` requires both the `base_revision` (the current head revision) and the `reviewed_manifest_digest` returned by `plan`. If another operator or CI job deploys a revision in the meantime, `apply` fails with `409 DEPLOYMENT_HEAD_CONFLICT` so two pipelines can never silently overwrite each other.

---

## Convergence and quarantine

Once a deployment revision is committed, each node's background reconciler builds a new runtime generation from the head manifest:

- **Convergence**: You can check whether a single node or all cluster nodes have loaded a revision using `orion-cli deployments wait <REV> --target observed` (or `GET /api/v2/admin/deployments/{revision}/convergence`).
- **Failed builds vs. quarantine**:
  - If a node cannot build the new generation at all, it keeps serving its previous generation and marks `engine_reload` as `degraded` on `/health`.
  - If a single member fails to load at runtime (for example, a channel whose `env://` auth secret is unset on that node, or a workflow using a plugin when `plugins.enabled = false`), Orion **quarantines** that specific channel or workflow, refuses requests to it, reports the issue in `orion-cli engine status`, and serves the rest of the deployment normally.

## Next steps

- [CI/CD and deployment workflows](../guides/ci-cd.md) — Set up automated PR gates and staged deployments with `orion-cli`.
- [Admin and Data REST API](../reference/api.md) — Full HTTP specification for definitions, connections, plans, and deployments.
- [CLI reference](../reference/cli.md) — Every flag for `orion-cli compile` and `orion-cli deployments`.
