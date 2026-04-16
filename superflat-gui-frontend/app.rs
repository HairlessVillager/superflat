use crate::bindings::{invoke, log};
use crate::handlers::MainContent;
use crate::hooks::{use_git_event_listeners, use_window_controls};
use crate::state::{
    load_initial_data, provide_app_state, setup_profile_change_effect, use_app_state,
};
use crate::types::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;

// Helper to do profile upsert
fn do_upsert_profile(p: Profile, profiles: RwSignal<Vec<Profile>>) {
    spawn_local(async move {
        let args = to_js(&UpsertProfileArgs { profile: p });
        if let Err(err) = invoke("upsert_profile", args).await {
            log(&format!(
                "upsert_profile failed: {}",
                js_error_to_string(err)
            ));
            return;
        }
        if let Ok(result) = invoke("get_profiles", JsValue::NULL).await {
            if let Ok(ps) = serde_wasm_bindgen::from_value::<Vec<Profile>>(result) {
                profiles.set(ps);
            }
        }
    });
}

// ── Profile list panel ──────────────────────────────────────────────────────

#[component]
fn ProfileCard(p: Profile) -> impl IntoView {
    let state = use_app_state();
    let p_clone = p.clone();
    let dir_clone = p.save_dir.clone();
    view! {
        <div class="profile-card" on:click=move |_| {
            state.active_profile.set(p.clone());
            state.show_profiles.set(false);
            state.right_panel.set(RightPanel::None);
        }>
            <div class="profile-card-path">{p.save_dir.clone()}</div>
            <div class="profile-card-row">
                <div class="profile-card-meta">{
                    p.mc_version.clone()
                }</div>
                <div class="profile-card-actions">
                    <button class="btn btn-edit" on:click=move |ev| {
                        ev.stop_propagation();
                        state.right_panel.set(RightPanel::EditProfile(p_clone.save_dir.clone()));
                    }>"Edit"</button>
                    <button class="btn btn-remove" on:click=move |ev| {
                        ev.stop_propagation();
                        let dir = dir_clone.clone();
                        state.profiles.update(|ps| ps.retain(|x| x.save_dir != dir));
                        spawn_local(async move {
                            let args = to_js(&DeleteProfileArgs { save_dir: dir });
                            if let Err(err) = invoke("delete_profile", args).await {
                                log(&format!("delete_profile failed: {}", js_error_to_string(err)));
                            }
                        });
                    }>"Remove"</button>
                </div>
            </div>
        </div>
    }
}

#[component]
fn ProfileListPanel() -> impl IntoView {
    let state = use_app_state();

    let open_add_profile = move |_: leptos::ev::MouseEvent| {
        state.right_panel.set(RightPanel::AddProfile);
        state.show_profiles.set(true);
    };

    let open_clone_from_remote = move |_: leptos::ev::MouseEvent| {
        state.right_panel.set(RightPanel::CloneFromRemote);
        state.show_profiles.set(true);
    };

    view! {
        <div class="sidebar"
            class:open=move || state.show_profiles.get()
                && !matches!(state.right_panel.get(), RightPanel::AddProfile | RightPanel::EditProfile(_) | RightPanel::CloneFromRemote)
            class:no-transition=move || state.list_instant.get()
        >
            <div class="sidebar-panel-list">
                <div class="sidebar-body">
                    <div class="btn-add-profile-group">
                        <button class="btn btn-add-profile" on:click=open_add_profile>"+ Track Local Save"</button>
                        <button class="btn btn-add-profile" on:click=open_clone_from_remote>"+ Clone From Remote"</button>
                    </div>
                    <Show when=move || state.profiles.get().is_empty() fallback=|| view! {}>
                        <p class="sidebar-empty">"No profiles yet"</p>
                    </Show>
                    <div class="profile-list">
                        <For each=move || {
                                let mut ps = state.profiles.get();
                                ps.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
                                ps
                            } key=|p| format!("{}:{}", p.save_dir, p.updated_at)
                            children=move |p| view! {
                                <ProfileCard p=p />
                            }
                        />
                    </div>
                </div>
            </div>
        </div>
    }
}

