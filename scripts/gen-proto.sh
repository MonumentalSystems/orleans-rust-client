#!/usr/bin/env bash
#
# Generate the C# protobuf/gRPC sources for the .NET projects by invoking
# `protoc` directly, instead of going through the MSBuild `Grpc.Tools`
# integration.
#
# Why this exists: on arm64 the `protoc` that `Grpc.Tools` bundles segfaults
# (exit 139) when MSBuild spawns it, while the *same* binary runs cleanly from a
# shell. Running protoc here, ahead of the build, sidesteps that crash and gives
# both architectures one consistent codegen path. The generated sources are
# committed under each project's `Generated/` directory, so a plain
# `dotnet build` needs no protoc at all.
#
# Re-run this whenever a `.proto` contract changes, then commit the result.
#
# The protoc binary and the well-known-type includes come from the `Grpc.Tools`
# NuGet package that the .NET build already restores. Override its location with
# GRPC_TOOLS_DIR if your cache lives elsewhere.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# Keep these in lockstep with Directory.Build.props (<GrpcToolsVersion>).
GRPC_TOOLS_VERSION="${GRPC_TOOLS_VERSION:-2.80.0}"
NUGET_PACKAGES="${NUGET_PACKAGES:-$HOME/.nuget/packages}"
GRPC_TOOLS_DIR="${GRPC_TOOLS_DIR:-$NUGET_PACKAGES/grpc.tools/$GRPC_TOOLS_VERSION}"

# Map the host architecture onto the Grpc.Tools tool layout.
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) RID="linux_x64" ;;
  Linux-aarch64 | Linux-arm64) RID="linux_arm64" ;;
  Darwin-x86_64) RID="macosx_x64" ;;
  Darwin-arm64) RID="macosx_arm64" ;;
  *)
    echo "gen-proto: unsupported host $(uname -s)-$(uname -m)" >&2
    exit 1
    ;;
esac

TOOLS="$GRPC_TOOLS_DIR/tools/$RID"
PROTOC="$TOOLS/protoc"
GRPC_PLUGIN="$TOOLS/grpc_csharp_plugin"
WKT_INCLUDE="$GRPC_TOOLS_DIR/build/native/include"

if [[ ! -x "$PROTOC" ]]; then
  cat >&2 <<EOF
gen-proto: protoc not found at $PROTOC

The Grpc.Tools NuGet package must be restored first. Run a restore (for example
\`dotnet restore orleans-rust-client.slnx\`) or set GRPC_TOOLS_DIR to a checkout
that contains tools/$RID/protoc.
EOF
  exit 1
fi

echo "gen-proto: using $PROTOC"

# gen <out_dir> <proto_root> <grpc_services:server|none> <proto_file...>
gen() {
  local out_dir="$1" proto_root="$2" grpc="$3"
  shift 3
  rm -rf "$out_dir"
  mkdir -p "$out_dir"

  local args=(
    --csharp_out="$out_dir"
    -I"$proto_root"
    -I"$WKT_INCLUDE"
  )
  if [[ "$grpc" == "server" ]]; then
    args+=(
      --grpc_out="$out_dir"
      --plugin=protoc-gen-grpc="$GRPC_PLUGIN"
    )
  fi

  "$PROTOC" "${args[@]}" "$@"
  echo "gen-proto: wrote $out_dir/*.cs"
}

# OrleansRustBridge — the bridge gRPC surface (messages + gRPC *server* base).
gen \
  "dotnet/OrleansRustBridge/Generated" \
  "crates/orleans-rust-client/proto" \
  "server" \
  "crates/orleans-rust-client/proto/orleans_bridge.proto"

# Counter example bridge — payload messages only (no gRPC service).
gen \
  "examples/counter/dotnet/Counter.Bridge/Generated" \
  "examples/counter/proto" \
  "none" \
  "examples/counter/proto/counter_messages.proto"

echo "gen-proto: done"
