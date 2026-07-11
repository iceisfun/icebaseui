//! Layout & UI-state persistence.
//!
//! A [`Store`] is a flat, string-keyed bag of JSON values saved to disk between
//! runs. Widgets that carry persistable state (split sizes, the active tab, which
//! property groups are collapsed, which tree nodes are expanded, scroll offsets)
//! read and write it through [`Widget::persist_save`](crate::widget::Widget::persist_save)
//! / [`persist_restore`](crate::widget::Widget::persist_restore), keyed by a
//! stable app-assigned id (e.g. `"split.main"`).
//!
//! The [`App`](crate::App) drives it: [`App::with_persistence`](crate::App::with_persistence)
//! loads the store (and window geometry) at startup, restores the widget tree,
//! and saves everything when the window closes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

/// A string-keyed collection of JSON values, optionally backed by a file.
#[derive(Debug, Default, Clone)]
pub struct Store {
    map: BTreeMap<String, Value>,
    path: Option<PathBuf>,
}

impl Store {
    /// An empty, file-less store.
    pub fn new() -> Self {
        Store::default()
    }

    /// Load a store from `path`. A missing or unparseable file yields an empty
    /// store still bound to `path` (so a later [`Store::save`] creates it).
    pub fn load(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let map = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<BTreeMap<String, Value>>(&s).ok())
            .unwrap_or_default();
        Store {
            map,
            path: Some(path),
        }
    }

    /// Store `value` under `key` (silently ignores serialization failure).
    pub fn set<T: Serialize>(&mut self, key: impl Into<String>, value: &T) {
        if let Ok(v) = serde_json::to_value(value) {
            self.map.insert(key.into(), v);
        }
    }

    /// Read and deserialize the value at `key`, if present and the right type.
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.map
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Whether `key` is present.
    pub fn contains(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    /// Write the store to its bound path (pretty JSON). No-op if unbound.
    pub fn save(&self) {
        if let Some(path) = &self.path {
            self.save_to(path);
        }
    }

    /// Write the store to an explicit path.
    pub fn save_to(&self, path: impl AsRef<Path>) {
        if let Ok(text) = serde_json::to_string_pretty(&self.map) {
            let _ = std::fs::write(path, text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_values() {
        let mut store = Store::new();
        store.set("split.main", &vec![260.0f32, 360.0]);
        store.set("tab.inspector", &1usize);
        store.set("group.collapsed", &vec![false, true, false]);

        assert_eq!(
            store.get::<Vec<f32>>("split.main"),
            Some(vec![260.0, 360.0])
        );
        assert_eq!(store.get::<usize>("tab.inspector"), Some(1));
        assert_eq!(store.get::<i32>("missing"), None);
        assert!(store.contains("tab.inspector"));
    }
}
