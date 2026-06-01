//! Exercises [`orleans_rust_client::GrainRef`] end-to-end against an in-process
//! mock bridge that returns *successful* responses. The mock echoes the request
//! payload back as the response payload, mirrors the request `payload_codec`,
//! and attaches a non-empty `response_context`, so we can drive the grain
//! accessors, the context/timeout override builders, and the JSON/protobuf
//! invocation paths without a .NET toolchain.

use std::collections::HashMap;
use std::time::Duration;

use orleans_rust_client::{GrainKey, OrleansClient, OrleansError, RequestContext};
use tonic::{Request, Response, Status};

#[allow(clippy::all, dead_code)]
mod pb {
    tonic::include_proto!("orleans.bridge.v1");
}

use pb::orleans_bridge_server::{OrleansBridge, OrleansBridgeServer};

/// A bridge whose `invoke` echoes the request payload back, unless the method
/// is the special `"NotJson"` sentinel, in which case it returns bytes that are
/// not valid JSON (used to force a response-deserialization failure).
struct EchoBridge;

#[tonic::async_trait]
impl OrleansBridge for EchoBridge {
    async fn health(
        &self,
        _request: Request<pb::HealthRequest>,
    ) -> Result<Response<pb::HealthResponse>, Status> {
        Ok(Response::new(pb::HealthResponse {
            status: "healthy".to_owned(),
            ..Default::default()
        }))
    }

    async fn invoke(
        &self,
        request: Request<pb::InvokeRequest>,
    ) -> Result<Response<pb::InvokeResponse>, Status> {
        let req = request.into_inner();

        let payload = if req.method == "NotJson" {
            b"not-json".to_vec()
        } else {
            req.payload
        };

        Ok(Response::new(pb::InvokeResponse {
            payload,
            payload_codec: req.payload_codec,
            response_context: HashMap::from([("trace".to_owned(), "1".to_owned())]),
        }))
    }

    async fn get_manifest(
        &self,
        _request: Request<pb::GetManifestRequest>,
    ) -> Result<Response<pb::GetManifestResponse>, Status> {
        Ok(Response::new(pb::GetManifestResponse::default()))
    }
}

/// Stand up the mock bridge and return a connected client plus the server task
/// handle. Caller is responsible for aborting the handle.
async fn connect_to_mock() -> anyhow::Result<(OrleansClient, tokio::task::JoinHandle<()>)> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    drop(listener);

    let server =
        tonic::transport::Server::builder().add_service(OrleansBridgeServer::new(EchoBridge));
    let handle = tokio::spawn(async move {
        let _ = server.serve(addr).await;
    });

    let url = format!("http://{addr}");
    let client = loop {
        match OrleansClient::builder(&url).connect().await {
            Ok(client) => break client,
            Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
        }
    };

    Ok((client, handle))
}

#[tokio::test(flavor = "multi_thread")]
async fn accessors_return_constructor_values() -> anyhow::Result<()> {
    let (client, handle) = connect_to_mock().await?;

    let grain = client.grain("Some.IGrain", "gtype", GrainKey::String("k".into()));

    assert_eq!(grain.interface_name(), "Some.IGrain");
    assert_eq!(grain.grain_type(), "gtype");
    assert_eq!(grain.key(), &GrainKey::String("k".into()));

    handle.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn with_context_and_with_timeout_invoke_successfully() -> anyhow::Result<()> {
    let (client, handle) = connect_to_mock().await?;

    let grain = client
        .grain("Some.IGrain", "gtype", GrainKey::String("k".into()))
        .with_context(RequestContext::new().with("tenant", "acme"))
        .with_timeout(Duration::from_secs(5));

    // Echo round-trip proves the override builders produced an invokable grain
    // and the effective_context merge path ran without error.
    let value: String = grain.invoke_json("Say", &"hello".to_owned()).await?;
    assert_eq!(value, "hello");

    handle.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn invoke_json_round_trips_via_echo() -> anyhow::Result<()> {
    let (client, handle) = connect_to_mock().await?;

    let grain = client.grain("Some.IGrain", "gtype", GrainKey::String("k".into()));

    let sent = serde_json::json!({ "n": 7, "label": "seven" });
    let received: serde_json::Value = grain.invoke_json("Get", &sent).await?;
    assert_eq!(received, sent);

    handle.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn invoke_json_with_context_returns_value_and_context() -> anyhow::Result<()> {
    let (client, handle) = connect_to_mock().await?;

    let grain = client.grain("Some.IGrain", "gtype", GrainKey::String("k".into()));

    let sent = 42_i64;
    let (value, context): (i64, HashMap<String, String>) =
        grain.invoke_json_with_context("Get", &sent).await?;

    assert_eq!(value, 42);
    assert_eq!(context.get("trace"), Some(&"1".to_owned()));

    handle.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn invoke_json_surfaces_request_serialization_error() -> anyhow::Result<()> {
    let (client, handle) = connect_to_mock().await?;

    let grain = client.grain("Some.IGrain", "gtype", GrainKey::String("k".into()));

    // A map with integer keys cannot be serialized to JSON (object keys must be
    // strings), so serde_json fails before any RPC is made.
    let bad = HashMap::<i32, i32>::from([(1, 2)]);
    let result: Result<(), OrleansError> = grain.invoke_json("Get", &bad).await;

    assert!(
        matches!(&result, Err(OrleansError::Serialization(_))),
        "expected a serialization error, got {result:?}"
    );

    handle.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn invoke_json_surfaces_response_deserialization_error() -> anyhow::Result<()> {
    let (client, handle) = connect_to_mock().await?;

    let grain = client.grain("Some.IGrain", "gtype", GrainKey::String("k".into()));

    // The mock returns the literal bytes `not-json` for the "NotJson" method,
    // which cannot deserialize into an i64.
    let result: Result<i64, OrleansError> = grain.invoke_json("NotJson", &()).await;

    assert!(
        matches!(&result, Err(OrleansError::Serialization(_))),
        "expected a serialization error, got {result:?}"
    );

    handle.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn invoke_protobuf_round_trips_via_echo() -> anyhow::Result<()> {
    use prost::Message;

    let (client, handle) = connect_to_mock().await?;

    let grain = client.grain("Some.IGrain", "gtype", GrainKey::String("k".into()));

    // HealthResponse has a `status` field, so it encodes to non-trivial bytes
    // and gives the echo round-trip something to verify after decode.
    let request = pb::HealthResponse {
        status: "ping".to_owned(),
        ..Default::default()
    };
    let response: pb::HealthResponse = grain.invoke_protobuf("Echo", &request).await?;

    assert_eq!(response.encode_to_vec(), request.encode_to_vec());
    assert_eq!(response.status, "ping");

    handle.abort();
    Ok(())
}
