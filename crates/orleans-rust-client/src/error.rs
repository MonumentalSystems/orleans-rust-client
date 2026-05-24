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

#[cfg(test)]
mod tests {
    use prost::Message as _;
    use tonic::metadata::MetadataValue;

    use super::*;

    fn status_with_bridge_error(error: &pb::BridgeError) -> tonic::Status {
        let mut status = tonic::Status::new(tonic::Code::Unimplemented, "boom");
        let bytes = error.encode_to_vec();
        status
            .metadata_mut()
            .insert_bin(BRIDGE_ERROR_TRAILER, MetadataValue::from_bytes(&bytes));
        status
    }

    #[test]
    fn decodes_structured_trailer() {
        let bridge = pb::BridgeError {
            code: codes::UNKNOWN_METHOD.to_owned(),
            message: "no such method".to_owned(),
            detail: String::new(),
            retryable: false,
        };
        let error = OrleansError::from_status(status_with_bridge_error(&bridge));
        assert_eq!(error.code(), Some(codes::UNKNOWN_METHOD));
        assert!(!error.is_retryable());
        match error {
            OrleansError::Bridge {
                message, detail, ..
            } => {
                assert_eq!(message, "no such method");
                assert_eq!(detail, None);
            }
            other => panic!("expected bridge error, got {other:?}"),
        }
    }

    #[test]
    fn retryable_trailer_is_retryable() {
        let bridge = pb::BridgeError {
            code: codes::ORLEANS_REJECTION.to_owned(),
            message: "rejected".to_owned(),
            detail: "overloaded".to_owned(),
            retryable: true,
        };
        let error = OrleansError::from_status(status_with_bridge_error(&bridge));
        assert!(error.is_retryable());
        assert!(matches!(
            error,
            OrleansError::Bridge {
                detail: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn maps_bare_status_codes() {
        let timeout = OrleansError::from_status(tonic::Status::deadline_exceeded("late"));
        assert!(matches!(timeout, OrleansError::Timeout));

        let unavailable = OrleansError::from_status(tonic::Status::unavailable("down"));
        assert_eq!(unavailable.code(), Some(codes::ORLEANS_UNAVAILABLE));
        assert!(unavailable.is_retryable());

        let other = OrleansError::from_status(tonic::Status::internal("oops"));
        assert!(matches!(other, OrleansError::Status(_)));
        assert_eq!(other.code(), None);
    }

    #[test]
    fn cancelled_status_maps_to_cancelled_code() {
        let err = OrleansError::from_status(tonic::Status::cancelled("stop"));
        assert_eq!(err.code(), Some(codes::CANCELLED));
        assert!(!err.is_retryable());
    }

    #[test]
    fn non_bridge_errors_have_no_code_and_are_not_retryable() {
        let serialization = OrleansError::Serialization("bad".to_owned());
        assert_eq!(serialization.code(), None);
        assert!(!serialization.is_retryable());
        assert!(serialization.to_string().contains("serialization error"));

        let config = OrleansError::InvalidConfig("nope".to_owned());
        assert_eq!(config.code(), None);
        assert!(!config.is_retryable());

        assert_eq!(OrleansError::Timeout.to_string(), "timeout");
        assert!(!OrleansError::Timeout.is_retryable());
    }
}
