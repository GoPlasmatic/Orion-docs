<!-- description: A three-stage Orion delivery pipeline: prove the logic offline, compile it into a deployment, then plan and apply it to each environment. -->
<!-- type: guide -->
<!-- last_verified: 2026-10-25 -->

# CI/CD with deployments

A delivery pipeline for Orion has three jobs: prove the logic offline, plan the change against the target, then apply it. The offline checks need no server or secrets. Planning and applying need access to the target instance.

The definitions are the source of truth. [`orion-cli compile`](../../reference/cli/orion-cli/compile.md) turns them into a deployment directory, and `orion-cli deployments apply -d` deploys it. The same set compiles to the same bytes, so the deployment that passed staging is the one production gets. Each target assigns its own revision numbers.

A target that serves an edition driver, an edition provider or a channel traffic split needs the target's own capability file. Capture it once per job and pass it to the offline tools, so the gate validates what that target serves rather than Community's defaults.

## Before you start

Tested with Orion 2.0.0-rc.1. You need Git, `orion-cli`, a repository for the definitions, and a CI system that can run shell commands. Planning and applying also need network access to the target instance and an admin key from the CI secret store. The GitHub Actions YAML below is an example; adapt its secret names and installation policy for your provider.

## Lay out the repository

Keep the definitions in the repository and the compiled output out of it:

```text
services/payments/
  workflow.json              # source form: readable, reviewable, diffable
  channel.json
  connector.json
  tests/
    flags-high-value.case.json
```

`dist/` is a build output. Compile it in each job from the commit being deployed:

```bash
orion-cli compile services -o dist/deployment
```

## Gate the pull request

Everything here runs offline. No server, no database, no credentials, so it works on a fork's pull request:

```yaml
name: Validate

on:
  pull_request:
    paths:
      - 'services/**'

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install orion-cli
        run: |
          curl --proto '=https' --tlsv1.2 -LsSf \
            https://github.com/GoPlasmatic/Orion/releases/latest/download/orion-cli-installer.sh | sh
          echo "$HOME/.cargo/bin" >> "$GITHUB_PATH"

      - name: Check formatting
        run: orion-cli fmt --check services

      - name: Lint the definition set
        run: orion-cli lint services --deny-warnings

      - name: Advisory checks
        run: orion-cli clippy services

      - name: Run the offline regression suite
        run: orion-cli test services

      - name: Compile the deployment
        run: orion-cli compile services -o dist/deployment
```

Five checks catch five failure modes before review. `fmt` catches a file not in the house style. `lint` catches a definition the server would reject, or one that disagrees with the definitions beside it. `clippy`'s `deny` rules catch a definition that is valid but cannot behave as written; its warnings print and do not fail the step. `test` catches logic that changed behaviour, and `compile` catches a set that cannot become a deployment, such as a connector holding a literal credential.

These checks run offline against Community's built-in drivers. A set that uses an edition protocol is checked again in the deploy job, where the target's capability file is available.

Point `lint` at the directory, not at each file in turn. A per-file loop validates each workflow in isolation and cannot see whether it is consistent with the channels and connectors next to it. It misses a `channel_call` to a channel that exists nowhere, a task naming a connector of the wrong type, and two channels claiming one route. Set mode resolves those, and resolves any shared `$from` or `use` references at the same time.

> [!TIP]
> `orion-cli test` walks a directory for `*.case.json` files, so pointing it at `services` picks up every service's cases. Cases resolve `workflow` paths relative to themselves, so each service's tests live next to the workflow they test.

## Deploy to staging

`plan` commits nothing and prints what `apply` would change, so run it as a gate and `apply` as the action:

```yaml
name: Deploy to staging

on:
  push:
    branches: [main]
    paths: ['services/**']

jobs:
  deploy:
    runs-on: ubuntu-latest
    environment: staging
    env:
      ORION_API_KEY: ${{ secrets.ORION_API_KEY }}
      ORION_SERVER_URL: https://staging.orion.internal
    steps:
      - uses: actions/checkout@v4

      - name: Install orion-cli
        run: |
          curl --proto '=https' --tlsv1.2 -LsSf \
            https://github.com/GoPlasmatic/Orion/releases/latest/download/orion-cli-installer.sh | sh
          echo "$HOME/.cargo/bin" >> "$GITHUB_PATH"

      - name: Capture the target's capabilities
        run: orion-cli deployments capabilities --output json > target-capabilities.json

      - name: Check against the target
        run: |
          orion-cli lint services --deny-warnings --capabilities target-capabilities.json
          orion-cli clippy services --capabilities target-capabilities.json

      - name: Compile
        run: orion-cli compile services -o dist/deployment --capabilities target-capabilities.json

      - name: Plan
        run: orion-cli deployments plan -d dist/deployment

      - name: Apply
        run: orion-cli deployments apply -d dist/deployment --yes --reason "$GITHUB_SHA"
```

