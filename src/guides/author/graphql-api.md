<!-- description: Serve one GraphQL route over Orion workflows: GET and POST, one run per root field, document limits, selection projection and field errors. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Serve a GraphQL API

Available in Orion Standard and Enterprise only. A <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span> `graphql` channel serves one route and runs its workflow once per root field. This guide builds one with two root fields, limits and field errors.

## Before you start

You need:

- An `orion-ext` node at `http://localhost:8080`. A Community node refuses `graphql` at upload with `400 VALIDATION_ERROR` on `protocol`.
- A definition set, and admin access to the node.
- `orion-cli` on your `PATH`, and `curl` for the variables request.

The channel is `orders-graphql`, the route is `/graphql`, and the workflow is `orders-graphql`. [Channels](../../concepts/channels.md) explains how a channel and a workflow relate.

## 1. Write the channel

Save this as `orders-graphql.json` beside the workflow:

```json
{
  "channel_id": "orders-graphql",
  "name": "orders-graphql",
  "channel_type": "sync",
  "protocol": "graphql",
  "methods": ["GET", "POST"],
  "route_pattern": "/graphql",
  "workflow_id": "orders-graphql",
  "transport_config": {
    "max_depth": 10,
    "max_complexity": 200,
    "max_aliases": 30,
    "max_root_fields": 100
  }
}
```

A `graphql` channel is synchronous: the driver answers the request itself, and `/async` answers `405`. `methods` takes `GET`, `POST`, or both, each once. `route_pattern` is the one route the channel serves.

The `transport_config` limits are optional, and an absent block is every default:

| Field | Default | Range | Counts |
|---|---|---|---|
| `max_depth` | 10 | 1 to 32 | The deepest selected field, a root field being depth 1. |
| `max_complexity` | 200 | 1 to 10000 | Fields selected after fragments expand, `__typename` included. |
| `max_aliases` | 30 | 0 to 1000 | Aliased fields in the document. |
| `max_root_fields` | 100 | 1 to 10000 | Root fields of the selected operation, each one run. |

The limits are counted on the selected operation after fragments expand and `@skip`/`@include` apply. They bound what runs, not how the document is written.

## 2. Write the resolver

There is no schema. Each root field's run receives the field as `data`, with its arguments and selection:

| `data` key | Meaning |
|---|---|
| `field` | The field name. |
| `alias` | The alias, or `null`. |
| `arguments` | The field's arguments, with variables substituted. |
| `selection` | The selected child fields, in the same shape. |

The run's `metadata.trigger` carries `operation` (`query` or `mutation`), `operation_name`, `field` and `response_key`. The workflow answers with `data.result`, or with `data.error` for a field error. Save this as `orders-graphql-workflow.json`:

```json
{
  "workflow_id": "orders-graphql",
  "name": "Orders GraphQL resolver",
  "condition": true,
  "tasks": [
    {
      "id": "missing",
      "name": "Answer a missing id",
      "condition": {
        "and": [
          { "==": [{ "var": "metadata.trigger.field" }, "order"] },
          { "==": [{ "var": "data.arguments.id" }, null] }
        ]
      },
      "terminal": true,
      "function": {
        "name": "map",
        "input": {
          "mappings": [
            {
              "path": "data.error",
              "logic": { "message": "the order was not found", "code": "NOT_FOUND" }
            }
          ]
        }
      }
    },
    {
      "id": "order",
      "name": "Resolve one order",
      "condition": {
        "==": [{ "var": "metadata.trigger.field" }, "order"]
      },
      "function": {
        "name": "map",
        "input": {
          "mappings": [
            {
              "path": "data.result",
              "logic": {
                "id": { "var": "data.arguments.id" },
                "status": "confirmed",
                "total": 42.5
              }
            }
          ]
        }
      }
    },
    {
      "id": "orders",
      "name": "Resolve the order list",
      "condition": {
        "==": [{ "var": "metadata.trigger.field" }, "orders"]
      },
      "function": {
        "name": "map",
        "input": {
          "mappings": [
            {
              "path": "data.result",
              "logic": [
                { "id": "ord_1001", "status": "confirmed", "total": 42.5 },
                { "id": "ord_1002", "status": "pending", "total": 18 }
              ]
            }
          ]
        }
      }
    }
  ]
}
```

The fixed rows keep the example runnable without a database. Replace these `map` tasks with `data_query` calls against [`orders-db`](./connectors.md) when the resolver reads real orders; the run's `data.arguments` is the filter.

A query's root fields run concurrently, at most eight at a time. A mutation's root fields run in order. All runs of one request execute on one generation and share its request id, so their traces group.

## 3. Answer the three request forms

The channel reads the document from one of three places:

| Request | Form |
|---|---|
| `GET` | `?query=…&operationName=…&variables=<JSON>` |
| `POST` with `application/json` | `{"query": …, "operationName": …, "variables": {…}}` |
| `POST` with `application/graphql` | The body is the document; `operationName` and `variables` come from the query string. |

Any other POST media type answers `415 UNSUPPORTED_MEDIA_TYPE`. A method other than `GET` or `POST` answers `405` with `allow: GET, POST`. A mutation over `GET` also answers `405`, because a GET must be safe to repeat.

