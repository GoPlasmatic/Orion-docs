<!-- description: Reference for a GraphQL channel: the one GET and POST route, each root field as a workflow run, the document limits, the guards and the errors. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# GraphQL transport

Available in Orion Standard and Enterprise only: a Community node refuses `graphql` at upload with `400 VALIDATION_ERROR` on `protocol`. A GraphQL channel serves one route and runs the workflow once per root field, with no schema between the request and the workflow. See [Channels](../../concepts/channels.md) for the model and [Configure a channel](../../guides/author/channels.md) for the surrounding keys.

## Synopsis

A complete channel with the default limits:

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
  "config": { "timeout_ms": 5000 }
}
```

## Description

A GraphQL channel reads a document, selects one operation, and resolves each of its root fields with one workflow run. The answer is the specification's `{"data": …, "errors": […]}` shape, and the channel is synchronous: `/async` answers `405`.

There is no schema. A field is a name, an optional alias, JSON arguments and its own selection, and the workflow supplies the value. That makes the channel a GraphQL endpoint over workflows rather than a GraphQL server over a data model.

## Request forms

| Request | Form |
|---|---|
| `GET` | The document arrives in `query`, with `operationName` and `variables` (JSON) in the query string. |
| `POST` with `application/json` | The body holds `query`, `operationName`, `variables` and `extensions`; only `query` is required. |
| `POST` with `application/graphql` | The body is the document; `operationName` and `variables` come from the query string. |

A method other than `GET` or `POST` answers `405 METHOD_NOT_ALLOWED` with `allow: GET, POST`. Any other POST media type answers `415 UNSUPPORTED_MEDIA_TYPE`. A body that is not a JSON object answers `400 BAD_REQUEST`.

## Fields <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `channel_type` | string | yes | — | `sync`; the driver answers the request itself, and `/async` answers `405`. |
| `methods` | array of strings | yes | — | `GET`, `POST`, or both, each once; no other method is accepted. |
| `route_pattern` | string | yes | — | The one route the channel serves, for example `/graphql`. |
| `transport_config.max_depth` | integer | no | `10` | Deepest selected field, a root field being depth 1; 1 to 32. |
| `transport_config.max_complexity` | integer | no | `200` | Fields selected after fragment expansion, `__typename` included; 1 to 10000. |
| `transport_config.max_aliases` | integer | no | `30` | Aliased fields in the document; 0 to 1000. |
| `transport_config.max_root_fields` | integer | no | `100` | Root fields in the selected operation, each one run; 1 to 10000. |

The limits are counted on the selected operation after fragments expand and `@skip`/`@include` apply. They bound what runs, not how the document is written. An absent `transport_config` is the same as every default.

## Workflow runs

A run's `data` is the root field as the driver renders it:

| Key | Meaning |
|---|---|
| `field` | The field name. |
| `alias` | The alias, or `null` when the field has none. |
| `arguments` | The field's arguments as JSON, with variables substituted. |
| `selection` | The selected child fields, in the same shape. |
| `on` | The fragment's type condition, when a fragment selected the field. |

The run's `metadata.trigger` carries `operation` (`query` or `mutation`), `operation_name`, `field` and `response_key`.

The workflow answers a field with `data.result`, which the driver projects onto the field's selection. An object keeps its selected fields, read by alias first and then by name. A list is projected item by item, and a scalar is the value as it is.

To answer a field error instead, the workflow sets `data.error` to a message string or to `{"message": …, "code": …}`.

A query's root fields run concurrently, at most eight at a time. A mutation's root fields run in order, and a mutation over `GET` answers `405`. All runs of one request execute on one generation and share its request id, so their traces group.

## Guards <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

- `validation_logic`: judges each root field's `data` before its run. A refused field is `null`, with its error beside the others.
- `deduplication`: claims the request's idempotency key once, so a replay answers `409` before the driver reads the body.
- `backpressure`: one permit per request, however many root fields it resolves.
- `rate_limit`, `auth` and `origin_allow_list`: always on for the request, and each runs once.

The upload refuses `cache`, `request`, `response` and `oauth2_login` on a `graphql` channel, because the driver owns the body and the answer. See [Guards by ingress](./guards-by-ingress.md) for the full matrix.

## Response

An executed request answers `200` with `{"data": {...}}`, and with `"errors": [...]` beside it when any field failed. Each field error carries `message`, `locations`, `path` and `extensions.code`.

A request refused before any field ran answers its status with `{"errors": [...]}` and no `data` key. That is how the specification answers a request it could not execute.

## Errors

A request refusal carries one entry in `errors`, with the code in `extensions.code`.

| Code | Status | When |
|---|---|---|
| `GRAPHQL_PARSE_FAILED` | 400 | The document does not parse. |
| `GRAPHQL_VALIDATION_FAILED` | 400 | The document names no single operation, names an unknown fragment, spreads a fragment into itself, is a subscription, uses an undeclared variable, or has conflicting root fields. |
| `DEPTH_LIMIT_EXCEEDED` | 400 | The document selects deeper than `max_depth`. |
| `COMPLEXITY_LIMIT_EXCEEDED` | 400 | The document selects more fields than `max_complexity`. |
| `ALIAS_LIMIT_EXCEEDED` | 400 | The document uses more aliases than `max_aliases`. |
| `ROOT_FIELD_LIMIT_EXCEEDED` | 400 | The operation has more root fields than `max_root_fields`. |
| `BAD_REQUEST` | 400 | The document is missing, the body is not a JSON object, or `variables` is malformed. |
| `UNSUPPORTED_MEDIA_TYPE` | 415 | A POST body is neither `application/json` nor `application/graphql`. |
| `METHOD_NOT_ALLOWED` | 405 | The method is not GET or POST, or a mutation arrives over GET. |

A field error also answers `200`, with `null` in `data` for that field and one entry in `errors`.

| Code | When |
|---|---|
| `RESOLVER_ERROR` | `data.error` carried a string, or carried no code. |
| `INTROSPECTION_UNAVAILABLE` | The field is `__schema` or `__type`; there is no schema. |
| `SERVICE_UNAVAILABLE` | The run was deferred or the channel is unavailable. |
| `INTERNAL_ERROR` | The run ended in no recognised outcome. |

A failed run answers its field with `TIMEOUT`, `WORKFLOW_ERROR` or `ENGINE_ERROR`, and a fixed message that never carries the engine's own text.

## Examples

An illustrative query with two root fields, which run concurrently:

```bash
curl -s http://localhost:8080/graphql \
  -H 'content-type: application/json' \
  -d '{"query": "query { order(id: \"7\") { id total } customer(id: \"7\") { name } }"}'
