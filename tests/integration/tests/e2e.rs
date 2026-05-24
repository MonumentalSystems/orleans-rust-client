//! End-to-end integration test against a live silo + bridge.
//!
//! Marked `#[ignore]` because it requires the .NET SDK; run with:
//!
//! ```sh
//! cargo test -p orleans-bridge-integration -- --ignored --nocapture
//! ```
//!
//! A single test owns one [`TestCluster`] (so its processes are cleaned up via
//! `Drop`) and exercises every scenario from the build brief in sequence:
//!
//! 1. `health_works`      4. `unknown_method`   7. `parallel_calls`
//! 2. `manifest_works`    5. `invalid_payload`  8. `request_context`
//! 3. `counter_smoke`     6. `timeout`          9. `auth_metadata`

use orleans_bridge_integration::TestCluster;
use orleans_rust_client::{GrainKey, OrleansClient, OrleansError, codes};

const INTERFACE: &str = "Counter.Abstractions.ICounterGrain";
const GRAIN_TYPE: &str = "counter";

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the .NET SDK and a built sample; run with --ignored"]
async fn counter_end_to_end() -> anyhow::Result<()> {
    let cluster = TestCluster::start().await?;
    let client = cluster.client().await?;

    health_works(&client, &cluster).await?;
    manifest_works(&client).await?;
    counter_smoke(&client).await?;
    unknown_method(&client).await?;
    invalid_payload(&client).await?;
    timeout(&client).await?;
    parallel_calls(&client).await?;
    request_context(&client).await?;
    auth_metadata(&cluster).await?;

    Ok(())
}

async fn health_works(client: &OrleansClient, cluster: &TestCluster) -> anyhow::Result<()> {
    let health = client.health().await?;
    assert_eq!(health.status.to_lowercase(), "healthy");
    assert_eq!(health.service_id, cluster.service_id);
    assert_eq!(health.cluster_id, cluster.cluster_id);
    assert!(!health.orleans_version.is_empty());
    println!("[health_works] ok: {health:?}");
    Ok(())
}

async fn manifest_works(client: &OrleansClient) -> anyhow::Result<()> {
    let manifest = client.manifest().await?;
    let counter = manifest
        .grains
        .iter()
        .find(|g| g.interface_name == INTERFACE)
        .expect("counter grain present in manifest");

    let method_names: Vec<&str> = counter.methods.iter().map(|m| m.name.as_str()).collect();
    for expected in ["Get", "Add", "Reset"] {
        assert!(
            method_names.contains(&expected),
            "manifest missing method {expected}; have {method_names:?}"
        );
    }
    println!("[manifest_works] ok: methods={method_names:?}");
    Ok(())
}

async fn counter_smoke(client: &OrleansClient) -> anyhow::Result<()> {
    let counter = client.grain(INTERFACE, GRAIN_TYPE, GrainKey::String("smoke".into()));
    counter.invoke_json::<_, ()>("Reset", &()).await?;
    let _: i64 = counter.invoke_json("Add", &1_i64).await?;
    let _: i64 = counter.invoke_json("Add", &41_i64).await?;
    let value: i64 = counter.invoke_json("Get", &()).await?;
    assert_eq!(value, 42);
    println!("[counter_smoke] ok: value={value}");
    Ok(())
}

async fn unknown_method(client: &OrleansClient) -> anyhow::Result<()> {
    let counter = client.grain(INTERFACE, GRAIN_TYPE, GrainKey::String("smoke".into()));
    let result: Result<i64, OrleansError> = counter.invoke_json("DoesNotExist", &()).await;
    match result {
        Err(OrleansError::Bridge { code, .. }) => assert_eq!(code, codes::UNKNOWN_METHOD),
        other => panic!("expected unknown_method bridge error, got {other:?}"),
    }
    println!("[unknown_method] ok");
    Ok(())
}

async fn invalid_payload(client: &OrleansClient) -> anyhow::Result<()> {
    let counter = client.grain(INTERFACE, GRAIN_TYPE, GrainKey::String("smoke".into()));
    // "Add" expects a JSON integer; send malformed bytes.
    let result = counter.invoke("Add", b"not-json".to_vec(), "json").await;
    match result {
        Err(OrleansError::Bridge { code, .. }) => assert!(
            code == codes::INVALID_PAYLOAD || code == codes::SERIALIZATION_ERROR,
            "unexpected code {code}"
        ),
        other => panic!("expected invalid_payload/serialization_error, got {other:?}"),
    }
    println!("[invalid_payload] ok");
    Ok(())
}

async fn timeout(client: &OrleansClient) -> anyhow::Result<()> {
    let counter = client
        .grain(INTERFACE, GRAIN_TYPE, GrainKey::String("slow".into()))
        .with_timeout(std::time::Duration::from_millis(100));
    // Delay sleeps for 3000ms server-side, well beyond the 100ms deadline.
    let result: Result<i64, OrleansError> = counter.invoke_json("Delay", &3000_i32).await;
    match result {
        Err(OrleansError::Timeout) => {}
        Err(OrleansError::Bridge { code, .. }) if code == codes::ORLEANS_TIMEOUT => {}
        other => panic!("expected timeout, got {other:?}"),
    }
    println!("[timeout] ok");
    Ok(())
}

async fn parallel_calls(client: &OrleansClient) -> anyhow::Result<()> {
    let mut handles = Vec::new();
    for i in 0..50_i64 {
        let client = client.clone();
        handles.push(tokio::spawn(async move {
            let key = format!("parallel-{i}");
            let counter = client.grain(INTERFACE, GRAIN_TYPE, GrainKey::String(key));
            counter.invoke_json::<_, ()>("Reset", &()).await?;
            let _: i64 = counter.invoke_json("Add", &i).await?;
            let value: i64 = counter.invoke_json("Get", &()).await?;
            Ok::<(i64, i64), OrleansError>((i, value))
        }));
    }

    for handle in handles {
        let (i, value) = handle.await??;
        assert_eq!(value, i, "cross-key contamination for key parallel-{i}");
    }
    println!("[parallel_calls] ok: 50 isolated keys");
    Ok(())
}

async fn request_context(client: &OrleansClient) -> anyhow::Result<()> {
    use orleans_rust_client::RequestContext;
    let counter = client
        .grain(INTERFACE, GRAIN_TYPE, GrainKey::String("ctx".into()))
        .with_context(RequestContext::new().with("caller", "rusty"));
    let who: String = counter.invoke_json("WhoCalled", &()).await?;
    assert_eq!(who, "rusty");
    println!("[request_context] ok: who={who}");
    Ok(())
}

async fn auth_metadata(cluster: &TestCluster) -> anyhow::Result<()> {
    // Build a client that attaches auth headers to every request. The bridge
    // does not validate them (a proxy would); this confirms the headers are
    // valid and transmitted without breaking the call end-to-end.
    let client = OrleansClient::builder(&cluster.bridge_url)
        .bearer_token("integration-test-token")
        .api_key("x-api-key", "abc123")
        .connect()
        .await?;

    let counter = client.grain(INTERFACE, GRAIN_TYPE, GrainKey::String("auth".into()));
    counter.invoke_json::<_, ()>("Reset", &()).await?;
    let value: i64 = counter.invoke_json("Add", &3_i64).await?;
    assert_eq!(value, 3);
    println!("[auth_metadata] ok");
    Ok(())
}
