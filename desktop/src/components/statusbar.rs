use crate::state::{AppState, StatusLevel};
use dioxus::prelude::*;

#[component]
pub fn StatusBar() -> Element {
    let mut app_state = use_context::<Signal<AppState>>();
    let connection_status = app_state.read().connection_status;
    let status_message = app_state.read().status_message;
    let console_visible = *app_state.read().console_visible.read();
    let entry_count = app_state.read().console_log.read().len();

    let display_text = connection_status
        .read()
        .clone()
        .unwrap_or_else(|| "No active connection".to_string());

    let message_class = status_message.read().as_ref().map(|m| match m.level {
        StatusLevel::Info => "status-msg status-msg-info",
        StatusLevel::Success => "status-msg status-msg-success",
        StatusLevel::Error => "status-msg status-msg-error",
    });

    let message_text = status_message.read().as_ref().map(|m| m.text.clone());

    rsx! {
        div {
            class: "status-bar",
            onclick: move |_| {
                let current = *app_state.read().console_visible.read();
                app_state.write().console_visible.set(!current);
            },
            style: "cursor: pointer;",

            div {
                class: "status-left",
                span {
                    class: if console_visible { "status-item status-console-active" } else { "status-item" },
                    "Console ({entry_count})"
                }
            }

            div {
                class: "status-center",
                // Show status message if present, otherwise connection status
                if let Some(ref msg) = message_text {
                    span {
                        class: message_class.unwrap_or("status-msg"),
                        "{msg}"
                    }
                } else {
                    span {
                        class: "status-item",
                        "{display_text}"
                    }
                }
            }

            div {
                class: "status-right",
                button {
                    class: "status-button",
                    onclick: move |e| {
                        e.stop_propagation();
                        let theme = app_state.read().theme;
                        AppState::toggle_theme(theme);
                    },
                    "🌙"
                }
            }
        }
    }
}
