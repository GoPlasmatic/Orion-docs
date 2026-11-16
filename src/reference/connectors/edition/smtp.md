<!-- description: The com.orion.smtp provider: relay connection fields, the send operation input and answer, and the SMTP error codes it raises. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# SMTP provider

Available in Orion Standard and Enterprise only. The connection fields and the `send` operation of `com.orion.smtp`, which sends transactional email through an SMTP relay.

## Synopsis

A definition set declares the connection with `provider` in place of `connector_type`:

```json
{
  "name": "mailer",
  "provider": "com.orion.smtp",
  "config": {
    "host": "smtp.example.com",
    "port": 587,
    "tls": "starttls",
    "auth": {
      "type": "basic",
      "username": "mailer",
      "password": "env://SMTP_PASSWORD"
    },
    "from": "Orders <orders@example.com>",
    "max_connections": 10
  }
}
```

## Description <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

`send` delivers one RFC 5322 message through the connection's relay. `send_email` is a compatibility alias for the same operation. The provider drives the envelope one command at a time and answers the relay's final reply line.

The provider records an `Unknown` effect immediately before the message body leaves. A timeout after that point is indistinguishable from an accepted message, so the node never retries past it. A failure before it, such as a refused recipient, is safe to retry.

A message is judged before a session opens, so an invalid input never reaches the relay. Recipient addresses and bodies are workflow data: the provider logs neither, and its errors name only the field and the relay's reply.

## Fields

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `host` | string | yes | — | The relay host name or IP address; a URL is refused. |
| `port` | integer | no | `587` | The relay port; 465 pairs with implicit TLS and 25 with internal relays. |
| `tls` | string | no | `"starttls"` | Session security: `starttls` upgrades a plaintext connect, `implicit` starts with TLS. |
| `auth` | object | no | `{"type": "none"}` | Session authentication: `none` for a trusted relay or `basic` for PLAIN and LOGIN. |
| `from` | string | yes | — | The default sender, as an address alone or as `Name <addr@example.com>`. |
| `allow_from_override` | boolean | no | `false` | Whether a send may name its own sender in the input `from` field. |
| `allow_private_urls` | boolean | no | `false` | Whether the relay may resolve to a private or internal address. |
| `timeout_ms` | integer | no | `10000` | The connect and per-command timeout in milliseconds, within the run deadline. |
| `max_connections` | integer | no | `10` | The concurrent sessions this node opens to the relay for this connection. |

### `auth` fields

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `auth.type` | string | no | `"none"` | `none` or `basic`; `basic` authenticates with PLAIN or LOGIN. |
| `auth.username` | string | no | — | The login name, which a `basic` connection requires when a send runs. |
| `auth.password` | string | no | — | The login password, always a secret locator at `auth.password`. |

## Operations

| Operation | Aliases | Answer |
|-----------|---------|--------|
| `send` | `send_email` | `{message_id, response}` |

### `send`

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `to` | string or array of strings | yes | — | One recipient address, or an array of addresses, each with an optional name. |
| `cc` | string or array of strings | no | — | Carbon-copy recipients, which appear in the message headers every recipient sees. |
| `bcc` | string or array of strings | no | — | Blind recipients, which travel only in the SMTP envelope. |
| `subject` | string | yes | — | The message subject line, which the structured `Subject` header carries. |
| `text` | string | no | — | The plain-text body; at least one body is required. |
| `html` | string | no | — | The HTML body; with `text` the message is multipart/alternative. |
| `reply_to` | string | no | — | The address replies are sent to, written as `Reply-To` in the message. |
| `headers` | object | no | `{}` | Extra headers, each a string value; structured names are refused. |
| `from` | string | no | — | The sender, accepted only when the connection sets `allow_from_override` to true. |

A call requires at least one of `text` and `html`. An address is `addr@example.com` or `Name <addr@example.com>`.

## Returns

| Field | Type | Description |
|-------|------|-------------|
| `message_id` | string | The generated `Message-ID`, in angle brackets. |
| `response` | string | The relay's final reply line for the accepted message. |

## Errors

| Code | Class | Condition |
|------|-------|-----------|
| `SMTP_REJECTED` | provider_failure | The relay refused with a 5xx reply; `details.reply_code` names the code. |
| `SMTP_DEFERRED` | unavailable | The relay deferred with a 4xx reply; retryable only before the body is sent. |
| `SMTP_TLS_FAILED` | unavailable | TLS could not be established with the relay. |
| `SMTP_PROTOCOL` | provider_failure | The session broke the SMTP protocol. |
| `SMTP_CONNECTION_INVALID` | provider_failure | The connection settings build no transport. |
| `SMTP_CONNECTION_INCOMPLETE` | provider_failure | Basic authentication lacks its username or its password. |
| `CONNECTOR_INVALID_INPUT` | invalid_input | The input or the connection names an unknown field or value. |
| `CONNECTOR_OPERATION_FORBIDDEN` | forbidden | The input sets `from` on a connection without `allow_from_override`. |
| `CONNECTOR_EGRESS_DENIED` | forbidden | The relay resolves to a private address and the connection does not allow it. |
| `CONNECTOR_DOWNSTREAM_UNAUTHORIZED` | unauthorized | The relay refused the credentials. |
| `CONNECTOR_UNAVAILABLE` | unavailable | The relay did not answer, dropped the session or timed out. |
| `CONNECTOR_DEADLINE_EXCEEDED` | deadline | The run's deadline passed during the send. |

## Examples

The task sends one message through the `mailer` connection:

```json
{
  "name": "connector_call",
  "input": {
    "connection": "mailer",
    "target_id": "com.orion.smtp",
    "operation": "send",
    "input": {
      "to": "user@example.com",
      "subject": "Your order shipped",
      "text": "Order 1042 left the warehouse today."
    },
    "output": "data.mail"
  }
}
```

The answer names the generated message and the relay's reply:

```json
{
  "message_id": "<8f3c1a9d@example.com>",
  "response": "250 2.0.0 Ok: queued as 4Wx1"
}
```

## Caveats

- `bcc` recipients travel in the envelope only. No `Bcc` header is written, so other recipients do not see them.
- A `headers` entry that names one of the structured fields is refused, so the two cannot disagree. The refused names are `from`, `to`, `cc`, `bcc`, `subject`, `date`, `message-id`, `content-type`, `content-transfer-encoding`, `mime-version` and `reply-to`.
- A send makes one attempt. After the body is written, a timeout or a dropped session leaves the outcome unknown, so the node never retries it.
- Certificates validate against the platform trust store. A relay with a private CA needs that CA installed there, because no skip-verification setting exists.
- A send beyond `max_connections` waits for a free session until the run's deadline.

## Related

- [Edition providers](./index.md): the seven providers and the rules they share.
- [`connector_call`](../../functions/connector_call.md): the task that invokes `send`.
- [Definition and identity](../identity.md): the connection revision and the secret locators it holds.
- [Edition capability matrix](../../editions.md): the seven providers with their minimum edition.
