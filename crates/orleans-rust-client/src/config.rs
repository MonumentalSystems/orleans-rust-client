//! Client configuration.

use std::time::Duration;

use crate::request_context::RequestContext;

/// Default per-call deadline applied when neither the call nor the client
/// overrides it.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Placeholder for transport security configuration.
///
/// TLS is a v0 roadmap item. The type exists so the configuration surface is
/// stable, but [`crate::OrleansClient::from_config`] currently rejects a
/// populated value rather than silently connecting in cleartext. See
/// `SECURITY.md` for deployment guidance.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct TlsConfig {
    /// Expected server domain name (SNI / certificate validation).
    pub domain_name: Option<String>,
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
        }
    }
}