// ── Add profile panel ────────────────────────────────────────────────────────

#[component]
fn AddProfilePanel() -> impl IntoView {
    let state = use_app_state();
    let (form_save_dir, set_form_save_dir) = signal(String::new());
    let (form_branch, set_form_branch) = signal(DEFAULT_BRANCH.to_string());
    let (form_mc_version, set_form_mc_version) = signal(String::new());
    let (show_errors, set_show_errors) = signal(false);

    let save_profile_form = move |_: leptos::ev::MouseEvent| {
        let p = Profile {
            save_dir: form_save_dir.get_untracked(),
            branch: form_branch.get_untracked(),
            mc_version: form_mc_version.get_untracked(),
            remote_url: String::new(),
            updated_at: String::new(),
        };
        if p.save_dir.is_empty() {
            return;
        }
        do_upsert_profile(p, state.profiles);
        state.form_closing.set(true);
        state.list_instant.set(true);
        spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(FORM_CLOSE_ANIMATION_MS).await;
            state.right_panel.set(RightPanel::None);
            state.form_closing.set(false);
            state.list_instant.set(false);
        });
    };

    view! {
        <div class="sidebar"
            class:open=move || (state.right_panel.get() == RightPanel::AddProfile
                || (state.form_closing.get() && !matches!(state.right_panel.get(), RightPanel::EditProfile(_))))
                && state.show_profiles.get()
        >
            <div class="sidebar-panel-form" class:closing=move || state.form_closing.get()>
                <div class="sidebar-body">
                    <div class="panel-body">
                        <label class="panel-label">
                            "Save directory"
                            <div class="panel-dir-row">
                                <input type="text" prop:value=move || form_save_dir.get()
                                    on:input=move |ev| {
                                        set_form_save_dir.set(event_target_value(&ev));
                                        set_show_errors.set(false);
                                    }
                                    class:invalid=move || show_errors.get() && form_save_dir.get().trim().is_empty()
                                    placeholder=".minecraft/saves/<save-name>/" />
                                <button class="btn btn-browse" on:click=move |_| {
                                    spawn_local(async move {
                                        if let Ok(r) = invoke("pick_directory", JsValue::NULL).await {
                                            if let Some(p) = r.as_string() { set_form_save_dir.set(p); }
                                        }
                                    });
                                }>"Browse"</button>
                            </div>
                        </label>
                        <label class="panel-label">"Branch"
                            <input type="text" prop:value=move || form_branch.get()
                                on:input=move |ev| {
                                    set_form_branch.set(event_target_value(&ev));
                                    set_show_errors.set(false);
                                }
                                class:invalid=move || show_errors.get() && form_branch.get().trim().is_empty()
                                placeholder="main" />
                        </label>
                        <label class="panel-label">"MC Version"
                            <input type="text" prop:value=move || form_mc_version.get()
                                on:input=move |ev| {
                                    set_form_mc_version.set(event_target_value(&ev));
                                    set_show_errors.set(false);
                                }
                                class:invalid=move || show_errors.get() && form_mc_version.get().trim().is_empty()
                                placeholder="e.g. 1.21.11" />
                        </label>
                        <button class="btn btn-panel-primary" on:click=move |ev| {
                            let dir_ok = !form_save_dir.get_untracked().trim().is_empty();
                            let branch_ok = !form_branch.get_untracked().trim().is_empty();
                            let ver_ok = !form_mc_version.get_untracked().trim().is_empty();
                            if dir_ok && branch_ok && ver_ok {
                                save_profile_form(ev);
                            } else {
                                set_show_errors.set(false);
                                spawn_local(async move { set_show_errors.set(true); });
                            }
                        }>"Track"</button>
                    </div>
                </div>
            </div>
        </div>
    }
}

// ── Edit profile panel ───────────────────────────────────────────────────────

