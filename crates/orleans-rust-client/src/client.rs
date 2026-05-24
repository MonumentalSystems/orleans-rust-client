//! The gRPC client over the Orleans bridge.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tonic::transport::{Channel, Endpoint};

use crate::config::{ClientConfig, TlsConfig};
use crate::error::OrleansError;
use crate::generated::pb;
use crate::grain::GrainRef;
use crate::key::GrainKey;
use crate::request_context::RequestContext;
use crate::retry::RetryPolicy;

type BridgeClient = pb::orleans_bridge_client::OrleansBridgeClient<Channel>;

/// A cheaply-cloneable handle to an Orleans bridge.
///
/// Cloning shares the underlying gRPC channel and configuration, so a single
/// connected client can be shared across tasks.
#[derive(Clone)]
pub struct OrleansClient {
    inner: BridgeClient,
    config: Arc<ClientConfig>,
    retry: Arc<RetryPolicy>,
}

/// Borrowed parameters for a single raw invocation.
pub(crate) struct InvokeCall<'a> {
    pub interface_name: &'a str,
    pub grain_type: &'a str,
    pub key: &'a GrainKey,
    pub method: &'a str,
    pub payload: Vec<u8>,
    pub codec: &'a str,
    pub context: &'a RequestContext,
    pub timeout: Option<Duration>,
}

/// The raw result of an [`OrleansClient`] invocation: opaque payload bytes plus
/// any response-context entries the grain produced.
#[derive(Debug, Clone)]
pub struct RawResponse {
    /// Codec-encoded response bytes.
    pub payload: Vec<u8>,
    /// The codec the bridge used to encode `payload`.
    pub codec: String,
    /// Response-context entries returned by the bridge.
    pub response_context: HashMap<String, String>,
}

impl OrleansClient {
    /// Connect to a bridge at `endpoint` using default settings.
    ///
    /// # Errors
    /// Returns [`OrleansError::Transport`] if the channel cannot be
    /// established, or [`OrleansError::InvalidConfig`] for a malformed
    /// endpoint.
    pub async fn connect(endpoint: impl Into<String>) -> Result<Self, OrleansError> {
        Self::from_config(ClientConfig::new(endpoint)).await
    }

    /// Start building a client with non-default settings.
    #[must_use]
    pub fn builder(endpoint: impl Into<String>) -> OrleansClientBuilder {
        OrleansClientBuilder::new(endpoint)
    }

    /// Connect using an explicit [`ClientConfig`] and no retries.
    ///
    /// # Errors
    /// See [`OrleansClient::connect`].
    pub async fn from_config(config: ClientConfig) -> Result<Self, OrleansError> {
        Self::build(config, RetryPolicy::disabled()).await
    }

    async fn build(config: ClientConfig, retry: RetryPolicy) -> Result<Self, OrleansError> {
        if let Some(TlsConfig { .. }) = config.tls {
            return Err(OrleansError::InvalidConfig(
                "TLS support is not implemented in v0; terminate TLS at a proxy \
                 or restrict the bridge to a trusted network (see SECURITY.md)"
                    .to_owned(),
            ));
        }

        let mut endpoint = Endpoint::from_shared(config.endpoint.clone())
            .map_err(|e| OrleansError::InvalidConfig(format!("invalid endpoint: {e}")))?;
        if let Some(connect_timeout) = config.connect_timeout {
            endpoint = endpoint.connect_timeout(connect_timeout);
        }

        let channel = endpoint.connect().await?;
        let mut client = BridgeClient::new(channel);
        if let Some(n) = config.max_decoding_message_size {
            client = client.max_decoding_message_size(n);
        }
        if let Some(n) = config.max_encoding_message_size {
            client = client.max_encoding_message_size(n);
        }

        Ok(Self {
            inner: client,
            config: Arc::new(config),
            retry: Arc::new(retry),
        })
    }

    /// The configuration this client was built with.
    #[must_use]
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Query bridge and cluster identity.
    ///
    /// # Errors
    /// Returns an [`OrleansError`] if the bridge is unreachable.
    pub async fn health(&self) -> Result<pb::HealthResponse, OrleansError> {
        let mut client = self.inner.clone();
        let response = client
            .health(pb::HealthRequest {})
            .await
            .map_err(OrleansError::from_status)?;
        Ok(response.into_inner())
    }

    /// Fetch the contract manifest describing dispatchable grains.
    ///
    /// # Errors
    /// Returns an [`OrleansError`] if the bridge is unreachable.
    pub async fn manifest(&self) -> Result<pb::ContractManifest, OrleansError> {
        let mut client = self.inner.clone();
        let response = client
            .get_manifest(pb::GetManifestRequest {})
            .await
            .map_err(OrleansError::from_status)?;
        Ok(response.into_inner().manifest.unwrap_or_default())
    }

