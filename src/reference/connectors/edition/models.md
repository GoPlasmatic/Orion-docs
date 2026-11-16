<!-- description: The com.orion.models ONNX provider: artifact and tensor connection fields, admission, the infer operation and error codes. -->
<!-- type: reference -->
<!-- last_verified: 2026-11-16 -->

# Models provider

Available in Orion Standard and Enterprise only. The connection fields and the `infer` operation of `com.orion.models`, which admits an ONNX model and runs it on tract.

## Synopsis

A definition set declares the connection with `provider` in place of `connector_type`:

```json
{
  "name": "c4-policy",
  "provider": "com.orion.models",
  "config": {
    "artifact": {
      "s3": {
        "endpoint": "https://s3.eu-west-1.amazonaws.com",
        "region": "eu-west-1",
        "bucket": "models",
        "key": "c4-tiny/1.onnx",
        "access_key": "env://MODELS_ACCESS_KEY",
        "secret_key": "env://MODELS_SECRET_KEY"
      }
    },
    "digest": "sha256:a846b06040de77b5c51789ba7b02e05618008f6524f1fc2789e4382f3f811619",
    "inputs": [{ "name": "board", "dtype": "f32", "shape": [1, 2, 6, 7] }],
    "outputs": [{ "name": "policy", "dtype": "f32", "shape": [1, 7] }]
  }
}
```

## Description <span class="edition-badge edition-standard-plus" title="Available in Orion Standard and Enterprise">Standard+</span>

`infer` runs one inference over the model a connection revision admits. `model_infer` is a compatibility alias. The call supplies one tensor per declared input, by name, and receives one tensor per declared output, plus the admission's statistics.

A new model version is a new connection revision, because a revision is immutable and the digest names the bytes. The provider fetches the artifact, verifies the digest, reads the graph, binds it to the declared boundary and probes it.

A node counts `orion_model_admissions_total` and `orion_model_admission_seconds` for admission, and `orion_model_inferences_total` and `orion_model_inference_seconds` for inference.

### Admission

Admission runs on a revision's first call, once. Concurrent calls wait for that one load, and the admitted plan is kept for the revision until the node no longer pins it. A failed admission is remembered for ten seconds before another call tries.

A connection probe, `POST /api/v2/admin/connections/{id}/revisions/{revision}/probes`, runs a fresh admission and reports the stage that failed. That is how a deployment proves a model before it serves.

The probe runs five inferences over zero-filled inputs, with each named dimension at its `probe_dims` size. The outputs must match their declared dtypes and shapes, and the median inference must land within `max_probe_ms`.

## Fields

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `artifact` | object | yes | — | The artifact source, one of `storage`, `s3` or `http`. |
| `digest` | string | yes | — | The `sha256:` digest the artifact must hash to, as 64 lowercase hex digits. |
| `format` | string | no | `"onnx"` | The artifact format; `onnx` is the only accepted value. |
| `inputs` | array | yes | — | The graph's inputs a call supplies, each a tensor declaration. |
| `outputs` | array | yes | — | The graph's outputs a call receives, in answer order. |
| `probe_dims` | object | no | `{}` | The size each named dimension is probed at; unset names use 1. |
| `allow_private_urls` | boolean | no | `false` | Whether the artifact may come from a private or internal address. |
| `max_artifact_bytes` | integer | no | `268435456` | The artifact's byte ceiling, from 1 byte to 2 GiB. |
| `max_parameters` | integer | no | — | The graph's parameter ceiling; unset leaves it without a ceiling. |
| `max_input_elements` | integer | no | `1048576` | The elements one inference may hand the model, summed over its inputs. |
| `max_output_elements` | integer | no | `1048576` | The elements one inference may take back, summed over its outputs. |
| `timeout_ms` | integer | no | `1000` | The wall-clock ceiling per inference, a cold load included, at most 60000. |
| `max_probe_ms` | integer | no | `250` | The ceiling on the median probe inference, at most 60000. |
| `fetch_timeout_ms` | integer | no | `300000` | How long one artifact fetch may take, at most 3600000. |
| `max_concurrency` | integer | no | `16` | The inferences of this model that may run at once, at most 1024. |

### `artifact` fields

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `artifact.storage.key` | string | yes for `storage` | — | The object key in the store bound to the alias `artifact_store`. |
| `artifact.s3.endpoint` | string | yes for `s3` | — | The S3-compatible base URL, as a storage connection takes it. |
| `artifact.s3.region` | string | yes for `s3` | — | The SigV4 signing region that the S3-compatible store expects. |
| `artifact.s3.bucket` | string | yes for `s3` | — | The bucket holding the artifact object within the store. |
| `artifact.s3.key` | string | yes for `s3` | — | The object key, from 1 to 1024 bytes within the bucket. |
| `artifact.s3.force_path_style` | boolean | no | `false` | Whether requests address the bucket as a path rather than a host prefix. |
| `artifact.s3.access_key` | string | no | — | The access key, always a secret locator at `artifact.s3.access_key`. |
| `artifact.s3.secret_key` | string | no | — | The secret key, always a secret locator at `artifact.s3.secret_key`. |
| `artifact.s3.session_token` | string | no | — | An STS token, always a secret locator at `artifact.s3.session_token`. |
| `artifact.http.url` | string | yes for `http` | — | The artifact's HTTP or HTTPS URL, which the fetch reads. |
| `artifact.http.token` | string | no | — | A bearer token, always a secret locator at `artifact.http.token`. |