#[component]
fn EditProfilePanel() -> impl IntoView {
    let state = use_app_state();
    // Form state - use RwSignal so we can set values
    let form_save_dir = RwSignal::new(String::new());
    let form_branch = RwSignal::new(String::new());
    let form_mc_version = RwSignal::new(String::new());
    let form_remote_url = RwSignal::new(String::new());
    let (remote_url_invalid, set_remote_url_invalid) = signal(false);
    let (show_errors, set_show_errors) = signal(false);

    // Sync form with active profile when edit panel opens
    Effect::new(move |_| {
        if let RightPanel::EditProfile(ref save_dir) = state.right_panel.get() {
            if let Some(p) = state
                .profiles
                .get()
                .iter()
                .find(|p| &p.save_dir == save_dir)
            {
                form_save_dir.set(p.save_dir.clone());
                form_branch.set(p.branch.clone());
                form_mc_version.set(p.mc_version.clone());
                form_remote_url.set(p.remote_url.clone());
            }
        }
    });

    let save_profile_form = move |_: leptos::ev::MouseEvent| {
        let p = Profile {
            save_dir: form_save_dir.get_untracked(),
            branch: form_branch.get_untracked(),
            mc_version: form_mc_version.get_untracked(),
            remote_url: form_remote_url.get_untracked(),
            updated_at: String::new(),
        };
        // If editing the active profile, update it
        if state.active_profile.get_untracked().save_dir == p.save_dir {
            state.active_profile.set(p.clone());
        }
        do_upsert_profile(p, state.profiles);
        state.form_closing.set(true);
        state.list_instant.set(true);
        spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(FORM_CLOSE_ANIMATION_MS).await;
            state.right_panel.set(RightPanel::None);
            state.form_closing.set(false);
            state.list_instant.set(false);
        });
    };

    view! {
        <div class="sidebar"
            class:open=move || (matches!(state.right_panel.get(), RightPanel::EditProfile(_))
                || state.form_closing.get()) && state.show_profiles.get()
        >
            <div class="sidebar-panel-form" class:closing=move || state.form_closing.get()>
                <div class="sidebar-body">
                    <div class="panel-body">
                        <label class="panel-label">
                            "Save directory"
                            <input type="text" prop:value=move || form_save_dir.get()
                                disabled=true />
                        </label>
                        <label class="panel-label">"Branch"
                            <input type="text" prop:value=move || form_branch.get()
                                on:input=move |ev| {
                                    form_branch.set(event_target_value(&ev));
                                    set_show_errors.set(false);
                                }
                                class:invalid=move || show_errors.get() && form_branch.get().trim().is_empty()
                                placeholder="main" />
                        </label>
                        <label class="panel-label">"MC Version"
                            <input type="text" prop:value=move || form_mc_version.get()
                                on:input=move |ev| {
                                    form_mc_version.set(event_target_value(&ev));
                                    set_show_errors.set(false);
                                }
                                class:invalid=move || show_errors.get() && form_mc_version.get().trim().is_empty()
                                placeholder="e.g. 1.21.11" />
                        </label>
                        <label class="panel-label">"Remote URL"
                            <input type="text" prop:value=move || form_remote_url.get()
                                on:input=move |ev| {
                                    form_remote_url.set(event_target_value(&ev));
                                    set_remote_url_invalid.set(false);
                                }
                                class:invalid=move || remote_url_invalid.get()
                                placeholder="ssh://..." />
                        </label>
                        <button class="btn btn-panel-primary" on:click=move |ev| {
                            let branch_ok = !form_branch.get_untracked().trim().is_empty();
                            let ver_ok = !form_mc_version.get_untracked().trim().is_empty();
                            if branch_ok && ver_ok {
                                save_profile_form(ev);
                            } else {
                                set_show_errors.set(false);
                                spawn_local(async move { set_show_errors.set(true); });
                            }
                        }>"OK"</button>
                    </div>
                </div>
            </div>
        </div>
    }
}

// ── Clone from remote form panel ────────────────────────────────────────────

