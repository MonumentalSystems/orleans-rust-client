//! Request-context propagation.
//!
//! Orleans exposes an ambient `RequestContext` whose entries flow with a grain
//! call. The bridge copies the entries supplied here into Orleans'
//! `RequestContext` before invoking the grain, and clears them afterwards so
//! values never leak between calls.

use std::collections::BTreeMap;

/// An ordered set of string key/value pairs propagated with a grain call.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestContext {
    entries: BTreeMap<String, String>,
}

impl RequestContext {
    /// Create an empty context.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder-style insert.
    #[must_use]
    pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.entries.insert(key.into(), value.into());
        self
    }

    /// Insert or replace an entry.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.entries.insert(key.into(), value.into());
        self
    }

    /// Look up an entry.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }

    /// Whether the context carries no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Iterate over the entries in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Return a new context with `other`'s entries overlaid on top of this
    /// one's. Used to apply per-call overrides over client defaults.
    #[must_use]
    pub fn merged_with(&self, other: &RequestContext) -> RequestContext {
        let mut merged = self.clone();
        for (k, v) in &other.entries {
            merged.entries.insert(k.clone(), v.clone());
        }
        merged
    }

    pub(crate) fn into_map(self) -> std::collections::HashMap<String, String> {
        self.entries.into_iter().collect()
    }
}

impl<K, V> FromIterator<(K, V)> for RequestContext
where
    K: Into<String>,
    V: Into<String>,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        Self {
            entries: iter
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_and_accessors() {
        let ctx = RequestContext::new().with("a", "1").with("b", "2");
        assert_eq!(ctx.len(), 2);
        assert!(!ctx.is_empty());
        assert_eq!(ctx.get("a"), Some("1"));
        assert_eq!(ctx.get("missing"), None);
    }

    #[test]
    fn iter_is_key_ordered() {
        let ctx = RequestContext::new().with("z", "1").with("a", "2");
        let keys: Vec<&str> = ctx.iter().map(|(k, _)| k).collect();
        assert_eq!(keys, vec!["a", "z"]);
    }

    #[test]
    fn merged_with_overlays_other() {
        let base = RequestContext::new().with("a", "1").with("b", "1");
        let over = RequestContext::new().with("b", "2").with("c", "3");
        let merged = base.merged_with(&over);
        assert_eq!(merged.get("a"), Some("1"));
        assert_eq!(merged.get("b"), Some("2"));
        assert_eq!(merged.get("c"), Some("3"));
    }

    #[test]
    fn into_map_preserves_entries() {
        let map = RequestContext::from_iter([("k", "v")]).into_map();
        assert_eq!(map.get("k"), Some(&"v".to_owned()));
    }

    #[test]
    fn set_mutates_in_place_and_overwrites() {
        let mut ctx = RequestContext::new();
        assert!(ctx.is_empty());
        ctx.set("a", "1").set("b", "2").set("a", "overwritten");
        assert_eq!(ctx.len(), 2);
        assert_eq!(ctx.get("a"), Some("overwritten"));
        assert_eq!(ctx.get("b"), Some("2"));
    }
}