    /// Obtain a reference to a specific grain.
    #[must_use]
    pub fn grain(
        &self,
        interface_name: impl Into<String>,
        grain_type: impl Into<String>,
        key: impl Into<GrainKey>,
    ) -> GrainRef {
        GrainRef::new(
            self.clone(),
            interface_name.into(),
            grain_type.into(),
            key.into(),
        )
    }

    pub(crate) async fn invoke_raw(
        &self,
        call: InvokeCall<'_>,
    ) -> Result<RawResponse, OrleansError> {
        let effective_timeout = call.timeout.unwrap_or(self.config.default_timeout);
        let target = pb::GrainTarget {
            interface_name: call.interface_name.to_owned(),
            grain_type: call.grain_type.to_owned(),
            key: Some(call.key.to_proto()),
        };
        let context_map = call.context.clone().into_map();

        let mut attempt: u32 = 0;
        loop {
            let request = pb::InvokeRequest {
                target: Some(target.clone()),
                method: call.method.to_owned(),
                payload: call.payload.clone(),
                payload_codec: call.codec.to_owned(),
                request_context: context_map.clone(),
                timeout_ms: u32::try_from(effective_timeout.as_millis()).unwrap_or(u32::MAX),
            };

            match self.invoke_once(request, effective_timeout).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    let can_retry = self.retry.is_enabled()
                        && attempt < self.retry.max_retries
                        && error.is_retryable();
                    if !can_retry {
                        return Err(error);
                    }
                    let backoff = self.retry.backoff_for(attempt + 1);
                    if !backoff.is_zero() {
                        tokio::time::sleep(backoff).await;
                    }
                    attempt += 1;
                }
            }
        }
    }

    async fn invoke_once(
        &self,
        request: pb::InvokeRequest,
        timeout: Duration,
    ) -> Result<RawResponse, OrleansError> {
        let mut client = self.inner.clone();
        // The deadline is enforced server-side via `InvokeRequest.timeout_ms`,
        // which lets the bridge return a structured `orleans_timeout`. We do
        // not set a gRPC deadline here: tonic would surface its own expiry as a
        // `Cancelled` status ("Timeout expired"), masking the richer error.
        // Instead we apply a slightly longer client-side backstop so a hung
        // connection still fails rather than hanging forever.
        let request = tonic::Request::new(request);
        let guard = timeout.saturating_add(Duration::from_secs(5));
        let call = client.invoke(request);
        let result = match tokio::time::timeout(guard, call).await {
            Ok(result) => result,
            Err(_) => return Err(OrleansError::Timeout),
        };

        match result {
            Ok(response) => {
                let inner = response.into_inner();
                Ok(RawResponse {
                    payload: inner.payload,
                    codec: inner.payload_codec,
                    response_context: inner.response_context,
                })
            }
            Err(status) => Err(OrleansError::from_status(status)),
        }
    }
}

/// Builder for [`OrleansClient`] with non-default connection settings.
pub struct OrleansClientBuilder {
    config: ClientConfig,
    retry: RetryPolicy,
}

impl OrleansClientBuilder {
    fn new(endpoint: impl Into<String>) -> Self {
        Self {
            config: ClientConfig::new(endpoint),
            retry: RetryPolicy::disabled(),
        }
    }

    /// Set the default per-call deadline.
    #[must_use]
    pub fn default_timeout(mut self, timeout: Duration) -> Self {
        self.config.default_timeout = timeout;
        self
    }

    /// Set the channel connect timeout.
    #[must_use]
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.config.connect_timeout = Some(timeout);
        self
    }

    /// Set the maximum decodable response size in bytes.
    #[must_use]
    pub fn max_decoding_message_size(mut self, bytes: usize) -> Self {
        self.config.max_decoding_message_size = Some(bytes);
        self
    }

    /// Set the maximum encodable request size in bytes.
    #[must_use]
    pub fn max_encoding_message_size(mut self, bytes: usize) -> Self {
        self.config.max_encoding_message_size = Some(bytes);
        self
    }

    /// Set request-context entries applied to every call.
    #[must_use]
    pub fn default_context(mut self, context: RequestContext) -> Self {
        self.config.default_context = context;
        self
    }

    /// Enable a retry policy (disabled by default).
    #[must_use]
    pub fn retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry = policy;
        self
    }

    /// Configure transport security (see [`TlsConfig`]).
    #[must_use]
    pub fn tls(mut self, tls: TlsConfig) -> Self {
        self.config.tls = Some(tls);
        self
    }

    /// Connect using the accumulated settings.
    ///
    /// # Errors
    /// See [`OrleansClient::connect`].
    pub async fn connect(self) -> Result<OrleansClient, OrleansError> {
        OrleansClient::build(self.config, self.retry).await
    }
}
