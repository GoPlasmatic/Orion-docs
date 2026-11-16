<!-- description: Serve a GraphQL API over HTTP from an Orion channel by resolving root Query and Mutation fields with workflows and enforcing document limits. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# GraphQL APIs

<span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

A `graphql` channel exposes a synchronous GraphQL HTTP endpoint (`GET` and `POST`) backed directly by an Orion workflow. When a client sends a GraphQL query or mutation, Orion parses the document, enforces depth and complexity limits, runs the bound workflow once for each requested root field, and projects each run's `data.result` onto the client's selection set.

## How GraphQL channels work

Unlike traditional GraphQL servers that require maintaining a separate schema file, an Orion `graphql` channel is **schema-less at the transport layer**:
1. **Document parsing and limits**: Orion parses the incoming GraphQL document, expands fragments, evaluates `@skip` and `@include` directives, and checks `max_depth`, `max_complexity`, `max_aliases`, and `max_root_fields` before executing any workflow.
2. **One workflow run per root field**:
   - **Queries**: Root fields in a `query` operation execute concurrently (up to 8 in parallel).
   - **Mutations**: Root fields in a `mutation` operation execute sequentially in document order. Mutations sent over HTTP `GET` are rejected with `405 METHOD_NOT_ALLOWED`.
3. **Result projection**: Your workflow writes the resolved object, array, or scalar to `data.result` (or sets `data.error` to return a field-level error). Orion automatically filters `data.result` to keep only the child fields requested in the client's selection set.

## Configure a `graphql` channel

Create a channel with `"channel_type": "sync"` and `"protocol": "graphql"`:

```json
{
  "channel_id": "order-graph",
  "name": "order-graph",
  "channel_type": "sync",
  "protocol": "graphql",
  "methods": ["GET", "POST"],
  "route_pattern": "/graphql",
  "workflow_id": "order-resolver",
  "transport_config": {
    "max_depth": 10,
    "max_complexity": 200,
    "max_aliases": 30,
    "max_root_fields": 100
  },
  "config": {
    "timeout_ms": 5000
  }
}
```

### Channel and `transport_config` fields

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `channel_type` | string | yes | — | Must be `"sync"`. Async queueing (`/async`) is not supported and returns `405`. |
| `methods` | array of strings | yes | — | `["GET"]`, `["POST"]`, or `["GET", "POST"]`. |
| `route_pattern` | string | yes | — | HTTP route served by the channel (for example, `"/graphql"`). |
| `transport_config.max_depth` | integer | no | `10` | Maximum selection nesting depth (`1` to `32`, where a root field is depth `1`). |
| `transport_config.max_complexity` | integer | no | `200` | Maximum total selected fields after fragment expansion (`1` to `10000`). |
| `transport_config.max_aliases` | integer | no | `30` | Maximum aliased fields allowed in the document (`0` to `1000`). |
| `transport_config.max_root_fields` | integer | no | `100` | Maximum root fields in the selected operation (`1` to `10000`). |

---

## Write a resolver workflow

For each root field in the operation, Orion invokes the channel's `workflow_id` with the parsed field in `data` and operation context in `metadata.trigger`.

### Input context (`data` and `metadata.trigger`)

For the query `query GetOrder { currentOrder: order(id: "7") { id total } }`, the workflow receives:

```json
{
  "data": {
    "field": "order",
    "alias": "currentOrder",
    "arguments": { "id": "7" },
    "selection": [
      { "field": "id", "alias": null, "arguments": {}, "selection": [], "on": null },
      { "field": "total", "alias": null, "arguments": {}, "selection": [], "on": null }
    ],
    "on": null
  },
  "metadata": {
    "trigger": {
      "operation": "query",
      "operation_name": "GetOrder",
      "field": "order",
      "response_key": "currentOrder"
    }
  }
}
```

### Returning `data.result` or `data.error`

Dispatch on `data.field` (for example, using task `condition` expressions or [`channel_call`](../reference/functions.md)) and write the resolved value to `data.result`:

