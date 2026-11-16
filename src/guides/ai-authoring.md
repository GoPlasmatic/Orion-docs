<!-- description: Author, test, and deploy Orion services with AI coding assistants using the Orion Agent Skill, Claude Code, the LLM Prompt Pack, and worked examples. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Build with AI assistants

Because Orion services are declarative JSON files validated by deterministic offline tools (`orion-cli lint`, `clippy`, `dry-run`, `test`, and `compile`), AI coding assistants can author, verify, deploy, and roll back services safely under your review.

This guide covers three ways to work with AI assistants:
1. **The Orion Agent Skill** for CLI-capable coding agents (such as Claude Code, Cursor, or Antigravity).
2. **The Self-Contained Prompt Pack** for chat-only LLMs without shell access.
3. **Four Worked Prompt-to-Service Examples** taken directly from Orion's CI-tested example packages.

## Before you start

Tested with Orion 2.0.0-rc.1. You need `orion-server` running locally on `http://localhost:8080` and `orion-cli` installed on your `PATH` (see [Install and run Orion](../get-started/install.md)).

---

## 1. Set up the Orion agent skill (for Claude Code and CLI agents)

An **agent skill** is a markdown instruction bundle (`SKILL.md` plus reference files) that teaches an AI coding agent Orion's schemas, CLI workflow, and common pitfalls. The skill runs no background daemon and opens no network ports—instead, the agent executes `orion-cli` commands directly in your shell using your existing permissions and audit trail.

