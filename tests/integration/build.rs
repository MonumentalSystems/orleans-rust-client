use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/counter/proto")
        .canonicalize()?;
    let proto = proto_root.join("counter_messages.proto");

    println!("cargo:rerun-if-changed={}", proto.display());
    prost_build::compile_protos(&[proto], &[proto_root])?;
    Ok(())
}
