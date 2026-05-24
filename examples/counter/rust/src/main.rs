//! Example Rust client for the Orleans counter sample.
//!
//! Demonstrates both the generic `invoke_json` API and a typed client
//! generated from the bridge manifest by `orleans-rust-codegen`.

use orleans_rust_client::{GrainKey, OrleansClient};

#[allow(dead_code, clippy::all)]
mod generated {
    include!(concat!(env!("OUT_DIR"), "/counter_client.rs"));
}

use generated::CounterGrainClient;

const INTERFACE: &str = "Counter.Abstractions.ICounterGrain";
const GRAIN_TYPE: &str = "counter";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let endpoint =
        std::env::var("ORLEANS_BRIDGE_URL").unwrap_or_else(|_| "http://127.0.0.1:50051".to_owned());

    let client = OrleansClient::connect(&endpoint).await?;

    let health = client.health().await?;
    println!(
        "connected to bridge {} (service={}, cluster={}, orleans={})",
        health.bridge_version, health.service_id, health.cluster_id, health.orleans_version
    );

    // --- Generic, untyped API ---------------------------------------------
    let counter = client.grain(INTERFACE, GRAIN_TYPE, GrainKey::String("demo".to_owned()));

    counter.invoke_json::<_, ()>("Reset", &()).await?;
    let after_add: i64 = counter.invoke_json("Add", &5_i64).await?;
    let value: i64 = counter.invoke_json("Get", &()).await?;
    println!("generic API: add(5) -> {after_add}, get() -> {value}");
    assert_eq!(after_add, 5);
    assert_eq!(value, 5);

    // --- Generated typed client -------------------------------------------
    let typed = CounterGrainClient::new(client.clone(), "demo-typed");
    typed.reset().await?;
    typed.add(40).await?;
    let two = typed.add(2).await?;
    let total = typed.get().await?;
    println!("typed API: add(40), add(2) -> {two}, get() -> {total}");
    assert_eq!(total, 42);

    println!("counter value: {total}");
    Ok(())
}
