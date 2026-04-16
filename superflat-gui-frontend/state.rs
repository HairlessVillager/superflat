use crate::bindings::{invoke, log};
use crate::types::DEFAULT_BRANCH;
use crate::types::{
    CheckRepoExistsArgs, CommitInfo, GetCommitsArgs, Profile, RightPanel,
    RunCheckoutArgs, RunCloneArgs, RunCommitArgs, RunPullArgs, RunPushArgs, UpsertProfileArgs,
    to_js,
};
use leptos::prelude::*;
use leptos::task::spawn_local;

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

    // Helper for remote operations
    pub fn run_remote_op(&self, cmd: &'static str, args: JsValue) {
        self.output_lines.set(Vec::new());
        self.is_running.set(true);
        let p = self.active_profile.get_untracked();
        let state = *self;
        spawn_local(async move {
            if let Err(err) = invoke(cmd, args).await {
                state.output_lines.update(|l| l.push(format!("Error: {}", crate::types::js_error_to_string(err))));
            }
            // Refresh after operation
            let dir = p.save_dir.clone();
            if !dir.is_empty() {
                let args = to_js(&CheckRepoExistsArgs { save_dir: dir.clone() });
                if let Ok(val) = invoke("check_repo_exists", args).await {
                    if let Some(exists) = val.as_bool() {
                        state.repo_exists.set(exists);
                    }
                }
                let args = to_js(&GetCommitsArgs { save_dir: dir });
                match invoke("get_commits", args).await {
                    Ok(val) => {
                        if let Ok(c) = serde_wasm_bindgen::from_value::<Vec<CommitInfo>>(val) {
                            state.commits.set(c);
                        }
                    }
                    Err(_) => {}
                }
            }
        });
    }

    pub fn do_pull(&self) {
        let p = self.active_profile.get_untracked();
        self.op_start_ms.set(js_sys::Date::now());
        self.last_raw_line.set(String::new());
        self.right_panel.set(RightPanel::None);
        let args = to_js(&RunPullArgs { save_dir: p.save_dir.clone(), url: p.remote_url.clone() });
        self.run_remote_op("run_pull", args);
    }

    pub fn do_push(&self) {
        let p = self.active_profile.get_untracked();
        self.op_start_ms.set(js_sys::Date::now());
        self.last_raw_line.set(String::new());
        self.right_panel.set(RightPanel::None);
        let args = to_js(&RunPushArgs { save_dir: p.save_dir.clone(), url: p.remote_url.clone() });
        self.run_remote_op("run_push", args);
    }

    pub fn run_pull(&self) {
        let p = self.active_profile.get_untracked();
        if p.remote_url.is_empty() {
            self.right_panel.set(RightPanel::EditProfile(p.save_dir.clone()));
            self.show_profiles.set(true);
            return;
        }
        self.right_panel.set(RightPanel::ConfirmPull);
    }

    pub fn run_push(&self) {
        let p = self.active_profile.get_untracked();
        if p.remote_url.is_empty() {
            self.right_panel.set(RightPanel::EditProfile(p.save_dir.clone()));
            self.show_profiles.set(true);
            return;
        }
        self.right_panel.set(RightPanel::ConfirmPush);
    }

    pub fn run_clone(&self) {
        let p = self.active_profile.get_untracked();
        if p.save_dir.is_empty() {
            self.output_lines.update(|l| l.push("Error: Please load a profile first".to_string()));
            return;
        }
        if p.remote_url.is_empty() {
            self.output_lines.update(|l| l.push("Error: Remote URL is empty".to_string()));
            return;
        }
        self.op_start_ms.set(js_sys::Date::now());
        self.last_raw_line.set(String::new());
        let args = to_js(&RunCloneArgs { save_dir: p.save_dir.clone(), url: p.remote_url.clone() });
        self.run_remote_op("run_clone", args);
    }

    // Helper to do profile upsert
    fn do_upsert_profile(&self, p: Profile) {
        let profiles = self.profiles;
        spawn_local(async move {
            let args = to_js(&UpsertProfileArgs { profile: p });
            if let Err(err) = invoke("upsert_profile", args).await {
                log(&format!("upsert_profile failed: {}", crate::types::js_error_to_string(err)));
                return;
            }
            if let Ok(result) = invoke("get_profiles", JsValue::NULL).await {
                if let Ok(ps) = serde_wasm_bindgen::from_value::<Vec<Profile>>(result) {
                    profiles.set(ps);
                }
            }
        });
    }

    pub fn run_commit(&self, msg: String, draft_message: RwSignal<String>) {
        if msg.is_empty() {
            self.output_lines.update(|l| l.push("Error: Commit message is empty".to_string()));
            return;
        }
        let p = self.active_profile.get_untracked();
        self.op_start_ms.set(js_sys::Date::now());
        self.last_raw_line.set(String::new());
        self.output_lines.set(Vec::new());
        self.is_running.set(true);
        self.right_panel.set(RightPanel::None);
        draft_message.set(String::new());
        let state = *self;
        spawn_local(async move {
            let args = to_js(&RunCommitArgs {
                save_dir: p.save_dir.clone(),
                branch: p.branch.clone(),
                message: msg,
                mc_version: p.mc_version.clone(),
            });
            if let Err(err) = invoke("run_commit", args).await {
                state.output_lines.update(|l| l.push(format!("Error: {}", crate::types::js_error_to_string(err))));
            }
            state.do_upsert_profile(p);
        });
    }

    pub fn run_checkout(&self, commit: String) {
        let p = self.active_profile.get_untracked();
        if p.save_dir.is_empty() {
            self.output_lines.update(|l| l.push("Error: Please load a profile first".to_string()));
            return;
        }
        self.op_start_ms.set(js_sys::Date::now());
        self.last_raw_line.set(String::new());
        self.output_lines.set(Vec::new());
        self.is_running.set(true);

        let state = *self;
        spawn_local(async move {
            let args = to_js(&RunCheckoutArgs {
                save_dir: p.save_dir.clone(),
                commit,
                mc_version: p.mc_version.clone(),
            });
            if let Err(err) = invoke("run_checkout", args).await {
                state.output_lines.update(|l| l.push(format!("Error: {}", crate::types::js_error_to_string(err))));
            }
            state.do_upsert_profile(p);
        });
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

    // Check git is available first
    spawn_local(async move {
        if let Err(_) = invoke("check_git_available", JsValue::NULL).await {
            state.right_panel.set(RightPanel::GitMissing);
            return;
        }

        // Check git user config on startup
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
