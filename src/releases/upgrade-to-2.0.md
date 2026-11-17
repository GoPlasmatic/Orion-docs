<!-- description: Breaking changes and migration guide for upgrading from Orion 1.x to Orion 2.0: /api/v2 routes, immutable deployments, cursor pagination, and edition changes. -->
<!-- type: migration -->
<!-- last_verified: 2026-11-17 -->

# Upgrade to Orion 2.0

Orion 2.0 is a major release that replaces mutable in-database entity editing with immutable, content-addressed deployments, moves all HTTP routes under `/api/v2/`, standardizes cursor pagination across every list endpoint, and introduces the Community, Standard, and Enterprise edition split.

Because Orion 2.0 uses a new state database schema and deployment model, **there is no in-place upgrade from 1.x**. You deploy Orion 2.0 alongside your 1.x environment with a fresh database, compile and apply your definitions from source files, and cut traffic over.

See [Backups, upgrades, and troubleshooting](../operate/maintenance.md) for the step-by-step blue/green cutover procedure. This page catalogs every breaking change between Orion 1.x and 2.0 so you can update your definitions, client code, dashboards, and automation.

## Before you start

- **Upgrade `orion-server` and `orion-cli` together**: `orion-cli` checks `api_version` on the server and refuses to run against a mismatched major version.
- **Start with a fresh 2.0 database**: Orion 2.0 refuses to boot against a 1.x database schema. Historical execution traces, DLQ rows, audit logs, and cron ledgers remain in your 1.x database.
- **Export definitions to files before retiring 1.x**: Orion 2.0 deploys workflows, channels, and connections from a directory of source files compiled via `orion-cli compile`. If your 1.x definitions exist only in the 1.x database, export them to JSON files while your 1.x instance is still running.

## Summary of breaking changes

| Area | Orion 1.x | Orion 2.0 |
|---|---|---|
| **HTTP API prefix** | `/api/v1/*` | `/api/v2/*` (`/api/v1/*` returns `404 Not Found`) |
| **Deployment model** | Mutable `POST`/`PUT` per workflow or channel | Immutable deployment revisions compiled via `orion-cli compile` and promoted with `orion-cli deployments apply -d` |
| **Definition parsing** | Tolerated legacy aliases (`validate`, `response_path`, task-level `continue_on_error`) | Strict schema validation: unknown keys, `halt_on`, task-level `continue_on_error`, and `validate` (use `validation`) are rejected |
| **Pagination** | Offset pagination (`{data, total, limit, offset}`, `?offset=`, `?sort_by=`) | Cursor pagination (`{data, limit, next_cursor?, total?}`, `?cursor=`, `?include_total=true`) |
| **Timestamps** | Zone-less strings (`2026-01-01T00:00:00`) | RFC 3339 UTC strings ending with `Z` (`2026-01-01T00:00:00Z`) |
| **Default bind host** | `0.0.0.0:8080` | `127.0.0.1:8080` (Docker images and Helm charts still set `0.0.0.0`) |
| **`environment` values** | Any string (only `prod*` triggered production checks) | Must be `development`, `staging`, `test`, or `production` (or `dev`, `stage`, `prod`); unknown values fail startup |
| **CLI/server check** | Permissive | Strict `api_version` match |

## Cursor pagination on all list endpoints

Every list endpoint in `/api/v2/admin/*` now returns a single cursor-paginated envelope:

```json
{
  "data": [],
  "limit": 50,
  "next_cursor": "eyJpZCI6IjAxOTI...",
  "total": 137
}
```

- **`next_cursor`**: Present when additional pages exist. Pass it back unmodified as `?cursor=<next_cursor>`.
- **`total`**: Returned only when you pass `?include_total=true`. Computing exact totals requires a full scan on PostgreSQL and MySQL, so it is opt-in.
- **Removed query parameters**: `?offset=`, `?sort_by=`, and `?sort_order=` have been removed. Lists are always ordered newest-first.
- **Newly paginated endpoints**: `GET /api/v2/admin/connections`, `GET /api/v2/admin/connections/{id}/revisions`, and `GET /api/v2/admin/connector-packages` now return paged envelopes instead of unpaged arrays.
- **CLI flags**: In `orion-cli`, `--offset`, `--sort-by`, and `--sort-order` are replaced by `--limit`, `--cursor`, and `--include-total`.

## API and definition shape changes

