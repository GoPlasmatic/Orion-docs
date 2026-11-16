<!-- description: Secure Orion in production: configure admin API keys, per-channel data-plane authentication, TLS, trusted proxies, secret locators, and SSRF bounds. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Security and access control

Orion separates **admin-plane security** (`/api/v2/admin/**`), **data-plane ingress security** (`/api/v2/data/**`), and **outbound connector/plugin isolation** so each boundary can be locked down independently.

---

## 1. Authenticate the admin plane

Until you enable `[admin_auth]`, `/api/v2/admin/**` and `/metrics` (on the main listener) are unauthenticated. In `ORION_ENVIRONMENT=production`, Orion refuses to boot unless `admin_auth.enabled = true` with at least one strong key.

```toml
[admin_auth]
enabled = true
api_keys = [
  "sha256:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
]
read_only_api_keys = [
  "sha256:4b227777d4dd1fc61c6f884f48641d02b4d121d3fd328cb08b5531fcacdabf8a"
]
header = "Authorization"
```

### Best practices for admin keys

- **Store SHA-256 digests instead of plaintext keys**: Compute `printf '%s' "$KEY" | shasum -a 256` and configure `"sha256:<hex>"`. Neither `orion.toml` nor `orion-server validate-config` will ever hold the raw secret.
- **Keep two active keys during rotation**: Add the new key digest alongside the current one, roll out the updated client secret, and remove the old digest on the next config update.
- **Use `read_only_api_keys` for CI checks and dashboards**: A read-only key authorizes `GET` and `HEAD` requests plus side-effect-free validation routes (`POST /api/v2/admin/deployment-plans`, `authoring/scaffold`, `authoring/lint`, `authoring/pack`). It rejects mutating routes and live-backend execution routes (`definitions/test` and connection `probes`) with `403`.
- **Automatic brute-force backoff**: After 5 consecutive failed authentication attempts from a client IP, Orion enforces exponential backoff up to 30 seconds and increments `orion_admin_auth_failures_total{reason="locked_out"}`.

Pass the key in `Authorization: Bearer <key>` (default) or configure a custom header name such as `header = "X-API-Key"`:

```bash
curl -H "Authorization: Bearer $ORION_ADMIN_KEY" http://localhost:8080/api/v2/admin/deployments
```

---

## 2. Authenticate data-plane channels

> [!WARNING]
> `admin_auth` protects only `/api/v2/admin/**`. Every data-plane channel (`/api/v2/data/**`) is open to callers unless that channel configures its own `auth` guard or sits behind an authenticating gateway.

Configure one of three built-in authentication modes in a channel's `config.auth` block (see [Configure channels and guards](../guides/channels.md) for examples):
- **`api_key`**: Constant-time comparison against the SHA-256 digest of each accepted key (`env://` or `vault://` locators).
- **`hmac`**: Signature verification (`SHA-1`, `SHA-256`, or `SHA-512`) over the raw request body or a templated timestamp + body string before JSON parsing. Includes presets for Stripe, GitHub, Shopify, Slack, Zoom, and Webex webhooks.
- **`jwt`**: Verifies Bearer tokens against static keys or a remote JWKS endpoint and populates `metadata.auth.claims.*` for your workflow. Combine with `oauth2_login` when Orion acts as an OIDC relying party for browser sign-in.

All three modes return a uniform `401` response on failure without leaking which check failed.

---

## 3. Terminate TLS

Terminate TLS either at a reverse proxy/load balancer in front of Orion or directly on the Orion listener:

```toml
[server.tls]
enabled = true
cert_path = "/etc/orion/tls/server.crt"
key_path  = "/etc/orion/tls/server.key"
```

When `server.tls.enabled = true`, Orion loads the certificate and private key at startup (failing closed if either file is missing or invalid) and automatically adds the `Strict-Transport-Security` header to responses.

---

## 4. Trust reverse proxy CIDRs

Rate limiting and failed-auth lockouts identify clients by their TCP peer IP unless the peer matches `rate_limit.trusted_proxies`:

```toml
[rate_limit]
trusted_proxies = ["10.0.0.0/8", "fd00::/8"]
```

- **Why this matters**: Behind a load balancer or Kubernetes Ingress, the TCP peer is always the proxy. Without `trusted_proxies`, all external users share a single rate-limit bucket and trigger false `429` rejections.
- **Spoofing protection**: Orion ignores `X-Forwarded-For` and `Forwarded` headers unless the direct TCP connection originates from a CIDR in `trusted_proxies`.

