<!-- description: Run Orion workflows on a schedule with cron channels: six-field expressions, time zones, misfire policies, singleton concurrency, and occurrence tracking. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Schedule jobs with cron

A `cron` channel triggers a workflow on a recurring schedule instead of waiting for an HTTP caller. Each scheduled execution instant is recorded in the state database as a durable **occurrence** before and after the workflow runs, giving you an auditable ledger of every scheduled run, misfire, and retry.

---

## 1. Define a cron channel

A cron channel sets `"protocol": "cron"`, `"channel_type": "async"`, and declares its schedule and fixed input payload inside `transport_config`:

```json
{
  "channel_id": "nightly-order-rollup",
  "name": "Nightly order rollup",
  "channel_type": "async",
  "protocol": "cron",
  "workflow_id": "order-rollup",
  "transport_config": {
    "schedule": "0 15 2 * * *",
    "timezone": "Asia/Kolkata",
    "payload": { "window": "previous_day" }
  },
  "config": {
    "timeout_ms": 1800000
  }
}
```

- **Six-field cron syntax**: The `schedule` expression always uses **six fields**: `second minute hour day-of-month month day-of-week` (for example, `"0 15 2 * * *"` runs at 02:15:00 every day). Five-field expressions are rejected at validation time so there is never ambiguity about whether the first field is seconds or minutes.
- **Time zones and DST**: `timezone` accepts any IANA time zone name (defaulting to `"UTC"`). During a daylight-saving spring-forward gap, skipped local times fire once at the first valid instant after the gap; during a fall-back overlap, an ambiguous local time fires once on the first pass.
- **No HTTP caller**: Cron channels do not register an HTTP route and cannot be called via `/api/v2/data/*` or `channel_call`. Caller-only guards (`auth`, `oauth2_login`, `origin_allow_list`, `rate_limit`, `deduplication`, `cache`, `request`, `response`) are rejected on cron channels.

---

## 2. Read the payload and schedule metadata in the workflow

The `transport_config.payload` object is passed into the workflow just like an HTTP request payload—your workflow reads `data.window` directly or parses it with `parse_json`.

In addition, Orion populates `metadata.trigger` with the occurrence details:

```json
{
  "type": "cron",
  "occurrence_id": "01a070e1-5e6a-7552-99d3-66dd70c1feff",
  "scheduled_for": "2026-09-04T20:45:00Z",
  "started_at": "2026-09-04T20:45:01.153Z",
  "timezone": "Asia/Kolkata",
  "attempt": 1
}
```

Use `metadata.trigger.scheduled_for` (rather than wall-clock `started_at`) when querying time windows in your database so that a delayed or retried run processes the exact window it was scheduled for.

---

## 3. Configure misfire recovery (`misfire_policy`)

If all Orion nodes are down when a scheduled time passes, that instant is a **misfire** (any delay beyond `cron.misfire_grace_secs`). Set `transport_config.misfire_policy` to control what happens when the server starts back up:

| Policy | Behavior After Downtime | Best For |
| :--- | :--- | :--- |
| **`latest`** *(default)* | Runs only the single most recent missed occurrence and records earlier misses as `skipped_misfire`. | Rollups, cache refreshes, and sync jobs where one run brings state up to date. |
| **`skip`** | Runs nothing until the next future scheduled time; records missed instants as `skipped_misfire`. | Time-sensitive snapshots (such as market-open checks) that are meaningless if run late. |
| **`catch_up`** | Runs missed occurrences oldest-first, up to `max_catch_up` (required when using `catch_up`). | Partitioned window jobs where each missed interval must be processed individually. |

---

## 4. Prevent overlapping runs (`concurrency`)

By default, cron occurrences can overlap if a run takes longer than the interval between schedules. To prevent overlap across the entire cluster, set `concurrency.policy` to `"forbid"`:

```json
{
  "transport_config": {
    "schedule": "0 */5 * * * *",
    "payload": {},
    "concurrency": {
      "policy": "forbid",
      "key": "order-pipeline"
    }
  }
}
```

- **Cluster-wide singleton**: With `"policy": "forbid"`, at most one occurrence with that `key` (defaulting to the channel ID) runs at a time across all nodes sharing the database. If an earlier run is still active when the next instant arrives, the new instant is recorded in the ledger as `skipped_singleton`.
- **Bounded worker slots (`slots`)**: To allow up to `N` concurrent runs (for example, 4 parallel queue-draining lanes), set `"slots": 4`. Each running occurrence claims a slot index from `0` to `3`, exposed to the workflow at `metadata.trigger.singleton_slot`.

> [!NOTE]
> Cron occurrences provide **at-least-once** execution. If a node crashes mid-run and its lease expires, another node will claim and re-run the occurrence (`attempt: 2`). Make your workflow's database writes idempotent (for example, upserting keyed on `metadata.trigger.scheduled_for`).

---

## 5. Trigger, inspect, retry, and cancel occurrences

You do not need to wait for the clock to test a cron channel. Trigger an immediate occurrence through the Admin API:

```bash
curl -sS -X POST http://localhost:8080/api/v2/admin/channels/nightly-order-rollup/trigger | jq
```

Use `orion-cli cron` to inspect schedule status, view occurrence history, retry failed occurrences, or cancel a stuck run:

```bash
orion-cli cron status
orion-cli cron list --channel-id nightly-order-rollup
orion-cli cron get <OCCURRENCE_ID>
orion-cli cron retry <OCCURRENCE_ID>
orion-cli cron cancel <OCCURRENCE_ID>
```

Each occurrence record includes its `trace_id`, so you can inspect the step-by-step execution with `orion-cli traces get <TRACE_ID>`.

## Next steps

- [Channel configuration schema](../reference/channels.md) — Full field table for `cron` `transport_config`.
- [Monitoring, metrics, and alerts](../operate/monitoring.md) — Alert on `orion_cron_schedule_lag_seconds` and `orion_cron_pending_occurrences`.
- [CLI reference](../reference/cli.md) — Flags and output options for `orion-cli cron`.
