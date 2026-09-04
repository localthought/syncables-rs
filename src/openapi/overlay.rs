//! An intentionally minimal [OpenAPI Overlay](https://spec.openapis.org/overlay/v1.0.0.html)
//! implementation: `update`/`remove` actions with plain dot-path targets
//! like `$.components`, not the full JSONPath grammar.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::load::{parse_yaml, OpenApiSource};
use crate::error::{Error, Result};

/// A single overlay action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverlayAction {
    /// JSONPath target — `$` or a simple dot-path such as `$.components`.
    pub target: String,
    /// Object to deep-merge onto the target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update: Option<Map<String, Value>>,
    /// When true, delete the target instead of merging.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remove: Option<bool>,
}

/// Metadata of an overlay document.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OverlayInfo {
    /// Overlay title.
    pub title: String,
    /// Overlay version.
    pub version: String,
}

/// An OpenAPI Overlay document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverlayDocument {
    /// Overlay specification version.
    pub overlay: String,
    /// Overlay metadata.
    pub info: OverlayInfo,
    /// Actions to apply, in order.
    pub actions: Vec<OverlayAction>,
}

/// Loads an OpenAPI Overlay document from a YAML/JSON file path or value.
pub async fn load_overlay<'a>(
    source: impl Into<OpenApiSource<'a>> + Send,
) -> Result<OverlayDocument> {
    let raw = match source.into() {
        OpenApiSource::Path(path) => {
            let text = tokio::fs::read_to_string(Path::new(path)).await?;
            parse_yaml(&text)?
        }
        OpenApiSource::Value(value) => value,
    };
    serde_json::from_value(raw).map_err(Error::from)
}

fn deep_merge_value(existing: Option<&Value>, incoming: &Value) -> Value {
    match (existing, incoming) {
        (Some(Value::Object(existing)), Value::Object(incoming)) => {
            let mut merged = existing.clone();
            for (key, value) in incoming {
                merged.insert(key.clone(), deep_merge_value(merged.get(key), value));
            }
            Value::Object(merged)
        }
        _ => incoming.clone(),
    }
}

/// Parses the intentionally small subset of Overlay JSONPath targets this
/// crate supports: `$` (the document root) or a plain dot-path like
/// `$.components.schemas.Foo`. No wildcards, filters, or bracket/array
/// indexing — every overlay this library has needed to apply so far only
/// targets `$.components`.
fn parse_target(target: &str) -> Result<Vec<String>> {
    if target == "$" {
        return Ok(Vec::new());
    }
    if !target.starts_with("$.") || target.contains(['[', ']', '*']) {
        return Err(Error::UnsupportedOverlayTarget(target.to_string()));
    }
    Ok(target[2..].split('.').map(str::to_string).collect())
}

fn navigate<'v>(
    root: &'v mut Value,
    segments: &[String],
    create_missing: bool,
) -> Result<Option<&'v mut Value>> {
    let mut node = root;
    for segment in segments {
        let object = node
            .as_object_mut()
            .ok_or_else(|| Error::OverlayTargetNotAnObject(segment.clone()))?;
        if !object.contains_key(segment) {
            if !create_missing {
                return Ok(None);
            }
            object.insert(segment.clone(), Value::Object(Map::new()));
        }
        let child = object
            .get_mut(segment)
            .expect("segment inserted or already present");
        if !child.is_object() {
            return Err(Error::OverlayTargetNotAnObject(segment.clone()));
        }
        node = child;
    }
    Ok(Some(node))
}

/// Applies an overlay to a document, returning a new document.
///
/// Supports `update` (deep-merged onto the target) and `remove` actions.
pub fn apply_overlay(document: &Value, overlay: &OverlayDocument) -> Result<Value> {
    let mut result = document.clone();

    for action in &overlay.actions {
        let segments = parse_target(&action.target)?;
        if action.remove == Some(true) {
            let Some((key, parent_segments)) = segments.split_last() else {
                return Err(Error::OverlayRemovesRoot);
            };
            if let Some(parent) = navigate(&mut result, parent_segments, false)? {
                if let Some(object) = parent.as_object_mut() {
                    object.shift_remove(key);
                }
            }
        } else if let Some(update) = &action.update {
            let target = navigate(&mut result, &segments, true)?
                .expect("navigate with create_missing always yields a node");
            let object = target
                .as_object_mut()
                .ok_or_else(|| Error::OverlayTargetNotAnObject(action.target.clone()))?;
            for (key, value) in update {
                let merged = deep_merge_value(object.get(key), value);
                object.insert(key.clone(), merged);
            }
        }
    }

    Ok(result)
}