#[component]
fn CloneFromRemoteFormPanel() -> impl IntoView {
    let state = use_app_state();
    let (form_clone_git_dir, set_form_clone_git_dir) = signal(String::new());
    let (form_remote_url, set_form_remote_url) = signal(String::new());
    let (form_branch, set_form_branch) = signal(DEFAULT_BRANCH.to_string());
    let (form_mc_version, set_form_mc_version) = signal(String::new());
    let (clone_show_errors, set_clone_show_errors) = signal(false);

    let clone_profile_form = move |_: leptos::ev::MouseEvent| {
        let save_dir = form_clone_git_dir.get_untracked();
        let remote_url = form_remote_url.get_untracked();
        let branch = form_branch.get_untracked();
        let mc_version = form_mc_version.get_untracked();
        if save_dir.is_empty() || remote_url.is_empty() {
            return;
        }
        let p = Profile {
            save_dir: save_dir.clone(),
            branch: branch.clone(),
            mc_version: mc_version.clone(),
            remote_url: remote_url.clone(),
            updated_at: String::new(),
        };
        state.active_profile.set(p.clone());
        do_upsert_profile(p.clone(), state.profiles);
        state.op_start_ms.set(js_sys::Date::now());
        state.last_raw_line.set(String::new());
        state.output_lines.set(Vec::new());
        state.is_running.set(true);
        spawn_local(async move {
            let args = to_js(&RunCloneArgs {
                save_dir,
                url: remote_url,
            });
            if let Err(err) = invoke("run_clone", args).await {
                state
                    .output_lines
                    .update(|l| l.push(format!("Error: {}", js_error_to_string(err))));
            }
            do_upsert_profile(p, state.profiles);
        });
        state.form_closing.set(true);
        state.list_instant.set(true);
        spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(FORM_CLOSE_ANIMATION_MS).await;
            state.right_panel.set(RightPanel::None);
            state.show_profiles.set(false);
            state.form_closing.set(false);
            state.list_instant.set(false);
        });
    };

    view! {
        <div class="sidebar"
            class:open=move || (state.right_panel.get() == RightPanel::CloneFromRemote
                || state.form_closing.get()) && state.show_profiles.get()
        >
            <div class="sidebar-panel-form" class:closing=move || state.form_closing.get()>
                <div class="sidebar-body">
                    <div class="panel-body">
                        <label class="panel-label">
                            "Save directory"
                            <div class="panel-dir-row">
                                <input type="text" prop:value=move || form_clone_git_dir.get()
                                    on:input=move |ev| {
                                        set_form_clone_git_dir.set(event_target_value(&ev));
                                        set_clone_show_errors.set(false);
                                    }
                                    class:invalid=move || clone_show_errors.get() && form_clone_git_dir.get().trim().is_empty()
                                    placeholder=".minecraft/saves/<save-name>/" />
                                <button class="btn btn-browse" on:click=move |_| {
                                    spawn_local(async move {
                                        if let Ok(r) = invoke("pick_directory", JsValue::NULL).await {
                                            if let Some(p) = r.as_string() { set_form_clone_git_dir.set(p); }
                                        }
                                    });
                                }>"Browse"</button>
                            </div>
                        </label>
                        <label class="panel-label">"Remote URL"
                            <input type="text" prop:value=move || form_remote_url.get()
                                on:input=move |ev| {
                                    set_form_remote_url.set(event_target_value(&ev));
                                    set_clone_show_errors.set(false);
                                }
                                class:invalid=move || clone_show_errors.get() && form_remote_url.get().trim().is_empty()
                                placeholder="ssh://..." />
                        </label>
                        <label class="panel-label">"Branch"
                            <input type="text" prop:value=move || form_branch.get()
                                on:input=move |ev| {
                                    set_form_branch.set(event_target_value(&ev));
                                    set_clone_show_errors.set(false);
                                }
                                class:invalid=move || clone_show_errors.get() && form_branch.get().trim().is_empty()
                                placeholder="main" />
                        </label>
                        <label class="panel-label">"MC Version"
                            <input type="text" prop:value=move || form_mc_version.get()
                                on:input=move |ev| {
                                    set_form_mc_version.set(event_target_value(&ev));
                                    set_clone_show_errors.set(false);
                                }
                                class:invalid=move || clone_show_errors.get() && form_mc_version.get().trim().is_empty()
                                placeholder="e.g. 1.21.11" />
                        </label>
                        <button class="btn btn-panel-primary" on:click=move |ev| {
                            let git_dir_ok = !form_clone_git_dir.get_untracked().trim().is_empty();
                            let url_ok = !form_remote_url.get_untracked().trim().is_empty();
                            let branch_ok = !form_branch.get_untracked().trim().is_empty();
                            let ver_ok = !form_mc_version.get_untracked().trim().is_empty();
                            if git_dir_ok && url_ok && branch_ok && ver_ok {
                                clone_profile_form(ev);
                            } else {
                                set_clone_show_errors.set(false);
                                spawn_local(async move { set_clone_show_errors.set(true); });
                            }
                        }>"Clone"</button>
                    </div>
                </div>
            </div>
        </div>
    }
}

