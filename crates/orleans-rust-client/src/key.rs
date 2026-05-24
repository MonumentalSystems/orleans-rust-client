//! Grain key types.
//!
//! Orleans grains are addressed by a compound identity. This crate models the
//! three primitive key kinds Orleans supports for single-key grains. Compound
//! keys (`IGrainWithGuidCompoundKey`, `IGrainWithIntegerCompoundKey`) are not
//! modelled in v0.

use uuid::Uuid;

use crate::generated::pb;

/// The primary key used to address an Orleans grain activation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrainKey {
    /// `IGrainWithStringKey`.
    String(String),
    /// `IGrainWithIntegerKey`.
    Int64(i64),
    /// `IGrainWithGuidKey`.
    Guid(Uuid),
}

impl GrainKey {
    /// The stable name of this key kind, matching the values reported in a
    /// grain contract's `supported_key_kinds`.
    #[must_use]
    pub fn kind_name(&self) -> &'static str {
        match self {
            GrainKey::String(_) => "string",
            GrainKey::Int64(_) => "int64",
            GrainKey::Guid(_) => "guid",
        }
    }

    pub(crate) fn to_proto(&self) -> pb::GrainKey {
        let kind = match self {
            GrainKey::String(s) => pb::grain_key::Kind::StringKey(s.clone()),
            GrainKey::Int64(i) => pb::grain_key::Kind::Int64Key(*i),
            GrainKey::Guid(g) => pb::grain_key::Kind::GuidKey(g.to_string()),
        };
        pb::GrainKey { kind: Some(kind) }
    }
}

impl From<String> for GrainKey {
    fn from(value: String) -> Self {
        GrainKey::String(value)
    }
}

impl From<&str> for GrainKey {
    fn from(value: &str) -> Self {
        GrainKey::String(value.to_owned())
    }
}

impl From<i64> for GrainKey {
    fn from(value: i64) -> Self {
        GrainKey::Int64(value)
    }
}

impl From<Uuid> for GrainKey {
    fn from(value: Uuid) -> Self {
        GrainKey::Guid(value)
    }
}

impl std::fmt::Display for GrainKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GrainKey::String(s) => write!(f, "{s}"),
            GrainKey::Int64(i) => write!(f, "{i}"),
            GrainKey::Guid(g) => write!(f, "{g}"),
        }
    }
}
