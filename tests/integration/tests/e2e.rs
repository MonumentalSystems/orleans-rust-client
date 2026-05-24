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
//!  1. `health_works`      6. `timeout`          11. `integer_key`
//!  2. `manifest_works`    7. `parallel_calls`   12. `guid_key`
//!  3. `counter_smoke`     8. `request_context`  13. `auth_metadata`
//!  4. `unknown_method`    9. `protobuf_invoke`
//!  5. `invalid_payload`  10. `multi_argument`
//!
//! A separate `tls_end_to_end` test covers TLS.

use orleans_bridge_integration::TestCluster;
use orleans_rust_client::{GrainKey, OrleansClient, OrleansError, codes};

#[allow(clippy::all)]
mod counter_messages {
    include!(concat!(env!("OUT_DIR"), "/counter.v1.rs"));
}

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
    protobuf_invoke(&client).await?;
    multi_argument(&client).await?;
    integer_key(&client).await?;
    guid_key(&client).await?;
    auth_metadata(&cluster).await?;

    Ok(())
}

async fn multi_argument(client: &OrleansClient) -> anyhow::Result<()> {
    // `Adjust(delta, floor)` is a two-argument method; the client serializes
    // the tuple as a JSON array the bridge invoker decodes positionally.
    let counter = client.grain(INTERFACE, GRAIN_TYPE, GrainKey::String("multi".into()));
    counter.invoke_json::<_, ()>("Reset", &()).await?;

    let raised: i64 = counter.invoke_json("Adjust", &(5_i64, 0_i64)).await?;
    assert_eq!(raised, 5);

    // delta drops below the floor, so the result clamps to the floor.
    let clamped: i64 = counter.invoke_json("Adjust", &(-100_i64, 2_i64)).await?;
    assert_eq!(clamped, 2);

    println!("[multi_argument] ok: raised={raised}, clamped={clamped}");
    Ok(())
}

async fn integer_key(client: &OrleansClient) -> anyhow::Result<()> {
    // Exercises GrainKey::Int64 against an IGrainWithIntegerKey grain.
    let acc = client.grain(
        "Counter.Abstractions.IAccumulatorGrain",
        "accumulator",
        GrainKey::Int64(42),
    );
    let after_add: i64 = acc.invoke_json("Add", &10_i64).await?;
    let value: i64 = acc.invoke_json("Get", &()).await?;
    assert_eq!(after_add, 10);
    assert_eq!(value, 10);
    println!("[integer_key] ok: value={value}");
    Ok(())
}

async fn guid_key(client: &OrleansClient) -> anyhow::Result<()> {
    // Exercises GrainKey::Guid against an IGrainWithGuidKey grain.
    let id = uuid::Uuid::from_u128(0x0000_0000_0000_4000_8000_0000_0000_0001);
    let register = client.grain(
        "Counter.Abstractions.IRegisterGrain",
        "register",
        GrainKey::Guid(id),
    );
    let set: String = register.invoke_json("Set", &"hello").await?;
    let value: String = register.invoke_json("Get", &()).await?;
    assert_eq!(set, "hello");
    assert_eq!(value, "hello");
    println!("[guid_key] ok: value={value}");
    Ok(())
}

async fn protobuf_invoke(client: &OrleansClient) -> anyhow::Result<()> {
    use counter_messages::{AddRequest, CounterValue};

    // Exercises the optional protobuf codec end-to-end: the request and
    // response are protobuf messages decoded/encoded by the bridge invoker.
    let counter = client.grain(INTERFACE, GRAIN_TYPE, GrainKey::String("protobuf".into()));

    let first: CounterValue = counter
        .invoke_protobuf("Add", &AddRequest { amount: 10 })
        .await?;
    assert_eq!(first.value, 10);

    let second: CounterValue = counter
        .invoke_protobuf("Add", &AddRequest { amount: 32 })
        .await?;
    assert_eq!(second.value, 42);

    println!("[protobuf_invoke] ok: value={}", second.value);
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

    // The runtime manifest carries per-method parameters for multi-arg methods.
    let adjust = counter
        .methods
        .iter()
        .find(|m| m.name == "Adjust")
        .expect("Adjust method present");
    let param_names: Vec<&str> = adjust.parameters.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(param_names, ["delta", "floor"]);

    println!("[manifest_works] ok: methods={method_names:?}, Adjust params={param_names:?}");
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

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the .NET SDK and a built sample; run with --ignored"]
async fn tls_end_to_end() -> anyhow::Result<()> {
    // The bridge generates a dev CA + server cert and serves HTTPS/HTTP2; the
    // client trusts the CA and connects over TLS.
    let cluster = TestCluster::start_tls().await?;
    assert!(
        cluster.bridge_url.starts_with("https://"),
        "expected a TLS endpoint"
    );

    let client = cluster.client().await?;
    let health = client.health().await?;
    assert_eq!(health.status.to_lowercase(), "healthy");

    let counter = client.grain(INTERFACE, GRAIN_TYPE, GrainKey::String("tls".into()));
    counter.invoke_json::<_, ()>("Reset", &()).await?;
    let value: i64 = counter.invoke_json("Add", &7_i64).await?;
    assert_eq!(value, 7);
    println!("[tls_end_to_end] ok");
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
