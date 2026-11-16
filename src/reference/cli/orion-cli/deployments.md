<!-- description: Plan, apply, list, wait on and roll back immutable v2 deployments, read the target's capabilities, and deploy a compiled directory. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# `orion-cli deployments`

Plans, applies, lists, waits on and rolls back immutable v2 deployments. A canary, a channel split across workflow revisions, is available in Orion Standard and Enterprise only <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>.

## Synopsis

```bash
orion-cli deployments <capabilities|plan|apply|list|get|wait|rollback> [args] [flags]
```

## Description

Every subcommand reads the server's v2 admin API revision before it sends a request. The CLI refuses a revision it does not speak and names both values. `plan` and `apply` take a manifest from `-f`, a hand-written JSON file, or from `-d`, a directory [`orion-cli compile`](./compile.md) wrote.

An apply commits the next revision only while the head is still the base revision. Two pipelines that race on one instance cannot both win. The reviewed manifest digest binds the committed bytes to the plan a person reviewed.

## Subcommands

| Subcommand | Description |
|---|---|
| `capabilities` | Shows the node's edition, admission policy, drivers and provider backends as a human summary. |
| `plan` | Validates a candidate against a base revision and prints the diff and reviewed manifest digest. |
| `apply` | Commits a reviewed candidate under compare-and-swap and an idempotency key. |
| `list` | Lists committed revisions, newest first, without their manifests. |
| `get <revision>` | Fetches one committed revision with its full manifest. |
| `wait <revision>` | Polls convergence for a committed revision on this node or across the cluster. |
| `rollback` | Commits a historical revision's manifest as a new forward revision. |

`list` prints one row per revision: revision, member count, reason, author, creation time and manifest digest. A compacted revision shows `compacted` in the member cell.

`get` and every apply receipt print the server's JSON document. `wait` prints the target revision, its status, and the converged, pending and degraded nodes.

## Capabilities

`capabilities` renders the server's typed answer as a human summary. The first line names the edition, the v2 admin API revision and the admission policy. The lines below report the head revision and epoch, this node and its boot, and the reconciler's desired revision. They also carry retained revisions, scheduler load, the last error and every quarantined entity.

The summary then prints two tables. **Channel drivers** lists each protocol with sync, async and HTTP support, its guard kind and the guards its deliveries pass. It also says whether the driver publishes a transport schema. **Provider backends** lists each driver id with its kind, package digest, operation count and dependency names.

With `--output json` or `--output yaml`, the command prints the server's answer unchanged. With `--quiet`, it prints the edition alone. A capabilities file is what [`--capabilities`](./compile.md#options) takes on the offline authoring commands.

```bash
orion-cli deployments capabilities
```

```text
Orion Standard · v2 admin API 2 · admission policy orion.ext.admission/0
  Head:        revision 8 (epoch 12) · this node serves 8
  Node:        orion-1 (boot 3f2a)
  Reconciler:  desired revision 8 · configuration ok
  Scheduler:   0 in flight / 4
```

## Compiled directory

`-d <dir>` deploys a directory `orion-cli compile` wrote. The directory carries no revisions, because only the target knows them. The CLI reads `deployment.json`, the definition files under `definitions/`, the connection files under `connections/`, and `target.json` when it is present.

1. It checks `target.json` against the live node. A recorded API revision that differs from the node's is refused before anything is written. A directory compiled for a higher edition prints a warning, and its members are checked against the target's capabilities.
2. It refuses a channel whose protocol the target does not serve, and names the protocols the target does serve.
3. It uploads every definition. The digest the target answers must equal the file name, or the target canonicalises definitions differently and the directory must be recompiled with that target's version.
4. It reuses a stored connection revision with the same provider, public settings, locators and bindings, and otherwise creates the next revision.
5. It assigns each member a revision. A member the head deploys with the same definition keeps its revision. A member an earlier revision deployed with the same definition takes that revision back. Anything else takes one past the highest revision the entity ever had, or 1.
6. It plans against the head and applies with the plan's reviewed digest. The default idempotency key is derived from the head and the manifest digest, so a retried apply replays the first answer.

`--base-revision` refuses the plan or apply unless the head is that revision. `--reviewed-digest` must equal the plan's digest when it is given with `-d`, and it is required with `-f`. A directory the head already deploys is reported as up to date and is not applied again.

## Hand-written manifest

`-f <path>` reads a candidate manifest, and `-` reads it from standard input. Apply requires three reviewed values with `-f`: `--base-revision`, `--reviewed-digest` and `--idempotency-key`. A missing one is a usage error.

