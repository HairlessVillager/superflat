use crate::bindings::{invoke, set_timeout};
use crate::handlers::{
    make_refresh_repo_state, make_upsert_profile, run_remote_op, setup_event_listeners,
    MainContent,
};
use crate::types::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;

// ── Profile list panel ──────────────────────────────────────────────────────

#[component]
fn ProfileCard(
    p: Profile,
    set_active_profile: WriteSignal<Profile>,
    set_show_profiles: WriteSignal<bool>,
    set_right_panel: WriteSignal<RightPanel>,
    set_form_save_dir: WriteSignal<String>,
    set_form_branch: WriteSignal<String>,
    set_form_mc_version: WriteSignal<String>,
    set_form_remote_url: WriteSignal<String>,
    set_profiles: WriteSignal<Vec<Profile>>,
) -> impl IntoView {
    let p_edit = p.clone();
    let dir_remove = p.save_dir.clone();
view! {
        <div class="profile-card" on:click=move |_| {
            set_active_profile.set(p.clone());
            set_show_profiles.set(false);
            set_right_panel.set(RightPanel::None);
        }>
            <div class="profile-card-path">{p.save_dir.clone()}</div>
            <div class="profile-card-meta">{format!("{} · {}", p.branch, p.mc_version)}</div>
            <div class="profile-card-actions">
                <button class="btn-edit" on:click=move |ev| {
                    ev.stop_propagation();
                    set_form_save_dir.set(p_edit.save_dir.clone());
                    set_form_branch.set(p_edit.branch.clone());
                    set_form_mc_version.set(p_edit.mc_version.clone());
                    set_form_remote_url.set(p_edit.remote_url.clone());
                    set_right_panel.set(RightPanel::EditProfile(p_edit.save_dir.clone()));
                }>"Edit"</button>
                <button class="btn-remove" on:click=move |ev| {
                    ev.stop_propagation();
                    set_profiles.update(|ps| ps.retain(|x| x.save_dir != dir_remove));
                    let dir = dir_remove.clone();
                    spawn_local(async move {
                        let args = to_js(&DeleteProfileArgs { save_dir: dir });
                        if let Err(err) = invoke("delete_profile", args).await {
                            crate::bindings::log(&format!(
                                "delete_profile failed: {}", js_error_to_string(err)
                            ));
                        }
                    });
                }>"Remove"</button>
            </div>
        </div>
    }
}

#[component]
fn ProfileListPanel(
    show_profiles: ReadSignal<bool>,
    list_instant: ReadSignal<bool>,
    right_panel: ReadSignal<RightPanel>,
    profiles: ReadSignal<Vec<Profile>>,
    set_show_profiles: WriteSignal<bool>,
    set_active_profile: WriteSignal<Profile>,
    set_right_panel: WriteSignal<RightPanel>,
    set_form_save_dir: WriteSignal<String>,
    set_form_branch: WriteSignal<String>,
    set_form_mc_version: WriteSignal<String>,
    set_form_remote_url: WriteSignal<String>,
    set_profiles: WriteSignal<Vec<Profile>>,
    open_add_profile: impl Fn(leptos::ev::MouseEvent) + Copy + 'static,
) -> impl IntoView {
    view! {
        <div class="sidebar"
            class:open=move || show_profiles.get()
                && !matches!(right_panel.get(), RightPanel::AddProfile | RightPanel::EditProfile(_))
            class:no-transition=move || list_instant.get()
        >
            <div class="sidebar-panel-list">
                <div class="sidebar-header">
                    <span class="sidebar-title">"Profiles"</span>
                    <button class="sidebar-close"
                        on:click=move |_| set_show_profiles.set(false)>"✕"</button>
                </div>
                <button class="btn-add-profile" on:click=open_add_profile>"+ Add Profile"</button>
                <Show when=move || profiles.get().is_empty() fallback=|| view! {}>
                    <p class="sidebar-empty">"No profiles yet"</p>
                </Show>
                <div class="profile-list">
                    <For each=move || profiles.get() key=|p| p.save_dir.clone()
                        children=move |p| view! {
                            <ProfileCard p=p
                                set_active_profile=set_active_profile
                                set_show_profiles=set_show_profiles
                                set_right_panel=set_right_panel
                                set_form_save_dir=set_form_save_dir
                                set_form_branch=set_form_branch
                                set_form_mc_version=set_form_mc_version
                                set_form_remote_url=set_form_remote_url
                                set_profiles=set_profiles />
                        }
                    />
                </div>
            </div>
        </div>
    }
}

// ── Profile form panel ──────────────────────────────────────────────────────

