# Orion Documentation Style Guide

This guide defines the terminology, spelling, and formatting conventions used across Orion documentation. It complements [`DOCUMENTATION_STANDARD.md`](./DOCUMENTATION_STANDARD.md) and [`CONTRIBUTING-DOCS.md`](./CONTRIBUTING-DOCS.md), and serves as the source for our Vale rules in `styles/`.

---

## 1. Spelling

Use **Standard American English** throughout prose and headings:

<!-- vale off -->
- **behavior**, **color**, **favor**, **honor** (not *behaviour*, *colour*, *favour*, *honour*)
- **license**, **defense** (not *licence*, *defence*)
- **catalog**, **dialog** (not *catalogue*, *dialogue*)
- **normalize**, **serialize**, **initialize**, **authorize**, **organize**, **analyze** (not *-ise*)
- **canceled**, **labeled**, **modeled**, **signaled** (not *cancelled*, *labelled*, *modelled*, *signalled*)
- **artifact**, **center** (not *artefact*, *centre*)

### Wire-name exception
Always preserve the exact wire spelling inside code spans and configuration blocks:
- In `orion-ext`, the TOML configuration section is spelled `[extensions.licence]`. Write `"the license section (`[extensions.licence]`)"`—using American English in prose and the exact key in code font.
<!-- vale on -->
- In the Kafka example, if a topic is named `orders.cancelled`, keep the exact topic name in code font.

---

## 2. Product Nouns and Capitalization

### Capitalized proper nouns
Capitalize product and company names:
- **Orion**, **Orion Ext**, **Orion Console**, **Plasmatic**
- **Orion Community**, **Orion Standard**, **Orion Enterprise**

### Lowercase domain nouns
Keep Orion's declarative building blocks lowercase in prose:
- **channel**, **workflow**, **connector**, **connection**, **plugin**, **provider**, **package**, **deployment**, **manifest**, **task**, **task group**, **step**, **trace**, **occurrence**
- Write *"a channel"* and *"the workflow"*, never *"a Channel"* or *"the Workflow"*.

---

## 3. Preferred Developer Terminology

Use clear, standard engineering terms:

| Use | Avoid | Why |
| :--- | :--- | :--- |
| **definition** or **definition file** | spec | A definition is the workflow, channel, or plugin JSON document. Use `manifest` specifically for a deployment's member list or a plugin's `plugin.toml`. |
| **definition set** | config directory | A directory of workflow, channel, connector, and shared fragment files compiled by `orion-cli compile`. |
| **deploy** / **apply a deployment** | activate, publish, promote | In Orion v2, `orion-cli deployments apply` commits a deployment manifest as the new head revision. |
| **returns `409`** or **responds with `409`** | answers `409`, throws a 409 | "Returns" and "responds with" are natural developer English. Format HTTP status codes in code font (`200`, `400`, `409`, `503`). |
| **dry run** (noun), **`dry-run`** (CLI command/flag) | test run | Matches `orion-cli dry-run` and `--dry-run`. |
| **Admin API** (`/api/v2/admin/*`) | management API | Matches the route prefix and configuration sections (`[admin_auth]`). |
| **Data API** or **data plane** (`/api/v2/data/*`) | runtime API, invocation API | Pairs cleanly with Admin API / control plane. |
| **quarantined** | disabled, parked, deactivated | When a channel or workflow fails to load on a node, Orion marks it as quarantined and rejects requests to it while keeping the rest of the deployment running. |
| **ingress** | entry point | Collective term for how work enters Orion: sync HTTP, `/async` HTTP, `cron` schedules, in-process `channel_call`, or extension drivers (`kafka`, `graphql`, `sse`). |
| **task function** (or **function**) | action, operator | Reserve **operator** for JSONLogic operators (`==`, `cat`, `var`, `map`). |
| **the engine** | the core | Refers specifically to the workflow execution engine (`dataflow-rs`) inside Orion. |

---

## 4. Code Spans and Wire Identifiers

Format anything a developer types, configures, or reads from terminal output in inline code (`backticks`):
- **File names and paths**: `workflow.json`, `orion.toml`, `dist/deployment`
- **JSON/TOML keys and values**: `workflow_id`, `on_rejection: "halt"`, `[extensions.redis]`, `channel_type: "sync"`
- **Task functions and JSONLogic operators**: `parse_json`, `data_query`, `connector_call`, `var`, `cat`
- **CLI commands and flags**: `orion-cli compile`, `--capabilities`, `--require-edition`
- **Environment variables and secret locators**: `ORION_SERVER__PORT`, `env://ORDERS_DB_URL`, `vault://secret/data/db#url`
- **HTTP routes, methods, and status codes**: `POST /api/v2/admin/deployments`, `GET /readyz`, `409`, `TIER_NOT_ENABLED`

When referring to the concept in prose, use plain words (*"the workflow ID"*); when naming the JSON field, use code font (`` `workflow_id` ``).

---

## 5. Running Examples

Use consistent, recognizable examples across the documentation so readers don't have to re-learn a new domain on every page:

| Scenario | Workflow ID | Channel ID / Route | Repository Path |
| :--- | :--- | :--- | :--- |
| **Quickstart / High-value orders** | `quickstart-orders` / `high-value-order` | `orders` (`POST /orders`) / `high-value-orders` (`POST /high-value-orders`) | `examples/packages/high-value-order/` |
| **First service tutorial** | `order-summary` | `order-summary` (`POST /order-summary`) | `examples/packages/order-summary/` |
| **Database-backed API** | `record-order` (using connector `orders-db`) | `record-order` (`POST /record-order`) | `examples/packages/postgres-orders/` |
| **Secondary example** | `notification-routing` | `notifications` (`POST /notifications`) | `examples/packages/notification-routing/` |
