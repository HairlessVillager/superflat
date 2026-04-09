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

    #[wasm_bindgen(js_name = setInterval)]
    fn set_interval(closure: &Closure<dyn Fn()>, millis: u32) -> i32;

    #[wasm_bindgen(js_name = setTimeout)]
    fn set_timeout(closure: &Closure<dyn Fn()>, millis: u32) -> i32;

    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

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
    default_commit: String,
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

fn current_datetime_string() -> String {
    let d = js_sys::Date::new_0();
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        d.get_full_year(),
        d.get_month() + 1,
        d.get_date(),
        d.get_hours(),
        d.get_minutes(),
        d.get_seconds(),
    )
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
    // --- active profile state ---
    let (save_dir, set_save_dir) = signal(String::new());
    let (branch, set_branch) = signal(String::from("main"));
    let (mc_version, set_mc_version) = signal(String::from("1.21.11"));
    let (commit_id, set_commit_id) = signal(String::from("main"));
    let (remote_url, set_remote_url) = signal(String::new());

    // --- ui state ---
    let (clock, set_clock) = signal(current_datetime_string());
    let (output_lines, set_output_lines) = signal(Vec::<String>::new());
    let (is_running, set_is_running) = signal(false);
    let (show_profiles, set_show_profiles) = signal(false);
    let (right_panel, set_right_panel) = signal(RightPanel::None);
    let (profiles, set_profiles) = signal(Vec::<Profile>::new());
    let (commits, set_commits) = signal(Vec::<CommitInfo>::new());
    let (repo_exists, set_repo_exists) = signal(false);
    let (form_closing, set_form_closing) = signal(false);

    // Close the Add/Edit form with an exit animation, then switch panel
    let close_form = move |next: RightPanel| {
        set_form_closing.set(true);
        let cb = Closure::<dyn Fn()>::new(move || {
            set_right_panel.set(next.clone());
            set_form_closing.set(false);
        });
        set_timeout(&cb, 200);
        cb.forget();
    };

    // commit panel draft
    let (draft_message, set_draft_message) = signal(String::new());

    // profile form draft
    let (form_save_dir, set_form_save_dir) = signal(String::new());
    let (form_branch, set_form_branch) = signal(String::from("main"));
    let (form_mc_version, set_form_mc_version) = signal(String::from("1.21.11"));
    let (form_default_commit, set_form_default_commit) = signal(String::from("main"));
    let (form_remote_url, set_form_remote_url) = signal(String::new());

    // Reactively check repo + load commits when save_dir changes
    Effect::new(move |_| {
        let dir = save_dir.get();
        set_repo_exists.set(false);
        set_commits.set(vec![]);
        spawn_local(async move {
            if dir.is_empty() {
                return;
            }
            let args = serde_wasm_bindgen::to_value(&CheckRepoExistsArgs {
                save_dir: dir.clone(),
            })
            .expect("serialize");
            if let Ok(val) = invoke("check_repo_exists", args).await {
                if let Some(exists) = val.as_bool() {
                    set_repo_exists.set(exists);
                }
            }
            let args =
                serde_wasm_bindgen::to_value(&GetCommitsArgs { save_dir: dir }).expect("serialize");
            if let Ok(val) = invoke("get_commits", args).await {
                if let Ok(c) = serde_wasm_bindgen::from_value::<Vec<CommitInfo>>(val) {
                    set_commits.set(c);
                }
            }
        });
    });

    // Tick clock
    let tick = Closure::<dyn Fn()>::new(move || set_clock.set(current_datetime_string()));
    set_interval(&tick, 1000);
    tick.forget();

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
        let on_done = Closure::<dyn Fn(JsValue)>::new(move |_: JsValue| {
            set_running.set(false);
            // refresh commits after any operation
            let dir = save_dir.get_untracked();
            if !dir.is_empty() {
                spawn_local(async move {
                    let args = serde_wasm_bindgen::to_value(&GetCommitsArgs { save_dir: dir })
                        .expect("serialize");
                    if let Ok(val) = invoke("get_commits", args).await {
                        if let Ok(c) = serde_wasm_bindgen::from_value::<Vec<CommitInfo>>(val) {
                            set_commits.set(c);
                        }
                    }
                });
                // also refresh repo_exists
                let dir2 = save_dir.get_untracked();
                spawn_local(async move {
                    let args =
                        serde_wasm_bindgen::to_value(&CheckRepoExistsArgs { save_dir: dir2 })
                            .expect("serialize");
                    if let Ok(val) = invoke("check_repo_exists", args).await {
                        if let Some(exists) = val.as_bool() {
                            set_repo_exists.set(exists);
                        }
                    }
                });
            }
        });
        if let (Ok(_), Ok(_)) = (
            tauri_listen("commit-output", &on_output).await,
            tauri_listen("commit-done", &on_done).await,
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
            let args =
                serde_wasm_bindgen::to_value(&UpsertProfileArgs { profile: p }).expect("serialize");
            if let Err(err) = invoke("upsert_profile", args).await {
                log(&format!(
                    "upsert_profile failed: {}",
                    js_error_to_string(err)
                ));
            }
        });
    };

    let run_commit = move |_: leptos::ev::MouseEvent| {
        set_output_lines.set(Vec::new());
        set_is_running.set(true);
        set_right_panel.set(RightPanel::None);
        let save_dir_val = save_dir.get_untracked();
        let branch_val = branch.get_untracked();
        let mc_version_val = mc_version.get_untracked();
        let default_commit_val = commit_id.get_untracked();
        let remote_url_val = remote_url.get_untracked();
        let message_val = {
            let m = draft_message.get_untracked();
            if m.is_empty() {
                current_datetime_string()
            } else {
                m
            }
        };
        set_draft_message.set(String::new());
        spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&RunCommitArgs {
                save_dir: save_dir_val.clone(),
                branch: branch_val.clone(),
                message: message_val,
                mc_version: mc_version_val.clone(),
            })
            .expect("serialize");
            if let Err(err) = invoke("run_commit", args).await {
                set_output_lines.update(|l| l.push(format!("Error: {}", js_error_to_string(err))));
            }
            do_upsert_profile(Profile {
                save_dir: save_dir_val,
                mc_version: mc_version_val,
                branch: branch_val,
                default_commit: default_commit_val,
                remote_url: remote_url_val,
            });
        });
    };

    let run_checkout = move |commit: String| {
        set_output_lines.set(Vec::new());
        set_is_running.set(true);
        let save_dir_val = save_dir.get_untracked();
        let mc_version_val = mc_version.get_untracked();
        let branch_val = branch.get_untracked();
        let remote_url_val = remote_url.get_untracked();
        let commit_clone = commit.clone();
        spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&RunCheckoutArgs {
                save_dir: save_dir_val.clone(),
                commit: commit_clone.clone(),
                mc_version: mc_version_val.clone(),
            })
            .expect("serialize");
            if let Err(err) = invoke("run_checkout", args).await {
                set_output_lines.update(|l| l.push(format!("Error: {}", js_error_to_string(err))));
            }
            do_upsert_profile(Profile {
                save_dir: save_dir_val,
                mc_version: mc_version_val,
                branch: branch_val,
                default_commit: commit_clone,
                remote_url: remote_url_val,
            });
        });
    };

    let run_pull = move |_: leptos::ev::MouseEvent| {
        set_output_lines.set(Vec::new());
        set_is_running.set(true);
        let save_dir_val = save_dir.get_untracked();
        let remote_url_val = remote_url.get_untracked();
        let branch_val = branch.get_untracked();
        let mc_version_val = mc_version.get_untracked();
        let default_commit_val = commit_id.get_untracked();
        spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&RunPullArgs {
                save_dir: save_dir_val.clone(),
                url: remote_url_val.clone(),
            })
            .expect("serialize");
            if let Err(err) = invoke("run_pull", args).await {
                set_output_lines.update(|l| l.push(format!("Error: {}", js_error_to_string(err))));
            }
            do_upsert_profile(Profile {
                save_dir: save_dir_val,
                mc_version: mc_version_val,
                branch: branch_val,
                default_commit: default_commit_val,
                remote_url: remote_url_val,
            });
        });
    };

    let run_push = move |_: leptos::ev::MouseEvent| {
        set_output_lines.set(Vec::new());
        set_is_running.set(true);
        let save_dir_val = save_dir.get_untracked();
        let remote_url_val = remote_url.get_untracked();
        let branch_val = branch.get_untracked();
        let mc_version_val = mc_version.get_untracked();
        let default_commit_val = commit_id.get_untracked();
        spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&RunPushArgs {
                save_dir: save_dir_val.clone(),
                url: remote_url_val.clone(),
            })
            .expect("serialize");
            if let Err(err) = invoke("run_push", args).await {
                set_output_lines.update(|l| l.push(format!("Error: {}", js_error_to_string(err))));
            }
            do_upsert_profile(Profile {
                save_dir: save_dir_val,
                mc_version: mc_version_val,
                branch: branch_val,
                default_commit: default_commit_val,
                remote_url: remote_url_val,
            });
        });
    };

    let run_clone = move |_: leptos::ev::MouseEvent| {
        set_output_lines.set(Vec::new());
        set_is_running.set(true);
        let save_dir_val = save_dir.get_untracked();
        let remote_url_val = remote_url.get_untracked();
        let branch_val = branch.get_untracked();
        let mc_version_val = mc_version.get_untracked();
        let default_commit_val = commit_id.get_untracked();
        spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&RunCloneArgs {
                save_dir: save_dir_val.clone(),
                url: remote_url_val.clone(),
            })
            .expect("serialize");
            if let Err(err) = invoke("run_clone", args).await {
                set_output_lines.update(|l| l.push(format!("Error: {}", js_error_to_string(err))));
            }
            do_upsert_profile(Profile {
                save_dir: save_dir_val,
                mc_version: mc_version_val,
                branch: branch_val,
                default_commit: default_commit_val,
                remote_url: remote_url_val,
            });
        });
    };

    let open_add_profile = move |_: leptos::ev::MouseEvent| {
        set_form_save_dir.set(String::new());
        set_form_branch.set(String::from("main"));
        set_form_mc_version.set(String::from("1.21.11"));
        set_form_default_commit.set(String::from("main"));
        set_form_remote_url.set(String::new());
        set_right_panel.set(RightPanel::AddProfile);
        set_show_profiles.set(true);
    };

    let save_profile_form = move |_: leptos::ev::MouseEvent| {
        let p = Profile {
            save_dir: form_save_dir.get_untracked(),
            branch: form_branch.get_untracked(),
            mc_version: form_mc_version.get_untracked(),
            default_commit: form_default_commit.get_untracked(),
            remote_url: form_remote_url.get_untracked(),
        };
        if p.save_dir.is_empty() {
            return;
        }
        do_upsert_profile(p);
        close_form(RightPanel::None);
    };

    view! {
        <div class="app">

            // ── Left sidebar (profiles) ─────────────────────────────
            <div class="sidebar" class:open=move || show_profiles.get()>
                // Profile list view
                <Show when=move || !matches!(right_panel.get(), RightPanel::AddProfile | RightPanel::EditProfile(_))>
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
                                    let p_load = p.clone();
                                    let p_edit = p.clone();
                                    let p_del  = p.clone();
                                    view! {
                                        <div
                                            class="profile-card"
                                            on:click=move |_| {
                                                set_save_dir.set(p_load.save_dir.clone());
                                                set_branch.set(p_load.branch.clone());
                                                set_mc_version.set(p_load.mc_version.clone());
                                                set_commit_id.set(p_load.default_commit.clone());
                                                set_remote_url.set(p_load.remote_url.clone());
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
                                                    set_form_default_commit.set(p_edit.default_commit.clone());
                                                    set_form_remote_url.set(p_edit.remote_url.clone());
                                                    set_right_panel.set(RightPanel::EditProfile(p_edit.save_dir.clone()));
                                                }>
                                                    "Edit"
                                                </button>
                                                <button class="btn-remove" on:click=move |ev| {
                                                    ev.stop_propagation();
                                                    let dir = p_del.save_dir.clone();
                                                    set_profiles.update(|ps| ps.retain(|x| x.save_dir != dir));
                                                    spawn_local(async move {
                                                        let args = serde_wasm_bindgen::to_value(&DeleteProfileArgs { save_dir: dir })
                                                            .expect("serialize");
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
                </Show>

                // Add / Edit profile form view
                <Show when=move || matches!(right_panel.get(), RightPanel::AddProfile | RightPanel::EditProfile(_)) || form_closing.get()>
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
                                "Default commit"
                                <input
                                    type="text"
                                    prop:value=move || form_default_commit.get()
                                    on:input=move |ev| set_form_default_commit.set(event_target_value(&ev))
                                    placeholder="main@{10 minutes ago}"
                                />
                            </label>
                            <label class="panel-label">
                                "Remote URL"
                                <input
                                    type="text"
                                    prop:value=move || form_remote_url.get()
                                    on:input=move |ev| set_form_remote_url.set(event_target_value(&ev))
                                    placeholder="https://..."
                                />
                            </label>
                            // Load profile button (only in edit mode)
                            <Show when=move || matches!(right_panel.get(), RightPanel::EditProfile(_))>
                                <button class="btn-load-profile" on:click=move |_| {
                                    set_save_dir.set(form_save_dir.get_untracked());
                                    set_branch.set(form_branch.get_untracked());
                                    set_mc_version.set(form_mc_version.get_untracked());
                                    set_commit_id.set(form_default_commit.get_untracked());
                                    set_remote_url.set(form_remote_url.get_untracked());
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
                </Show>
            </div>

            // ── Sidebar overlay ─────────────────────────────────────
            <Show when=move || show_profiles.get()>
                <div class="sidebar-overlay" on:click=move |_| {
                    set_show_profiles.set(false);
                    if matches!(right_panel.get_untracked(), RightPanel::AddProfile | RightPanel::EditProfile(_)) {
                        close_form(RightPanel::None);
                    }
                }/>
            </Show>

            // ── Main content ────────────────────────────────────────
            <div class="main">

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
                        <Show when=move || !save_dir.get().is_empty()>
                            <div class="topbar-dir-display" title=move || save_dir.get()>
                                <span class="topbar-dir-name">
                                    {move || {
                                        let d = save_dir.get();
                                        d.trim_end_matches(['/', '\\'])
                                            .rsplit(['/', '\\'])
                                            .next()
                                            .unwrap_or(&d)
                                            .to_string()
                                    }}
                                </span>
                            </div>
                        </Show>
                    </div>
                    <Show when=move || !save_dir.get().is_empty()>
                        <div class="topbar-actions">
                            <Show
                                when=move || repo_exists.get()
                                fallback=move || view! {
                                    <button
                                        class="btn-action btn-clone"
                                        on:click=run_clone
                                        disabled=move || is_running.get()
                                    >
                                        "Clone"
                                    </button>
                                }
                            >
                                <button
                                    class="btn-action btn-pull"
                                    on:click=run_pull
                                    disabled=move || is_running.get()
                                >
                                    "Pull"
                                </button>
                            </Show>
                            <button
                                class="btn-action btn-commit"
                                on:click=move |_| {
                                    if right_panel.get_untracked() == RightPanel::Commit {
                                        set_right_panel.set(RightPanel::None);
                                    } else {
                                        set_right_panel.set(RightPanel::Commit);
                                    }
                                }
                                disabled=move || is_running.get()
                            >
                                "Commit"
                            </button>
                            <Show when=move || repo_exists.get()>
                                <button
                                    class="btn-action btn-push"
                                    on:click=run_push
                                    disabled=move || is_running.get()
                                >
                                    "Push"
                                </button>
                            </Show>
                        </div>
                    </Show>
                </div>

                // ── Body: commit list + right panel ──────────────────
                <div class="body">

                    // ── Commit list ─────────────────────────────────
                    <div class="commit-area">
                        <Show
                            when=move || !output_lines.get().is_empty()
                            fallback=|| view! {}
                        >
                            <pre class="console">{move || output_lines.get().join("\n")}</pre>
                        </Show>
                        <Show
                            when=move || commits.get().is_empty() && output_lines.get().is_empty()
                            fallback=|| view! {}
                        >
                            <div class="commit-empty">
                                <span>"Select a save directory to view commit history"</span>
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
                    <Show when=move || right_panel.get() == RightPanel::Commit>
                        <div class="right-panel">
                            <div class="panel-header">
                                <span>"Commit"</span>
                                <button class="panel-close" on:click=move |_| set_right_panel.set(RightPanel::None)>"✕"</button>
                            </div>
                            <div class="panel-body">
                                <label class="panel-label">
                                    "Branch"
                                    <input
                                        type="text"
                                        prop:value=move || branch.get()
                                        on:input=move |ev| set_branch.set(event_target_value(&ev))
                                        placeholder="main"
                                    />
                                </label>
                                <label class="panel-label">
                                    "MC Version"
                                    <input
                                        type="text"
                                        prop:value=move || mc_version.get()
                                        on:input=move |ev| set_mc_version.set(event_target_value(&ev))
                                        placeholder="e.g. 1.21.11"
                                    />
                                </label>
                                <label class="panel-label">
                                    "Remote URL"
                                    <input
                                        type="text"
                                        prop:value=move || remote_url.get()
                                        on:input=move |ev| set_remote_url.set(event_target_value(&ev))
                                        placeholder="https://..."
                                    />
                                </label>
                                <label class="panel-label">
                                    "Commit message"
                                    <input
                                        type="text"
                                        prop:value=move || draft_message.get()
                                        on:input=move |ev| set_draft_message.set(event_target_value(&ev))
                                        placeholder=move || clock.get()
                                    />
                                </label>
                                <button
                                    class="btn-panel-primary"
                                    on:click=run_commit
                                    disabled=move || is_running.get()
                                >
                                    "Commit"
                                </button>
                            </div>
                        </div>
                    </Show>

                </div>
            </div>
        </div>
    }
}