#[component]
fn ProfileFormPanel(
    show_profiles: ReadSignal<bool>,
    right_panel: ReadSignal<RightPanel>,
    form_closing: ReadSignal<bool>,
    form_save_dir: ReadSignal<String>,
    form_branch: ReadSignal<String>,
    form_mc_version: ReadSignal<String>,
    form_remote_url: ReadSignal<String>,
    set_form_save_dir: WriteSignal<String>,
    set_form_branch: WriteSignal<String>,
    set_form_mc_version: WriteSignal<String>,
    set_form_remote_url: WriteSignal<String>,
    set_active_profile: WriteSignal<Profile>,
    set_show_profiles: WriteSignal<bool>,
    set_form_closing: WriteSignal<bool>,
    set_list_instant: WriteSignal<bool>,
    set_right_panel: WriteSignal<RightPanel>,
    save_profile_form: impl Fn(leptos::ev::MouseEvent) + Copy + 'static,
) -> impl IntoView {
    let do_close = move |next: RightPanel| {
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
    view! {
        <div class="sidebar"
            class:open=move || (matches!(right_panel.get(),
                RightPanel::AddProfile | RightPanel::EditProfile(_))
                || form_closing.get()) && show_profiles.get()
        >
            <div class="sidebar-panel-form" class:closing=move || form_closing.get()>
                <div class="sidebar-header">
                    <span class="sidebar-title">
                        {move || if right_panel.get() == RightPanel::AddProfile
                            { "Add Profile" } else { "Edit Profile" }}
                    </span>
                    <button class="sidebar-close"
                        on:click=move |_| do_close(RightPanel::None)>"✕"</button>
                </div>
                <div class="panel-body">
                    <label class="panel-label">
                        "Save directory"
                        <div class="panel-dir-row">
                            <input type="text" prop:value=move || form_save_dir.get()
                                on:input=move |ev| set_form_save_dir.set(event_target_value(&ev))
                                placeholder="Path to save directory"
                                disabled=move || matches!(right_panel.get(), RightPanel::EditProfile(_)) />
                            <button class="btn-browse" on:click=move |_| {
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
                            on:input=move |ev| set_form_branch.set(event_target_value(&ev))
                            placeholder="main" />
                    </label>
                    <label class="panel-label">"MC Version"
                        <input type="text" prop:value=move || form_mc_version.get()
                            on:input=move |ev| set_form_mc_version.set(event_target_value(&ev))
                            placeholder="e.g. 1.21.11" />
                    </label>
                    <label class="panel-label">"Remote URL"
                        <input type="text" prop:value=move || form_remote_url.get()
                            on:input=move |ev| set_form_remote_url.set(event_target_value(&ev))
                            placeholder="https://..." />
                    </label>
                    <Show when=move || matches!(right_panel.get(), RightPanel::EditProfile(_))>
                        <button class="btn-load-profile" on:click=move |_| {
                            set_active_profile.set(Profile {
                                save_dir: form_save_dir.get_untracked(),
                                branch: form_branch.get_untracked(),
                                mc_version: form_mc_version.get_untracked(),
                                remote_url: form_remote_url.get_untracked(),
                            });
                            set_show_profiles.set(false);
                            do_close(RightPanel::None);
                        }>"Load this profile"</button>
                    </Show>
                    <button class="btn-panel-primary" on:click=save_profile_form>"Save"</button>
                </div>
            </div>
        </div>
    }
}

// ── App ─────────────────────────────────────────────────────────────────────

#[component]
pub fn App() -> impl IntoView {
    let (active_profile, set_active_profile) = signal(Profile {
        save_dir: String::new(),
        branch: DEFAULT_BRANCH.to_string(),
        mc_version: DEFAULT_MC_VERSION.to_string(),
        remote_url: String::new(),
    });
    let (output_lines, set_output_lines) = signal(Vec::<String>::new());
    let (is_running, set_is_running) = signal(false);
    let (show_profiles, set_show_profiles) = signal(false);
    let (right_panel, set_right_panel) = signal(RightPanel::None);
    let (profiles, set_profiles) = signal(Vec::<Profile>::new());
    let (commits, set_commits) = signal(Vec::<CommitInfo>::new());
    let (repo_exists, set_repo_exists) = signal(false);
    let (form_closing, set_form_closing) = signal(false);
    let (list_instant, set_list_instant) = signal(false);
    let (draft_message, set_draft_message) = signal(String::new());
    let (form_save_dir, set_form_save_dir) = signal(String::new());
    let (form_branch, set_form_branch) = signal(DEFAULT_BRANCH.to_string());
    let (form_mc_version, set_form_mc_version) = signal(DEFAULT_MC_VERSION.to_string());
    let (form_remote_url, set_form_remote_url) = signal(String::new());

    let refresh = make_refresh_repo_state(set_repo_exists, set_commits, set_output_lines);
    let do_upsert = make_upsert_profile(set_profiles);

    Effect::new(move |_| {
        let dir = active_profile.get().save_dir;
        set_repo_exists.set(false);
        set_commits.set(vec![]);
        refresh(dir);
    });

    spawn_local(async move {
        if let Ok(result) = invoke("get_profiles", JsValue::NULL).await {
            if let Ok(p) = serde_wasm_bindgen::from_value::<Vec<Profile>>(result) {
                set_profiles.set(p);
            }
        }
    });

    setup_event_listeners(set_output_lines, set_is_running, active_profile, refresh);

    let run_commit = move |_: leptos::ev::MouseEvent| {
        let msg = draft_message.get_untracked();
        if msg.is_empty() {
            set_output_lines.update(|l| l.push("Error: Commit message is empty".to_string()));
            return;
        }
        set_output_lines.set(Vec::new());
        set_is_running.set(true);
        set_right_panel.set(RightPanel::None);
        let p = active_profile.get_untracked();
        set_draft_message.set(String::new());
        spawn_local(async move {
            let args = to_js(&RunCommitArgs {
                save_dir: p.save_dir.clone(), branch: p.branch.clone(),
                message: msg, mc_version: p.mc_version.clone(),
            });
            if let Err(err) = invoke("run_commit", args).await {
                set_output_lines.update(|l| l.push(format!("Error: {}", js_error_to_string(err))));
            }
            do_upsert(p);
        });
    };

    let run_checkout = move |commit: String| {
        set_output_lines.set(Vec::new());
        set_is_running.set(true);
        let p = active_profile.get_untracked();
        spawn_local(async move {
            let args = to_js(&RunCheckoutArgs {
                save_dir: p.save_dir.clone(), commit, mc_version: p.mc_version.clone(),
            });
            if let Err(err) = invoke("run_checkout", args).await {
                set_output_lines.update(|l| l.push(format!("Error: {}", js_error_to_string(err))));
            }
            do_upsert(p);
        });
    };

    let run_pull = move |_: leptos::ev::MouseEvent| {
        run_remote_op("run_pull",
            |p| to_js(&RunPullArgs { save_dir: p.save_dir.clone(), url: p.remote_url.clone() }),
            active_profile, set_output_lines, set_is_running, do_upsert);
    };
    let run_push = move |_: leptos::ev::MouseEvent| {
        run_remote_op("run_push",
            |p| to_js(&RunPushArgs { save_dir: p.save_dir.clone(), url: p.remote_url.clone() }),
            active_profile, set_output_lines, set_is_running, do_upsert);
    };
    let run_clone = move |_: leptos::ev::MouseEvent| {
        let p = active_profile.get_untracked();
        if p.remote_url.is_empty() {
            set_output_lines.update(|l| l.push("Error: Remote URL is empty".to_string()));
            return;
        }
        run_remote_op("run_clone",
            |p| to_js(&RunCloneArgs { save_dir: p.save_dir.clone(), url: p.remote_url.clone() }),
            active_profile, set_output_lines, set_is_running, do_upsert);
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
        if p.save_dir.is_empty() { return; }
        do_upsert(p);
        set_form_closing.set(true);
        set_list_instant.set(true);
        let cb = Closure::<dyn Fn()>::new(move || {
            set_right_panel.set(RightPanel::None);
            set_form_closing.set(false);
            set_list_instant.set(false);
        });
        set_timeout(&cb, FORM_CLOSE_ANIMATION_MS);
        cb.forget();
    };

    view! {
        <div class="app">
            <ProfileListPanel
                show_profiles=show_profiles list_instant=list_instant right_panel=right_panel
                profiles=profiles set_show_profiles=set_show_profiles
                set_active_profile=set_active_profile set_right_panel=set_right_panel
                set_form_save_dir=set_form_save_dir set_form_branch=set_form_branch
                set_form_mc_version=set_form_mc_version set_form_remote_url=set_form_remote_url
                set_profiles=set_profiles open_add_profile=open_add_profile
            />
            <ProfileFormPanel
                show_profiles=show_profiles right_panel=right_panel form_closing=form_closing
                form_save_dir=form_save_dir form_branch=form_branch
                form_mc_version=form_mc_version form_remote_url=form_remote_url
                set_form_save_dir=set_form_save_dir set_form_branch=set_form_branch
                set_form_mc_version=set_form_mc_version set_form_remote_url=set_form_remote_url
                set_active_profile=set_active_profile set_show_profiles=set_show_profiles
                set_form_closing=set_form_closing set_list_instant=set_list_instant
                set_right_panel=set_right_panel save_profile_form=save_profile_form
            />
            <Show when=move || show_profiles.get() || right_panel.get() == RightPanel::Commit>
                <div class="sidebar-overlay" on:click=move |_| {
                    if right_panel.get_untracked() == RightPanel::Commit {
                        set_right_panel.set(RightPanel::None);
                    } else {
                        set_show_profiles.set(false);
                        if matches!(right_panel.get_untracked(),
                            RightPanel::AddProfile | RightPanel::EditProfile(_))
                        {
                            set_form_closing.set(true);
                            set_list_instant.set(true);
                            let cb = Closure::<dyn Fn()>::new(move || {
                                set_right_panel.set(RightPanel::None);
                                set_form_closing.set(false);
                                set_list_instant.set(false);
                            });
                            set_timeout(&cb, FORM_CLOSE_ANIMATION_MS);
                            cb.forget();
                        }
                    }
                }/>
            </Show>
            <MainContent
                active_profile=active_profile is_running=is_running
                right_panel=right_panel set_right_panel=set_right_panel
                repo_exists=repo_exists commits=commits output_lines=output_lines
                set_show_profiles=set_show_profiles
                draft_message=draft_message set_draft_message=set_draft_message
                run_commit=run_commit run_checkout=run_checkout
                run_pull=run_pull run_push=run_push run_clone=run_clone
            />
        </div>
    }
}