// ── Git user config panel

#[component]
fn GitUserConfigPanel() -> impl IntoView {
    let state = use_app_state();
    let (form_git_name, set_form_git_name) = signal(String::new());
    let (form_git_email, set_form_git_email) = signal(String::new());
    let (_form_closing_local, _set_form_closing_local) = signal(false);

    let save_git_config = move |_: leptos::ev::MouseEvent| {
        let name = form_git_name.get_untracked();
        let email = form_git_email.get_untracked();
        spawn_local(async move {
            let args = to_js(&SetGitUserConfigArgs {
                name: name.clone(),
                email: email.clone(),
            });
            if let Err(err) = invoke("set_git_user_config", args).await {
                state
                    .output_lines
                    .update(|l| l.push(format!("Error: {}", js_error_to_string(err))));
            } else {
                state.right_panel.set(RightPanel::None);
            }
        });
        state.form_closing.set(true);
        state.list_instant.set(true);
        spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(FORM_CLOSE_ANIMATION_MS).await;
            state.right_panel.set(RightPanel::None);
            state.form_closing.set(false);
            state.list_instant.set(false);
        });
    };

    view! {
        <Show when=move || state.right_panel.get() == RightPanel::GitUserConfig>
            <div class="sidebar-overlay" on:click=move |_| {}></div>
            <div class="sidebar"
                class:open=move || state.right_panel.get() == RightPanel::GitUserConfig
                    || (state.form_closing.get() && state.right_panel.get() == RightPanel::GitUserConfig)
                class:no-transition=move || _form_closing_local.get()
            >
                <div class="sidebar-panel-form">
                    <div class="sidebar-body">
                        <div class="panel-body">
                            <p class="panel-hint">"Please set your Git user name and email for commits."</p>
                            <label class="panel-label">"user.name"
                                <input type="text" prop:value=move || form_git_name.get()
                                    on:input=move |ev| { set_form_git_name.set(event_target_value(&ev)); }
                                    placeholder="Your Name" />
                            </label>
                            <label class="panel-label">"user.email"
                                <input type="text" prop:value=move || form_git_email.get()
                                    on:input=move |ev| { set_form_git_email.set(event_target_value(&ev)); }
                                    placeholder="you@example.com" />
                            </label>
                            <div class="panel-row-buttons">
                                <button class="btn btn-panel-secondary" on:click=move |_| {
                                    spawn_local(async move {
                                        if let Err(err) = invoke("window_close", JsValue::NULL).await {
                                            log(&format!("quit failed: {}", js_error_to_string(err)));
                                        }
                                    });
                                }>"Quit"</button>
                                <button class="btn btn-panel-primary" on:click=move |ev| {
                                    let name_ok = !form_git_name.get_untracked().trim().is_empty();
                                    let email_ok = !form_git_email.get_untracked().trim().is_empty();
                                    if name_ok && email_ok {
                                        save_git_config(ev);
                                    } else {
                                        state.output_lines.update(|l| l.push("Error: name and email are required".to_string()));
                                    }
                                }>"Save"</button>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </Show>
    }
}

// ── App ─────────────────────────────────────────────────────────────────────

