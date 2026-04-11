use crate::bindings::{invoke, log, set_timeout};
use crate::handlers::{MainContent, make_refresh_repo_state, make_upsert_profile, run_remote_op};
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
            <div class="profile-card-row">
                <div class="profile-card-meta">{
                    p.mc_version.clone()
                }</div>
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
    open_clone_from_remote: impl Fn(leptos::ev::MouseEvent) + Copy + 'static,
) -> impl IntoView {
    view! {
        <div class="sidebar"
            class:open=move || show_profiles.get()
                && !matches!(right_panel.get(), RightPanel::AddProfile | RightPanel::EditProfile(_) | RightPanel::CloneFromRemote)
            class:no-transition=move || list_instant.get()
        >
            <div class="sidebar-panel-list">
                <div class="sidebar-body">
                    <div class="btn-add-profile-group">
                        <button class="btn-add-profile" on:click=open_add_profile>"+ Track Local Save"</button>
                        <button class="btn-add-profile" on:click=open_clone_from_remote>"+ Clone From Remote"</button>
                    </div>
                    <Show when=move || profiles.get().is_empty() fallback=|| view! {}>
                        <p class="sidebar-empty">"No profiles yet"</p>
                    </Show>
                    <div class="profile-list">
                        <For each=move || {
                                let mut ps = profiles.get();
                                ps.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
                                ps
                            } key=|p| format!("{}:{}", p.save_dir, p.updated_at)
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
        </div>
    }
}

// ── Add profile panel ────────────────────────────────────────────────────────

