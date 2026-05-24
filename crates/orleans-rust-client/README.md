# orleans-rust-client

Rust client bindings for Microsoft Orleans services via a small official .NET
bridge.

This crate provides an ergonomic, async Rust API for calling Orleans grains
without reimplementing Orleans' internal gateway protocol or serialization
runtime. A small .NET bridge hosts the official Orleans `IClusterClient` and
exposes a generic gRPC surface that this crate talks to.

```rust
use orleans_rust_client::{GrainKey, OrleansClient};

# async fn run() -> Result<(), orleans_rust_client::OrleansError> {
let client = OrleansClient::connect("http://127.0.0.1:50051").await?;

let counter = client.grain(
    "Counter.Abstractions.ICounterGrain",
    "counter",
    GrainKey::String("demo".into()),
);

let value: i64 = counter.invoke_json("Get", &()).await?;
println!("value = {value}");
# Ok(())
# }
```

See the [repository](https://github.com/MonumentalSystems/orleans-rust-client)
for the bridge, examples, and architecture.

## Features

- `json` (default) — JSON payloads via `serde_json`.
- `protobuf` — opaque protobuf payloads (caller-encoded bytes).

## License

MIT. Copyright (c) 2026 Monumental Systems, LLC.
