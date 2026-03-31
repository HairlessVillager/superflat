use leptos::task::spawn_local;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use serde::Serialize;
use std::cell::RefCell;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = listen)]
    async fn tauri_listen(event: &str, handler: &Closure<dyn Fn(JsValue)>) -> JsValue;

    #[wasm_bindgen(js_name = setInterval)]
    fn set_interval(closure: &Closure<dyn Fn()>, millis: u32) -> i32;
}

#[derive(Serialize)]
struct RunCommitArgs {
    save_dir: String,
    branch: String,
    message: String,
}

#[derive(Serialize)]
struct RunCheckoutArgs {
    save_dir: String,
    commit: String,
}

#[derive(Serialize)]
struct SaveSettingsArgs {
    branch: String,
}

thread_local! {
    static UNLISTEN_FNS: RefCell<Vec<js_sys::Function>> = RefCell::new(Vec::new());
}

fn cleanup_listeners() {
    UNLISTEN_FNS.with(|fns| {
        for f in fns.borrow().iter() {
            let _ = f.call0(&JsValue::NULL);
        }
        fns.borrow_mut().clear();
    });
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

#[component]
pub fn App() -> impl IntoView {
    let (save_dir, set_save_dir) = signal(String::new());
    let (branch, set_branch) = signal(String::from("main"));
    let (message, set_message) = signal(String::new());
    let (commit_id, set_commit_id) = signal(String::from("main@{10 minutes ago}"));
    let (clock, set_clock) = signal(current_datetime_string());
    let (output_lines, set_output_lines) = signal(Vec::<String>::new());
    let (is_running, set_is_running) = signal(false);
    let (show_settings, set_show_settings) = signal(false);
    // draft branch edited inside the settings dialog
    let (draft_branch, set_draft_branch) = signal(String::new());

    // Tick clock every second
    let tick = Closure::<dyn Fn()>::new(move || {
        set_clock.set(current_datetime_string());
    });
    set_interval(&tick, 1000);
    tick.forget();

    // Load persisted settings on mount
    spawn_local(async move {
        let result = invoke("get_settings", JsValue::NULL).await;
        if let Some(b) = result.as_string() {
            set_branch.set(b.clone());
            set_draft_branch.set(b);
        }
    });

    let pick_dir = move |_| {
        spawn_local(async move {
            let result = invoke("pick_directory", JsValue::NULL).await;
            if let Some(path) = result.as_string() {
                set_save_dir.set(path);
            }
        });
    };

    let open_settings = move |_| {
        set_draft_branch.set(branch.get_untracked());
        set_show_settings.set(true);
    };

    let close_settings = move |_| {
        set_show_settings.set(false);
    };

    let apply_settings = move |_| {
        let b = draft_branch.get_untracked();
        set_branch.set(b.clone());
        set_show_settings.set(false);
        spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&SaveSettingsArgs { branch: b }).unwrap();
            invoke("save_settings", args).await;
        });
    };

    let run_commit = move |_| {
        cleanup_listeners();
        set_output_lines.set(Vec::new());
        set_is_running.set(true);

        spawn_local(async move {
            let save_dir_val = save_dir.get_untracked();
            let branch_val = branch.get_untracked();
            let message_val = {
                let m = message.get_untracked();
                if m.is_empty() { current_datetime_string() } else { m }
            };

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

            let unlisten_output = tauri_listen("commit-output", &on_output).await;
            let unlisten_done = tauri_listen("commit-done", &on_done).await;

            on_output.forget();
            on_done.forget();

            if let (Some(u1), Some(u2)) = (
                unlisten_output.dyn_into::<js_sys::Function>().ok(),
                unlisten_done.dyn_into::<js_sys::Function>().ok(),
            ) {
                UNLISTEN_FNS.with(|fns| {
                    fns.borrow_mut().push(u1);
                    fns.borrow_mut().push(u2);
                });
            }

            let args = serde_wasm_bindgen::to_value(&RunCommitArgs {
                save_dir: save_dir_val,
                branch: branch_val,
                message: message_val,
            })
            .unwrap();
            invoke("run_commit", args).await;
        });
    };

    let run_checkout = move |_| {
        cleanup_listeners();
        set_output_lines.set(Vec::new());
        set_is_running.set(true);

        spawn_local(async move {
            let save_dir_val = save_dir.get_untracked();
            let commit_val = commit_id.get_untracked();

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

            let unlisten_output = tauri_listen("commit-output", &on_output).await;
            let unlisten_done = tauri_listen("commit-done", &on_done).await;

            on_output.forget();
            on_done.forget();

            if let (Some(u1), Some(u2)) = (
                unlisten_output.dyn_into::<js_sys::Function>().ok(),
                unlisten_done.dyn_into::<js_sys::Function>().ok(),
            ) {
                UNLISTEN_FNS.with(|fns| {
                    fns.borrow_mut().push(u1);
                    fns.borrow_mut().push(u2);
                });
            }

            let args = serde_wasm_bindgen::to_value(&RunCheckoutArgs {
                save_dir: save_dir_val,
                commit: commit_val,
            })
            .unwrap();
            invoke("run_checkout", args).await;
        });
    };

    view! {
        <main>
            <div class="toolbar">
                <div class="toolbar-row">
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
