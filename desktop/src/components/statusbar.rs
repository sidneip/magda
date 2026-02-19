use dioxus::prelude::*;
use crate::state::AppState;

#[component]
pub fn StatusBar() -> Element {
    let app_state = use_context::<Signal<AppState>>();
    let mut connection_info = use_signal(|| "No active connection".to_string());

    // Poll connection status
    use_effect(move || {
        spawn(async move {
            if let Some(conn) = app_state.read().connection_manager.get_active_connection().await {
                let keyspace_info = match &conn.config.keyspace {
                    Some(ks) => format!(" / {}", ks),
                    None => String::new(),
                };
                connection_info.set(format!(
                    "Connected: {}:{}{}", conn.config.host, conn.config.port, keyspace_info
                ));
            } else {
                connection_info.set("No active connection".to_string());
            }
        });
    });

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
                    "{connection_info.read()}"
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
