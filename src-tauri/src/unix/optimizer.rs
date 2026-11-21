//! The optimizer changes Windows settings only. Elsewhere it offers nothing.

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TweakStatus {
    pub id: String,
    pub on: bool,
    pub available: bool,
    pub backed_up: bool,
}

const NOT_HERE: &str = "The optimizer only exists on Windows.";

pub fn status() -> Vec<TweakStatus> {
    Vec::new()
}

pub fn set(_id: &str, _on: bool) -> Result<(), String> {
    Err(NOT_HERE.into())
}

pub fn optimize_all() -> Vec<String> {
    vec![NOT_HERE.into()]
}

pub fn undo_all() -> Vec<String> {
    vec![NOT_HERE.into()]
}
