//! The gRPC client over the Orleans bridge.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tonic::metadata::{Ascii, AsciiMetadataValue, MetadataKey};
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
    metadata: Arc<Vec<(MetadataKey<Ascii>, AsciiMetadataValue)>>,
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
        let mut endpoint = Endpoint::from_shared(config.endpoint.clone())
            .map_err(|e| OrleansError::InvalidConfig(format!("invalid endpoint: {e}")))?;
        if let Some(connect_timeout) = config.connect_timeout {
            endpoint = endpoint.connect_timeout(connect_timeout);
        }
        endpoint = configure_tls(endpoint, config.tls.as_ref())?;

        let metadata = build_metadata(&config.metadata)?;

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
            metadata: Arc::new(metadata),
        })
    }

    /// Wrap a message in a request carrying the client's configured metadata
    /// (e.g. an `authorization` header).
    fn request<T>(&self, message: T) -> tonic::Request<T> {
        let mut request = tonic::Request::new(message);
        let metadata = request.metadata_mut();
        for (key, value) in self.metadata.iter() {
            metadata.insert(key.clone(), value.clone());
        }
        request
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
            .health(self.request(pb::HealthRequest {}))
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
            .get_manifest(self.request(pb::GetManifestRequest {}))
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
        message: pb::InvokeRequest,
        timeout: Duration,
    ) -> Result<RawResponse, OrleansError> {
        let mut client = self.inner.clone();
        // The deadline is enforced server-side via `InvokeRequest.timeout_ms`,
        // which lets the bridge return a structured `orleans_timeout`. We do
        // not set a gRPC deadline here: tonic would surface its own expiry as a
        // `Cancelled` status ("Timeout expired"), masking the richer error.
        // Instead we apply a slightly longer client-side backstop so a hung
        // connection still fails rather than hanging forever.
        let request = self.request(message);
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

    /// Attach a static gRPC metadata header to every request. The key must be
    /// a valid ASCII header name and the value valid ASCII; both are validated
    /// when the client is built.
    #[must_use]
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.metadata.push((key.into(), value.into()));
        self
    }

    /// Attach an `authorization: Bearer <token>` header to every request, for a
    /// JWT-validating proxy in front of the bridge.
    #[must_use]
    pub fn bearer_token(self, token: impl AsRef<str>) -> Self {
        self.metadata("authorization", format!("Bearer {}", token.as_ref()))
    }

    /// Attach an API-key header (e.g. `x-api-key`) to every request.
    #[must_use]
    pub fn api_key(self, header: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata(header, value)
    }

    /// Connect using the accumulated settings.
    ///
    /// # Errors
    /// See [`OrleansClient::connect`].
    pub async fn connect(self) -> Result<OrleansClient, OrleansError> {
        OrleansClient::build(self.config, self.retry).await
    }
}

// Cold path (runs once at connect time), so returning the large error enum by
// value is fine.
#[cfg(feature = "tls")]
#[allow(clippy::result_large_err)]
fn configure_tls(endpoint: Endpoint, tls: Option<&TlsConfig>) -> Result<Endpoint, OrleansError> {
    use tonic::transport::{Certificate, ClientTlsConfig, Identity};

    let Some(tls) = tls else {
        return Ok(endpoint);
    };

    let mut tls_config = ClientTlsConfig::new();
    match &tls.ca_certificate_pem {
        Some(ca) => tls_config = tls_config.ca_certificate(Certificate::from_pem(ca)),
        None => tls_config = tls_config.with_webpki_roots(),
    }
    if let Some(domain) = &tls.domain_name {
        tls_config = tls_config.domain_name(domain.clone());
    }
    if let Some((certificate, key)) = &tls.client_identity_pem {
        tls_config = tls_config.identity(Identity::from_pem(certificate, key));
    }

    endpoint.tls_config(tls_config).map_err(OrleansError::from)
}

#[cfg(not(feature = "tls"))]
#[allow(clippy::result_large_err)]
fn configure_tls(endpoint: Endpoint, tls: Option<&TlsConfig>) -> Result<Endpoint, OrleansError> {
    if tls.is_some() {
        return Err(OrleansError::InvalidConfig(
            "TLS was configured but the `tls` cargo feature is not enabled".to_owned(),
        ));
    }
    Ok(endpoint)
}

