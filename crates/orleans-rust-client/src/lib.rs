//! Rust client bindings for Microsoft Orleans services via an official .NET
//! gRPC bridge.
//!
//! This crate provides an ergonomic, async Rust API for calling Orleans grains
//! **without** reimplementing Orleans' internal gateway protocol or
//! serialization runtime. A small .NET bridge (see the `dotnet/` directory in
//! the repository) hosts the official Orleans [`IClusterClient`] and exposes a
//! generic gRPC surface that this crate talks to.
//!
//! ```no_run
//! use orleans_rust_client::{GrainKey, OrleansClient};
//!
//! # async fn run() -> Result<(), orleans_rust_client::OrleansError> {
//! let client = OrleansClient::connect("http://127.0.0.1:50051").await?;
//!
//! let counter = client.grain(
//!     "Counter.Abstractions.ICounterGrain",
//!     "counter",
//!     GrainKey::String("demo".into()),
//! );
//!
//! let value: i64 = counter.invoke_json("Get", &()).await?;
//! println!("value = {value}");
//! # Ok(())
//! # }
//! ```
//!
//! [`IClusterClient`]: https://learn.microsoft.com/dotnet/orleans/

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod client;
mod config;
mod error;
mod generated;
mod grain;
mod key;
mod request_context;
mod retry;

pub use client::{OrleansClient, OrleansClientBuilder, RawResponse};
pub use config::{ClientConfig, DEFAULT_TIMEOUT, TlsConfig};
pub use error::{OrleansError, codes};
pub use grain::GrainRef;
pub use key::GrainKey;
pub use request_context::RequestContext;
pub use retry::RetryPolicy;

/// Generated protobuf message types for the bridge protocol
/// (`orleans.bridge.v1`).
pub mod pb {
    pub use crate::generated::pb::*;
}
