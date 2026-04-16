use crate::bindings::invoke;
use crate::types::DEFAULT_BRANCH;
use crate::types::{CheckRepoExistsArgs, CommitInfo, GetCommitsArgs, Profile, RightPanel, to_js};
use leptos::prelude::*;

use wasm_bindgen::JsValue;

// ── App State ──────────────────────────────────────────────────────────────────

#[derive(Copy, Clone)]
pub struct AppState {
    // Core state
    pub active_profile: RwSignal<Profile>,
    pub output_lines: RwSignal<Vec<String>>,
    pub last_raw_line: RwSignal<String>,
    pub op_start_ms: RwSignal<f64>,
    pub is_running: RwSignal<bool>,
    pub log_exists: RwSignal<bool>,
    pub show_profiles: RwSignal<bool>,
    pub right_panel: RwSignal<RightPanel>,
    pub profiles: RwSignal<Vec<Profile>>,
    pub commits: RwSignal<Vec<CommitInfo>>,
    pub repo_exists: RwSignal<bool>,
    pub form_closing: RwSignal<bool>,
    pub list_instant: RwSignal<bool>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            active_profile: RwSignal::new(Profile {
                save_dir: String::new(),
                branch: DEFAULT_BRANCH.to_string(),
                mc_version: String::new(),
                remote_url: String::new(),
                updated_at: String::new(),
            }),
            output_lines: RwSignal::new(Vec::<String>::new()),
            last_raw_line: RwSignal::new(String::new()),
            op_start_ms: RwSignal::new(0.0_f64),
            is_running: RwSignal::new(false),
            log_exists: RwSignal::new(false),
            show_profiles: RwSignal::new(false),
            right_panel: RwSignal::new(RightPanel::None),
            profiles: RwSignal::new(Vec::<Profile>::new()),
            commits: RwSignal::new(Vec::<CommitInfo>::new()),
            repo_exists: RwSignal::new(false),
            form_closing: RwSignal::new(false),
            list_instant: RwSignal::new(false),
        }
    }
}

// Provide app state to context
pub fn provide_app_state() -> AppState {
    let state = AppState::new();
    provide_context(state);
    state
}

// Get app state from context
pub fn use_app_state() -> AppState {
    use_context::<AppState>().expect("AppState not provided")
}

// ── Actions that modify AppState ───────────────────────────────────────────────

// Load initial data (profiles, git config, log file exists)
pub fn load_initial_data(state: AppState) {
    use leptos::task::spawn_local;

    // Check git user config on startup
    spawn_local(async move {
        if let Ok(result) = invoke("get_git_user_config", JsValue::NULL).await {
            if let Ok(config) =
                serde_wasm_bindgen::from_value::<crate::types::GitUserConfig>(result)
            {
                if config.name.is_empty() || config.email.is_empty() {
                    state.right_panel.set(RightPanel::GitUserConfig);
                }
            }
        }
    });

    // Load profiles
    spawn_local(async move {
        if let Ok(result) = invoke("get_profiles", JsValue::NULL).await {
            if let Ok(p) = serde_wasm_bindgen::from_value::<Vec<Profile>>(result) {
                state.profiles.set(p);
            }
        }
    });

    // Check log file exists
    spawn_local(async move {
        if let Ok(val) = invoke("log_file_exists", JsValue::NULL).await {
            if let Ok(exists) = serde_wasm_bindgen::from_value::<bool>(val) {
                if exists {
                    state.log_exists.set(true);
                }
            }
        }
    });
}

// Refresh repo state when active profile changes
pub fn setup_profile_change_effect(state: AppState) {
    use leptos::task::spawn_local;

    let refresh = move |dir: String| {
        if dir.is_empty() {
            return;
        }
        let dir1 = dir.clone();
        spawn_local(async move {
            let args = to_js(&CheckRepoExistsArgs { save_dir: dir1 });
            if let Ok(val) = invoke("check_repo_exists", args).await {
                if let Some(exists) = val.as_bool() {
                    state.repo_exists.set(exists);
                }
            }
        });
        spawn_local(async move {
            let args = to_js(&GetCommitsArgs { save_dir: dir });
            match invoke("get_commits", args).await {
                Ok(val) => {
                    if let Ok(c) = serde_wasm_bindgen::from_value::<Vec<CommitInfo>>(val) {
                        state.commits.set(c);
                    }
                }
                Err(err) => state.output_lines.update(|l| {
                    l.push(format!(
                        "Failed to load commits: {}",
                        crate::types::js_error_to_string(err)
                    ))
                }),
            }
        });
    };

    Effect::new(move |_| {
        let dir = state.active_profile.get().save_dir;
        state.repo_exists.set(false);
        state.commits.set(vec![]);
        refresh(dir);
    });
}
