use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, js_namespace = ["window", "__TAURI__", "event"], js_name = listen)]
    async fn tauri_listen(
        event: &str,
        handler: &Closure<dyn Fn(JsValue)>,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_name = setTimeout)]
    fn set_timeout(closure: &Closure<dyn Fn()>, millis: u32) -> i32;

    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_namespace = ["navigator", "clipboard"], js_name = writeText)]
    fn clipboard_write_text(s: &str);

}

const DEFAULT_BRANCH: &str = "main";
const DEFAULT_MC_VERSION: &str = "1.21.11";
const FORM_CLOSE_ANIMATION_MS: u32 = 200;
const EVENT_OUTPUT: &str = "commit-output";
const EVENT_DONE: &str = "commit-done";
const DEFAULT_REMOTE_HINT: &str = "https://example.com/your-save.git";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunCommitArgs {
    save_dir: String,
    branch: String,
    message: String,
    mc_version: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunCheckoutArgs {
    save_dir: String,
    commit: String,
    mc_version: String,
}

#[derive(Serialize)]
struct UpsertProfileArgs {
    profile: Profile,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunPullArgs {
    save_dir: String,
    url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunPushArgs {
    save_dir: String,
    url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunCloneArgs {
    save_dir: String,
    url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeleteProfileArgs {
    save_dir: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckRepoExistsArgs {
    save_dir: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GetCommitsArgs {
    save_dir: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct Profile {
    save_dir: String,
    mc_version: String,
    branch: String,
    #[serde(default)]
    remote_url: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct CommitInfo {
    hash: String,
    short_hash: String,
    subject: String,
    author: String,
    timestamp: String,
}

fn js_error_to_string(value: JsValue) -> String {
    value
        .as_string()
        .or_else(|| {
            js_sys::JSON::stringify(&value)
                .ok()
                .and_then(|s| s.as_string())
        })
        .unwrap_or_else(|| "unknown JS error".to_string())
}

fn to_js<T: Serialize>(value: &T) -> JsValue {
    serde_wasm_bindgen::to_value(value).unwrap_or_else(|e| {
        log(&format!("serialization error: {e}"));
        JsValue::NULL
    })
}

// Which right-side panel is open
#[derive(Clone, PartialEq)]
enum RightPanel {
    None,
    Commit,
    AddProfile,
    EditProfile(String), // save_dir key
}

#[component]
pub fn App() -> impl IntoView {
    // --- active profile state (single signal for the whole profile) ---
    let (active_profile, set_active_profile) = signal(Profile {
        save_dir: String::new(),
        branch: DEFAULT_BRANCH.to_string(),
        mc_version: DEFAULT_MC_VERSION.to_string(),
        remote_url: String::new(),
    });

    // --- ui state ---
    let (output_lines, set_output_lines) = signal(Vec::<String>::new());
    let (is_running, set_is_running) = signal(false);
    let (show_profiles, set_show_profiles) = signal(false);
    let (right_panel, set_right_panel) = signal(RightPanel::None);
    let (profiles, set_profiles) = signal(Vec::<Profile>::new());
    let (commits, set_commits) = signal(Vec::<CommitInfo>::new());
    let (repo_exists, set_repo_exists) = signal(false);
    let (form_closing, set_form_closing) = signal(false);
    let (list_instant, set_list_instant) = signal(false);
    let (current_action, set_current_action) = signal(String::new());
    let (show_log, set_show_log) = signal(false);
    let log_console_ref = NodeRef::<leptos::html::Pre>::new();

    Effect::new(move |_| {
        let _ = output_lines.get();
        if let Some(el) = log_console_ref.get() {
            el.set_scroll_top(el.scroll_height());
        }
    });

    // Close the Add/Edit form with an exit animation, then switch panel
    let close_form = move |next: RightPanel| {
        set_form_closing.set(true);
        set_list_instant.set(true);
        let cb = Closure::<dyn Fn()>::new(move || {
            set_right_panel.set(next.clone());
            set_form_closing.set(false);
            set_list_instant.set(false);
        });
        set_timeout(&cb, FORM_CLOSE_ANIMATION_MS);
        cb.forget();
    };

    // commit panel draft
    let (draft_message, set_draft_message) = signal(String::new());

    // profile form draft
    let (form_save_dir, set_form_save_dir) = signal(String::new());
    let (form_branch, set_form_branch) = signal(DEFAULT_BRANCH.to_string());
    let (form_mc_version, set_form_mc_version) = signal(DEFAULT_MC_VERSION.to_string());
    let (form_remote_url, set_form_remote_url) = signal(String::new());

    // Fetch both repo_exists and commits for a given directory in one async task.
    let refresh_repo_state = move |dir: String| {
        if dir.is_empty() {
            return;
        }
        spawn_local(async move {
            let args = to_js(&CheckRepoExistsArgs {
                save_dir: dir.clone(),
            });
            if let Ok(val) = invoke("check_repo_exists", args).await {
                if let Some(exists) = val.as_bool() {
                    set_repo_exists.set(exists);
                }
            }
            let args = to_js(&GetCommitsArgs { save_dir: dir });
            match invoke("get_commits", args).await {
                Ok(val) => {
                    if let Ok(c) = serde_wasm_bindgen::from_value::<Vec<CommitInfo>>(val) {
                        set_commits.set(c);
                    }
                }
                Err(err) => {
                    set_output_lines.update(|l| {
                        l.push(format!(
                            "Failed to load commits: {}",
                            js_error_to_string(err)
                        ))
                    });
                }
            }
        });
    };

    // Reactively check repo + load commits when save_dir changes
    Effect::new(move |_| {
        let dir = active_profile.get().save_dir;
        set_repo_exists.set(false);
        set_commits.set(vec![]);
        refresh_repo_state(dir);
    });

    // Load profiles on mount
    spawn_local(async move {
        if let Ok(result) = invoke("get_profiles", JsValue::NULL).await {
            if let Ok(p) = serde_wasm_bindgen::from_value::<Vec<Profile>>(result) {
                set_profiles.set(p);
            }
        }
    });

    // Backend output listeners
    spawn_local(async move {
        let set_lines = set_output_lines;
        let on_output = Closure::<dyn Fn(JsValue)>::new(move |event: JsValue| {
            let payload = js_sys::Reflect::get(&event, &JsValue::from_str("payload"))
                .unwrap_or(JsValue::NULL);
            if let Some(line) = payload.as_string() {
                set_lines.update(|lines| lines.push(line));
            }
        });
        let set_running = set_is_running;
        let set_action = set_current_action;
        let on_done = Closure::<dyn Fn(JsValue)>::new(move |_: JsValue| {
            set_running.set(false);
            set_action.set(String::new());
            refresh_repo_state(active_profile.get_untracked().save_dir);
        });
        if let (Ok(_), Ok(_)) = (
            tauri_listen(EVENT_OUTPUT, &on_output).await,
            tauri_listen(EVENT_DONE, &on_done).await,
        ) {
            on_output.forget();
            on_done.forget();
        }
    });

    // helpers
    let do_upsert_profile = move |p: Profile| {
        set_profiles.update(|ps| {
            if let Some(existing) = ps.iter_mut().find(|x| x.save_dir == p.save_dir) {
                *existing = p.clone();
            } else {
                ps.push(p.clone());
            }
        });
        spawn_local(async move {
            let args = to_js(&UpsertProfileArgs { profile: p });
            if let Err(err) = invoke("upsert_profile", args).await {
                log(&format!(
                    "upsert_profile failed: {}",
                    js_error_to_string(err)
                ));
            }
        });
    };

    let run_commit = move |_: leptos::ev::MouseEvent| {
        let p = active_profile.get_untracked();
        if p.save_dir.is_empty() {
            set_output_lines.update(|l| l.push("Error: Please load a profile first".to_string()));
            return;
        }
        let message_val = draft_message.get_untracked();
        if message_val.is_empty() {
            set_output_lines.update(|l| l.push("Error: Commit message is empty".to_string()));
            return;
        }
        set_output_lines.set(Vec::new());
        set_is_running.set(true);
        set_current_action.set("Committing save".to_string());
        set_right_panel.set(RightPanel::None);
        set_draft_message.set(String::new());
        spawn_local(async move {
            let args = to_js(&RunCommitArgs {
                save_dir: p.save_dir.clone(),
                branch: p.branch.clone(),
                message: message_val,
                mc_version: p.mc_version.clone(),
            });
            if let Err(err) = invoke("run_commit", args).await {
                set_output_lines.update(|l| l.push(format!("Error: {}", js_error_to_string(err))));
            }
            do_upsert_profile(p);
        });
    };

    let run_checkout = move |commit: String| {
        let p = active_profile.get_untracked();
        if p.save_dir.is_empty() {
            set_output_lines.update(|l| l.push("Error: Please load a profile first".to_string()));
            return;
        }
        set_output_lines.set(Vec::new());
        set_is_running.set(true);
        set_current_action.set(format!("Checking out {}", commit));
        spawn_local(async move {
            let args = to_js(&RunCheckoutArgs {
                save_dir: p.save_dir.clone(),
                commit,
                mc_version: p.mc_version.clone(),
            });
            if let Err(err) = invoke("run_checkout", args).await {
                set_output_lines.update(|l| l.push(format!("Error: {}", js_error_to_string(err))));
            }
            do_upsert_profile(p);
        });
    };

    let run_pull = move |_: leptos::ev::MouseEvent| {
        let p = active_profile.get_untracked();
        if p.save_dir.is_empty() {
            set_output_lines.update(|l| l.push("Error: Please load a profile first".to_string()));
            return;
        }
        if p.remote_url.is_empty() {
            set_output_lines.update(|l| l.push("Error: Remote URL is empty".to_string()));
            return;
        }
        set_output_lines.set(Vec::new());
        set_is_running.set(true);
        set_current_action.set("Fetching remote history".to_string());
        spawn_local(async move {
            let args = to_js(&RunPullArgs {
                save_dir: p.save_dir.clone(),
                url: p.remote_url.clone(),
            });
            if let Err(err) = invoke("run_pull", args).await {
                set_output_lines.update(|l| l.push(format!("Error: {}", js_error_to_string(err))));
            }
            do_upsert_profile(p);
        });
    };

    let run_push = move |_: leptos::ev::MouseEvent| {
        let p = active_profile.get_untracked();
        if p.save_dir.is_empty() {
            set_output_lines.update(|l| l.push("Error: Please load a profile first".to_string()));
            return;
        }
        if p.remote_url.is_empty() {
            set_output_lines.update(|l| l.push("Error: Remote URL is empty".to_string()));
            return;
        }
        set_output_lines.set(Vec::new());
        set_is_running.set(true);
        set_current_action.set("Pushing history".to_string());
        spawn_local(async move {
            let args = to_js(&RunPushArgs {
                save_dir: p.save_dir.clone(),
                url: p.remote_url.clone(),
            });
            if let Err(err) = invoke("run_push", args).await {
                set_output_lines.update(|l| l.push(format!("Error: {}", js_error_to_string(err))));
            }
            do_upsert_profile(p);
        });
    };

    let run_clone = move |_: leptos::ev::MouseEvent| {
        let p = active_profile.get_untracked();
        if p.save_dir.is_empty() {
            set_output_lines.update(|l| l.push("Error: Please load a profile first".to_string()));
            return;
        }
        if p.remote_url.is_empty() {
            set_output_lines.update(|l| l.push("Error: Remote URL is empty".to_string()));
            return;
        }
        set_output_lines.set(Vec::new());
        set_is_running.set(true);
        set_current_action.set("Cloning remote repository".to_string());
        spawn_local(async move {
            let args = to_js(&RunCloneArgs {
                save_dir: p.save_dir.clone(),
                url: p.remote_url.clone(),
            });
            if let Err(err) = invoke("run_clone", args).await {
                set_output_lines.update(|l| l.push(format!("Error: {}", js_error_to_string(err))));
            }
            do_upsert_profile(p);
        });
    };

    let open_add_profile = move |_: leptos::ev::MouseEvent| {
        set_form_save_dir.set(String::new());
        set_form_branch.set(DEFAULT_BRANCH.to_string());
        set_form_mc_version.set(DEFAULT_MC_VERSION.to_string());
        set_form_remote_url.set(String::new());
        set_right_panel.set(RightPanel::AddProfile);
        set_show_profiles.set(true);
    };

    let save_profile_form = move |_: leptos::ev::MouseEvent| {
        let p = Profile {
            save_dir: form_save_dir.get_untracked(),
            branch: form_branch.get_untracked(),
            mc_version: form_mc_version.get_untracked(),
            remote_url: form_remote_url.get_untracked(),
        };
        if p.save_dir.is_empty() {
            return;
        }
        do_upsert_profile(p);
        close_form(RightPanel::None);
    };

    let handle_window_minimize = move |_: leptos::ev::MouseEvent| {
        spawn_local(async move {
            if let Err(err) = invoke("window_minimize", JsValue::NULL).await {
                log(&format!("minimize failed: {}", js_error_to_string(err)));
            }
        });
    };

    let handle_window_toggle_maximize = move |_: leptos::ev::MouseEvent| {
        spawn_local(async move {
            if let Err(err) = invoke("window_toggle_maximize", JsValue::NULL).await {
                log(&format!("toggle maximize failed: {}", js_error_to_string(err)));
            }
        });
    };

    let handle_window_close = move |_: leptos::ev::MouseEvent| {
        spawn_local(async move {
            if let Err(err) = invoke("window_close", JsValue::NULL).await {
                log(&format!("close failed: {}", js_error_to_string(err)));
            }
        });
    };

    let handle_window_drag = move |_: leptos::ev::MouseEvent| {
        spawn_local(async move {
            if let Err(err) = invoke("window_start_dragging", JsValue::NULL).await {
                log(&format!("start dragging failed: {}", js_error_to_string(err)));
            }
        });
    };

    view! {
        <div class="app">

            // ── Profiles list modal ─────────────────────────────────
            <div class="sidebar"
                class:open=move || show_profiles.get() && !matches!(right_panel.get(), RightPanel::AddProfile | RightPanel::EditProfile(_))
                class:no-transition=move || list_instant.get()
            >
                <div class="sidebar-panel-list">
                    <div class="sidebar-header">
                        <span class="sidebar-title">"Profiles"</span>
                        <button class="sidebar-close" on:click=move |_| set_show_profiles.set(false)>"✕"</button>
                    </div>
                    <button class="btn-add-profile" on:click=open_add_profile>
                        "+ Add Profile"
                    </button>
                    <Show
                        when=move || profiles.get().is_empty()
                        fallback=|| view! {}
                    >
                        <p class="sidebar-empty">"No profiles yet"</p>
                    </Show>
                    <div class="profile-list">
                        <For
                            each=move || profiles.get()
                            key=|p| p.save_dir.clone()
                            children=move |p| {
                                let p_edit = p.clone();
                                let dir_remove = p.save_dir.clone();
                                view! {
                                    <div
                                        class="profile-card"
                                        on:click=move |_| {
                                            set_active_profile.set(p.clone());
                                            set_show_profiles.set(false);
                                            set_right_panel.set(RightPanel::None);
                                        }
                                    >
                                        <div class="profile-card-path">{p.save_dir.clone()}</div>
                                        <div class="profile-card-meta">
                                            {format!("{} · {}", p.branch, p.mc_version)}
                                        </div>
                                        <div class="profile-card-actions">
                                            <button class="btn-edit" on:click=move |ev| {
                                                ev.stop_propagation();
                                                set_form_save_dir.set(p_edit.save_dir.clone());
                                                set_form_branch.set(p_edit.branch.clone());
                                                set_form_mc_version.set(p_edit.mc_version.clone());
                                                set_form_remote_url.set(p_edit.remote_url.clone());
                                                set_right_panel.set(RightPanel::EditProfile(p_edit.save_dir.clone()));
                                            }>
                                                "Edit"
                                            </button>
                                            <button class="btn-remove" on:click=move |ev| {
                                                ev.stop_propagation();
                                                set_profiles.update(|ps| ps.retain(|x| x.save_dir != dir_remove));
                                                let dir = dir_remove.clone();
                                                spawn_local(async move {
                                                    let args = to_js(&DeleteProfileArgs { save_dir: dir });
                                                    if let Err(err) = invoke("delete_profile", args).await {
                                                        log(&format!("delete_profile failed: {}", js_error_to_string(err)));
                                                    }
                                                });
                                            }>
                                                "Remove"
                                            </button>
                                        </div>
                                    </div>
                                }
                            }
                        />
                    </div>
                </div>
            </div>

            // ── Add / Edit profile modal ─────────────────────────────
            <div class="sidebar" class:open=move || (matches!(right_panel.get(), RightPanel::AddProfile | RightPanel::EditProfile(_)) || form_closing.get()) && show_profiles.get()>
                <div class="sidebar-panel-form" class:closing=move || form_closing.get()>
                    <div class="sidebar-header">
                        <span class="sidebar-title">
                            {move || if right_panel.get() == RightPanel::AddProfile { "Add Profile" } else { "Edit Profile" }}
                        </span>
                        <button class="sidebar-close" on:click=move |_| close_form(RightPanel::None)>"✕"</button>
                    </div>
                    <div class="panel-body">
                        <label class="panel-label">
                            "Save directory"
                            <div class="panel-dir-row">
                                <input
                                    type="text"
                                    prop:value=move || form_save_dir.get()
                                    on:input=move |ev| set_form_save_dir.set(event_target_value(&ev))
                                    placeholder="Path to save directory"
                                    disabled=move || matches!(right_panel.get(), RightPanel::EditProfile(_))
                                />
                                <button class="btn-browse" on:click=move |_| {
                                    spawn_local(async move {
                                        if let Ok(result) = invoke("pick_directory", JsValue::NULL).await {
                                            if let Some(path) = result.as_string() {
                                                set_form_save_dir.set(path);
                                            }
                                        }
                                    });
                                }>
                                    "Browse"
                                </button>
                            </div>
                        </label>
                        <label class="panel-label">
                            "Branch"
                            <input
                                type="text"
                                prop:value=move || form_branch.get()
                                on:input=move |ev| set_form_branch.set(event_target_value(&ev))
                                placeholder="main"
                            />
                        </label>
                        <label class="panel-label">
                            "MC Version"
                            <input
                                type="text"
                                prop:value=move || form_mc_version.get()
                                on:input=move |ev| set_form_mc_version.set(event_target_value(&ev))
                                placeholder="e.g. 1.21.11"
                            />
                        </label>
                        <label class="panel-label">
                            "Remote URL"
                            <input
                                type="text"
                                prop:value=move || form_remote_url.get()
                                on:input=move |ev| set_form_remote_url.set(event_target_value(&ev))
                                placeholder=DEFAULT_REMOTE_HINT
                            />
                        </label>
                        // Load profile button (only in edit mode)
                        <Show when=move || matches!(right_panel.get(), RightPanel::EditProfile(_))>
                            <button class="btn-load-profile" on:click=move |_| {
                                set_active_profile.set(Profile {
                                    save_dir: form_save_dir.get_untracked(),
                                    branch: form_branch.get_untracked(),
                                    mc_version: form_mc_version.get_untracked(),
                                    remote_url: form_remote_url.get_untracked(),
                                });
                                set_show_profiles.set(false);
                                close_form(RightPanel::None);
                            }>
                                "Load this profile"
                            </button>
                        </Show>
                        <button
                            class="btn-panel-primary"
                            on:click=save_profile_form
                        >
                            "Save"
                        </button>
                    </div>
                </div>
            </div>

            // ── Overlay ─────────────────────────────────────────────
            <Show when=move || show_profiles.get() || right_panel.get() == RightPanel::Commit>
                <div class="sidebar-overlay" on:click=move |_| {
                    if right_panel.get_untracked() == RightPanel::Commit {
                        set_right_panel.set(RightPanel::None);
                    } else {
                        set_show_profiles.set(false);
                        if matches!(right_panel.get_untracked(), RightPanel::AddProfile | RightPanel::EditProfile(_)) {
                            close_form(RightPanel::None);
                        }
                    }
                }/>
            </Show>

            // ── Main content ────────────────────────────────────────
            <div class="main">
                <div class="window-titlebar">
                    <div class="window-title-drag" data-tauri-drag-region=true on:mousedown=handle_window_drag>
                        <span class="window-title">"Superflat - Minecraft Save Backup"</span>
                    </div>
                    <div class="window-controls">
                        <button class="window-btn" on:click=handle_window_minimize title="Minimize">"—"</button>
                        <button class="window-btn" on:click=handle_window_toggle_maximize title="Maximize / Restore">"□"</button>
                        <button class="window-btn window-btn-close" on:click=handle_window_close title="Close">"✕"</button>
                    </div>
                </div>

                // ── Top bar ─────────────────────────────────────────
                <div class="topbar">
                    <button
                        class="btn-menu"
                        on:click=move |_| set_show_profiles.update(|v| *v = !*v)
                        disabled=move || is_running.get()
                    >
                        "☰"
                    </button>
                    <div class="topbar-dir-wrap">
                        <Show when=move || !active_profile.get().save_dir.is_empty()>
                            <div class="topbar-dir-display" title=move || active_profile.get().save_dir>
                                <span class="topbar-dir-name">
                                    {move || {
                                        let d = active_profile.get().save_dir;
                                        d.trim_end_matches(['/', '\\'])
                                            .rsplit(['/', '\\'])
                                            .next()
                                            .unwrap_or(&d)
                                            .to_string()
                                    }}
                                </span>
                            </div>
                        </Show>
                        <Show when=move || active_profile.get().save_dir.is_empty()>
                            <div class="topbar-dir-placeholder">
                                "No profile loaded (click ☰ to create/load)"
                            </div>
                        </Show>
                    </div>
                    <div class="topbar-actions">
                        <button
                            class="btn-action btn-commit"
                            on:click=move |_| {
                                if right_panel.get_untracked() == RightPanel::Commit {
                                    set_right_panel.set(RightPanel::None);
                                } else {
                                    set_right_panel.set(RightPanel::Commit);
                                }
                            }
                            disabled=move || is_running.get() || active_profile.get().save_dir.is_empty()
                        >
                            "Commit"
                        </button>
                        <Show
                            when=move || repo_exists.get()
                            fallback=move || view! {
                                <button
                                    class="btn-action btn-clone"
                                    on:click=run_clone
                                    disabled=move || is_running.get() || active_profile.get().save_dir.is_empty()
                                >
                                    "Clone"
                                </button>
                            }
                        >
                            <button
                                class="btn-action btn-pull"
                                on:click=run_pull
                                disabled=move || is_running.get() || active_profile.get().save_dir.is_empty()
                            >
                                "Pull"
                            </button>
                        </Show>
                        <Show when=move || repo_exists.get()>
                            <button
                                class="btn-action btn-push"
                                on:click=run_push
                                disabled=move || is_running.get() || active_profile.get().save_dir.is_empty()
                            >
                                "Push"
                            </button>
                        </Show>
                    </div>
                </div>

                // ── Body: commit list + right panel ──────────────────
                <div class="body">

                    // ── Commit list ─────────────────────────────────
                    <div class="commit-area">
                        <Show
                            when=move || commits.get().is_empty()
                            fallback=|| view! {}
                        >
                            <div class="commit-empty">
                                <span>"Load a profile to view history and operation logs"</span>
                            </div>
                        </Show>
                        <div class="commit-list">
                            <For
                                each=move || commits.get()
                                key=|c| c.hash.clone()
                                children=move |c| {
                                    let hash = c.hash.clone();
                                    view! {
                                        <div class="commit-row">
                                            <div class="commit-info">
                                                <div class="commit-subject">{c.subject.clone()}</div>
                                                <div class="commit-meta">
                                                    {format!("{} {}  {}", c.timestamp, c.author, c.short_hash)}
                                                </div>
                                            </div>
                                            <button
                                                class="btn-checkout"
                                                disabled=move || is_running.get()
                                                on:click=move |_| run_checkout(hash.clone())
                                            >
                                                "Checkout"
                                            </button>
                                        </div>
                                    }
                                }
                            />
                        </div>
                    </div>

                    // ── Right panel ─────────────────────────────────
                </div>

                // ── Status bar ──────────────────────────────────
                <div class="status-bar">
                    <Show when=move || repo_exists.get() && active_profile.get().remote_url.is_empty()>
                        <span class="status-bar-item warn">"⚠ No Remote"</span>
                    </Show>
                    <Show when=move || is_running.get()>
                        <button class="status-bar-btn running" on:click=move |_| set_show_log.set(true)>
                            {move || format!("⟳ {}", current_action.get())}
                        </button>
                    </Show>
                    <Show when=move || !is_running.get() && !output_lines.get().is_empty()>
                        <button class="status-bar-btn" on:click=move |_| set_show_log.set(true)>
                            "📋 Last output"
                        </button>
                    </Show>
                </div>
            </div>

            // ── Log modal ────────────────────────────────────────────
            <div class="sidebar" class:open=move || show_log.get()>
                <div class="sidebar-panel-form">
                    <div class="sidebar-header">
                        <span class="sidebar-title">"Output Log"</span>
                        <div style="display:flex;gap:6px;align-items:center">
                            <button
                                class="sidebar-close"
                                style="background:#2a5a3a;border-color:#1a3a2a"
                                on:click=move |_| clipboard_write_text(&output_lines.get_untracked().join("\n"))
                                title="Copy to clipboard"
                            >"Copy"</button>
                            <button class="sidebar-close" on:click=move |_| set_show_log.set(false)>"✕"</button>
                        </div>
                    </div>
                    <div class="panel-body" style="padding:0">
                        <pre class="log-console" node_ref=log_console_ref>{move || output_lines.get().join("\n")}</pre>
                    </div>
                </div>
            </div>

            // ── Commit modal ─────────────────────────────────────────
            <div class="sidebar" class:open=move || right_panel.get() == RightPanel::Commit>
                <div class="sidebar-panel-form">
                    <div class="sidebar-header">
                        <span class="sidebar-title">"Commit"</span>
                        <button class="sidebar-close" on:click=move |_| set_right_panel.set(RightPanel::None)>"✕"</button>
                    </div>
                    <div class="panel-body">
                        <label class="panel-label">
                            "Commit message"
                            <input
                                type="text"
                                prop:value=move || draft_message.get()
                                on:input=move |ev| set_draft_message.set(event_target_value(&ev))
                                placeholder="e.g. Auto backup before mining trip"
                            />
                        </label>
                        <button
                            class="btn-panel-primary btn-commit-modal"
                            on:click=run_commit
                            disabled=move || is_running.get()
                        >
                            "Commit"
                        </button>
                    </div>
                </div>
            </div>
        </div>
    }
}
