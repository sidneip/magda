use dioxus::prelude::*;
use crate::state::AppState;

#[component]
pub fn StatusBar() -> Element {
    let app_state = use_context::<Signal<AppState>>();
    
    rsx! {
        div {
            class: "status-bar",
            
            div {
                class: "status-left",
                
                span {
                    class: "status-item",
                    "🟢 Ready"
                }
            }
            
            div {
                class: "status-center",
                
                span {
                    class: "status-item",
                    "No active connection"
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