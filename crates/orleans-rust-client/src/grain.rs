//! Grain references and method invocation.

use std::time::Duration;

use crate::client::{OrleansClient, RawResponse};
use crate::error::OrleansError;
use crate::key::GrainKey;
use crate::request_context::RequestContext;

/// A handle to a specific grain (interface + grain type + key) on which methods
/// can be invoked.
///
/// `GrainRef` is cheap to clone and carries optional per-grain overrides for
/// request context and timeout.
#[derive(Clone)]
pub struct GrainRef {
    client: OrleansClient,
    interface_name: String,
    grain_type: String,
    key: GrainKey,
    context: RequestContext,
    timeout: Option<Duration>,
}

impl GrainRef {
    pub(crate) fn new(
        client: OrleansClient,
        interface_name: String,
        grain_type: String,
        key: GrainKey,
    ) -> Self {
        Self {
            client,
            interface_name,
            grain_type,
            key,
            context: RequestContext::new(),
            timeout: None,
        }
    }

    /// The grain interface name this reference targets.
    #[must_use]
    pub fn interface_name(&self) -> &str {
        &self.interface_name
    }

    /// The grain type alias this reference targets.
    #[must_use]
    pub fn grain_type(&self) -> &str {
        &self.grain_type
    }

    /// The key this reference targets.
    #[must_use]
    pub fn key(&self) -> &GrainKey {
        &self.key
    }

    /// Return a copy of this reference with request-context entries that are
    /// applied to every call made through it.
    #[must_use]
    pub fn with_context(mut self, context: RequestContext) -> Self {
        self.context = context;
        self
    }

    /// Return a copy of this reference with a per-call deadline override.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    fn effective_context(&self) -> RequestContext {
        self.client
            .config()
            .default_context
            .merged_with(&self.context)
    }

    /// Invoke `method` with an opaque, already-encoded payload under `codec`.
    ///
    /// This is the lowest-level entry point; prefer [`GrainRef::invoke_json`]
    /// for typical use.
    ///
    /// # Errors
    /// Returns an [`OrleansError`] on transport failure, timeout, or a
    /// structured bridge error.
    pub async fn invoke(
        &self,
        method: &str,
        payload: Vec<u8>,
        codec: &str,
    ) -> Result<RawResponse, OrleansError> {
        let context = self.effective_context();
        self.client
            .invoke_raw(crate::client::InvokeCall {
                interface_name: &self.interface_name,
                grain_type: &self.grain_type,
                key: &self.key,
                method,
                payload,
                codec,
                context: &context,
                timeout: self.timeout,
            })
            .await
    }

    /// Invoke `method`, serializing `request` to JSON and deserializing the
    /// JSON response.
    ///
    /// Use a unit value (`&()`) for methods that take no argument.
    ///
    /// # Errors
    /// Returns [`OrleansError::Serialization`] if (de)serialization fails, or a
    /// transport/bridge error otherwise.
    #[cfg(feature = "json")]
    pub async fn invoke_json<Req, Resp>(
        &self,
        method: &str,
        request: &Req,
    ) -> Result<Resp, OrleansError>
    where
        Req: serde::Serialize + ?Sized,
        Resp: serde::de::DeserializeOwned,
    {
        let payload =
            serde_json::to_vec(request).map_err(|e| OrleansError::Serialization(e.to_string()))?;
        let response = self.invoke(method, payload, "json").await?;
        serde_json::from_slice(&response.payload)
            .map_err(|e| OrleansError::Serialization(e.to_string()))
    }

    /// Invoke `method` with a protobuf-encoded request and response.
    ///
    /// The payload bytes are opaque to the bridge transport; the bridge's grain
    /// invoker is responsible for decoding them.
    ///
    /// # Errors
    /// Returns [`OrleansError::Serialization`] on decode failure, or a
    /// transport/bridge error otherwise.
    #[cfg(feature = "protobuf")]
    pub async fn invoke_protobuf<Req, Resp>(
        &self,
        method: &str,
        request: &Req,
    ) -> Result<Resp, OrleansError>
    where
        Req: prost::Message,
        Resp: prost::Message + Default,
    {
        let payload = request.encode_to_vec();
        let response = self.invoke(method, payload, "protobuf").await?;
        Resp::decode(response.payload.as_slice())
            .map_err(|e| OrleansError::Serialization(e.to_string()))
    }
}
