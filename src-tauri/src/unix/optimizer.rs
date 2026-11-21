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

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanResult {
    pub freed_bytes: u64,
    pub files: u64,
    pub skipped: u64,
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

pub fn clean_temp() -> CleanResult {
    CleanResult::default()
}