A manifest may give a channel member a `rollouts` list, which splits traffic across workflow revisions. Each interval names a workflow revision the channel also requires, and the intervals partition `[0, 100)` without a gap or an overlap. A split is <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>. A Community node refuses it with `403 TIER_NOT_ENABLED`, and the CLI refuses it before the request when the target reports a lower edition.

## Options

| Flag | Used by | Description |
|---|---|---|
| `-f, --file <path>` | `plan`, `apply` | Candidate manifest JSON, or `-` for standard input. Conflicts with `-d`. |
| `-d, --dir <dir>` | `plan`, `apply` | A compiled deployment directory. Conflicts with `-f`. |
| `--base-revision <n>` | `plan`, `apply` | Expected head revision. Required with `-f` for apply; `0` is the first deployment. |
| `--reviewed-digest <digest>` | `apply` | The reviewed manifest digest that `plan` printed. Required with `-f`. |
| `--idempotency-key <key>` | `apply`, `rollback` | Client-generated key that makes a retry replay the first answer. Required with `-f` and for `rollback`. |
| `--reason <text>` | `apply`, `rollback` | Audit reason recorded on the committed revision. Default: `cli apply`, or `rollback`. |
| `--wait-ms <ms>` | `apply` | Poll convergence after commit for this many milliseconds. Default: `0`, which does not wait. |
| `--wait-target <target>` | `apply` | Convergence target, `this-node` or `observed`. Default: `this-node`. |
| `--target <target>` | `wait` | Convergence target, `this-node` or `observed`. Default: `this-node`. |
| `--timeout-ms <ms>` | `wait` | Maximum wait time in milliseconds. Default: `10000`. |
| `--target-revision <n>` | `rollback` | Historical revision to restore. Required. |
| `--expected-head-revision <n>` | `rollback` | Expected current head for the compare-and-swap. Required. |
| `--limit <n>` | `list` | Revisions per page. Default: `50`; the server clamps it to `1000`. |
| `--cursor <c>` | `list` | The `next_cursor` a previous page printed. |

## Exit codes

| Code | Meaning |
|---|---|
| `0` | The command succeeded, or the polled revision converged. |
| `1` | A preflight, server or network refusal, or an apply that failed. |
| `2` | A usage error: an argument clap rejects, such as a missing required flag. |
| `3` | `wait` timed out, or `apply --wait-ms` elapsed before convergence. |

## Examples

Capture the target's answer and write a file the authoring tools accept:

```bash
orion-cli deployments capabilities --output json > target-capabilities.json
orion-cli compile ./definitions -o dist/deployment --capabilities target-capabilities.json
```

Plan and apply a compiled directory:

```bash
orion-cli deployments plan -d dist/deployment
orion-cli deployments apply -d dist/deployment --reason "release 2026.11"
```

Review a hand-written manifest, then apply exactly what was reviewed:

```bash
orion-cli deployments plan -f manifest.json --base-revision 7
orion-cli deployments apply -f manifest.json --base-revision 7 \
  --reviewed-digest sha256:4f9c… --idempotency-key orders-8
```

Wait for every node the cluster has observed to serve revision 8:

```bash
orion-cli deployments wait 8 --target observed --timeout-ms 120000
```

Restore revision 6 on top of head 9:

```bash
orion-cli deployments rollback --target-revision 6 --expected-head-revision 9 \
  --idempotency-key rollback-9-6 --reason "revert checkout change"
```

## Caveats

- A compiled-directory plan or apply prints its summary to standard error, so standard output carries only the plan or receipt in the chosen format.
- `apply -d` prints nothing and exits `0` when the head already deploys the directory. Under `--quiet` it prints the head revision.
- An apply that commits but fails to converge still reports its receipt. The convergence report and exit code `3` follow the commit.
- `rollback` always creates a new revision. The target revision stays in history unchanged.
- A canary requires the intervals to partition `[0, 100)`. A gap, an overlap or a revision missing from `requires` is refused before the request.
- The server remains the authority for every deployment rule. The CLI checks the same rules early so a refusal names the member.

## Related

- [Deployment endpoints](../../admin-api/deployments.md): the plan, apply, rollback and convergence contracts.
- [Deploy a compiled directory](../../admin-api/deployments.md#deploy-a-compiled-directory): the upload, revision and apply flow in full.
- [`orion-cli compile`](./compile.md): the command that writes a deployment directory.
- [Deployments](../../../concepts/deployments.md): the manifest, revision and convergence model.
- [CI/CD with deployments](../../../guides/patterns/ci-cd.md): a pipeline that plans and applies each environment.
