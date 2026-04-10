use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;

use crate::bindings::log;

pub const DEFAULT_BRANCH: &str = "main";
pub const DEFAULT_MC_VERSION: &str = "1.21.11";
pub const FORM_CLOSE_ANIMATION_MS: u32 = 200;
pub const EVENT_OUTPUT: &str = "commit-output";
pub const EVENT_DONE: &str = "commit-done";

// ── Arg structs ────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunCommitArgs {
    pub save_dir: String,
    pub branch: String,
    pub message: String,
    pub mc_version: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunCheckoutArgs {
    pub save_dir: String,
    pub commit: String,
    pub mc_version: String,
}

#[derive(Serialize)]
pub struct UpsertProfileArgs {
    pub profile: Profile,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunPullArgs {
    pub save_dir: String,
    pub url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunPushArgs {
    pub save_dir: String,
    pub url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunCloneArgs {
    pub save_dir: String,
    pub url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteProfileArgs {
    pub save_dir: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckRepoExistsArgs {
    pub save_dir: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCommitsArgs {
    pub save_dir: String,
}

// ── Domain structs ─────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct Profile {
    pub save_dir: String,
    pub mc_version: String,
    pub branch: String,
    #[serde(default)]
    pub remote_url: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub subject: String,
    pub author: String,
    pub timestamp: String,
}

// ── UI state ───────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
pub enum RightPanel {
    None,
    Commit,
    AddProfile,
    EditProfile(String), // save_dir key
}

// ── Helpers ────────────────────────────────────────────────────────────────

pub fn js_error_to_string(value: JsValue) -> String {
    value
        .as_string()
        .or_else(|| {
            js_sys::JSON::stringify(&value)
                .ok()
                .and_then(|s| s.as_string())
        })
        .unwrap_or_else(|| "unknown JS error".to_string())
}

pub fn to_js<T: Serialize>(value: &T) -> JsValue {
    serde_wasm_bindgen::to_value(value).unwrap_or_else(|e| {
        log(&format!("serialization error: {e}"));
        JsValue::NULL
    })
}