```

```json
{
  "data": {
    "order": { "id": "7", "total": 42.5 },
    "customer": { "name": "Ada" }
  }
}
```

A mutation with one field answered and one refused:

```bash
curl -s http://localhost:8080/graphql \
  -H 'content-type: application/json' \
  -d '{"query": "mutation { placeOrder(id: \"7\") { id } cancelOrder(id: \"7\") { id } }"}'
```

```json
{
  "data": {
    "placeOrder": { "id": "7" },
    "cancelOrder": null
  },
  "errors": [
    {
      "message": "the order is already shipped",
      "locations": [{ "line": 1, "column": 39 }],
      "path": ["cancelOrder"],
      "extensions": { "code": "CONFLICT" }
    }
  ]
}
```

## Caveats

- There is no schema and no introspection: `__schema` and `__type` are field errors, while a root `__typename` answers `Query` or `Mutation` without a run.
- A subscription is refused with `GRAPHQL_VALIDATION_FAILED`; this driver serves queries and mutations only.
- Two root fields under one response key must agree on name and arguments, or the request is refused.
- A field error is one entry beside the others, not a failed request: the rest of `data` is still answered.
- The `validation_logic` guard judges each run's `data`, never the raw document or the request body.
- `/async` is refused with `405`, because the driver answers the request itself.
- Aliases are how one field is asked for many times, and each aliased root field is its own run.

## Related

- [Channels](../../concepts/channels.md): the channel model, its ingresses and its guards.
- [Configure a channel](../../guides/author/channels.md): the guards as a walkthrough.
- [Guards by ingress](./guards-by-ingress.md): the hook rows and the per-request guards.
- [Data API](../data-api.md): the request path, headers and error envelopes beside the route.
- [Edition capability matrix](../editions.md): the gate and its refusal.
