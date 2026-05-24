use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Fall back to a vendored `protoc` when the caller has not provided one, so
    // the crate builds with no system `protoc` — including on docs.rs and on
    // both x86-64 and arm64.
    if std::env::var_os("PROTOC").is_none()
        && let Ok(protoc) = protoc_bin_vendored::protoc_bin_path()
    {
        // SAFETY: build scripts run single-threaded before any other code.
        unsafe {
            std::env::set_var("PROTOC", protoc);
        }
    }

    let proto_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("proto")
        .canonicalize()?;
    let proto = proto_root.join("orleans_bridge.proto");

    println!("cargo:rerun-if-changed={}", proto.display());

    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(&[proto], &[proto_root])?;

    Ok(())
}
