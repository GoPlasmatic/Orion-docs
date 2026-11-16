<!-- description: Install orion-server and orion-cli on macOS, Linux, or Windows using Homebrew, shell scripts, Docker, or Cargo, and verify your local instance. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Install and run Orion

Orion ships as two matched binaries:
- **`orion-server`**: The single-binary services runtime with an embedded SQLite database (plus built-in PostgreSQL and MySQL drivers).
- **`orion-cli`**: The command-line tool for offline workflow linting, formatting, dry-runs, testing, compilation, and server deployments.

Always install the same release version for both `orion-server` and `orion-cli` so their v2 API contracts match.

## Before you start

Tested with Orion 2.0.0-rc.1. Choose any installation method below based on your operating system:

| Method | Supported Platforms | Prerequisites |
| :--- | :--- | :--- |
| **Homebrew** | macOS (Apple Silicon) and Linux | [Homebrew](https://brew.sh) |
| **Shell installer** | Linux and macOS (Apple Silicon) | POSIX shell and `curl` |
| **PowerShell installer** | Windows | Windows PowerShell |
| **Docker** | Linux, macOS, Windows | Docker Engine or Docker Desktop |
| **From source (Cargo)** | Any platform with Rust | Git and Rust `1.98` or newer |

> [!NOTE]
> While `2.0.0-rc.1` is a release candidate, use the **From source** tab or build from a local checkout if prebuilt release artifacts are not yet published for your tag.

## 1. Install `orion-server`

<div class="tabs">
<section data-tab="Homebrew">

On macOS (Apple Silicon) and Linux:

```bash
brew install GoPlasmatic/tap/orion-server
```

</section>
<section data-tab="Shell">

On Linux and macOS (Apple Silicon):

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/GoPlasmatic/Orion/releases/latest/download/orion-server-installer.sh | sh
```

</section>
<section data-tab="PowerShell">

On Windows PowerShell:

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://github.com/GoPlasmatic/Orion/releases/latest/download/orion-server-installer.ps1 | iex"
```

</section>
<section data-tab="Docker">

Run the container image directly (exposes port `8080`):

```bash
docker run -p 8080:8080 ghcr.io/goplasmatic/orion:latest
```

</section>
<section data-tab="Source">

Build from source with Cargo (requires Rust `1.98+`):

```bash
cargo install --git https://github.com/GoPlasmatic/Orion --locked orion-server
```

Pass `--locked` so Cargo compiles the exact dependency tree tested in CI.

</section>
</div>

## 2. Install `orion-cli`

Install `orion-cli` using the same method you used for the server:

<div class="tabs">
<section data-tab="Homebrew">

```bash
brew install GoPlasmatic/tap/orion-cli
```

</section>
<section data-tab="Shell">

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/GoPlasmatic/Orion/releases/latest/download/orion-cli-installer.sh | sh
```

</section>
<section data-tab="PowerShell">

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://github.com/GoPlasmatic/Orion/releases/latest/download/orion-cli-installer.ps1 | iex"
```

</section>
<section data-tab="Docker">

```bash
docker run --rm ghcr.io/goplasmatic/orion-cli:latest \
  --server http://host.docker.internal:8080 health
```

</section>
<section data-tab="Source">

```bash
cargo install --git https://github.com/codetiger/Orion-cli --locked orion-cli
```

</section>
</div>

## 3. Start the server and verify readiness

Run `orion-server` with no arguments to start on its local defaults (HTTP on `127.0.0.1:8080` and an embedded SQLite database at `./orion.db`):

```bash
orion-server
```

On first boot, Orion creates `./orion.db` and runs its embedded database migrations automatically.

In a second terminal, point `orion-cli` at your local instance and check its health:

<div class="tabs">
<section data-tab="CLI">

```bash
orion-cli config set-server http://localhost:8080
orion-cli health
```

</section>
<section data-tab="curl">

```bash
curl -s http://localhost:8080/health | jq
```

</section>
</div>

The `/health` endpoint returns the server status and loaded state:

```json
{
  "status": "ok",
  "version": "<installed-version>",
  "uptime_seconds": 5,
  "components": {
    "database": "ok",
    "engine": "ok",
    "connectors": "ok",
    "channels": "ok"
  },
  "git_hash": "<build-commit>",
  "build_timestamp": "<build-timestamp>",
  "workflows_loaded": 0,
  "connectors": { "circuit_breaker_scope": "node", "circuit_breakers": {}, "failed_to_load": [] },
  "channels": { "quarantined": [] }
}
```

`"status": "ok"` with `"workflows_loaded": 0` confirms the server is healthy and waiting for your first deployment. Outside production mode, Orion also serves interactive OpenAPI documentation at [http://localhost:8080/docs](http://localhost:8080/docs).

## 4. Configure the server

You can configure `orion-server` with a TOML file (`-c config.toml`), environment variables (`ORION_SECTION__KEY`), or both:

```bash
orion-server -c config.toml
ORION_SERVER__PORT=9090 orion-server
```

To validate a configuration file and inspect the merged settings (with secrets masked) without starting the server, run:

```bash
orion-server validate-config -c config.toml
```

See [Server configuration and environment variables](../reference/configuration.md) for every setting and default.

## Install Orion Standard or Enterprise (`orion-ext`) <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

Available in Orion Standard and Enterprise only. Both commercial editions ship as a single binary and container image named **`orion-ext`**, distributed through the private registry. It uses the same `orion-cli` binary as Community.

Pull the licensed image or build it from your `Orion-ext` checkout:

```bash
docker build -t orion-ext:local .
docker run -p 8080:8080 -v "$PWD/orion-ext.toml:/app/config.toml:ro" orion-ext:local -c /app/config.toml
```

Configure the reported edition and the Redis cluster store in `orion-ext.toml`:

```toml
[extensions.licence]
tier = "enterprise"   # or "standard" (default when omitted)

[cluster]
enabled = true

[extensions.redis]
host = "redis.internal"
password = "env://REDIS_PASSWORD"
```

See [Standard & Enterprise overview](../enterprise/overview.md) and [Multi-node clustering and HA](../enterprise/clustering.md) for complete setup instructions.

## Next steps

- [Quickstart](./quickstart.md) — Deploy and call your first service in under five minutes.
- [Build your first service](./first-service.md) — Learn how workflows, channels, traces, and rollbacks work together.
- [Build a Postgres-backed orders API](./postgres-orders-api.md) — Connect PostgreSQL, validate inputs, and run offline tests with stubs.
- [Run the example packages](./examples.md) — Explore 11 ready-to-deploy example services.
