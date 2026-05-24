# orleans-rust-codegen

Manifest-driven generator for typed [`orleans-rust-client`](https://docs.rs/orleans-rust-client)
grain clients, available as both a library and a CLI.

It consumes a manifest emitted by the .NET bridge (`GetManifest` or
`orleans-rust-bridge-tools`) and produces one Rust struct per grain contract,
wrapping a `GrainRef` with typed methods. Type mapping covers the common
primitive .NET types plus nullable types, arrays, and the standard generic
collections (`List<T>` → `Vec<T>`, `Dictionary<K, V>` → `HashMap<K, V>`, ...);
unrecognised types fall back to `serde_json::Value`. Methods with multiple
parameters generate multi-argument functions, and an opt-in mode emits
`<method>_with_context` variants that also return the response context.

## CLI

```sh
orleans-rust-codegen \
  --manifest ./orleans-manifest.json \
  --out ./src/generated.rs \
  --with-response-context        # optional
```

## Library

```rust
use orleans_rust_codegen::{generate, CodegenOptions, Manifest};

let manifest = Manifest::from_json_str(&json)?;
let code = generate(&manifest, &CodegenOptions::default())?;
```

Include the generated file inside a module annotated
`#[allow(dead_code, clippy::all)]`.

See the [repository](https://github.com/MonumentalSystems/orleans-rust-client)
for the full picture.

## License

MIT. Copyright (c) 2026 Monumental Systems, LLC.
