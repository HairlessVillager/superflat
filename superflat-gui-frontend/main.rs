mod app;
mod bindings;
mod components;
mod handlers;
mod hooks;
mod state;
mod types;

use app::App;
use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| {
        view! {
            <App/>
        }
    })
}
