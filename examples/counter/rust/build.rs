use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("orleans-manifest.json");
    println!("cargo:rerun-if-changed={}", manifest_path.display());

    let json = std::fs::read_to_string(&manifest_path)?;
    let manifest = orleans_rust_codegen::Manifest::from_json_str(&json)?;
    let code = orleans_rust_codegen::generate(&manifest, &Default::default())?;

    let out = PathBuf::from(std::env::var("OUT_DIR")?).join("counter_client.rs");
    std::fs::write(&out, code)?;
    Ok(())
}
