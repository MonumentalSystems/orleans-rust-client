//! Error types and the stable bridge error vocabulary.

use crate::generated::pb;

/// Metadata trailer key under which the bridge encodes a [`pb::BridgeError`]
/// for non-OK responses. The `-bin` suffix marks it as binary gRPC metadata.
pub(crate) const BRIDGE_ERROR_TRAILER: &str = "bridge-error-bin";

/// Stable, machine-readable error codes returned by the bridge.
///
/// These strings are part of the bridge contract: clients may match on them.
/// They are intentionally decoupled from gRPC status codes and from any
/// particular .NET exception type.
pub mod codes {
    /// The target interface/grain type is not registered with the bridge.
    pub const UNKNOWN_GRAIN: &str = "unknown_grain";
    /// The grain exists but does not expose the requested method.
    pub const UNKNOWN_METHOD: &str = "unknown_method";
    /// The supplied key kind is not valid for the target grain.
    pub const INVALID_KEY: &str = "invalid_key";
    /// The request payload could not be interpreted under the declared codec.
    pub const INVALID_PAYLOAD: &str = "invalid_payload";
    /// A response value could not be serialized back to the caller.
    pub const SERIALIZATION_ERROR: &str = "serialization_error";
    /// Orleans rejected the message (e.g. overload, placement failure).
    pub const ORLEANS_REJECTION: &str = "orleans_rejection";
    /// The grain call exceeded its deadline.
    pub const ORLEANS_TIMEOUT: &str = "orleans_timeout";
    /// The cluster could not be reached.
    pub const ORLEANS_UNAVAILABLE: &str = "orleans_unavailable";
    /// The grain method threw an application exception.
    pub const APPLICATION_ERROR: &str = "application_error";
    /// The call was cancelled before completion.
    pub const CANCELLED: &str = "cancelled";
    /// An unexpected bridge-internal failure.
    pub const INTERNAL: &str = "internal";
}

/// Errors returned by [`crate::OrleansClient`] and grain calls.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum OrleansError {
    /// The gRPC channel could not be established.
    #[error("transport error: {0}")]
    Transport(#[from] tonic::transport::Error),

    /// A transport-level gRPC status that did not carry structured bridge
    /// error metadata.
    #[error("grpc status: {0}")]
    Status(#[from] tonic::Status),

    /// A request or response payload could not be (de)serialized on the client
    /// side.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// A structured, Orleans-level error reported by the bridge. The `code`
    /// field is one of [`codes`].
    #[error("bridge error {code}: {message}")]
    Bridge {
        /// Stable error code; see [`codes`].
        code: String,
        /// Human-readable description.
        message: String,
        /// Optional additional detail (only populated in dev mode).
        detail: Option<String>,
        /// Whether the caller may safely retry the request.
        retryable: bool,
    },

    /// The call exceeded its client-side deadline.
    #[error("timeout")]
    Timeout,

    /// The client was misconfigured.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}

impl OrleansError {
    /// Convert a gRPC [`tonic::Status`] into an [`OrleansError`], decoding the
    /// structured bridge error trailer when present.
    pub(crate) fn from_status(status: tonic::Status) -> Self {
        if let Some(bridge) = decode_bridge_error(&status) {
            return OrleansError::Bridge {
                code: bridge.code,
                message: bridge.message,
                detail: (!bridge.detail.is_empty()).then_some(bridge.detail),
                retryable: bridge.retryable,
            };
        }

        match status.code() {
            tonic::Code::DeadlineExceeded => OrleansError::Timeout,
            tonic::Code::Cancelled => OrleansError::Bridge {
                code: codes::CANCELLED.to_owned(),
                message: status.message().to_owned(),
                detail: None,
                retryable: false,
            },
            tonic::Code::Unavailable => OrleansError::Bridge {
                code: codes::ORLEANS_UNAVAILABLE.to_owned(),
                message: status.message().to_owned(),
                detail: None,
                retryable: true,
            },
            _ => OrleansError::Status(status),
        }
    }

    /// Whether the failed call is safe to retry. Retryable bridge errors,
    /// `Unavailable`, and bare timeouts are considered transient.
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            OrleansError::Bridge { retryable, .. } => *retryable,
            OrleansError::Status(status) => {
                matches!(status.code(), tonic::Code::Unavailable)
            }
            OrleansError::Timeout => false,
            _ => false,
        }
    }

    /// The stable error code, if this is a structured bridge error.
    #[must_use]
    pub fn code(&self) -> Option<&str> {
        match self {
            OrleansError::Bridge { code, .. } => Some(code.as_str()),
            _ => None,
        }
    }
}

fn decode_bridge_error(status: &tonic::Status) -> Option<pb::BridgeError> {
    let value = status.metadata().get_bin(BRIDGE_ERROR_TRAILER)?;
    let bytes = value.to_bytes().ok()?;
    <pb::BridgeError as prost::Message>::decode(bytes).ok()
}
