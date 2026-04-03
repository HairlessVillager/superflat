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
struct SetDebugLoggingArgs {
    debug: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct Profile {
    save_dir: String,
    mc_version: String,
    branch: String,
    default_commit: String,
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

#[component]
pub fn App() -> impl IntoView {
    let (save_dir, set_save_dir) = signal(String::new());
    let (branch, set_branch) = signal(String::from("main"));
    let (mc_version, set_mc_version) = signal(String::from("1.21.11"));
    let (message, set_message) = signal(String::new());
    let (commit_id, set_commit_id) = signal(String::from("main@{10 minutes ago}"));
    let (clock, set_clock) = signal(current_datetime_string());
    let (output_lines, set_output_lines) = signal(Vec::<String>::new());
    let (is_running, set_is_running) = signal(false);
    let (show_settings, set_show_settings) = signal(false);
    let (show_profiles, set_show_profiles) = signal(false);
    let (profiles, set_profiles) = signal(Vec::<Profile>::new());
    // draft values edited inside the settings dialog
    let (draft_branch, set_draft_branch) = signal(String::new());
    let (draft_mc_version, set_draft_mc_version) = signal(String::new());
    let (draft_default_commit, set_draft_default_commit) = signal(String::new());
    let (debug_enabled, set_debug_enabled) = signal(false);
    let (draft_debug_enabled, set_draft_debug_enabled) = signal(false);

    // Tick clock every second
    let tick = Closure::<dyn Fn()>::new(move || {
        set_clock.set(current_datetime_string());
    });
    set_interval(&tick, 1000);
    tick.forget();

    // Load profiles on mount
    spawn_local(async move {
        let result = invoke("get_profiles", JsValue::NULL).await;
        if let Ok(result) = result {
            if let Ok(p) = serde_wasm_bindgen::from_value::<Vec<Profile>>(result) {
                set_profiles.set(p);
            }
        } else if let Err(err) = result {
            log(&format!("get_profiles failed: {}", js_error_to_string(err)));
        }
    });

    // Keep backend output listeners active for the lifetime of the app.
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
        });

        let listen_output = tauri_listen("commit-output", &on_output).await;
        let listen_done = tauri_listen("commit-done", &on_done).await;

