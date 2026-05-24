//! Covers the client's retry loop deterministically using an in-process mock
//! bridge that always returns a retryable error. Needs no .NET toolchain, so it
//! runs as a normal test.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use orleans_rust_client::{GrainKey, OrleansClient, OrleansError, RetryPolicy, codes};
use tonic::{Request, Response, Status};

#[allow(clippy::all, dead_code)]
mod pb {
    tonic::include_proto!("orleans.bridge.v1");
}

use pb::orleans_bridge_server::{OrleansBridge, OrleansBridgeServer};

struct MockBridge {
    invokes: Arc<AtomicUsize>,
}

#[tonic::async_trait]
impl OrleansBridge for MockBridge {
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
        _request: Request<pb::InvokeRequest>,
    ) -> Result<Response<pb::InvokeResponse>, Status> {
        self.invokes.fetch_add(1, Ordering::SeqCst);
        Err(Status::unavailable("cluster down"))
    }

    async fn get_manifest(
        &self,
        _request: Request<pb::GetManifestRequest>,
    ) -> Result<Response<pb::GetManifestResponse>, Status> {
        Ok(Response::new(pb::GetManifestResponse::default()))
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn retries_retryable_errors_then_fails() -> anyhow::Result<()> {
    let invokes = Arc::new(AtomicUsize::new(0));

    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    drop(listener);

    let mock = MockBridge {
        invokes: invokes.clone(),
    };
    let server = tonic::transport::Server::builder().add_service(OrleansBridgeServer::new(mock));
    let handle = tokio::spawn(async move { server.serve(addr).await });

    let url = format!("http://{addr}");
    let client = loop {
        match OrleansClient::builder(&url)
            .retry_policy(RetryPolicy::conservative())
            .connect()
            .await
        {
            Ok(client) => break client,
            Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
        }
    };

    let grain = client.grain("Sample.IFooGrain", "foo", GrainKey::String("k".into()));
    let result: Result<i64, OrleansError> = grain.invoke_json("Get", &()).await;

    assert!(
        matches!(&result, Err(OrleansError::Bridge { code, retryable, .. })
            if code == codes::ORLEANS_UNAVAILABLE && *retryable),
        "expected a retryable orleans_unavailable error, got {result:?}"
    );
    // conservative policy retries twice, so the mock sees 1 + 2 = 3 attempts.
    assert_eq!(invokes.load(Ordering::SeqCst), 3);

    handle.abort();
    Ok(())
}
