use dioxus::prelude::*;
use crate::state::AppState;

#[component]
pub fn StatusBar() -> Element {
    let app_state = use_context::<Signal<AppState>>();
    let connection_status = app_state.read().connection_status.clone();

    let display_text = connection_status.read()
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
                        let theme = app_state.read().theme.clone();
                        AppState::toggle_theme(theme);
                    },
                    "🌙"
                }
            }
        }
    }
}
