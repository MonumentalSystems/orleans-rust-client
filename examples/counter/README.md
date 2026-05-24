# Counter example

An end-to-end sample: an Orleans silo hosting a trivial `ICounterGrain`, the
bridge in front of it, and a Rust client that calls it both generically and via
a generated typed client.

## Projects

| Project | What it is |
| --- | --- |
| `dotnet/Counter.Abstractions` | Grain interfaces: `ICounterGrain` (string key, incl. the multi-arg `Adjust`), `IAccumulatorGrain` (int64 key), `IRegisterGrain` (GUID key). |
| `dotnet/Counter.Silo` | An Orleans silo hosting the grain implementations. |
| `dotnet/Counter.Bridge` | A bridge host registering the grain invokers. |
| `rust/` | A Rust binary that calls the counter grain through the bridge. |

The three grains exist to exercise all three grain-key kinds and a
multi-argument method end-to-end (see the integration tests).

## Run it

From the repository root, in three terminals:

```sh
# Terminal 1 — silo (localhost clustering: silo port 11111, gateway 30000)
dotnet run --project examples/counter/dotnet/Counter.Silo

# Terminal 2 — bridge (gRPC on http://127.0.0.1:50051, connects to gateway 30000)
dotnet run --project examples/counter/dotnet/Counter.Bridge

# Terminal 3 — Rust client (override the endpoint with ORLEANS_BRIDGE_URL)
cargo run --manifest-path examples/counter/rust/Cargo.toml
```

Expected output from the Rust client ends with:

```text
counter value: 42
```

## Ports and configuration

All ports are configurable via environment variables, so the integration tests
can run many instances with dynamically allocated ports.

| Variable | Used by | Default |
| --- | --- | --- |
| `ORLEANS_SILO_PORT` | silo | `11111` |
| `ORLEANS_GATEWAY_PORT` | silo + bridge | `30000` |
| `ORLEANS_SERVICE_ID` | silo + bridge | `counter-sample` |
| `ORLEANS_CLUSTER_ID` | silo + bridge | `dev` |
| `ASPNETCORE_URLS` | bridge | `http://127.0.0.1:50051` |
| `ORLEANS_BRIDGE_URL` | Rust client | `http://127.0.0.1:50051` |

The silo and bridge must share the same gateway port, service id, and cluster
id to find each other.

## Typed client generation

The Rust binary generates a typed `CounterGrainClient` at build time from
[`rust/orleans-manifest.json`](rust/orleans-manifest.json) using
`orleans-rust-codegen` (see `rust/build.rs`). You can regenerate that manifest
from the grain assembly with the bridge tools:

```sh
dotnet run --project dotnet/OrleansRustBridge.Tools -- \
  manifest \
  --assembly examples/counter/dotnet/Counter.Abstractions/bin/Release/net10.0/Counter.Abstractions.dll \
  --service-id counter-sample --cluster-id dev \
  --out examples/counter/rust/orleans-manifest.json
```

At runtime, the same information is available from the bridge's `GetManifest`
RPC.
