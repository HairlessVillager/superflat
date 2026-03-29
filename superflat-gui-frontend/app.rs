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
}

#[derive(Serialize)]
struct RunLsArgs {
    path: String,
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

#[component]
pub fn App() -> impl IntoView {
    let (dir_path, set_dir_path) = signal(String::from("/"));
    let (output_lines, set_output_lines) = signal(Vec::<String>::new());
    let (is_running, set_is_running) = signal(false);

    let pick_dir = move |_| {
        spawn_local(async move {
            let result = invoke("pick_directory", JsValue::NULL).await;
            if let Some(path) = result.as_string() {
                set_dir_path.set(path);
            }
        });
    };

    let run_ls = move |_| {
        cleanup_listeners();
        set_output_lines.set(Vec::new());
        set_is_running.set(true);

        spawn_local(async move {
            let path = dir_path.get_untracked();

            // listen for output lines
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

            let unlisten_output = tauri_listen("ls-output", &on_output).await;
            let unlisten_done = tauri_listen("ls-done", &on_done).await;

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

            let args = serde_wasm_bindgen::to_value(&RunLsArgs { path }).unwrap();
            invoke("run_ls", args).await;
        });
    };

    view! {
        <main>
            <div class="toolbar">
                <input
                    type="text"
                    prop:value=move || dir_path.get()
                    on:input=move |ev| set_dir_path.set(event_target_value(&ev))
                    placeholder="/path/to/directory"
                />
                <button class="btn-pick" on:click=pick_dir disabled=move || is_running.get()>
                    "Browse"
                </button>
                <button on:click=run_ls disabled=move || is_running.get()>
                    {move || if is_running.get() { "Running..." } else { "Run" }}
                </button>
            </div>
            <pre class="console">
                {move || output_lines.get().join("\n")}
            </pre>
        </main>
    }
}