`apply` is safe to re-run. Its idempotency key is derived from the head and the manifest digest, so a retried job replays the first answer. A directory the head already deploys is reported as up to date and not applied again. The apply commits under compare-and-swap on the head, so two pipelines racing on one instance cannot both win.

The capture step writes the target's answer once. `lint` and `clippy` use it to check the set against the drivers and provider operations that target reports. `compile` uses it for those checks, for connection bindings, and to write `target.json` beside the deployment.

`plan|apply -d` then refuses an API revision mismatch or a channel protocol the target does not serve before it uploads anything.

## Promote to production

Promote on a tag, with a manual approval gate:

```yaml
name: Promote to production

on:
  push:
    tags: ['payments-v*']

jobs:
  promote:
    runs-on: ubuntu-latest
    environment: production      # attach required reviewers here
    env:
      ORION_API_KEY: ${{ secrets.ORION_API_KEY }}
      ORION_SERVER_URL: https://prod.orion.internal
    steps:
      - uses: actions/checkout@v4

      - name: Install orion-cli
        run: |
          curl --proto '=https' --tlsv1.2 -LsSf \
            https://github.com/GoPlasmatic/Orion/releases/latest/download/orion-cli-installer.sh | sh
          echo "$HOME/.cargo/bin" >> "$GITHUB_PATH"

      - name: Capture the target's capabilities
        run: orion-cli deployments capabilities --output json > target-capabilities.json

      - name: Check against the target
        run: |
          orion-cli lint services --deny-warnings --capabilities target-capabilities.json
          orion-cli clippy services --capabilities target-capabilities.json

      - name: Compile
        run: orion-cli compile services -o dist/deployment --capabilities target-capabilities.json

      - name: Plan
        run: orion-cli deployments plan -d dist/deployment

      - name: Apply
        run: orion-cli deployments apply -d dist/deployment --yes --reason "$GITHUB_REF_NAME"
```

GitHub's `environment` is where the approval lives. The job pauses before its first step until a reviewer approves, and the production key is scoped to that environment rather than to the repository.

## Target an edition <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

Available in Orion Standard and Enterprise only. A service needs a Standard or Enterprise target when it uses an `sse` or `kafka` channel, an edition provider, or a traffic split. A Community node refuses the definition at the upload or the plan, and a package above the node's tier is refused with `TIER_NOT_ENABLED`.

Make the edition a preflight, so the job stops before the plan uploads a definition:

```bash
orion-cli --require-edition standard deployments plan -d dist/deployment
```

An offline command warns that `--require-edition` has no effect, so keep the flag on a command that reads the server. The capability file is the same one `lint`, `clippy` and `compile` read through `--capabilities`.

## Verify

After an apply, read the head the instance serves, and in a cluster wait for every node to serve it:

```bash
curl -s https://prod.orion.internal/api/v2/admin/deployments | jq '{head_revision, applied_revision}'
orion-cli deployments wait 8 --target observed    # 8: the revision the apply committed
```

[Convergence](../../reference/admin-api/deployments.md#convergence) reports which nodes serve a revision.

## Roll back

A rollback commits an earlier revision's manifest as a new revision:

```bash
curl -s -X POST http://localhost:8080/api/v2/admin/deployments \
  -H 'Content-Type: application/json' \
  -d '{"idempotency_key": "rollback-7", "base_revision": 7, "rollback_to": 6}'
```

`orion-cli deployments rollback` takes the same values as flags. Re-running the production job against the previous tag works too: a member whose definition an earlier revision deployed takes that revision back.

## Keep secrets out of the deployment

A deployment carries secret references only. `compile` moves every `env://` or `vault://` string in a connector to the connection's `secret_locators`, and refuses a literal credential. Each environment resolves its own values, which is why the same deployment can go to staging and production unchanged.

## Next steps

- [Deployment endpoints](../../reference/admin-api/deployments.md): plan, apply, rollback and convergence.
- [Test a workflow offline](../author/testing.md): the gates the first job runs.
- [`orion-cli compile`](../../reference/cli/orion-cli/compile.md): what a deployment directory holds.
- [Audit logs](../../operate/run/audit-logs.md): every deployment is recorded with its reason.
