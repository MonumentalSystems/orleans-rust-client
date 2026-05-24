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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions_pick_the_right_variant() {
        assert_eq!(GrainKey::from("k"), GrainKey::String("k".to_owned()));
        assert_eq!(
            GrainKey::from("k".to_owned()),
            GrainKey::String("k".to_owned())
        );
        assert_eq!(GrainKey::from(7_i64), GrainKey::Int64(7));
        let id = Uuid::nil();
        assert_eq!(GrainKey::from(id), GrainKey::Guid(id));
    }

    #[test]
    fn kind_names_are_stable() {
        assert_eq!(GrainKey::String(String::new()).kind_name(), "string");
        assert_eq!(GrainKey::Int64(0).kind_name(), "int64");
        assert_eq!(GrainKey::Guid(Uuid::nil()).kind_name(), "guid");
    }

    #[test]
    fn to_proto_maps_each_variant() {
        assert!(matches!(
            GrainKey::String("s".into()).to_proto().kind,
            Some(pb::grain_key::Kind::StringKey(ref s)) if s == "s"
        ));
        assert!(matches!(
            GrainKey::Int64(9).to_proto().kind,
            Some(pb::grain_key::Kind::Int64Key(9))
        ));
        let id = Uuid::from_u128(0x1234);
        match GrainKey::Guid(id).to_proto().kind {
            Some(pb::grain_key::Kind::GuidKey(s)) => assert_eq!(s, id.to_string()),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn display_round_trips_value() {
        assert_eq!(GrainKey::Int64(42).to_string(), "42");
        assert_eq!(GrainKey::String("abc".into()).to_string(), "abc");
    }
}
