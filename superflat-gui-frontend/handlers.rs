use crate::bindings::{invoke, log};
use crate::types::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;

pub fn make_refresh_repo_state(
    set_repo_exists: WriteSignal<bool>,
    set_commits: WriteSignal<Vec<CommitInfo>>,
    set_output_lines: WriteSignal<Vec<String>>,
) -> impl Fn(String) + Copy + 'static {
    move |dir: String| {
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
                Err(err) => set_output_lines.update(|l| {
                    l.push(format!(
                        "Failed to load commits: {}",
                        js_error_to_string(err)
                    ))
                }),
            }
        });
    }
}

pub fn make_upsert_profile(
    set_profiles: WriteSignal<Vec<Profile>>,
) -> impl Fn(Profile) + Copy + 'static {
    move |p: Profile| {
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
                    set_profiles.set(ps);
                }
            }
        });
    }
}

pub fn run_remote_op<F: Fn(&Profile) -> JsValue>(
    cmd: &'static str,
    args_fn: F,
    active_profile: ReadSignal<Profile>,
    set_output_lines: WriteSignal<Vec<String>>,
    set_is_running: WriteSignal<bool>,
    do_upsert_profile: impl Fn(Profile) + 'static,
) {
    set_output_lines.set(Vec::new());
    set_is_running.set(true);
    let p = active_profile.get_untracked();
    let args = args_fn(&p);
    spawn_local(async move {
        if let Err(err) = invoke(cmd, args).await {
            set_output_lines.update(|l| l.push(format!("Error: {}", js_error_to_string(err))));
        }
        do_upsert_profile(p);
    });
}

#[component]
pub fn MainContent(
    active_profile: ReadSignal<Profile>,
    is_running: ReadSignal<bool>,
    right_panel: ReadSignal<RightPanel>,
    set_right_panel: WriteSignal<RightPanel>,
    repo_exists: ReadSignal<bool>,
    commits: ReadSignal<Vec<CommitInfo>>,
    set_show_profiles: WriteSignal<bool>,
    draft_message: ReadSignal<String>,
    set_draft_message: WriteSignal<String>,
    run_commit: impl Fn(leptos::ev::MouseEvent) + Copy + Send + Sync + 'static,
    run_checkout: impl Fn(String) + Copy + Send + Sync + 'static,
    run_pull: impl Fn(leptos::ev::MouseEvent) + Copy + Send + Sync + 'static,
    run_push: impl Fn(leptos::ev::MouseEvent) + Copy + Send + Sync + 'static,
    run_clone: impl Fn(leptos::ev::MouseEvent) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    view! {
        <div class="main">
            <div class="topbar">
                <button class="btn-menu"
                    on:click=move |_| set_show_profiles.update(|v| *v = !*v)
                    disabled=move || is_running.get()>"☰"
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
                </div>
                <Show when=move || !active_profile.get().save_dir.is_empty()>
                    <div class="topbar-actions">
                        <button class="btn-action btn-commit"
                            on:click=move |_| {
                                if right_panel.get_untracked() == RightPanel::Commit {
                                    set_right_panel.set(RightPanel::None);
                                } else {
                                    set_right_panel.set(RightPanel::Commit);
                                }
                            }
                            disabled=move || is_running.get()>"Commit"
                        </button>
                        <Show when=move || repo_exists.get()
                            fallback=move || view! {
                                <button class="btn-action btn-clone" on:click=run_clone
                                    disabled=move || is_running.get()>"Clone"</button>
                            }
                        >
                            <Show when=move || !active_profile.get().remote_url.is_empty()
                                fallback=move || view! {
                                    <button class="btn-action btn-pull" on:click=run_pull
                                        disabled=move || is_running.get()>"Set Remote"</button>
                                }
                            >
                                <button class="btn-action btn-pull" on:click=run_pull
                                    disabled=move || is_running.get()>"Pull"</button>
                            </Show>
                        </Show>
                        <Show when=move || repo_exists.get() && !active_profile.get().remote_url.is_empty()>
                            <button class="btn-action btn-push" on:click=run_push
                                disabled=move || is_running.get()>"Push"</button>
                        </Show>
                    </div>
                </Show>
            </div>
            <div class="body">
                <div class="commit-area">
                    <Show when=move || commits.get().is_empty() fallback=|| view! {}>
                        <div class="commit-empty">
                            <span>"Welcome to Superflat GUI"<br></br><br></br>"Click Menu ☰ to start"</span>
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
                                        <button class="btn-checkout"
                                            disabled=move || is_running.get()
                                            on:click=move |_| run_checkout(hash.clone())>
                                            "Checkout"
                                        </button>
                                    </div>
                                }
                            }
                        />
                    </div>
                </div>
            </div>
        </div>
        <div class="sidebar" class:open=move || right_panel.get() == RightPanel::Commit>
            <div class="sidebar-panel-form">
                <div class="panel-body">
                    {
                        let (commit_show_error, set_commit_show_error) = signal(false);
                        view! {
                            <label class="panel-label">
                                "Commit message"
                                <textarea
                                    prop:value=move || draft_message.get()
                                    on:input=move |ev| {
                                        set_draft_message.set(event_target_value(&ev));
                                        set_commit_show_error.set(false);
                                    }
                                    class:invalid=move || commit_show_error.get() && draft_message.get().trim().is_empty()
                                    placeholder="type(scope): subject\n\nbody (optional)"
                                    rows="4" />
                            </label>
                            <button class="btn-panel-primary btn-commit-modal"
                                on:click=move |ev| {
                                    if draft_message.get_untracked().trim().is_empty() {
                                        set_commit_show_error.set(true);
                                    } else {
                                        run_commit(ev);
                                    }
                                }
                                disabled=move || is_running.get()>
                                "Commit"
                            </button>
                        }
                    }
                </div>
            </div>
        </div>
    }
}
