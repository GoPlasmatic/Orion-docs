<!-- description: The integration settings of orion-server: the WebAssembly plugin sandbox and its ceilings, plus the edition licence and Redis store tables. -->
<!-- type: hub -->
<!-- last_verified: 2026-11-16 -->

# Integration settings

WebAssembly plugins, and the extension sections a licensed edition reads. Every plugin table on these pages carries the wire name, the default the code uses, and the `ORION_*` override. Extension sections have no override.

| Page | Holds |
|---|---|
| [Plugin settings](./plugins.md) | enabling the WebAssembly sandbox, the memory, size, time and concurrency ceilings, trust keys and per-plugin overrides. |
| [Extension settings](./extensions.md) | the `[extensions.licence]` label and the Redis `[extensions.redis]` cluster store. |

## Related

- [Server configuration](./index.md): every section, by what you are configuring.
- [How settings are resolved](./how-settings-are-resolved.md): defaults, the file, and the environment.
- [Production checklist](../../operate/production-checklist.md): which settings to change before real traffic.
