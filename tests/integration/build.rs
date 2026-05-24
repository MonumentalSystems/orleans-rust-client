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
    Ok(())
}
