# Contributing

Thanks for your interest in `orleans-rust-client`. Contributions of all kinds
are welcome: bug reports, documentation, examples, and code.

## Scope

This project is a **bridge-backed** integration layer. It deliberately does
**not** reimplement the Orleans gateway wire protocol or `Orleans.Serialization`
in Rust (see the README's "Non-goals"). Please open an issue to discuss before
starting work that expands this scope.

## Prerequisites

- A Rust toolchain (stable, edition 2024 / Rust 1.85+). `rustup` is recommended.
- The .NET SDK pinned in `global.json` (`protoc` is bundled via `Grpc.Tools`
  for the .NET build).
- `protoc` on your `PATH` for the Rust build (the `tonic-build` step). On
  Debian/Ubuntu: `apt-get install -y protobuf-compiler`.

## Building and testing

There is no hosted CI; checks run locally. The quickest path is `make check`,
which runs the whole suite. The individual commands are:

```sh
# Rust
cargo build --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# .NET
dotnet build orleans-rust-client.slnx
dotnet format orleans-rust-client.slnx --verify-no-changes

# End-to-end (requires the .NET SDK; starts a silo + bridge)
cargo test -p orleans-bridge-integration --release -- --ignored --nocapture
```

## Conventions

- Rust code must be `rustfmt`-clean and pass `clippy` with `-D warnings`.
- C# code must be `dotnet format`-clean and respect `.editorconfig`.
- Keep the public Rust API and the gRPC contract stable; discuss breaking
  changes in an issue first.
- Add or update tests for behavioral changes.

## Commit and PR process

1. Fork and create a topic branch.
2. Make focused commits with clear messages.
3. Run `make check` locally and ensure it passes.
4. Open a pull request describing the change and its motivation.

By contributing, you agree that your contributions are licensed under the
project's MIT license.
