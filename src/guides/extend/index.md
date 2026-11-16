<!-- description: Extend the runtime with your own code: ship a codec or calculation as a sandboxed WebAssembly plugin that a task calls like a built-in function. -->
<!-- type: hub -->
<!-- last_verified: 2026-09-14 -->

# Extend Orion

A definition covers most services. It cannot express compiled code that already exists, or a provider backend shipped as a package. A plugin covers the first and a connector provider package the second; anything with I/O stays a connector or a service.

- [Build a plugin](./plugins.md) takes a pure JSON-to-JSON function through the SDK, the manifest, the build, the offline test and the deployment.
- [Publish a connector provider package](./provider-packages.md) scaffolds, lints, packs, uploads and admits a provider backend, and sets the edition tier it requires.