| Surface | Change in 2.0 |
|---|---|
| **Trace DLQ entries** | `payload_json` and `metadata_json` (escaped JSON strings) are now `payload` and `metadata` (native JSON objects). |
| **`POST /api/v2/admin/backups`** | Returns `201 Created` (previously `200 OK`). In backup list items, `modified_at` is renamed to `created_at`. |
| **Channel `protocol` and `channel_type`** | Must be lowercase (`"rest"`, `"sync"`, `"async"`, `"cron"`). Uppercase or mixed-case spellings (`"REST"`, `"Sync"`) are rejected. |
| **`http_call` and `channel_call` output field** | Must use `"output"` instead of the legacy `"response_path"` alias. |
| **Validation function name** | Must use `"validation"` instead of the legacy `"validate"` alias. |

## Tightened server defaults

- **Loopback bind address by default**: `server.host` now defaults to `127.0.0.1` instead of `0.0.0.0`. Official Docker images and the Helm chart set `ORION_SERVER__HOST=0.0.0.0`, so container deployments are unaffected. If you run the `orion-server` binary directly on a VM behind a reverse proxy on another host, set `server.host = "0.0.0.0"` in `config.toml` or `ORION_SERVER__HOST=0.0.0.0`.
- **Validated `environment` name**: `ORION_ENVIRONMENT` (or `environment` in `config.toml`) must be one of `development` (`dev`), `staging` (`stage`), `test`, or `production` (`prod`). Previously, a custom name like `live` or `prd` was silently treated as non-production and skipped production security checks. Unrecognized environment names now fail fast at startup.
- **Read-only admin API keys**: Keys in `admin_auth.read_only_api_keys` are now permitted to call side-effect-free `POST` endpoints (`POST /api/v2/admin/deployment-plans`, `POST /api/v2/admin/authoring/scaffold`, `POST /api/v2/admin/authoring/lint`, and `POST /api/v2/admin/authoring/pack`).

## Observability and CLI output changes

- **`orion_http_request_duration_seconds` label change**: The histogram now uses `status_class` (`2xx`, `4xx`, `5xx`) instead of `status` to cap label cardinality. Exact HTTP status codes remain on the counter `orion_http_requests_total`. Update any Prometheus queries or Grafana panels that filter the latency histogram by `status`.
- **Structured CLI error output**: When you pass `--output json` or `--output yaml` to `orion-cli`, server errors are formatted as structured JSON or YAML on `stderr`, preserving `code`, `request_id`, and `fields`.

## Edition split and capabilities moved to Standard & Enterprise

Orion 2.0 separates the open-source Community server (`orion-server`) from the licensed Standard and Enterprise server (`orion-ext`). Several capabilities that were built into the 1.x server are now part of `orion-ext` and use the provider connection and task model (`provider_call` / `provider_stream`):

| Orion 1.x feature | Orion 2.0 replacement | Edition |
|---|---|---|
| `smtp` connector and `smtp_send` | `com.orion.smtp` provider (`operation: "send"`) | Standard & Enterprise |
| `storage` connector (`storage_get`, `storage_put`, etc.) | `com.orion.storage` provider | Standard & Enterprise |
| `mongodb` connector | `com.orion.mongodb` provider | Standard & Enterprise |
| `es` (Elasticsearch) connector | `com.orion.elasticsearch` provider | Standard & Enterprise |
| `cache` connector with `backend: "redis"` | `com.orion.redis` provider | Standard & Enterprise |
| ONNX model registrations (`model_infer`) | `com.orion.models` provider | Standard & Enterprise |
| Kafka channel ingestion and `publish_kafka` | `kafka` channel driver and `com.orion.kafka` provider | Standard & Enterprise |
| Multi-node clustering (`cluster.enabled = true`) | Redis-backed `SharedStateStore` (`[extensions.redis]` in `orion-ext`) | Standard & Enterprise |
| Canary traffic splitting on channels | Channel member `rollouts` in `orion-ext` | Standard & Enterprise |

For migration details on each enterprise capability, see the [Standard & Enterprise overview](../enterprise/overview.md) and [Enterprise connector providers](../enterprise/providers.md).

## Related pages

- [Backups, upgrades, and troubleshooting](../operate/maintenance.md) — blue/green migration procedure
- [Versioning and support policy](./versioning-policy.md) — 2.x compatibility guarantees
- [Orion 1.x release history](./history-1.x.md) — historical upgrade notes for Orion 1.0 through 1.8
