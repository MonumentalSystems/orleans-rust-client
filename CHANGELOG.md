# Changelog

All notable changes to this project are documented in this file. The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **.NET protobuf/gRPC codegen** now runs `protoc` directly (via
  `scripts/gen-proto.sh` / `make proto`) and the generated C# sources are
  committed under each project's `Generated/` directory, instead of being
  produced by the `Grpc.Tools` MSBuild integration. The bundled arm64 `protoc`
  segfaults (`exit 139`) when MSBuild spawns it, breaking `dotnet build` on
  arm64 hosts; the same binary runs cleanly from a shell. The `.csproj`s drop
  their `<Protobuf>` items (and the now-unused `Grpc.Tools` reference in the
  counter example) while keeping the `Grpc.AspNetCore` / `Google.Protobuf`
  runtime packages. A plain `dotnet build` now needs no `protoc`; both arm64 and
  amd64 use the same codegen path.

## [0.1.0]

Initial release.

### Added

- **`orleans-rust-client`** — async Rust client for Microsoft Orleans via a
  small .NET gRPC bridge:
  - `OrleansClient` with a builder, cheap cloning, and `GrainRef` handles.
  - JSON payloads (`invoke_json`, `invoke_json_with_context`) and an optional
    `protobuf` feature (`invoke_protobuf`).
  - All single-key kinds: `GrainKey::String`, `GrainKey::Int64`,
    `GrainKey::Guid`.
  - Multi-argument methods, request-context propagation, server-enforced
    timeouts, and a conservative opt-in retry policy.
  - Stable `OrleansError` model with machine-readable bridge error codes.
  - Optional TLS (`tls` feature): custom/self-signed CA, mutual TLS, or public
    webpki roots; plus auth metadata hooks (`bearer_token`, `api_key`,
    `metadata`).
- **`orleans-rust-codegen`** — manifest-driven generator (library + CLI) for
  typed grain clients: primitive/collection/generic type mapping,
  multi-argument methods, and opt-in response-context accessors.
- **.NET bridge** (`OrleansRustBridge`, `.Abstractions`, `.Tools`) — hosts the
  official Orleans `IClusterClient`, dispatches via type-safe invokers, supports
  JSON/protobuf codecs, maps exceptions to stable errors, and reflects grain
  contracts into manifests/invokers.
- Counter example (silo + bridge + Rust) and an end-to-end integration suite.

[Unreleased]: https://github.com/MonumentalSystems/orleans-rust-client/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/MonumentalSystems/orleans-rust-client/releases/tag/v0.1.0