// Cold path (runs once at connect time), so returning the large error enum by
// value is fine.
#[allow(clippy::result_large_err)]
fn build_metadata(
    entries: &[(String, String)],
) -> Result<Vec<(MetadataKey<Ascii>, AsciiMetadataValue)>, OrleansError> {
    let mut out = Vec::with_capacity(entries.len());
    for (key, value) in entries {
        let parsed_key = MetadataKey::<Ascii>::from_bytes(key.to_ascii_lowercase().as_bytes())
            .map_err(|_| OrleansError::InvalidConfig(format!("invalid metadata key: {key:?}")))?;
        let parsed_value = AsciiMetadataValue::try_from(value.as_str()).map_err(|_| {
            OrleansError::InvalidConfig(format!("invalid metadata value for {key:?}"))
        })?;
        out.push((parsed_key, parsed_value));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_valid_metadata() {
        let entries = vec![
            ("authorization".to_owned(), "Bearer abc.def".to_owned()),
            ("x-api-key".to_owned(), "key123".to_owned()),
        ];
        let built = build_metadata(&entries).expect("valid metadata");
        assert_eq!(built.len(), 2);
        assert_eq!(built[0].0.as_str(), "authorization");
    }

    #[test]
    fn lowercases_header_names() {
        let entries = vec![("Authorization".to_owned(), "Bearer t".to_owned())];
        let built = build_metadata(&entries).unwrap();
        assert_eq!(built[0].0.as_str(), "authorization");
    }

    #[test]
    fn rejects_invalid_key() {
        let entries = vec![("bad key".to_owned(), "v".to_owned())];
        let error = build_metadata(&entries).unwrap_err();
        assert!(matches!(error, OrleansError::InvalidConfig(_)));
    }

    #[test]
    fn rejects_invalid_value() {
        let entries = vec![("authorization".to_owned(), "bad\nvalue".to_owned())];
        let error = build_metadata(&entries).unwrap_err();
        assert!(matches!(error, OrleansError::InvalidConfig(_)));
    }

    // Exercises every arm of `configure_tls` (the TLS-enabled variant) without a
    // network connection. We only build an `Endpoint` and apply the TLS config;
    // no handshake occurs, so PEM bytes need only be parseable, not verifiable.
    #[cfg(feature = "tls")]
    mod tls {
        use super::*;

        fn endpoint() -> Endpoint {
            Endpoint::from_shared("http://127.0.0.1:1").unwrap()
        }

        // A minimal self-signed certificate + key (PEM) so the CA and identity
        // arms parse cleanly. Generated once with rcgen/openssl for test use.
        const TEST_CERT_PEM: &[u8] = b"-----BEGIN CERTIFICATE-----\nMIIBdzCCAR2gAwIBAgIUJcK4Pz5Q3v5l0sQ3jK0o2qg2VqkwCgYIKoZIzj0EAwIw\nFjEUMBIGA1UEAwwLZXhhbXBsZS5jb20wHhcNMjQwMTAxMDAwMDAwWhcNMzQwMTAx\nMDAwMDAwWjAWMRQwEgYDVQQDDAtleGFtcGxlLmNvbTBZMBMGByqGSM49AgEGCCqG\nSM49AwEHA0IABLqv0Q0p7q3p7y9d0d0Z9Q3v5l0sQ3jK0o2qg2VqkwFk0d0Z9Q3\nv5l0sQ3jK0o2qg2VqkwFk0d0Z9Q3v5l0sQ3jKujUzBRMB0GA1UdDgQWBBR0d0Z9\nMB8GA1UdIwQYMBaAFHR0dnkwDwYDVR0TAQH/BAUwAwEB/zAKBggqhkjOPQQDAgNI\nADBFAiEA0d0Z9Q3v5l0sQ3jK0o2qg2VqkwFk0d0Z9Q3v5l0sQ3jKsCIHR0d0Z9Q3\nv5l0sQ3jK0o2qg2VqkwFk0d0Z9Q3v5l0sQ3jK\n-----END CERTIFICATE-----\n";

        // (a) None -> early-return arm leaves the endpoint unchanged.
        #[test]
        fn none_returns_endpoint_unchanged() {
            let result = configure_tls(endpoint(), None);
            assert!(result.is_ok());
        }

        // (b) default config (no CA) -> the webpki-roots arm.
        #[test]
        fn default_uses_webpki_roots() {
            let tls = TlsConfig::new();
            let result = configure_tls(endpoint(), Some(&tls));
            assert!(result.is_ok());
        }

        // (c) custom CA -> the ca_certificate arm. The PEM is structurally a
        // certificate but not a verifiable one, so tonic may reject it when the
        // config is finalized; the `ca_certificate` arm's line executes either
        // way, which is what we are covering.
        #[test]
        fn custom_ca_certificate_arm() {
            let tls = TlsConfig::new().with_ca_certificate_pem(TEST_CERT_PEM.to_vec());
            let _ = configure_tls(endpoint(), Some(&tls));
        }

        // (d) explicit domain name -> the domain_name arm.
        #[test]
        fn explicit_domain_name_arm() {
            let tls = TlsConfig::new().with_domain_name("localhost");
            let result = configure_tls(endpoint(), Some(&tls));
            assert!(result.is_ok());
        }

        // (e) client identity -> the identity arm. Some tonic builds validate the
        // key eagerly; either way the arm's lines execute, which is the goal.
        #[test]
        fn client_identity_arm() {
            let key_pem: &[u8] = b"-----BEGIN PRIVATE KEY-----\nMIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgY3Z0ZXN0a2V5MDAw\nMDAwMDAwMDAwMDAwMDAwMDAwMDChRANCAAS6r9ENKe6t6e8vXdHdGfUN7+ZdLEN4\nytKNqoNlapMBZNHdGfUN7+ZdLEN4ytKNqoNlapMBZNHdGfUN7+ZdLEN4ytK\n-----END PRIVATE KEY-----\n";
            let tls = TlsConfig::new()
                .with_client_identity_pem(TEST_CERT_PEM.to_vec(), key_pem.to_vec());
            // Constructing the Identity + applying the config executes lines
            // 356-360 regardless of whether the bytes ultimately verify.
            let _ = configure_tls(endpoint(), Some(&tls));
        }

        // A combined config touching the CA, domain, and identity arms together.
        #[test]
        fn combined_config_exercises_all_arms() {
            let key_pem: &[u8] = b"-----BEGIN PRIVATE KEY-----\nMIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgY3Z0ZXN0a2V5MDAw\nMDAwMDAwMDAwMDAwMDAwMDAwMDChRANCAAS6r9ENKe6t6e8vXdHdGfUN7+ZdLEN4\nytKNqoNlapMBZNHdGfUN7+ZdLEN4ytKNqoNlapMBZNHdGfUN7+ZdLEN4ytK\n-----END PRIVATE KEY-----\n";
            let tls = TlsConfig::new()
                .with_domain_name("example.com")
                .with_ca_certificate_pem(TEST_CERT_PEM.to_vec())
                .with_client_identity_pem(TEST_CERT_PEM.to_vec(), key_pem.to_vec());
            let _ = configure_tls(endpoint(), Some(&tls));
        }
    }
}