## 4. Read the projection

The driver cuts `data.result` to the field's selection. An object keeps only the selected fields, read by alias first and then by name. A list is projected item by item. A scalar is returned as it is.

That means a resolver may return more than the caller asked for without leaking it. This query selects `id` and `total`, so `status` never appears:

```graphql
{
  order(id: "ord_1001") {
    id
    total
  }
}
```

A fragment with a type condition applies when the object's `__typename` matches, or when the object has no `__typename`. An alias is read from the object before the field name, so `a: posts(first: 1) b: posts(first: 2)` can be answered apart.

## 5. Deploy

Compile against the target's capabilities, then apply:

```bash
orion-cli deployments capabilities --output json > target-capabilities.json
orion-cli compile ./definitions -o dist/deployment --capabilities target-capabilities.json
orion-cli deployments apply -d dist/deployment
```

Without `--capabilities`, `compile` does not know the target serves `graphql` and refuses the channel as an unknown protocol.

## Verify

Send a document as `application/graphql`. The CLI posts the body verbatim and prints the driver's JSON:

```bash
orion-cli send orders-graphql --content-type application/graphql \
  -d '{ order(id: "ord_1001") { id total } }'
```

```json
{
  "data": {
    "order": {
      "id": "ord_1001",
      "total": 42.5
    }
  }
}
```

Send a query with a variable, and two root fields in one request, with `curl`:

```bash
curl -s http://localhost:8080/api/v2/data/graphql \
  -H 'content-type: application/json' \
  -d '{"query": "query One($id: ID) { order(id: $id) { id } orders { id } }",
       "operationName": "One", "variables": {"id": "ord_1001"}}'
```

```json
{
  "data": {
    "order": { "id": "ord_1001" },
    "orders": [{ "id": "ord_1001" }, { "id": "ord_1002" }]
  }
}
```

Ask for the order without its id, and the `order` field fails while `orders` still answers:

```bash
curl -s http://localhost:8080/api/v2/data/graphql \
  -H 'content-type: application/json' \
  -d '{"query": "{ order { id } orders { id } }"}'
```

```json
{
  "data": {
    "order": null,
    "orders": [{ "id": "ord_1001" }, { "id": "ord_1002" }]
  },
  "errors": [
    {
      "message": "the order was not found",
      "locations": [{ "line": 1, "column": 3 }],
      "path": ["order"],
      "extensions": { "code": "NOT_FOUND" }
    }
  ]
}
```

A request refused before any field runs answers its status with `errors` and no `data`. A subscription is one such refusal:

```bash
orion-cli send orders-graphql --content-type application/graphql \
  -d 'subscription { order { id } }'
```

```json
{
  "errors": [
    {
      "message": "subscriptions are not served by a graphql channel; an sse channel streams",
      "locations": [{ "line": 1, "column": 1 }],
      "extensions": { "code": "GRAPHQL_VALIDATION_FAILED" }
    }
  ]
}
```

The request-level codes are `GRAPHQL_PARSE_FAILED`, `GRAPHQL_VALIDATION_FAILED`, `DEPTH_LIMIT_EXCEEDED`, `COMPLEXITY_LIMIT_EXCEEDED`, `ALIAS_LIMIT_EXCEEDED`, `ROOT_FIELD_LIMIT_EXCEEDED`, `BAD_REQUEST`, `UNSUPPORTED_MEDIA_TYPE` and `METHOD_NOT_ALLOWED`. A field error is `RESOLVER_ERROR`, `INTROSPECTION_UNAVAILABLE`, `SERVICE_UNAVAILABLE` or `INTERNAL_ERROR`, and a failed run answers `TIMEOUT`, `WORKFLOW_ERROR` or `ENGINE_ERROR`.

Read the request and field counters to confirm the traffic:

```bash
curl -s http://localhost:8080/metrics | grep orion_graphql_
```

`orion_graphql_requests_total` counts requests by channel, operation and outcome. `orion_graphql_fields_total` counts each field's run by outcome. Each run is a trace with mode `graphql`; `orion-cli traces list --channel orders-graphql --mode graphql` finds them.

> [!NOTE]
> There is no schema to introspect. `__schema` and `__type` are field errors with `INTROSPECTION_UNAVAILABLE`, while a root `__typename` answers `Query` or `Mutation` without a run. A client that generates types from introspection needs a schema file kept beside the workflow, not a request to this endpoint.

Backpressure holds one permit per request, however many fields it resolves. When the channel sets `deduplication`, it claims the request's idempotency key once, so a replayed mutation answers `409` instead of running twice. The `validation_logic` guard judges each run's `data`, which is the root field, never the raw document.

## Next steps

- [GraphQL transport](../../reference/channel-config/graphql.md): every limit, the run shape and the complete error table.
- [Configure a channel](./channels.md): rate limits, auth, backpressure and deduplication for the route.
- [Author a workflow](./workflows.md): branching, task groups and the functions a resolver can use.
- [Traces and async processing](../../operate/run/traces.md): reading the trace a field's run writes.
