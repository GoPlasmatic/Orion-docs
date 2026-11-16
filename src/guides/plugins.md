<!-- description: Build, test, sign, and deploy custom WebAssembly task functions in Rust using orion-plugin-sdk, the plugin.toml manifest, and the Wasmtime sandbox. -->
<!-- type: guide -->
<!-- last_verified: 2026-11-16 -->

# Extend with WebAssembly plugins

A **plugin** adds custom task functions to Orion without modifying or recompiling the server. Compiled to a WebAssembly component (`orion:plugin@1.0.0`) using [`orion-plugin-sdk`](https://crates.io/crates/orion-plugin-sdk), a plugin runs in a capability-free Wasmtime sandbox that accepts a task's evaluated JSON `input` and returns a JSON value written to `output`.

---

## 1. When to write a plugin

Plugins are designed for **pure, CPU-bound JSON-to-JSON transformations** on the hot path that are awkward to express in JSONLogic—such as parsing banking/EDI formats (ISO 8583, SWIFT MT/MX, X12, ASN.1, fixed-width records) or running domain-specific calculations and state machines.

By design, the WebAssembly plugin world **imports nothing** from the host:
- No network sockets or HTTP requests (use `http_call` or a connector instead).
- No filesystem access, system clock, or ambient randomness.
- No access to connector pools or `env://` secrets.

Because a plugin is guaranteed to be a pure function with zero side effects, Orion can execute it for real during offline `orion-cli dry-run` and `orion-cli test` runs without stubs, and safely retry timed-out calls.

---

## 2. Write a plugin in Rust (`orion-plugin-sdk`)

Create a Rust `cdylib` crate that depends on `orion-plugin-sdk`:

```toml
# Cargo.toml
[package]
name = "acme-fixedwidth"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
orion-plugin-sdk = "1"

[profile.release]
opt-level = "s"
lto = true
panic = "abort"
strip = true
```

Implement the `Plugin` trait and export it with `export_plugin!` in `src/lib.rs`:

```rust
use orion_plugin_sdk::{Plugin, PluginError, Value, export_plugin, json};

struct FixedWidth;

impl Plugin for FixedWidth {
    fn invoke(function: &str, input: Value) -> Result<Value, PluginError> {
        match function {
            "acme.fixedwidth.parse" => parse(&input),
            other => Err(PluginError::caller_input(
                "UNKNOWN_FUNCTION",
                format!("unknown function '{other}'"),
            )),
        }
    }
}

fn parse(input: &Value) -> Result<Value, PluginError> {
    let record = input["record"].as_str().ok_or_else(|| {
        PluginError::caller_input("BAD_RECORD", "'record' must be a string")
    })?;
    if record.len() < 10 {
        return Err(PluginError::caller_input("SHORT_RECORD", "record is shorter than 10 bytes"));
    }
    Ok(json!({ "account": record[..10].trim() }))
}

export_plugin!(FixedWidth);
```

- **Error classes**: Return `PluginError::caller_input(code, message)` when the input data is invalid, or `PluginError::internal(code, message)` for unexpected internal errors. Error codes must match `^[A-Z][A-Z0-9_]{0,63}$`.
- **Complete working examples**: See [`examples/plugins/fixed-width/`](https://github.com/GoPlasmatic/Orion/tree/main/examples/plugins/fixed-width) and [`examples/plugins/c4-rules/`](https://github.com/GoPlasmatic/Orion/tree/main/examples/plugins/c4-rules) in the Orion repository.

---

## 3. Compile to a WebAssembly component

Compile the crate targeting `wasm32-unknown-unknown` (do not target `wasm32-wasip1`, as WASI imports are rejected by the sandbox) and wrap the module into a component with `wasm-tools`:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-tools
cargo build --release --target wasm32-unknown-unknown
wasm-tools component new target/wasm32-unknown-unknown/release/acme_fixedwidth.wasm \
  -o fixed-width.wasm
```

---

## 4. Write the `plugin.toml` manifest

Place `plugin.toml` in the same directory as `fixed-width.wasm`. The manifest declares the plugin identity, ABI version, and the input schema of each exported function:

```toml
# plugin.toml
abi = "orion:plugin@1.0.0"
name = "acme.fixedwidth"
version = "0.1.0"
component = "fixed-width.wasm"

[[functions]]
name = "acme.fixedwidth.parse"
description = "Parse a fixed-width record into typed JSON fields"
category = "transform"
output_default_root = "data"

[[functions.input_fields]]
name = "record"
kind = "string"
required = true
template_at = true

[[functions.input_fields]]
name = "spec"
kind = "array"
required = true
```

### Manifest field rules
| Field | Description |
| :--- | :--- |
| `abi` | Must be `"orion:plugin@1.0.0"`. |
| `name` | Reverse-domain plugin ID with at least two labels (`[a-z][a-z0-9-]*`). The `orion.*` namespace is reserved. |
| `version` | Informational semantic version string. |
| `component` | Relative filename of the `.wasm` component in the same directory. |
| `functions[].name` | Qualified function name (`<plugin_name>.<label>`). |
| `functions[].output_default_root` | Optional default root (`"data"`, `"temp_data"`, or `"metadata"`) when a task omits `output`. |
| `input_fields[].kind` | `"string"`, `"number"`, `"bool"`, `"object"`, `"array"`, or `"any"`. |
| `input_fields[].required` | Boolean (`true` or `false`). Required on every field declaration. |
| `input_fields[].template_at` | When `true`, the field value is evaluated as a full JSONLogic expression before calling the plugin. |
| `input_fields[].resolvable` | When `true`, `{"var": "..."}` references inside the field are resolved before calling the plugin (mutually exclusive with `template_at`). |

*(Never declare an `output` field in `input_fields`—`output` is implicitly supported on every plugin task.)*

---

## 5. Call and test the plugin offline

Call the plugin function from a workflow task just like any built-in function:

```json
{
  "id": "decode",
  "name": "Decode fixed-width line",
  "function": {
    "name": "acme.fixedwidth.parse",
    "input": {
      "record": { "var": "data.raw_line" },
      "spec": [{ "name": "account", "start": 0, "len": 10 }],
      "output": "data.statement"
    }
  }
}
```

Test it offline with `orion-cli dry-run` and `orion-cli test` by pointing `--plugin-dir` at the directory containing `plugin.toml` and `fixed-width.wasm`:

```bash
orion-cli lint ./definitions --plugin-dir ./plugin
orion-cli dry-run -w ./definitions/workflow.json -i input.json --plugin-dir ./plugin
orion-cli test ./definitions/tests --plugin-dir ./plugin
```

---

## 6. Sign and deploy the plugin

To run plugins on an Orion server, enable the sandbox in `orion.toml`:

```toml
[plugins]
enabled = true
max_memory_bytes = 16777216          # 16 MiB per instance
max_timeout_ms = 50                  # 50 ms per invocation
max_request_bytes = 262144           # 256 KiB input cap
max_response_bytes = 262144          # 256 KiB output cap
max_concurrency_per_function = 64

[plugins.trust]
public_keys = ["MCowBQYDK2VwAyEA…"]  # Optional: require Ed25519 signatures
```

When `[plugins.trust]` lists one or more `public_keys`, every deployed plugin must carry a detached Ed25519 signature over its `sha256:` component digest. Generate a keypair and sign your plugin with `orion-cli plugin`:

```bash
orion-cli plugin keygen -o signer.pem
orion-cli plugin sign ./definitions/plugin.toml --key signer.pem
orion-cli plugin verify ./definitions/plugin.toml --pubkey <BASE64_PUBLIC_KEY>
```

`plugin sign` writes `fixed-width.wasm.sig` alongside the component. When you run `orion-cli compile`, the CLI bundles the manifest, the base64-encoded `.wasm` component, and the `.sig` signature into a single plugin definition inside the deployment:

```bash
orion-cli compile ./definitions -o dist/deployment
orion-cli deployments apply -d dist/deployment --yes
```

Verify that the server loaded your plugin functions:

```bash
orion-cli functions list --output json | jq '.data[] | select(.source == "plugin")'
```

## Next steps

- [Run the example packages](../get-started/examples.md) — Inspect and run the `fixed-width-statement` and `c4-referee` plugin packages.
- [Server configuration and environment variables](../reference/configuration.md) — See all `[plugins]` ceiling and `[[plugins.overrides]]` settings.
- [CLI reference](../reference/cli.md) — Full command reference for `orion-cli plugin`.
