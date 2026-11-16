<!-- description: Which channel guard runs on which ingress: the four core ingresses, the driver and hook rows of the edition protocols, and the order guards apply in. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# Guards by ingress

Which of a channel's guards run on which ingress, and the order they apply in. A Community channel is reachable on up to four ingresses, and each guard runs on the ones marked Yes. The edition protocols add a driver ingress and two HTTP hooks.

## Description

| Guard | HTTP sync | HTTP `/async` | `channel_call` | Cron |
|---|---|---|---|---|
| `rate_limit` | Yes | Yes | Yes | No |
| `principal_rate_limit` | Yes | Yes | No | No |
| `auth` | Yes | Yes | No | No |
| `origin_allow_list` | Yes | Yes | No | No |
| `validation_logic` | Yes | Yes | Yes | Yes |
| `deduplication` | Yes | Yes | No | No |
| `cache` | Yes | No | No | No |
| `backpressure` | Yes | Yes | Yes | Yes |
| `oauth2_login` | Yes | No | No | No |
| `timeout_ms` | Yes | Yes¹ | Yes | Yes |

¹ Clamped to a transport ceiling. See [Timeouts](./timeout_ms.md). Every No cell is deliberate; the owning section below states why.

The Cron column is mostly No because that ingress has no caller, which is a stronger statement than `channel_call`'s "the caller authenticated elsewhere". A cron channel is refused these keys at authoring time rather than storing them and ignoring them — see [Cron transport](./cron.md).

## Edition protocols <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

A `kafka` channel runs its own listener, and the `graphql` and `sse` drivers answer their requests on the data plane. The two hook columns include the guards that are always on for a hooked request.

| Guard | Kafka driver ingress | GraphQL HTTP hook | SSE HTTP hook |
|---|---|---|---|
| `rate_limit` | Yes, keyed on the topic | Yes, once per request | Yes, once per request |
| `principal_rate_limit` | No | Yes, once per request | Yes, once per request |
| `auth` | No | Yes, once per request | Yes, once per request |
| `origin_allow_list` | No | Yes, once per request | Yes, once per request |
| `validation_logic` | Yes, per record | Yes, per root field | Yes, per run |
| `deduplication` | Yes, per record key | Yes, once per request | No |
| `cache` | No | No | No |
| `backpressure` | Yes, per delivery | Yes, one permit per request | Yes, one permit held for the stream |
| `oauth2_login` | No | No | No |

Every No cell is refused at upload, so a channel cannot configure a guard its ingress never applies.

A Kafka delivery is authenticated by the transport's connection, not per record. The upload therefore refuses `auth`, `principal_rate_limit`, `origin_allow_list`, `cache`, `request`, `response` and `oauth2_login` on a `kafka` channel. What remains is the four guards its driver declares. See [Kafka transport](./kafka.md).

A hook's own declaration adds only validation, deduplication and backpressure. Authentication, the principal limit, the origin allow-list and the rate limit are always on for its requests. They run once per request, so the hook declares none of them. The cache and the response are not the hook's: the driver owns the answer, and the upload refuses `cache`, `request`, `response` and `oauth2_login`. See [GraphQL transport](./graphql.md) and [SSE transport](./sse.md).

On a driver ingress, guards run in this order: rate limit → validation → deduplication → backpressure. On a hooked request they run rate limit → auth → origin allow-list → deduplication → backpressure. Each run a hook makes then passes validation, if its driver declares it.

## Order of application

Guards run in a fixed order: rate limit → auth → origin allow-list → validation → deduplication → cache lookup → backpressure → `oauth2_login`. Four consequences follow.

- A rejected request (bad origin, failed validation) still consumes a rate-limit token.
- A replayed idempotency key answers `409` before the cache is consulted.
- A cache hit never consumes a backpressure permit, and a request shed by backpressure releases its idempotency claim.
- `oauth2_login` runs last, *after* the backpressure permit, because its callback leg makes a round trip to the identity provider. That call must be bounded by the channel's concurrency cap. The consequence is that `validation_logic` sees the request and not the grant — the grant is what the workflow is for.

## Related

- [Channels](../../concepts/channels.md): what a channel is, and its ingresses.
- [Configure a channel](../../guides/author/channels.md): the guards, as a walkthrough.
- [Data API](../data-api.md): the HTTP ingress in full.
- [Channel configuration](./index.md): every key, with its page.