A `storage` artifact reads through the `com.orion.storage` connection bound to the alias `artifact_store`. The model connection never holds that store's keys: each load asks it to presign a GET for the key. An `s3` or `http` artifact refuses that binding.

### Tensor declarations

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `name` | string | yes | — | The graph's tensor name, as the ONNX file spells it. |
| `dtype` | string | yes | — | The element type: `bool`, `i8`, `u8`, `i16`, `u16`, `i32`, `u32`, `i64`, `u64`, `f32` or `f64`. |
| `shape` | array | yes | — | The dimensions, each a positive count or a named symbol such as `N`. |

A named dimension starts with a letter or `_` and holds letters, digits and `_`. One value binds a name across the whole boundary.

## Operations

| Operation | Aliases | Input | Answer |
|-----------|---------|-------|--------|
| `infer` | `model_infer` | `{inputs, timeout_ms?, layout?}` | `{outputs, stats}` |

### `infer`

| Field | Type | Required | Default | Description |
|-------|------|:--------:|---------|-------------|
| `inputs` | object | yes | — | One tensor per declared input, keyed by input name. |
| `timeout_ms` | integer | no | — | A positive ceiling that shortens the connection's `timeout_ms` for this call. |
| `layout` | string | no | `"nested"` | `nested` for arrays of arrays, or `flat` for `{shape, data}` objects. |

A tensor is a nested JSON array, such as `[[0.5, 1.0]]`, or with `layout: "flat"` an object `{"shape": [1, 2], "data": [0.5, 1.0]}`. Values are JSON numbers, and a `bool` tensor takes booleans. An integer dtype takes only whole numbers within its range.

## Returns

| Field | Type | Description |
|-------|------|-------------|
| `outputs` | object | One tensor per declared output, keyed by output name. |
| `stats` | object | The admitted model's numbers and this call's timings. |

`stats` holds `digest`, `runtime`, `device`, `artifact_bytes`, `parameters`, `nodes`, `operators`, `ir_version`, `opset`, `probe_ms` and `probed_at` from admission. The call adds `cold_load`, `queued_ms` and `inference_ms`.

## Errors

| Code | Class | Condition |
|------|-------|-----------|
| `MODEL_UNAVAILABLE` | unavailable | Admission failed; `details.stage` names the step, such as `fetch`, `digest`, `parse` or `probe`. |
| `MODEL_BUSY` | rate_limited | No inference slot freed within the deadline. |
| `MODEL_TIMEOUT` | deadline | The load or the inference passed its deadline; this is the only retryable failure. |
| `MODEL_OUTPUT_TOO_LARGE` | provider_failure | The outputs total more than `max_output_elements`. |
| `MODEL_RUN_FAILED` | provider_failure | The runtime failed, or produced what the connection does not declare. |
| `MODEL_CONNECTION_INCOMPLETE` | provider_failure | A credential the artifact source needs did not resolve. |
| `CONNECTOR_INVALID_INPUT` | invalid_input | The input or connection names an unknown field, tensor or dtype. |
| `CONNECTOR_EGRESS_DENIED` | forbidden | The artifact address is private and the connection does not allow it. |
| `CONNECTOR_DOWNSTREAM_UNAUTHORIZED` | unauthorized | The store refused the credentials. |
| `CONNECTOR_UNAVAILABLE` | unavailable | A transient admission failure, such as a store that did not answer. |

## Examples

The task runs the model over one board position:

```json
{
  "name": "connector_call",
  "input": {
    "connection": "c4-policy",
    "target_id": "com.orion.models",
    "operation": "infer",
    "input": {
      "inputs": { "board": { "var": "data.board" } },
      "layout": "nested"
    },
    "output": "data.policy"
  }
}
```

The answer carries the output tensor and the admission's statistics:

```json
{
  "outputs": {
    "policy": [[0.02, 0.11, 0.08, 0.31, 0.14, 0.09, 0.25]]
  },
  "stats": {
    "digest": "sha256:a846b06040de77b5c51789ba7b02e05618008f6524f1fc2789e4382f3f811619",
    "runtime": "tract",
    "device": "cpu",
    "artifact_bytes": 6171,
    "parameters": 1479,
    "nodes": 4,
    "operators": ["Flatten", "Gemm", "Relu"],
    "ir_version": 9,
    "opset": 17,
    "probe_ms": 0.4,
    "probed_at": "",
    "cold_load": true,
    "queued_ms": 0.1,
    "inference_ms": 0.3
  }
}
```

## Caveats

- A model is admitted per revision. Change the digest or the graph, and the next revision admits again.
- `f16` and `bf16` are not declarable dtypes. Declare the graph's `f32` boundary and let the graph cast.
- A storage artifact needs the `artifact_store` binding, and an S3 or HTTP artifact refuses that binding.
- A failed admission is remembered for ten seconds, so a broken artifact is not refetched on every call.
- Cold loads are limited to two at once across every model on the node, because each holds an artifact and a parsed graph.
- A non-finite float output is `null`, which is all JSON can carry.
- An inference is pure, so `MODEL_TIMEOUT` is the one failure worth retrying.

## Related

- [Edition providers](./index.md): the seven providers and the rules they share.
- [`connector_call`](../../functions/connector_call.md): the task that invokes `infer`.
- [Storage provider](./storage.md): the bucket a `storage` artifact reads through.
- [Edition capability matrix](../../editions.md): the seven providers with their minimum edition.