### Install the skill
Clone [`GoPlasmatic/Orion-skills`](https://github.com/GoPlasmatic/Orion-skills) and copy the `orion` skill folder into your project (or globally into `~/.claude/skills/`):

```bash
orion-cli config set-server http://localhost:8080
orion-cli health

# Per-project installation
mkdir -p .claude/skills
cp -r /path/to/Orion-skills/orion .claude/skills/
```

### Guided session with Claude Code
Start `claude` in your repository and describe the service you want:

> Create an Orion workflow called `order-triage` in `./definitions` that parses incoming orders, flags any order over $10,000 with an alert message, and sets `risk_level` to `"high"` above 10000 and `"normal"` otherwise. Dry-run it with a sample order before deploying. Then add a REST channel `POST /orders`, deploy both, and send a $25,000 test order.

The agent follows the exact same verification loop a human developer uses:

```bash
orion-cli lint definitions                                           # 1. Validate schema & references
orion-cli clippy definitions                                         # 2. Run static analysis
orion-cli dry-run -w definitions/order-triage.json -i sample.json    # 3. Verify output offline
orion-cli compile definitions -o dist/deployment                     # 4. Compile deployment bundle
orion-cli deployments plan -d dist/deployment                        # 5. Preview deployment diff
orion-cli deployments apply -d dist/deployment --yes                 # 6. Commit & hot-reload
orion-cli send orders -f big-order.json                              # 7. Call live endpoint
```

You can then ask the agent to inspect traces (`orion-cli traces list`), modify thresholds and re-plan, or roll back (`orion-cli deployments rollback`). To tag all admin actions made by an agent in the server's audit log, set `ORION_CHANGE_CONTEXT="agent=claude"`.

---

## 2. Prompt pack for any LLM (no shell required)

If you are using a browser-based LLM that cannot run `orion-cli` in a terminal, paste the following context block into your system prompt (or point your tool at [`https://docs.goplasmatic.io/llms.txt`](https://docs.goplasmatic.io/llms.txt) and [`https://docs.goplasmatic.io/llms-full.txt`](https://docs.goplasmatic.io/llms-full.txt)):

````text
You are working with Orion v2, a declarative services runtime.
Services are defined as JSON documents:
1. WORKFLOW (business logic pipeline of tasks + JSONLogic conditions)
2. CHANNEL (endpoint: rest, http, cron, or Standard+ kafka, graphql, sse)
3. CONNECTOR (named external connection: http, db [postgresql/mysql/sqlite], cache, or provider)

## Workflow JSON shape (in a definition set directory)
{
  "workflow_id": "orders-wf",
  "name": "Order Processing",
  "condition": true,
  "tasks": [
    {
      "id": "parse",
      "name": "Parse payload",
      "function": {
        "name": "parse_json",
        "input": { "source": "payload", "target": "order" }
      }
    },
    {
      "id": "flag",
      "name": "Flag high value",
      "condition": { ">": [{ "var": "data.order.total" }, 10000] },
      "function": {
        "name": "map",
        "input": {
          "mappings": [
            { "path": "data.order.flagged", "logic": true }
          ]
        }
      }
    }
  ]
}

## Channel JSON shape (in a definition set directory)
{
  "channel_id": "orders",
  "name": "orders",
  "channel_type": "sync",
  "protocol": "rest",
  "route_pattern": "/orders",
  "methods": ["POST"],
  "workflow_id": "orders-wf"
}

## Key rules
- Incoming request data starts in `data` (never write `{"var": "payload.x"}`; read `data.x` or `data.order.x` after `parse_json`).
- Built-in task functions: `parse_json`, `parse_xml`, `map`, `filter`, `validation` (set `"on_rejection": "halt"` to stop on invalid input), `log`, `publish_json`, `publish_xml`, `http_call`, `data_query`, `data_write` (portable SQL dialect, preferred), `db_read`, `db_write`, `cache_read`, `cache_write`, `cache_delete`, `cache_incr`, `cache_invalidate`, `connector_call`, `channel_call`, `crypto`, `jwt_sign`, `jwt_verify`.
- Fetch live function schemas from `GET /api/v2/admin/functions` (`orion-cli functions list --output json`).
- Never place literal secrets in connectors; use `"env://VAR_NAME"` or `"vault://..."`.
- Deploy with `orion-cli lint <dir>`, `orion-cli dry-run`, `orion-cli compile <dir> -o dist`, `orion-cli deployments plan -d dist`, and `orion-cli deployments apply -d dist`.
````

---

## 3. Worked examples: prompt to deployed service

Each example below shows a natural-language prompt and the exact CI-tested `workflow.json` from [`examples/packages/`](https://github.com/GoPlasmatic/Orion/tree/main/examples/packages).

### Example 1: Tiered order classification
**Prompt:**
> Create an Orion workflow for the `order-tiers` channel that parses the payload into `order` and assigns a tier based on `amount`: `vip` at 500 or above with a 15% discount, `premium` from 100 to 499.99 with a 5% discount, and `standard` below 100 with 0% discount.

**Generated `workflow.json` (`examples/packages/order-classification/workflow.json`):**

```json
{{#include ../../examples/packages/order-classification/workflow.json}}
```

**Deploy and test:**
```bash
orion-cli compile examples/packages/order-classification -o /tmp/order-classification
orion-cli deployments apply -d /tmp/order-classification --yes
orion-cli send order-tiers -d '{"amount": 750, "product": "Server Rack"}'
```

---

### Example 2: Range-based IoT sensor alerts
**Prompt:**
> Create an Orion workflow for the `sensors` channel that parses the payload into `reading` and sets `severity` from `temperature`: `critical` above 90 or below 0, `warning` from 70 to 90, and `normal` otherwise. Set an `alert` flag for `critical` and `warning`.

**Generated `workflow.json` (`examples/packages/iot-sensor-alert/workflow.json`):**

```json
{{#include ../../examples/packages/iot-sensor-alert/workflow.json}}
```

**Deploy and test:**
```bash
orion-cli compile examples/packages/iot-sensor-alert -o /tmp/iot-sensor-alert
orion-cli deployments apply -d /tmp/iot-sensor-alert --yes
orion-cli send sensors -d '{"sensor_id": "SENSOR-42", "temperature": 95}'
```

---

### Example 3: Null-safe webhook normalization
**Prompt:**
> Create an Orion workflow for the `webhooks` channel that parses incoming provider payloads into `event` and normalizes them into a standard schema, defaulting missing fields to `null`.

**Generated `workflow.json` (`examples/packages/webhook-transform/workflow.json`):**

```json
{{#include ../../examples/packages/webhook-transform/workflow.json}}
```

**Deploy and test:**
```bash
orion-cli compile examples/packages/webhook-transform -o /tmp/webhook-transform
orion-cli deployments apply -d /tmp/webhook-transform --yes
curl -sS -X POST http://localhost:8080/api/v2/data/webhooks \
  -H 'Content-Type: application/json' \
  --data @examples/packages/webhook-transform/request.json | jq
```

---

### Example 4: Progressive notification routing
**Prompt:**
> Create an Orion workflow for the `notifications` channel that parses the payload into `notification`, logs every message, marks email delivery for any severity except `low`, and marks SMS delivery only for `high` and `critical`.

**Generated `workflow.json` (`examples/packages/notification-routing/workflow.json`):**

```json
{{#include ../../examples/packages/notification-routing/workflow.json}}
```

**Deploy and test:**
```bash
orion-cli compile examples/packages/notification-routing -o /tmp/notification-routing
orion-cli deployments apply -d /tmp/notification-routing --yes
orion-cli send notifications -d '{"message": "Disk usage at 92%", "severity": "high"}'
```

## Next steps

- [Author workflows](./workflows.md) — Explore task groups, early exits, loops, and shared fragments.
- [Test and lint offline](./testing.md) — Write `*.case.json` regression suites to verify AI-authored workflows automatically.
- [CI/CD and deployment workflows](./ci-cd.md) — Gate all changes in GitHub Actions before deployment.
