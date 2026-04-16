use leptos::prelude::*;

/// Reusable confirmation dialog component for operations like Pull/Push
#[component]
pub fn ConfirmDialog(
    /// Whether the dialog is currently open
    is_open: impl Fn() -> bool + Clone + Send + 'static,
    /// Title/message to display
    title: impl Fn() -> String + Clone + Send + 'static,
    /// The remote URL or target to show
    target: impl Fn() -> String + Clone + Send + 'static,
    /// Label for the confirm button
    confirm_label: impl Fn() -> String + Clone + Send + 'static,
    /// Callback when confirm is clicked
    on_confirm: impl Fn() + Copy + Send + Sync + 'static,
    /// Callback when cancel is clicked
    on_cancel: impl Fn() + Copy + Send + Sync + 'static,
    /// Whether the operation is currently running (disables confirm button)
    is_running: impl Fn() -> bool + Clone + Send + 'static,
) -> impl IntoView {
    // Clone the closures so they can be used multiple times
    let title_clone = title.clone();
    let target_clone = target.clone();
    let confirm_label_clone = confirm_label.clone();

    view! {
        <div class="sidebar" class:open=is_open>
            <div class="sidebar-panel-form">
                <div class="panel-body">
                    <div class="panel-label">{move || title_clone()}
                        <div class="commit-checkout-hash">{move || target_clone()}</div>
                    </div>
                    <div class="commit-modal-actions">
                        <button class="btn btn-cancel-modal" on:click=move |_| on_cancel()>
                            "Cancel"
                        </button>
                        <button class="btn btn-checkout-confirm"
                            disabled=is_running
                            on:click=move |_| on_confirm()>
                            {move || confirm_label_clone()}
                        </button>
                    </div>
                </div>
            </div>
        </div>
    }
}
