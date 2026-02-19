use crate::state::AppState;
use dioxus::prelude::*;

#[component]
pub fn StatusBar() -> Element {
    let app_state = use_context::<Signal<AppState>>();
    let connection_status = app_state.read().connection_status;

    let display_text = connection_status
        .read()
        .clone()
        .unwrap_or_else(|| "No active connection".to_string());

    rsx! {
        div {
            class: "status-bar",

            div {
                class: "status-left",
                span {
                    class: "status-item",
                    "Ready"
                }
            }

            div {
                class: "status-center",
                span {
                    class: "status-item",
                    "{display_text}"
                }
            }

            div {
                class: "status-right",
                button {
                    class: "status-button",
                    onclick: move |_| {
                        let theme = app_state.read().theme;
                        AppState::toggle_theme(theme);
                    },
                    "🌙"
                }
            }
        }
    }
}
