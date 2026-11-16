<!-- description: Route percentages of live channel traffic across workflow revisions for canary releases, progressive rollouts, and instant rollbacks. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Canary rollouts and traffic splitting

<span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

Orion Standard and Enterprise allow a channel member in a deployment manifest to split live traffic across multiple workflow revisions. Because every workflow revision referenced in a deployment is compiled and loaded into memory when the revision activates, shifting traffic from `90% / 10%` to `100%`—or rolling back to the previous revision—happens atomically with no cold-start latency.

## How `rollouts` intervals work

In a deployment manifest (`manifest.json`), a channel member can include a `rollouts` array that partitions the `[0, 100)` bucket range into contiguous, non-overlapping integer intervals:

```json
{
  "entity": { "kind": "channel", "id": "orders", "revision": 1 },
  "definition_digest": "sha256:3b18...",
  "requires": [
    { "kind": "workflow", "id": "order-enrich", "revision": 1 },
    { "kind": "workflow", "id": "order-enrich", "revision": 2 }
  ],
  "rollouts": [
    { "workflow_revision": 1, "start": 0, "end": 90 },
    { "workflow_revision": 2, "start": 90, "end": 100 }
  ]
}
```

### Rollout invariants

- **Contiguous `[0, 100)` partition**: The first interval must start at `0`, each subsequent interval's `start` must equal the previous interval's `end`, and the final interval must end at `100` (`start < end`). Gaps or overlapping ranges are rejected before commit.
- **Explicit `requires` entries**: Every `workflow_revision` named in `rollouts` must also appear in the channel member's `requires` list and exist as a workflow member in the same deployment manifest.
- **Edition check**: A channel with two or more `rollouts` intervals requires Orion Standard or Enterprise. A Community node rejects multi-interval rollouts at plan, apply, and rollback with `403 TIER_NOT_ENABLED`.

### Deterministic sticky routing vs random bucket selection

For each incoming request, Orion selects a bucket in `0..99`:
- **With `X-Orion-Routing-Key`**: When the caller sends the `X-Orion-Routing-Key` HTTP header (for example, a customer ID, tenant ID, or session token), Orion hashes the header value deterministically into `0..99`. A given routing key always resolves to the same bucket for the duration of the rollout.
- **Without `X-Orion-Routing-Key`**: Orion chooses a bucket in `0..99` uniformly at random per request.

Every synchronous data-plane response includes the `X-Orion-Workflow-Revision` header indicating which workflow revision executed the request, and records that revision in the execution trace and metrics.

---

## Step-by-step: Shift 10% of traffic to a canary workflow

### 1. Upload the candidate workflow definition

Upload the new workflow definition (`workflow-v2.json`) to obtain its content digest:

```bash
curl -s -X POST http://localhost:8080/api/v2/admin/definitions \
  -H "X-Admin-Key: $ORION_ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d @workflow-v2.json | jq -r '.digest'
```

### 2. Write and plan the canary manifest

Reference both revision `1` (`90%`, buckets `0..90`) and revision `2` (`10%`, buckets `90..100`) in `manifest-canary.json`, then run `orion-cli deployments plan`:

<!-- tabs:start -->
#### **CLI**
```bash
orion-cli deployments plan -f manifest-canary.json --base-revision 2
```

#### **curl**
```bash
curl -s -X POST http://localhost:8080/api/v2/admin/deployment-plans \
  -H "X-Admin-Key: $ORION_ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d "{\"base_revision\": 2, \"manifest\": $(cat manifest-canary.json)}" | jq .
```
<!-- tabs:end -->

The plan output confirms the added workflow member (`order-enrich` revision `2`) and returns the `reviewed_manifest_digest`.

### 3. Apply the canary deployment

Commit the plan using compare-and-swap on `--base-revision` and the `--reviewed-digest` from the plan:

<!-- tabs:start -->
#### **CLI**
```bash
orion-cli deployments apply -f manifest-canary.json \
  --base-revision 2 \
  --reviewed-digest sha256:4f9c... \
  --idempotency-key orders-canary-10 \
  --reason "10% canary for order-enrich v2"
```

#### **curl**
```bash
curl -s -X POST http://localhost:8080/api/v2/admin/deployments \
  -H "X-Admin-Key: $ORION_ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d "{
    \"idempotency_key\": \"orders-canary-10\",
    \"base_revision\": 2,
    \"reason\": \"10% canary for order-enrich v2\",
    \"reviewed_manifest_digest\": \"sha256:4f9c...\",
    \"manifest\": $(cat manifest-canary.json)
  }" | jq .
```
<!-- tabs:end -->

### 4. Verify routing with `X-Orion-Routing-Key`

Send a request with `X-Orion-Routing-Key` and inspect the `X-Orion-Workflow-Revision` response header:

```bash
curl -i -s -X POST http://localhost:8080/api/v2/data/orders \
  -H "Content-Type: application/json" \
  -H "X-Orion-Routing-Key: customer-42" \
  -d '{"order_id": "ord_1001", "total": 49.99}'
```

```http
HTTP/1.1 200 OK
x-orion-workflow-revision: 2
content-type: application/json
```

### 5. Promote to 100% or roll back

- **Promote to 100%**: Update the manifest so the channel requires only workflow revision `2` (and drop the `rollouts` array or set `[{"workflow_revision": 2, "start": 0, "end": 100}]`), plan, and apply.
- **Instant rollback**: Restore the previous deployment revision (`2`) as a new forward revision (`4`):

<!-- tabs:start -->
#### **CLI**
```bash
orion-cli deployments rollback \
  --target-revision 2 \
  --expected-head-revision 3 \
  --idempotency-key orders-rollback-2 \
  --reason "revert order-enrich v2 canary"
```

#### **curl**
```bash
curl -s -X POST http://localhost:8080/api/v2/admin/deployments \
  -H "X-Admin-Key: $ORION_ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "idempotency_key": "orders-rollback-2",
    "base_revision": 3,
    "rollback_to": 2,
    "reason": "revert order-enrich v2 canary"
  }' | jq .
```
<!-- tabs:end -->

## Next steps

- [Deployments](../concepts/deployments.md): Understand compare-and-swap revisions, epochs, and rollout invariants.
- [CI/CD and GitOps](../guides/ci-cd.md): Automate deployment plans and rollbacks in your CI pipeline.