#[component]
pub fn App() -> impl IntoView {
    // Initialize AppState and provide context
    let state = provide_app_state();

    // Set up initial data loading
    load_initial_data(state);

    // Set up profile change effect
    setup_profile_change_effect(state);

    // Get window controls
    let window_controls = use_window_controls();

    // Set up git event listeners
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
                        js_error_to_string(err)
                    ))
                }),
            }
        });
    };
    use_git_event_listeners(
        state.op_start_ms,
        state.last_raw_line,
        state.output_lines,
        state.is_running,
        state.active_profile,
        refresh,
    );

    // Commit-related state
    let draft_message = RwSignal::new(String::new());
    let set_draft_message = draft_message;

    // Event handlers that wrap state methods
    let run_pull = move |_: leptos::ev::MouseEvent| {
        state.run_pull();
    };

    let run_push = move |_: leptos::ev::MouseEvent| {
        state.run_push();
    };

    let run_clone = move |_: leptos::ev::MouseEvent| {
        state.run_clone();
    };

    let run_commit = move |_: leptos::ev::MouseEvent| {
        let msg = draft_message.get_untracked();
        state.run_commit(msg, draft_message);
    };

    let run_checkout = move |commit: String| {
        state.run_checkout(commit);
    };

    let do_pull = move || {
        state.do_pull();
    };

    let do_push = move || {
        state.do_push();
    };

    view! {
        <div class="app">
            <ProfileListPanel />
            <AddProfilePanel />
            <EditProfilePanel />
            <GitUserConfigPanel />
            <CloneFromRemoteFormPanel />
            <Show when=move || state.show_profiles.get() || matches!(state.right_panel.get(), RightPanel::Commit | RightPanel::Checkout(_) | RightPanel::ConfirmPull | RightPanel::ConfirmPush)>
                <div class="sidebar-overlay" on:click=move |_| {
                    if matches!(state.right_panel.get_untracked(), RightPanel::Commit | RightPanel::Checkout(_) | RightPanel::ConfirmPull | RightPanel::ConfirmPush) {
                        state.right_panel.set(RightPanel::None);
                    } else {
                        state.show_profiles.set(false);
                        if matches!(state.right_panel.get_untracked(),
                            RightPanel::AddProfile | RightPanel::EditProfile(_) | RightPanel::CloneFromRemote)
                        {
                            state.form_closing.set(true);
                            state.list_instant.set(true);
                            spawn_local(async move {
                                gloo_timers::future::TimeoutFuture::new(FORM_CLOSE_ANIMATION_MS).await;
                                state.right_panel.set(RightPanel::None);
                                state.form_closing.set(false);
                                state.list_instant.set(false);
                            });
                        }
                    }
                }/>
            </Show>

            // ── Main content ────────────────────────────────────────
            <div class="main">
                <div class="window-titlebar">
                    <div class="window-title-drag" data-tauri-drag-region=true on:mousedown=window_controls.handle_drag>
                        <span class="window-title">"Superflat GUI"</span>
                    </div>
                    <div class="window-controls">
                        <button class="window-btn" on:click=window_controls.handle_minimize title="Minimize">"-"</button>
                        <button class="window-btn" on:click=window_controls.handle_toggle_maximize title="Maximize / Restore">"□"</button>
                        <button class="window-btn window-btn-close" on:click=window_controls.handle_close title="Close">"✕"</button>
                    </div>
                </div>
                <MainContent
                    active_profile=state.active_profile
                    is_running=state.is_running
                    right_panel=state.right_panel
                    set_right_panel=state.right_panel
                    repo_exists=state.repo_exists
                    commits=state.commits
                    set_show_profiles=state.show_profiles
                    draft_message=draft_message
                    set_draft_message=set_draft_message
                    run_commit=run_commit
                    run_checkout=run_checkout
                    run_pull=run_pull
                    run_push=run_push
                    run_clone=run_clone
                    do_pull=do_pull
                    do_push=do_push
                />
                // ── Status bar ──────────────────────────────────
                <div class="status-bar">
                    <span class="status-bar-latest-log">
                        {move || state.last_raw_line.get()}
                    </span>
                    <Show when=move || state.log_exists.get() || !state.output_lines.get().is_empty()>
                        <button class="status-bar-btn status-bar-log-btn"
                            on:click=move |_| {
                                spawn_local(async move {
                                    if let Err(e) = invoke("open_log_file", JsValue::NULL).await {
                                        log(&format!("Failed to open log file: {:?}", e));
                                    }
                                });
                            }>
                            "📋 Latest Log"
                        </button>
                    </Show>
                </div>
            </div>
        </div>
    }
}
