use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;

use crate::bindings::{invoke, log, tauri_listen};
use crate::types::{EVENT_OUTPUT, EVENT_DONE, js_error_to_string};

/// Sets up Git event listeners for log output and operation completion.
/// Returns a future that sets up the listeners.
pub fn use_git_event_listeners(
    op_start_ms: RwSignal<f64>,
    set_last_raw_line: RwSignal<String>,
    set_output_lines: RwSignal<Vec<String>>,
    is_running: RwSignal<bool>,
    active_profile: RwSignal<crate::types::Profile>,
    refresh: impl Fn(String) + Copy + 'static,
) {
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
            let time_prefix = format!("[{:>4}.{:03}s]", int_part, frac_digits);
            let log_line = format!("{} [{}] {}", time_prefix, level, message);
            let status_line = format!("{} {}", time_prefix, message);
            set_last_raw_line.set(status_line);
            set_lines.update(|lines| {
                lines.push(log_line);
                // cap at 2000 entries to prevent unbounded memory growth
                if lines.len() > 2000 {
                    lines.drain(0..200);
                }
            });
        });
        let on_done = Closure::<dyn Fn(JsValue)>::new(move |_: JsValue| {
            is_running.set(false);
            refresh(active_profile.get_untracked().save_dir.clone());
        });
        if let (Ok(_), Ok(_)) = (
            tauri_listen(EVENT_OUTPUT, &on_output).await,
            tauri_listen(EVENT_DONE, &on_done).await,
        ) {
            on_output.forget();
            on_done.forget();
        }
    });
}

/// Returns window control closures.
pub fn use_window_controls() -> WindowControls {
    WindowControls {
        handle_minimize: Box::new(move |_: leptos::ev::MouseEvent| {
            spawn_local(async move {
                if let Err(err) = invoke("window_minimize", JsValue::NULL).await {
                    log(&format!("minimize failed: {}", js_error_to_string(err)));
                }
            });
        }),
        handle_toggle_maximize: Box::new(move |_: leptos::ev::MouseEvent| {
            spawn_local(async move {
                if let Err(err) = invoke("window_toggle_maximize", JsValue::NULL).await {
                    log(&format!("toggle maximize failed: {}", js_error_to_string(err)));
                }
            });
        }),
        handle_close: Box::new(move |_: leptos::ev::MouseEvent| {
            spawn_local(async move {
                if let Err(err) = invoke("window_close", JsValue::NULL).await {
                    log(&format!("close failed: {}", js_error_to_string(err)));
                }
            });
        }),
        handle_drag: Box::new(move |_: leptos::ev::MouseEvent| {
            spawn_local(async move {
                if let Err(err) = invoke("window_start_dragging", JsValue::NULL).await {
                    log(&format!("start dragging failed: {}", js_error_to_string(err)));
                }
            });
        }),
    }
}

pub struct WindowControls {
    pub handle_minimize: Box<dyn Fn(leptos::ev::MouseEvent)>,
    pub handle_toggle_maximize: Box<dyn Fn(leptos::ev::MouseEvent)>,
    pub handle_close: Box<dyn Fn(leptos::ev::MouseEvent)>,
    pub handle_drag: Box<dyn Fn(leptos::ev::MouseEvent)>,
}
