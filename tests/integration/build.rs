use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("PROTOC").is_none()
        && let Ok(protoc) = protoc_bin_vendored::protoc_bin_path()
    {
        // SAFETY: build scripts run single-threaded before any other code.
        unsafe {
            std::env::set_var("PROTOC", protoc);
        }
    }

    let proto_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/counter/proto")
        .canonicalize()?;
    let proto = proto_root.join("counter_messages.proto");
    println!("cargo:rerun-if-changed={}", proto.display());
    prost_build::compile_protos(&[proto], &[proto_root])?;

    // Bridge protocol server stubs, for the in-process mock used by the retry
    // test.
    let bridge_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/orleans-rust-client/proto")
        .canonicalize()?;
    let bridge_proto = bridge_root.join("orleans_bridge.proto");
    println!("cargo:rerun-if-changed={}", bridge_proto.display());
    tonic_build::configure()
        .build_client(false)
        .build_server(true)
        .compile_protos(&[bridge_proto], &[bridge_root])?;

    Ok(())
}
