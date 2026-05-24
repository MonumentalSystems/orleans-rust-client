# orleans-rust-client

Rust client bindings for Microsoft Orleans services via a small official .NET bridge.

This project provides a Rust-native developer experience for calling Orleans
grains without reimplementing Orleans' internal gateway protocol or
serialization runtime. The bridge uses the official Orleans `IClusterClient`;
Rust applications communicate with the bridge over gRPC.

[![CI](https://github.com/MonumentalSystems/orleans-rust-client/actions/workflows/ci.yml/badge.svg)](https://github.com/MonumentalSystems/orleans-rust-client/actions/workflows/ci.yml)

## What this is

A practical integration layer in two halves:

- A **Rust async client crate** (`orleans-rust-client`) with an ergonomic,
  strongly-typed API.
- A reusable **.NET bridge** (`OrleansRustBridge`) that hosts the official
  Orleans client and exposes a generic, language-neutral gRPC surface.

A grain call from Rust is serialized (JSON by default), sent to the bridge over
gRPC, dispatched to the real grain through `IClusterClient`, and the response is
returned the same way. Orleans semantics are preserved because the bridge uses
the real Orleans runtime.

## What this is not

It is **not** a reimplementation of Orleans. See [Non-goals](#non-goals).

## Architecture

```text
Rust application
   |
   | typed Rust API
   v
orleans-rust-client crate
   |
   | tonic / gRPC
   v
C# Orleans bridge service (OrleansRustBridge)
   |
   | official Microsoft.Orleans.Client / IClusterClient
   v
Orleans cluster
```

The bridge owns the Orleans runtime integration; Rust owns the ergonomic client
API. Dispatch on the .NET side is type-safe: each grain interface gets a small
`IBridgeGrainInvoker` adapter (the "Mode A" approach), so the bridge never
guesses method signatures by reflection at call time.

## Repository layout

```text
proto/                      The bridge gRPC contract (orleans.bridge.v1).
crates/
  orleans-rust-client/      The Rust client crate.
  orleans-rust-codegen/     Manifest-driven typed-client generator + CLI.
dotnet/
  OrleansRustBridge/            Reusable bridge: gRPC service, codecs, DI.
  OrleansRustBridge.Abstractions/  Invoker contract, manifest types, attributes.
  OrleansRustBridge.Tools/      Reflection tool: emit manifests / generate invokers.
examples/counter/           End-to-end counter sample (silo + bridge + Rust).
tests/integration/          End-to-end integration tests.
```

## Quickstart

Prerequisites: a Rust toolchain (edition 2024 / 1.85+), the .NET SDK pinned in
`global.json`, and `protoc` on your `PATH` for the Rust build
(`apt-get install -y protobuf-compiler`, `brew install protobuf`, ...).

Run the counter sample in three terminals:

```sh
# Terminal 1 — Orleans silo
dotnet run --project examples/counter/dotnet/Counter.Silo

# Terminal 2 — bridge (listens on http://127.0.0.1:50051 by default)
dotnet run --project examples/counter/dotnet/Counter.Bridge

# Terminal 3 — Rust client
cargo run --manifest-path examples/counter/rust/Cargo.toml
```

See [`examples/counter/README.md`](examples/counter/README.md) for details and
port configuration.

## Example: Rust client

```rust
use orleans_rust_client::{GrainKey, OrleansClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = OrleansClient::connect("http://127.0.0.1:50051").await?;

    let counter = client.grain(
        "Counter.Abstractions.ICounterGrain",
        "counter",
        GrainKey::String("demo".into()),
    );

    counter.invoke_json::<_, ()>("Reset", &()).await?;
    let value: i64 = counter.invoke_json("Add", &5_i64).await?;
    println!("value = {value}");
    Ok(())
}
```

With a generated typed client (see `orleans-rust-codegen`):

```rust
let client = OrleansClient::connect("http://127.0.0.1:50051").await?;
let counter = CounterGrainClient::new(client.clone(), "demo");
counter.add(5).await?;
let value = counter.get().await?;
```

## Example: C# bridge registration

```csharp
var builder = WebApplication.CreateBuilder(args);

builder.Host.UseOrleansClient(client => client.UseLocalhostClustering());

builder.Services.AddOrleansRustBridge(options =>
{
    options.ServiceId = "my-service";
    options.ClusterId = "dev";
});

// Register one invoker per grain interface (hand-written or generated).
builder.Services.AddSingleton<IBridgeGrainInvoker, CounterGrainInvoker>();

var app = builder.Build();
app.MapOrleansRustBridge();
app.Run();
```

## Supported Orleans versions

Targets **.NET 10** and **Microsoft.Orleans 10.x** (pinned to `10.1.0` in
`Directory.Build.props`; the SDK is pinned in `global.json`). A .NET 10 host can
consume earlier Orleans 9.x packages if you need to pin lower; adjust
`OrleansVersion` accordingly.

## Supported payload codecs

- `json` (default) — uses `System.Text.Json` on the bridge and `serde_json` in
  Rust. Best for debugging and getting started.
- `protobuf` (optional) — opaque, caller-encoded `IMessage` payloads. Enable the
  `protobuf` Cargo feature on the client and add `protobuf` to the bridge's
  `PayloadCodecs`.

## Error model

Bridge calls fail with a stable, machine-readable code (decoupled from gRPC
status codes and from .NET exception types). The Rust client surfaces these as
`OrleansError::Bridge { code, message, detail, retryable }`. Codes:

```text
unknown_grain   unknown_method   invalid_key       invalid_payload
serialization_error               orleans_rejection orleans_timeout
orleans_unavailable               application_error cancelled          internal
```

Structured errors travel in a `bridge-error-bin` gRPC trailer, so the stable
code survives regardless of the transport status. Exception detail is omitted
unless `BridgeOptions.IncludeExceptionDetail` is enabled (development only).

## Security notes

The bridge is a privileged backend component. Do not expose it publicly without
TLS and authentication, bound message sizes, and keep exception detail off in
production. See [`SECURITY.md`](SECURITY.md).

## Roadmap

- **M0 — scaffold** ✅ workspace, solution, proto, docs.
- **M1 — health + manifest** ✅ bridge `Health` and `GetManifest`.
- **M2 — JSON invoke** ✅ `Reset`/`Add`/`Get` end-to-end with stable errors.
- **M3 — typed clients** ✅ `orleans-rust-codegen` generates typed wrappers.
- **M4 — hardening** TLS/auth in the client, configurable retries (conservative,
  opt-in today), broader codegen, more codecs.

## Non-goals

This project does not implement the Orleans gateway wire protocol,
`Orleans.Serialization`, or a replacement actor runtime. It is a practical
integration layer for Rust applications that need to call Orleans-backed
services. It is not a general actor framework and does not aim to compete with
Orleans.

## License

Licensed under the [MIT License](LICENSE). Copyright (c) 2026 Monumental
Systems, LLC.