---

## 5. Keep secrets out of definitions and databases

1. **Connector and channel secret locators**: Always write credentials as `env://VAR` or `vault://path#field` in source definitions. `orion-cli compile` extracts them into `secret_locators` and refuses literal secrets in `public_config`.
2. **Workflow key material (`[secrets]`)**: When a workflow task (`crypto`, `jwt_sign`, `jwt_verify`) needs a secret key, declare it in `orion.toml` under `[secrets]` and reference it via `{"secret": "partner_hmac"}`:

```toml
[secrets]
partner_hmac = "env://PARTNER_HMAC_KEY"
```

Orion resolves `[secrets]` at engine build time and blocks any workflow mapping that attempts to copy a secret into `data`, trace snapshots, or HTTP responses.
3. **Encrypt OAuth2 token state at rest**: Outbound `oauth2` HTTP connectors persist acquired access and refresh tokens in the state database. Encrypt them with AES-256-GCM by setting:

```toml
[storage]
connector_encryption_key = "env://ORION_SECRET_CONNECTOR_KEY"
```

4. **Automatic error redaction**: In `http_call` errors, Orion masks URL userinfo passwords and secret-shaped query parameters (`pwd`, `api_key`, `sig`, `token`) and truncates upstream non-2xx response bodies at 512 bytes before writing to logs, traces, or the DLQ.

---

## 6. Bound outbound connector targets (SSRF and operation gates)

Orion enforces two layers of outbound protection on every connector:
- **Scheme and private-IP blocking (SSRF protection)**: Each connector driver accepts only its protocol's URL schemes (`http`/`https` for HTTP connectors, `postgres`/`mysql`/`sqlite` for DB connectors) and blocks connections to RFC 1918 private networks, loopback (`127.0.0.0/8`), link-local (`169.254.0.0/16` cloud metadata), and CGNAT addresses by default. Set `"allow_private_urls": true` on connectors that intentionally target internal databases or services.
- **Operation gates**: Restrict what workflows can do over a connection by setting `"operations": {"delete": false, "raw_write": false}` on the connector definition.

---

## 7. Configure CORS and origin allowlists

Orion separates browser preflight negotiation from server-side origin enforcement:
- **`[cors]` in `orion.toml`**: Controls the browser CORS handshake. In production, list explicit origins (`allowed_origins = ["https://app.example.com"]`); Orion refuses `allowed_origins = ["*"]` in production or when `allow_credentials = true`.
- **`origin_allow_list` on a channel**: Enforces a server-side check on the `Origin` header for that channel, rejecting unlisted origins with `403`.

---

## 8. Sandbox and sign WebAssembly plugins

WebAssembly plugins (`plugins.enabled = true`) run in an isolated Wasmtime sandbox with **no WASI host imports** (no filesystem, network sockets, clocks, environment variables, or secrets) and strict per-invocation memory, wall-clock, fuel, and payload ceilings.

To require cryptographic signature verification on every uploaded plugin, configure Ed25519 public keys in `[plugins.trust]`:

```toml
[plugins]
enabled = true

[plugins.trust]
required = true
public_keys = ["ed25519:3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29"]
```

---

## 9. Close unused surfaces

- **Disable anonymous API docs**: Keep `server.docs.enabled = false` in production (use `orion-server dump-openapi` offline when you need the OpenAPI spec).
- **Isolate the Prometheus listener**: Set `metrics.bind_addr = "127.0.0.1:9090"` (or an internal pod IP) so Prometheus can scrape `/metrics` without holding an admin API key.
- **Cap request payload size**: `ingest.max_payload_size` defaults to `1048576` bytes (1 MiB). Raise it only when your channels legitimately accept larger payloads.

## Verify your security posture

```bash
orion-server validate-config -c config.toml
curl -s -o /dev/null -w '%{http_code}\n' http://localhost:8080/api/v2/admin/deployments
```

The first command prints the effective configuration with secrets masked; the second confirms that anonymous admin requests are rejected with `401`.

## Next steps

- [Production checklist](./production-checklist.md): Complete pre-launch verification checklist.
- [Channel configuration schema](../reference/channels.md): Full schema for `auth`, `origin_allow_list`, and `oauth2_login` guards.
- [Server configuration and environment variables](../reference/configuration.md): All `[admin_auth]`, `[cors]`, `[server.tls]`, and `[plugins]` settings.