        match (listen_output, listen_done) {
            (Ok(_), Ok(_)) => {
                on_output.forget();
                on_done.forget();
            }
            (output_result, done_result) => {
                if let Err(err) = output_result {
                    set_output_lines.update(|lines| {
                        lines.push(format!(
                            "Error: failed to listen for backend output: {}",
                            js_error_to_string(err)
                        ))
                    });
                }
                if let Err(err) = done_result {
                    set_output_lines.update(|lines| {
                        lines.push(format!(
                            "Error: failed to listen for backend completion: {}",
                            js_error_to_string(err)
                        ))
                    });
                }
            }
        }
    });

    let pick_dir = move |_| {
        spawn_local(async move {
            match invoke("pick_directory", JsValue::NULL).await {
                Ok(result) => {
                    if let Some(path) = result.as_string() {
                        set_save_dir.set(path);
                    }
                }
                Err(err) => log(&format!(
                    "pick_directory failed: {}",
                    js_error_to_string(err)
                )),
            }
        });
    };

    let open_settings = move |_| {
        set_draft_branch.set(branch.get_untracked());
        set_draft_mc_version.set(mc_version.get_untracked());
        set_draft_default_commit.set(commit_id.get_untracked());
        set_draft_debug_enabled.set(debug_enabled.get_untracked());
        set_show_settings.set(true);
    };

    let close_settings = move |_| {
        set_show_settings.set(false);
    };

    let apply_settings = move |_| {
        let b = draft_branch.get_untracked();
        let v = draft_mc_version.get_untracked();
        let c = draft_default_commit.get_untracked();
        let d = draft_debug_enabled.get_untracked();
        set_branch.set(b.clone());
        set_mc_version.set(v.clone());
        set_commit_id.set(c.clone());
        set_debug_enabled.set(d);
        set_show_settings.set(false);
        spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&SetDebugLoggingArgs { debug: d }).unwrap();
            if let Err(err) = invoke("set_debug_logging", args).await {
                log(&format!(
                    "set_debug_logging failed: {}",
                    js_error_to_string(err)
                ));
            }
        });
    };

    let run_commit = move |_| {
        log("run_commit clicked");
        set_output_lines.set(Vec::new());
        set_is_running.set(true);

        spawn_local(async move {
            log("spawn_local started");
            let save_dir_val = save_dir.get_untracked();
            let branch_val = branch.get_untracked();
            let mc_version_val = mc_version.get_untracked();
            let default_commit_val = commit_id.get_untracked();
            let message_val = {
                let m = message.get_untracked();
                if m.is_empty() {
                    current_datetime_string()
                } else {
                    m
                }
            };

            let args = serde_wasm_bindgen::to_value(&RunCommitArgs {
                save_dir: save_dir_val.clone(),
                branch: branch_val.clone(),
                message: message_val,
                mc_version: mc_version_val.clone(),
            })
            .unwrap();
            if let Err(err) = invoke("run_commit", args).await {
                set_output_lines.update(|lines| {
                    lines.push(format!(
                        "Error: run_commit failed: {}",
                        js_error_to_string(err)
                    ))
                });
            }
            set_is_running.set(false);

            let profile_args = serde_wasm_bindgen::to_value(&UpsertProfileArgs {
                profile: Profile {
                    save_dir: save_dir_val,
                    mc_version: mc_version_val,
                    branch: branch_val,
                    default_commit: default_commit_val,
                },
            })
            .unwrap();
            if let Err(err) = invoke("upsert_profile", profile_args).await {
                log(&format!(
                    "upsert_profile failed: {}",
                    js_error_to_string(err)
                ));
            }
        });
    };

    let run_checkout = move |_| {
        set_output_lines.set(Vec::new());
        set_is_running.set(true);

        spawn_local(async move {
            let save_dir_val = save_dir.get_untracked();
            let commit_val = commit_id.get_untracked();
            let mc_version_val = mc_version.get_untracked();
            let branch_val = branch.get_untracked();

            let args = serde_wasm_bindgen::to_value(&RunCheckoutArgs {
                save_dir: save_dir_val.clone(),
                commit: commit_val.clone(),
                mc_version: mc_version_val.clone(),
            })
            .unwrap();
            if let Err(err) = invoke("run_checkout", args).await {
                set_output_lines.update(|lines| {
                    lines.push(format!(
                        "Error: run_checkout failed: {}",
                        js_error_to_string(err)
                    ))
                });
            }
            set_is_running.set(false);

            let profile_args = serde_wasm_bindgen::to_value(&UpsertProfileArgs {
                profile: Profile {
                    save_dir: save_dir_val,
                    mc_version: mc_version_val,
                    branch: branch_val,
                    default_commit: commit_val,
                },
            })
            .unwrap();
            if let Err(err) = invoke("upsert_profile", profile_args).await {
                log(&format!(
                    "upsert_profile failed: {}",
                    js_error_to_string(err)
                ));
            }
        });
    };

    view! {
        <main>
            <div class="toolbar">
                <div class="toolbar-row">
                    <button class="btn-profiles" on:click=move |_| set_show_profiles.set(true) disabled=move || is_running.get()>
                        "Profiles"
                    </button>
                    <input
                        type="text"
                        prop:value=move || save_dir.get()
                        on:input=move |ev| set_save_dir.set(event_target_value(&ev))
                        placeholder="Path to save directory ($SAVE_DIR)"
                    />
                    <button class="btn-pick" on:click=pick_dir disabled=move || is_running.get()>
                        "Browse"
                    </button>
                    <button class="btn-settings" on:click=open_settings disabled=move || is_running.get()>
                        "Settings"
                    </button>
                </div>
                <div class="toolbar-row">
                    <input
                        type="text"
                        prop:value=move || message.get()
                        on:input=move |ev| set_message.set(event_target_value(&ev))
                        placeholder=move || clock.get()
                    />
                    <button on:click=run_commit disabled=move || is_running.get()>
                        {move || if is_running.get() { "Running..." } else { "Commit" }}
                    </button>
                    <input
                        type="text"
                        prop:value=move || commit_id.get()
                        on:input=move |ev| set_commit_id.set(event_target_value(&ev))
                        placeholder="main@{10 minutes ago}"
                    />
                    <button on:click=run_checkout disabled=move || is_running.get()>
                        "Checkout"
                    </button>
                </div>
            </div>

            <Show when=move || show_profiles.get()>
                <div class="modal-backdrop" on:click=move |_| set_show_profiles.set(false)>
                    <div class="modal" on:click=|ev| ev.stop_propagation()>
                        <h2>"Profiles"</h2>
                        <Show
                            when=move || !profiles.get().is_empty()
                            fallback=|| view! { <p class="profiles-empty">"No profiles yet."</p> }
                        >
                            <ul class="profiles-list">
                                <For
                                    each=move || profiles.get()
                                    key=|p| p.save_dir.clone()
                                    children=move |p| {
                                        let label = p.save_dir.clone();
                                        let p2 = p.clone();
                                        view! {
                                            <li>
                                                <button
                                                    class="btn-profile-entry"
                                                    on:click=move |_| {
                                                        set_save_dir.set(p2.save_dir.clone());
                                                        set_branch.set(p2.branch.clone());
                                                        set_mc_version.set(p2.mc_version.clone());
                                                        set_commit_id.set(p2.default_commit.clone());
                                                        set_show_profiles.set(false);
                                                    }
                                                >
                                                    <span class="profile-save-dir">{label}</span>
                                                    <span class="profile-meta">
                                                        {move || format!("{} · {} · {}", p.branch, p.mc_version, p.default_commit)}
                                                    </span>
                                                </button>
                                            </li>
                                        }
                                    }
                                />
                            </ul>
                        </Show>
                        <div class="modal-actions">
                            <button on:click=move |_| set_show_profiles.set(false)>"Close"</button>
                        </div>
                    </div>
                </div>
            </Show>

            <Show when=move || show_settings.get()>
                <div class="modal-backdrop" on:click=close_settings>
                    <div class="modal" on:click=|ev| ev.stop_propagation()>
                        <h2>"Settings"</h2>
                        <label>
                            "Default branch"
                            <input
                                type="text"
                                prop:value=move || draft_branch.get()
                                on:input=move |ev| set_draft_branch.set(event_target_value(&ev))
                            />
                        </label>
                        <label>
                            "Minecraft version"
                            <input
                                type="text"
                                prop:value=move || draft_mc_version.get()
                                on:input=move |ev| set_draft_mc_version.set(event_target_value(&ev))
                                placeholder="e.g. 1.21.11"
                            />
                        </label>
                        <label>
                            "Default checkout commit"
                            <input
                                type="text"
                                prop:value=move || draft_default_commit.get()
                                on:input=move |ev| set_draft_default_commit.set(event_target_value(&ev))
                                placeholder="e.g. main@{10 minutes ago}"
                            />
                        </label>
                        <label class="settings-switch-field">
                            <div class="settings-switch-row">
                                <span class="settings-field-label">"Debug logging"</span>
                                <input
                                    type="checkbox"
                                    class="settings-switch-input"
                                    prop:checked=move || draft_debug_enabled.get()
                                    on:change=move |ev| set_draft_debug_enabled.set(event_target_checked(&ev))
                                />
                            </div>
                        </label>
                        <div class="modal-actions">
                            <button on:click=close_settings>"Cancel"</button>
                            <button class="btn-primary" on:click=apply_settings>"Save"</button>
                        </div>
                    </div>
                </div>
            </Show>

            <pre class="console">
                {move || output_lines.get().join("\n")}
            </pre>
        </main>
    }
}
