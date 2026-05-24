//! CLI entry point for the manifest-driven grain client generator.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use orleans_rust_codegen::{CodegenOptions, Manifest, generate};

/// Generate typed Rust grain clients from a bridge manifest.
#[derive(Parser, Debug)]
#[command(name = "orleans-rust-codegen", version, about)]
struct Args {
    /// Path to a manifest JSON file (as emitted by the bridge `GetManifest`
    /// RPC or `OrleansRustBridge.Tools`).
    #[arg(long)]
    manifest: PathBuf,

    /// Output Rust file. Writes to stdout when omitted.
    #[arg(long)]
    out: Option<PathBuf>,

    /// Crate path used to reference the runtime client.
    #[arg(long, default_value = "orleans_rust_client")]
    client_crate: String,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let json = std::fs::read_to_string(&args.manifest)
        .map_err(|e| format!("reading {}: {e}", args.manifest.display()))?;
    let manifest = Manifest::from_json_str(&json)?;

    let options = CodegenOptions {
        client_crate: args.client_crate,
    };
    let code = generate(&manifest, &options)?;

    match args.out {
        Some(path) => {
            std::fs::write(&path, code).map_err(|e| format!("writing {}: {e}", path.display()))?;
            eprintln!("wrote {}", path.display());
        }
        None => print!("{code}"),
    }

    Ok(())
}