```json
{
  "id": "order-resolver",
  "name": "Resolve Order GraphQL Fields",
  "tasks": [
    {
      "id": "resolve_order",
      "name": "Return order details",
      "condition": { "==": [{ "var": "data.field" }, "order"] },
      "function": {
        "name": "map",
        "input": {
          "mappings": [
            {
              "path": "data.result",
              "logic": {
                "id": { "var": "data.arguments.id" },
                "total": 42.5,
                "internal_notes": "Stripped automatically unless selected by client"
              }
            }
          ]
        }
      }
    },
    {
      "id": "unknown_field",
      "name": "Report unknown root field",
      "condition": { "!=": [{ "var": "data.field" }, "order"] },
      "function": {
        "name": "map",
        "input": {
          "mappings": [
            {
              "path": "data.error",
              "logic": {
                "message": "Unknown GraphQL field",
                "code": "RESOLVER_ERROR"
              }
            }
          ]
        }
      }
    }
  ]
}
```

---

## Send GraphQL requests

A `graphql` channel accepts three standard HTTP request formats:
- **`POST` with `Content-Type: application/json`**: JSON object containing `query` (required), `operationName`, `variables`, and `extensions`.
- **`POST` with `Content-Type: application/graphql`**: Raw GraphQL document in the request body; `operationName` and `variables` in the query string.
- **`GET`**: `query`, `operationName`, and JSON-encoded `variables` in the URL query string (for `query` operations only).

```bash
curl -s http://localhost:8080/api/v2/data/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "query { order(id: \"7\") { id total } }"}' | jq .
```

```json
{
  "data": {
    "order": {
      "id": "7",
      "total": 42.5
    }
  }
}
```

## Error codes and partial responses

### Request-level errors (before any workflow runs)

When a request fails parsing or exceeds a document limit, Orion returns the HTTP status below with `{"errors": [{"message": "...", "extensions": {"code": "..."}}]}` and no `data` key:

| `extensions.code` | HTTP status | Cause |
|---|---|---|
| `GRAPHQL_PARSE_FAILED` | `400` | Document contains a GraphQL syntax error. |
| `GRAPHQL_VALIDATION_FAILED` | `400` | Missing or ambiguous operation name, unknown or cyclic fragment, undeclared variable, conflicting root fields, or a `subscription` operation. |
| `DEPTH_LIMIT_EXCEEDED` | `400` | Selection depth exceeds `transport_config.max_depth`. |
| `COMPLEXITY_LIMIT_EXCEEDED` | `400` | Total selected fields exceed `transport_config.max_complexity`. |
| `ALIAS_LIMIT_EXCEEDED` | `400` | Aliased fields exceed `transport_config.max_aliases`. |
| `ROOT_FIELD_LIMIT_EXCEEDED` | `400` | Root fields exceed `transport_config.max_root_fields`. |
| `BAD_REQUEST` | `400` | Missing `query` field, malformed JSON body, or invalid `variables`. |
| `METHOD_NOT_ALLOWED` | `405` | HTTP method is not `GET`/`POST`, or a `mutation` was sent over `GET`. |
| `UNSUPPORTED_MEDIA_TYPE` | `415` | `POST` content type is neither `application/json` nor `application/graphql`. |

### Field-level errors (during execution)

When an individual root field sets `data.error`, fails `validation_logic`, or encounters a workflow error, Orion returns HTTP `200 OK` with `null` at that field's key in `data` and appends an entry to `errors` while still returning all other resolved fields:

| `extensions.code` | Cause |
|---|---|
| Custom code or `RESOLVER_ERROR` | Workflow set `data.error` to `{"message": "...", "code": "..."}` or a plain string. |
| `INTROSPECTION_UNAVAILABLE` | Client requested `__schema` or `__type` (root `__typename` resolves to `"Query"` or `"Mutation"`). |
| `TIMEOUT`, `WORKFLOW_ERROR`, `ENGINE_ERROR` | The field's workflow run timed out or failed. |
| `SERVICE_UNAVAILABLE` | The run was deferred or the channel is unavailable. |

## Next steps

- [Configure channels](../guides/channels.md): Add `auth`, `rate_limit`, `origin_allow_list`, and `validation_logic` guards to your GraphQL channel.
- [Author workflows](../guides/workflows.md): Compose sub-workflows with `channel_call` for modular GraphQL resolvers.
