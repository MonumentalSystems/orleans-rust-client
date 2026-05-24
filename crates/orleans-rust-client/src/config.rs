//! Client configuration.

use std::time::Duration;

use crate::request_context::RequestContext;

/// Default per-call deadline applied when neither the call nor the client
/// overrides it.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Transport security configuration for the gRPC channel.
///
/// Requires the `tls` cargo feature; building a client with a populated
/// `TlsConfig` while the feature is disabled returns
/// [`crate::OrleansError::InvalidConfig`]. With no custom CA, the system's
/// public webpki roots are used. See `SECURITY.md` for deployment guidance.
#[derive(Debug, Clone, Default)]
pub struct TlsConfig {
    /// Expected server domain name (SNI / certificate validation). Defaults to
    /// the endpoint host when unset.
    pub domain_name: Option<String>,
    /// PEM-encoded CA certificate to trust (for private or self-signed CAs).
    /// When `None`, public webpki roots are used.
    pub ca_certificate_pem: Option<Vec<u8>>,
    /// PEM-encoded client certificate and private key for mutual TLS.
    pub client_identity_pem: Option<(Vec<u8>, Vec<u8>)>,
}

impl TlsConfig {
    /// A configuration that validates the server against public webpki roots.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the expected server domain name.
    #[must_use]
    pub fn with_domain_name(mut self, name: impl Into<String>) -> Self {
        self.domain_name = Some(name.into());
        self
    }

    /// Trust a custom (private or self-signed) CA certificate, PEM-encoded.
    #[must_use]
    pub fn with_ca_certificate_pem(mut self, pem: impl Into<Vec<u8>>) -> Self {
        self.ca_certificate_pem = Some(pem.into());
        self
    }

    /// Present a client certificate for mutual TLS (PEM cert + PEM key).
    #[must_use]
    pub fn with_client_identity_pem(
        mut self,
        certificate: impl Into<Vec<u8>>,
        key: impl Into<Vec<u8>>,
    ) -> Self {
        self.client_identity_pem = Some((certificate.into(), key.into()));
        self
    }
}

/// Connection and per-call defaults for an [`crate::OrleansClient`].
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Bridge endpoint, e.g. `http://127.0.0.1:50051`.
    pub endpoint: String,
    /// Default per-call deadline.
    pub default_timeout: Duration,
    /// Timeout for establishing the underlying channel.
    pub connect_timeout: Option<Duration>,
    /// Maximum size of a response message the client will accept.
    pub max_decoding_message_size: Option<usize>,
    /// Maximum size of a request message the client will send.
    pub max_encoding_message_size: Option<usize>,
    /// Request-context entries applied to every call (per-call entries are
    /// overlaid on top of these).
    pub default_context: RequestContext,
    /// Transport security (see [`TlsConfig`]).
    pub tls: Option<TlsConfig>,
    /// Static gRPC metadata (ASCII header name/value pairs) attached to every
    /// request — typically an `authorization` bearer token or an API-key
    /// header validated by a proxy in front of the bridge.
    pub metadata: Vec<(String, String)>,
}

impl ClientConfig {
    /// Create a configuration targeting `endpoint` with default settings.
    #[must_use]
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            default_timeout: DEFAULT_TIMEOUT,
            connect_timeout: Some(Duration::from_secs(10)),
            max_decoding_message_size: Some(16 * 1024 * 1024),
            max_encoding_message_size: Some(16 * 1024 * 1024),
            default_context: RequestContext::new(),
            tls: None,
            metadata: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let config = ClientConfig::new("http://127.0.0.1:50051");
        assert_eq!(config.endpoint, "http://127.0.0.1:50051");
        assert_eq!(config.default_timeout, DEFAULT_TIMEOUT);
        assert!(config.tls.is_none());
        assert!(config.default_context.is_empty());
        assert!(config.max_decoding_message_size.is_some());
        assert!(config.metadata.is_empty());
    }

    #[test]
    fn tls_config_builders_set_fields() {
        let tls = TlsConfig::new()
            .with_domain_name("example.com")
            .with_ca_certificate_pem(b"ca-pem".to_vec())
            .with_client_identity_pem(b"cert".to_vec(), b"key".to_vec());
        assert_eq!(tls.domain_name.as_deref(), Some("example.com"));
        assert_eq!(tls.ca_certificate_pem.as_deref(), Some(&b"ca-pem"[..]));
        let (cert, key) = tls.client_identity_pem.expect("identity set");
        assert_eq!(cert, b"cert");
        assert_eq!(key, b"key");
    }
}