#[component]
fn AddProfilePanel(
    show_profiles: ReadSignal<bool>,
    right_panel: ReadSignal<RightPanel>,
    form_closing: ReadSignal<bool>,
    form_save_dir: ReadSignal<String>,
    form_branch: ReadSignal<String>,
    form_mc_version: ReadSignal<String>,
    set_form_save_dir: WriteSignal<String>,
    set_form_branch: WriteSignal<String>,
    set_form_mc_version: WriteSignal<String>,
    save_profile_form: impl Fn(leptos::ev::MouseEvent) + Copy + 'static,
) -> impl IntoView {
    let (show_errors, set_show_errors) = signal(false);
    view! {
        <div class="sidebar"
            class:open=move || (right_panel.get() == RightPanel::AddProfile
                || (form_closing.get() && right_panel.get() != RightPanel::EditProfile(String::new())))
                && show_profiles.get()
        >
            <div class="sidebar-panel-form" class:closing=move || form_closing.get()>
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
                        <button class="btn-panel-primary" on:click=move |ev| {
                            let dir_ok = !form_save_dir.get_untracked().trim().is_empty();
                            let branch_ok = !form_branch.get_untracked().trim().is_empty();
                            let ver_ok = !form_mc_version.get_untracked().trim().is_empty();
                            if dir_ok && branch_ok && ver_ok {
                                save_profile_form(ev);
                            } else {
                                set_show_errors.set(false);
                                let cb = Closure::<dyn Fn()>::new(move || set_show_errors.set(true));
                                set_timeout(&cb, 0);
                                cb.forget();
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
fn EditProfilePanel(
    show_profiles: ReadSignal<bool>,
    right_panel: ReadSignal<RightPanel>,
    form_closing: ReadSignal<bool>,
    form_save_dir: ReadSignal<String>,
    form_branch: ReadSignal<String>,
    form_mc_version: ReadSignal<String>,
    form_remote_url: ReadSignal<String>,
    set_form_branch: WriteSignal<String>,
    set_form_mc_version: WriteSignal<String>,
    set_form_remote_url: WriteSignal<String>,
    remote_url_invalid: ReadSignal<bool>,
    set_remote_url_invalid: WriteSignal<bool>,
    save_profile_form: impl Fn(leptos::ev::MouseEvent) + Copy + 'static,
) -> impl IntoView {
    let (show_errors, set_show_errors) = signal(false);
    view! {
        <div class="sidebar"
            class:open=move || (matches!(right_panel.get(), RightPanel::EditProfile(_))
                || form_closing.get()) && show_profiles.get()
        >
            <div class="sidebar-panel-form" class:closing=move || form_closing.get()>
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
                        <label class="panel-label">"Remote URL"
                            <input type="text" prop:value=move || form_remote_url.get()
                                on:input=move |ev| {
                                    set_form_remote_url.set(event_target_value(&ev));
                                    set_remote_url_invalid.set(false);
                                }
                                class:invalid=move || remote_url_invalid.get()
                                placeholder="ssh://..." />
                        </label>
                        <button class="btn-panel-primary" on:click=move |ev| {
                            let branch_ok = !form_branch.get_untracked().trim().is_empty();
                            let ver_ok = !form_mc_version.get_untracked().trim().is_empty();
                            if branch_ok && ver_ok {
                                save_profile_form(ev);
                            } else {
                                set_show_errors.set(false);
                                let cb = Closure::<dyn Fn()>::new(move || set_show_errors.set(true));
                                set_timeout(&cb, 0);
                                cb.forget();
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
fn CloneFromRemoteFormPanel(
    show_profiles: ReadSignal<bool>,
    right_panel: ReadSignal<RightPanel>,
    form_closing: ReadSignal<bool>,
    form_clone_git_dir: ReadSignal<String>,
    form_remote_url: ReadSignal<String>,
    form_branch: ReadSignal<String>,
    form_mc_version: ReadSignal<String>,
    set_form_clone_git_dir: WriteSignal<String>,
    set_form_remote_url: WriteSignal<String>,
    set_form_branch: WriteSignal<String>,
    set_form_mc_version: WriteSignal<String>,
    set_form_closing: WriteSignal<bool>,
    set_list_instant: WriteSignal<bool>,
    set_right_panel: WriteSignal<RightPanel>,
    clone_profile_form: impl Fn(leptos::ev::MouseEvent) + Copy + 'static,
) -> impl IntoView {
    let _do_close = move |next: RightPanel| {
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
    let (clone_show_errors, set_clone_show_errors) = signal(false);
    view! {
        <div class="sidebar"
            class:open=move || (right_panel.get() == RightPanel::CloneFromRemote
                || form_closing.get()) && show_profiles.get()
        >
            <div class="sidebar-panel-form" class:closing=move || form_closing.get()>
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
                                <button class="btn-browse" on:click=move |_| {
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
                        <button class="btn-panel-primary" on:click=move |ev| {
                            let git_dir_ok = !form_clone_git_dir.get_untracked().trim().is_empty();
                            let url_ok = !form_remote_url.get_untracked().trim().is_empty();
                            let branch_ok = !form_branch.get_untracked().trim().is_empty();
                            let ver_ok = !form_mc_version.get_untracked().trim().is_empty();
                            if git_dir_ok && url_ok && branch_ok && ver_ok {
                                clone_profile_form(ev);
                            } else {
                                set_clone_show_errors.set(false);
                                let cb = Closure::<dyn Fn()>::new(move || set_clone_show_errors.set(true));
                                set_timeout(&cb, 0);
                                cb.forget();
                            }
                        }>"Clone"</button>
                    </div>
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
        mc_version: String::new(),
        remote_url: String::new(),
        updated_at: String::new(),
    });
    let (output_lines, set_output_lines) = signal(Vec::<String>::new());
    let (last_raw_line, set_last_raw_line) = signal(String::new());
    let (op_start_ms, set_op_start_ms) = signal(0.0_f64);
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
    let (form_mc_version, set_form_mc_version) = signal(String::new());
    let (form_remote_url, set_form_remote_url) = signal(String::new());
    let (form_clone_git_dir, set_form_clone_git_dir) = signal(String::new());

    let (show_log, set_show_log) = signal(false);
    let (remote_url_invalid, set_remote_url_invalid) = signal(false);
    let log_console_ref = NodeRef::<leptos::html::Pre>::new();

    Effect::new(move |_| {
        let _ = output_lines.get();
        if let Some(el) = log_console_ref.get() {
            el.set_scroll_top(el.scroll_height());
        }
    });

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

    // Wrap setup_event_listeners to also clear current_action on done
    spawn_local(async move {
        let set_lines = set_output_lines;
        let on_output = Closure::<dyn Fn(JsValue)>::new(move |event: JsValue| {
            let payload = js_sys::Reflect::get(&event, &JsValue::from_str("payload"))
                .unwrap_or(JsValue::NULL);
            let level = js_sys::Reflect::get(&payload, &JsValue::from_str("level"))
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default();
            let message = js_sys::Reflect::get(&payload, &JsValue::from_str("message"))
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default();
            if message.is_empty() {
                return;
            }
            let elapsed_ms = js_sys::Date::now() - op_start_ms.get_untracked();
            let elapsed_s = elapsed_ms / 1000.0;
            let int_part = elapsed_s.floor() as u64;
            let frac_digits = ((elapsed_s - int_part as f64) * 1000.0).round() as u64;
            let time_prefix = format!("[{:>4}.{:03}]", int_part, frac_digits);
            // status bar: [xxxx.xxx] message
            let status_line = format!("{} {}", time_prefix, message);
            // log modal: [xxxx.xxx] [LEVEL] message
            let log_line = format!("{} [{}] {}", time_prefix, level, message);
            set_last_raw_line.set(status_line);
            set_lines.update(|lines| lines.push(log_line));
        });
        let on_done = Closure::<dyn Fn(JsValue)>::new(move |_: JsValue| {
            set_is_running.set(false);
            refresh(active_profile.get_untracked().save_dir);
        });
        if let (Ok(_), Ok(_)) = (
            crate::bindings::tauri_listen(EVENT_OUTPUT, &on_output).await,
            crate::bindings::tauri_listen(EVENT_DONE, &on_done).await,
        ) {
            on_output.forget();
            on_done.forget();
        }
    });

    let run_commit = move |_: leptos::ev::MouseEvent| {
        let msg = draft_message.get_untracked();
        if msg.is_empty() {
            set_output_lines.update(|l| l.push("Error: Commit message is empty".to_string()));
            return;
        }
        let p = active_profile.get_untracked();
        set_op_start_ms.set(js_sys::Date::now());
        set_last_raw_line.set(String::new());
        set_output_lines.set(Vec::new());
        set_draft_message.set(String::new());
        spawn_local(async move {
            let args = to_js(&RunCommitArgs {
                save_dir: p.save_dir.clone(),
                branch: p.branch.clone(),
                message: msg,
                mc_version: p.mc_version.clone(),
            });
            if let Err(err) = invoke("run_commit", args).await {
                set_output_lines.update(|l| l.push(format!("Error: {}", js_error_to_string(err))));
            }
            do_upsert(p);
        });
    };

    let run_checkout = move |commit: String| {
        let p = active_profile.get_untracked();
        if p.save_dir.is_empty() {
            set_output_lines.update(|l| l.push("Error: Please load a profile first".to_string()));
            return;
        }
        set_op_start_ms.set(js_sys::Date::now());
        set_last_raw_line.set(String::new());
        set_output_lines.set(Vec::new());
        set_is_running.set(true);

        spawn_local(async move {
            let args = to_js(&RunCheckoutArgs {
                save_dir: p.save_dir.clone(),
                commit,
                mc_version: p.mc_version.clone(),
            });
            if let Err(err) = invoke("run_checkout", args).await {
                set_output_lines.update(|l| l.push(format!("Error: {}", js_error_to_string(err))));
            }
            do_upsert(p);
        });
    };

    let do_pull = move || {
        set_op_start_ms.set(js_sys::Date::now());
        set_last_raw_line.set(String::new());
        set_right_panel.set(RightPanel::None);
        run_remote_op(
            "run_pull",
            |p| {
                to_js(&RunPullArgs {
                    save_dir: p.save_dir.clone(),
                    url: p.remote_url.clone(),
                })
            },
            active_profile,
            set_output_lines,
            set_is_running,
            do_upsert,
        );
    };

    let run_pull = move |_: leptos::ev::MouseEvent| {
        let p = active_profile.get_untracked();
        if p.remote_url.is_empty() {
            set_form_save_dir.set(p.save_dir.clone());
            set_form_branch.set(p.branch.clone());
            set_form_mc_version.set(p.mc_version.clone());
            set_form_remote_url.set(String::new());
            set_remote_url_invalid.set(false);
            let cb = Closure::<dyn Fn()>::new(move || set_remote_url_invalid.set(true));
            set_timeout(&cb, 0);
            cb.forget();
            set_right_panel.set(RightPanel::EditProfile(p.save_dir.clone()));
            set_show_profiles.set(true);
            return;
        }
        set_right_panel.set(RightPanel::ConfirmPull);
    };

    let do_push = move || {
        set_op_start_ms.set(js_sys::Date::now());
        set_last_raw_line.set(String::new());
        set_right_panel.set(RightPanel::None);
        run_remote_op(
            "run_push",
            |p| {
                to_js(&RunPushArgs {
                    save_dir: p.save_dir.clone(),
                    url: p.remote_url.clone(),
                })
            },
            active_profile,
            set_output_lines,
            set_is_running,
            do_upsert,
        );
    };

    let run_push = move |_: leptos::ev::MouseEvent| {
        let p = active_profile.get_untracked();
        if p.remote_url.is_empty() {
            set_form_save_dir.set(p.save_dir.clone());
            set_form_branch.set(p.branch.clone());
            set_form_mc_version.set(p.mc_version.clone());
            set_form_remote_url.set(String::new());
            set_remote_url_invalid.set(false);
            let cb = Closure::<dyn Fn()>::new(move || set_remote_url_invalid.set(true));
            set_timeout(&cb, 0);
            cb.forget();
            set_right_panel.set(RightPanel::EditProfile(p.save_dir.clone()));
            set_show_profiles.set(true);
            return;
        }
        set_right_panel.set(RightPanel::ConfirmPush);
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
        set_op_start_ms.set(js_sys::Date::now());
        set_last_raw_line.set(String::new());
        run_remote_op(
            "run_clone",
            |p| {
                to_js(&RunCloneArgs {
                    save_dir: p.save_dir.clone(),
                    url: p.remote_url.clone(),
                })
            },
            active_profile,
            set_output_lines,
            set_is_running,
            do_upsert,
        );
    };

    let open_add_profile = move |_: leptos::ev::MouseEvent| {
        set_form_save_dir.set(String::new());
        set_form_branch.set(DEFAULT_BRANCH.to_string());
        set_form_mc_version.set(String::new());
        set_right_panel.set(RightPanel::AddProfile);
        set_show_profiles.set(true);
    };

    let save_profile_form = move |_: leptos::ev::MouseEvent| {
        let p = Profile {
            save_dir: form_save_dir.get_untracked(),
            branch: form_branch.get_untracked(),
            mc_version: form_mc_version.get_untracked(),
            remote_url: form_remote_url.get_untracked(),
            updated_at: String::new(),
        };
        if p.save_dir.is_empty() {
            return;
        }
        // 如果是编辑模式且当前加载的就是这个 profile，同步更新 active_profile
        if matches!(right_panel.get_untracked(), RightPanel::EditProfile(_))
            && active_profile.get_untracked().save_dir == p.save_dir
        {
            set_active_profile.set(p.clone());
        }
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

    let open_clone_from_remote = move |_: leptos::ev::MouseEvent| {
        set_form_clone_git_dir.set(String::new());
        set_form_remote_url.set(String::new());
        set_form_branch.set(DEFAULT_BRANCH.to_string());
        set_form_mc_version.set(String::new());
        set_right_panel.set(RightPanel::CloneFromRemote);
        set_show_profiles.set(true);
    };

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
        set_active_profile.set(p.clone());
        do_upsert(p.clone());
        set_op_start_ms.set(js_sys::Date::now());
        set_last_raw_line.set(String::new());
        set_output_lines.set(Vec::new());
        set_is_running.set(true);
        spawn_local(async move {
            let args = to_js(&RunCloneArgs {
                save_dir,
                url: remote_url,
            });
            if let Err(err) = invoke("run_clone", args).await {
                set_output_lines
                    .update(|l| l.push(format!("Error: {}", js_error_to_string(err))));
            }
            do_upsert(p);
        });
        set_form_closing.set(true);
        set_list_instant.set(true);
        let cb = Closure::<dyn Fn()>::new(move || {
            set_right_panel.set(RightPanel::None);
            set_show_profiles.set(false);
            set_form_closing.set(false);
            set_list_instant.set(false);
        });
        set_timeout(&cb, FORM_CLOSE_ANIMATION_MS);
        cb.forget();
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
                log(&format!(
                    "toggle maximize failed: {}",
                    js_error_to_string(err)
                ));
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
                log(&format!(
                    "start dragging failed: {}",
                    js_error_to_string(err)
                ));
            }
        });
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
                open_clone_from_remote=open_clone_from_remote
            />
            <AddProfilePanel
                show_profiles=show_profiles right_panel=right_panel form_closing=form_closing
                form_save_dir=form_save_dir form_branch=form_branch form_mc_version=form_mc_version
                set_form_save_dir=set_form_save_dir set_form_branch=set_form_branch
                set_form_mc_version=set_form_mc_version save_profile_form=save_profile_form
            />
            <EditProfilePanel
                show_profiles=show_profiles right_panel=right_panel form_closing=form_closing
                form_save_dir=form_save_dir form_branch=form_branch
                form_mc_version=form_mc_version form_remote_url=form_remote_url
                set_form_branch=set_form_branch set_form_mc_version=set_form_mc_version
                set_form_remote_url=set_form_remote_url
                remote_url_invalid=remote_url_invalid set_remote_url_invalid=set_remote_url_invalid
                save_profile_form=save_profile_form
            />
            <CloneFromRemoteFormPanel
                show_profiles=show_profiles right_panel=right_panel form_closing=form_closing
                form_clone_git_dir=form_clone_git_dir form_remote_url=form_remote_url
                form_branch=form_branch form_mc_version=form_mc_version
                set_form_clone_git_dir=set_form_clone_git_dir set_form_remote_url=set_form_remote_url
                set_form_branch=set_form_branch set_form_mc_version=set_form_mc_version
                set_form_closing=set_form_closing set_list_instant=set_list_instant
                set_right_panel=set_right_panel clone_profile_form=clone_profile_form
            />
            <Show when=move || show_profiles.get() || matches!(right_panel.get(), RightPanel::Commit | RightPanel::Checkout(_) | RightPanel::ConfirmPull | RightPanel::ConfirmPush) || show_log.get()>
                <div class="sidebar-overlay" on:click=move |_| {
                    if show_log.get_untracked() {
                        set_show_log.set(false);
                    } else if matches!(right_panel.get_untracked(), RightPanel::Commit | RightPanel::Checkout(_) | RightPanel::ConfirmPull | RightPanel::ConfirmPush) {
                        set_right_panel.set(RightPanel::None);
                    } else {
                        set_show_profiles.set(false);
                        if matches!(right_panel.get_untracked(),
                            RightPanel::AddProfile | RightPanel::EditProfile(_) | RightPanel::CloneFromRemote)
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

            // ── Main content ────────────────────────────────────────
            <div class="main">
                <div class="window-titlebar">
                    <div class="window-title-drag" data-tauri-drag-region=true on:mousedown=handle_window_drag>
                        <span class="window-title">"Superflat GUI"</span>
                    </div>
                    <div class="window-controls">
                        <button class="window-btn" on:click=handle_window_minimize title="Minimize">"-"</button>
                        <button class="window-btn" on:click=handle_window_toggle_maximize title="Maximize / Restore">"□"</button>
                        <button class="window-btn window-btn-close" on:click=handle_window_close title="Close">"✕"</button>
                    </div>
                </div>
                <MainContent
                    active_profile=active_profile is_running=is_running
                    right_panel=right_panel set_right_panel=set_right_panel
                    repo_exists=repo_exists commits=commits
                    set_show_profiles=set_show_profiles
                    draft_message=draft_message set_draft_message=set_draft_message
                    run_commit=run_commit run_checkout=run_checkout
                    run_pull=run_pull run_push=run_push run_clone=run_clone
                    do_pull=do_pull do_push=do_push
                />
                // ── Status bar ──────────────────────────────────
                <div class="status-bar">
                    <span class="status-bar-latest-log">
                        {move || last_raw_line.get()}
                    </span>
                    <Show when=move || is_running.get() || !output_lines.get().is_empty()>
                        <button class="status-bar-btn status-bar-log-btn"
                            class:running=move || is_running.get()
                            on:click=move |_| set_show_log.set(true)>
                            "📋 Latest Log"
                        </button>
                    </Show>
                </div>
            </div>

            // ── Log modal ────────────────────────────────────────────
            <div class="sidebar" class:open=move || show_log.get()>
                <div class="sidebar-panel-form">
                    <div class="sidebar-body">
                        <pre class="log-console" node_ref=log_console_ref>{move || output_lines.get().join("\n")}</pre>
                    </div>
                </div>
            </div>
        </div>
    }
}
