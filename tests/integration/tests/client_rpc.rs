//! Exercises the happy-path RPC surface of `OrleansClient` against an in-process
//! mock bridge that returns SUCCESSFUL responses. Mirrors the mock-bridge wiring
//! in `retry.rs` (bind 127.0.0.1:0, spawn the server, connect in a retry loop)
//! but its handlers return populated `HealthResponse`/`GetManifestResponse`
//! payloads and echo back `Invoke` payloads. Needs no .NET toolchain, so it runs
//! as a normal test and covers the builder setters, `build`, `request`, `health`,
//! `manifest`, and the `invoke_once` success arm in `client.rs`.

use std::time::Duration;

use orleans_rust_client::{GrainKey, OrleansClient, RequestContext, RetryPolicy};
use tonic::{Request, Response, Status};

#[allow(clippy::all, dead_code)]
mod pb {
    tonic::include_proto!("orleans.bridge.v1");
}

use pb::orleans_bridge_server::{OrleansBridge, OrleansBridgeServer};

/// A mock bridge whose handlers all succeed.
struct SuccessBridge;

#[tonic::async_trait]
impl OrleansBridge for SuccessBridge {
    async fn health(
        &self,
        _request: Request<pb::HealthRequest>,
    ) -> Result<Response<pb::HealthResponse>, Status> {
        Ok(Response::new(pb::HealthResponse {
            status: "healthy".to_owned(),
            service_id: "svc-1".to_owned(),
            cluster_id: "cluster-1".to_owned(),
            bridge_version: "1.2.3".to_owned(),
            orleans_version: "8.0.0".to_owned(),
        }))
    }

    async fn invoke(
        &self,
        request: Request<pb::InvokeRequest>,
    ) -> Result<Response<pb::InvokeResponse>, Status> {
        let inner = request.into_inner();
        let mut response_context = std::collections::HashMap::new();
        response_context.insert("server".to_owned(), "ok".to_owned());
        // Echo the payload and codec straight back so a JSON round-trip decodes.
        Ok(Response::new(pb::InvokeResponse {
            payload: inner.payload,
            payload_codec: inner.payload_codec,
            response_context,
        }))
    }

    async fn get_manifest(
        &self,
        _request: Request<pb::GetManifestRequest>,
    ) -> Result<Response<pb::GetManifestResponse>, Status> {
        Ok(Response::new(pb::GetManifestResponse {
            manifest: Some(pb::ContractManifest {
                service_id: "svc-1".to_owned(),
                cluster_id: "cluster-1".to_owned(),
                bridge_version: "1.2.3".to_owned(),
                schema_version: "v1".to_owned(),
                grains: vec![pb::GrainContract {
                    interface_name: "Sample.IFooGrain".to_owned(),
                    grain_type: "foo".to_owned(),
                    methods: vec![],
                    supported_key_kinds: vec!["string".to_owned()],
                }],
            }),
        }))
    }
}

/// Bind, spawn the mock server, and connect a client built through every
/// builder setter at least once.
async fn connect_full_builder(url: &str) -> anyhow::Result<OrleansClient> {
    let client = loop {
        match OrleansClient::builder(url)
            .default_timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .max_decoding_message_size(8 * 1024 * 1024)
            .max_encoding_message_size(8 * 1024 * 1024)
            .default_context(RequestContext::new().with("tenant", "acme"))
            .metadata("x-test", "1")
            .bearer_token("tok")
            .api_key("x-api-key", "k")
            .retry_policy(RetryPolicy::disabled())
            .connect()
            .await
        {
            Ok(client) => break client,
            Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
        }
    };
    Ok(client)
}

fn spawn_mock() -> anyhow::Result<(String, tokio::task::JoinHandle<()>)> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    drop(listener);

    let server =
        tonic::transport::Server::builder().add_service(OrleansBridgeServer::new(SuccessBridge));
    let handle = tokio::spawn(async move {
        let _ = server.serve(addr).await;
    });
    Ok((format!("http://{addr}"), handle))
}

#[tokio::test(flavor = "multi_thread")]
async fn builder_setters_and_health_succeed() -> anyhow::Result<()> {
    let (url, handle) = spawn_mock()?;
    let client = connect_full_builder(&url).await?;

    // config() reflects a setter we passed.
    assert_eq!(client.config().default_timeout, Duration::from_secs(10));
    assert_eq!(
        client.config().max_decoding_message_size,
        Some(8 * 1024 * 1024)
    );
    assert_eq!(client.config().default_context.get("tenant"), Some("acme"));

    let health = client.health().await?;
    assert_eq!(health.status, "healthy");
    assert_eq!(health.service_id, "svc-1");

    handle.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn manifest_returns_contract() -> anyhow::Result<()> {
    let (url, handle) = spawn_mock()?;
    let client = connect_full_builder(&url).await?;

    let manifest = client.manifest().await?;
    assert_eq!(manifest.service_id, "svc-1");
    assert_eq!(manifest.grains.len(), 1);
    assert_eq!(manifest.grains[0].grain_type, "foo");

    handle.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn invoke_json_round_trip_succeeds() -> anyhow::Result<()> {
    let (url, handle) = spawn_mock()?;
    let client = connect_full_builder(&url).await?;

    let grain = client.grain("Sample.IFooGrain", "foo", GrainKey::String("k".into()));
    // The mock echoes the JSON payload back; sending 42 round-trips to 42.
    let value: i64 = grain.invoke_json("Get", &42i64).await?;
    assert_eq!(value, 42);

    // The mock attaches a response-context entry; verify it surfaces.
    let (echoed, ctx) = grain
        .invoke_json_with_context::<i64, i64>("Get", &7i64)
        .await?;
    assert_eq!(echoed, 7);
    assert_eq!(ctx.get("server").map(String::as_str), Some("ok"));

    handle.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn conservative_retry_policy_connects() -> anyhow::Result<()> {
    // A second builder path: retry_policy(conservative()) plus the tls setter is
    // intentionally omitted here (TLS handshake would need a real cert); this
    // still covers the conservative-policy branch of the builder.
    let (url, handle) = spawn_mock()?;
    let client = loop {
        match OrleansClient::builder(&url)
            .retry_policy(RetryPolicy::conservative())
            .metadata("x-other", "v")
            .connect()
            .await
        {
            Ok(client) => break client,
            Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
        }
    };

    let health = client.health().await?;
    assert_eq!(health.status, "healthy");

    handle.abort();
    Ok(())
}
