use dioxus::prelude::*;
use uuid::Uuid;

use crate::config::save_saved_queries;
use crate::connection::ConnectionConfig;
use crate::state::{ActiveTab, AppState, ConsoleCategory, StatusLevel};

#[component]
pub fn Sidebar() -> Element {
    let app_state = use_context::<Signal<AppState>>();
    let mut connections = use_signal(Vec::<ConnectionConfig>::new);
    let mut selected_connection = use_signal(|| None::<Uuid>);
    let mut show_connection_dialog = use_signal(|| false);
    let mut editing_connection = use_signal(|| None::<ConnectionConfig>);

    // Load connections on mount
    use_effect(move || {
        spawn(async move {
            let cm = app_state.read().connection_manager.clone();
            let configs = cm.get_configs().await;
            connections.set(configs);
        });
    });

    rsx! {
        div {
            class: "sidebar",

            // Header with title and add button
            div {
                class: "sidebar-header",

                h3 {
                    class: "sidebar-title",
                    "Connections"
                }

                button {
                    class: "btn-icon",
                    onclick: move |_| {
                        editing_connection.set(None);
                        show_connection_dialog.set(true);
                    },
                    title: "Add Connection",
                    "+"
                }
            }

            // Connection list
            div {
                class: "connection-list",

                for connection in connections.read().iter() {
                    ConnectionItem {
                        connection: connection.clone(),
                        is_selected: selected_connection.read().as_ref() == Some(&connection.id),
                        on_select: move |id| {
                            selected_connection.set(Some(id));
                            spawn(async move {
                                let cm = app_state.read().connection_manager.clone();
                                let _ = cm.set_active_connection(id).await;
                            });
                        },
                        on_edit: move |config: ConnectionConfig| {
                            editing_connection.set(Some(config));
                            show_connection_dialog.set(true);
                        },
                        on_delete: move |id: Uuid| {
                            spawn(async move {
                                let cm = app_state.read().connection_manager.clone();
                                let console_log = app_state.read().console_log;
                                let status_message = app_state.read().status_message;
                                match cm.remove_config(id).await {
                                    Ok(_) => {
                                        let updated = cm.get_configs().await;
                                        connections.set(updated);
                                        AppState::console_push(console_log, status_message, StatusLevel::Info, ConsoleCategory::Connection, "Connection removed");
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to remove connection: {}", e);
                                        AppState::console_push(console_log, status_message, StatusLevel::Error, ConsoleCategory::Connection, format!("Failed to remove: {}", e));
                                    }
                                }
                            });
                        }
                    }
                }
            }

            // Tables list (shown when connected)
            if let Some(selected_id) = *selected_connection.read() {
                if let Some(selected_conn) = connections.read().iter().find(|c| c.id == selected_id) {
                    TablesSection {
                        connection_name: selected_conn.name.clone()
                    }
                }
            }

            // Saved queries section
            SavedQueriesSection {}

            // Connection dialog (new or edit)
            if *show_connection_dialog.read() {
                super::connection_dialog::ConnectionDialog {
                    on_close: move |_| {
                        show_connection_dialog.set(false);
                        editing_connection.set(None);
                    },
                    on_save: move |config: ConnectionConfig| {
                        let is_edit = editing_connection.read().is_some();
                        show_connection_dialog.set(false);
                        editing_connection.set(None);
                        spawn(async move {
                            let cm = app_state.read().connection_manager.clone();
                            let console_log = app_state.read().console_log;
                            let status_message = app_state.read().status_message;
                            let result = if is_edit {
                                cm.update_config(config).await.map(|_| ())
                            } else {
                                cm.add_config(config).await.map(|_| ())
                            };
                            match result {
                                Ok(_) => {
                                    let msg = if is_edit { "Connection updated" } else { "Connection saved" };
                                    tracing::info!("{}", msg);
                                    let updated = cm.get_configs().await;
                                    connections.set(updated);
                                    AppState::console_push(console_log, status_message, StatusLevel::Success, ConsoleCategory::Connection, msg);
                                }
                                Err(e) => {
                                    tracing::error!("Failed to save connection: {}", e);
                                    AppState::console_push(console_log, status_message, StatusLevel::Error, ConsoleCategory::Connection, format!("Failed to save: {}", e));
                                }
                            }
                        });
                    },
                    existing: editing_connection.read().clone()
                }
            }
        }
    }
}

#[component]
fn ConnectionItem(
    connection: ConnectionConfig,
    is_selected: bool,
    on_select: EventHandler<Uuid>,
    on_edit: EventHandler<ConnectionConfig>,
    on_delete: EventHandler<Uuid>,
) -> Element {
    let mut app_state = use_context::<Signal<AppState>>();
    let mut is_connected = use_signal(|| false);
    let mut is_connecting = use_signal(|| false);
    let mut confirm_delete = use_signal(|| false);

    // Check connection status
    use_effect(move || {
        let id = connection.id;
        spawn(async move {
            let cm = app_state.read().connection_manager.clone();
            let connected = cm.is_connected(id).await;
            is_connected.set(connected);
        });
    });

    let status_class = if *is_connected.read() {
        "status-connected"
    } else if *is_connecting.read() {
        "status-connecting"
    } else {
        "status-disconnected"
    };

    rsx! {
        div {
            class: format!("connection-item {}", if is_selected { "selected" } else { "" }),
            onclick: move |_| on_select.call(connection.id),

            div {
                class: format!("connection-status {}", status_class)
            }

            div {
                class: "connection-info",

                div {
                    class: "connection-name",
                    "{connection.name}"
                }

                div {
                    class: "connection-host",
                    "{connection.host}:{connection.port}"
                }
            }

            div {
                class: "connection-actions",

                if !*is_connected.read() && !*is_connecting.read() {
                    button {
                        class: "btn-small",
                        onclick: {
                            let connection = connection.clone();
                            move |e| {
                                e.stop_propagation();
                                is_connecting.set(true);
                                let id = connection.id;
                                let conn_name = connection.name.clone();
                                tracing::info!("Connecting to: {}", conn_name);
                                let console_log = app_state.read().console_log;
                                let status_msg = app_state.read().status_message;
                                AppState::console_push(console_log, status_msg, StatusLevel::Info, ConsoleCategory::Connection, format!("Connecting to {}...", conn_name));
                                spawn(async move {
                                    tracing::debug!("Attempting to connect to: {} (id: {})", conn_name, id);
                                    let cm = app_state.read().connection_manager.clone();
                                    let console_log = app_state.read().console_log;
                                    let status_msg = app_state.read().status_message;
                                    match cm.connect(id).await {
                                        Ok(_) => {
                                            tracing::info!("Connected to: {}", conn_name);
                                            is_connected.set(true);
                                            let config = cm.get_config(id).await;
                                            if let Some(cfg) = config {
                                                let ks = cfg.keyspace.map(|k| format!(" / {}", k)).unwrap_or_default();
                                                app_state.write().connection_status.set(
                                                    Some(format!("Connected: {}:{}{}", cfg.host, cfg.port, ks))
                                                );
                                            }
                                            AppState::console_push(console_log, status_msg, StatusLevel::Success, ConsoleCategory::Connection, format!("Connected to {}", conn_name));
                                        }
                                        Err(e) => {
                                            let err_msg = format!("{}", e);
                                            tracing::error!("Failed to connect to {}: {}", conn_name, err_msg);
                                            AppState::console_push(console_log, status_msg, StatusLevel::Error, ConsoleCategory::Connection, format!("Connection failed: {}", err_msg));
                                        }
                                    }
                                    is_connecting.set(false);
                                });
                            }
                        },
                        "Connect"
                    }
                }

                if *is_connecting.read() {
                    span {
                        class: "connecting-indicator",
                        "..."
                    }
                }

                if *is_connected.read() {
                    button {
                        class: "btn-small btn-danger",
                        onclick: move |e| {
                            e.stop_propagation();
                            let id = connection.id;
                            spawn(async move {
                                let cm = app_state.read().connection_manager.clone();
                                let _ = cm.disconnect(id).await;
                                is_connected.set(false);
                                app_state.write().connection_status.set(None);
                                let console_log = app_state.read().console_log;
                                let status_msg = app_state.read().status_message;
                                AppState::console_push(console_log, status_msg, StatusLevel::Info, ConsoleCategory::Connection, "Disconnected");
                            });
                        },
                        "Disconnect"
                    }
                }

                // Edit/Delete buttons (only when not connected, visible on hover)
                if !*is_connected.read() && !*is_connecting.read() {
                    if *confirm_delete.read() {
                        span {
                            class: "confirm-delete",
                            onclick: move |e| e.stop_propagation(),

                            span { class: "confirm-label", "Delete?" }
                            button {
                                class: "btn-small btn-danger",
                                onclick: {
                                    let id = connection.id;
                                    move |e| {
                                        e.stop_propagation();
                                        confirm_delete.set(false);
                                        on_delete.call(id);
                                    }
                                },
                                "Yes"
                            }
                            button {
                                class: "btn-small",
                                onclick: move |e| {
                                    e.stop_propagation();
                                    confirm_delete.set(false);
                                },
                                "No"
                            }
                        }
                    } else {
                        div {
                            class: "connection-secondary-actions",

                            button {
                                class: "btn-icon-small",
                                title: "Edit connection",
                                onclick: {
                                    let connection = connection.clone();
                                    move |e| {
                                        e.stop_propagation();
                                        on_edit.call(connection.clone());
                                    }
                                },
                                "✎"
                            }
                            button {
                                class: "btn-icon-small btn-danger-text",
                                title: "Delete connection",
                                onclick: move |e| {
                                    e.stop_propagation();
                                    confirm_delete.set(true);
                                },
                                "✕"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TablesSection(connection_name: String) -> Element {
    let mut app_state = use_context::<Signal<AppState>>();
    let mut tables = use_signal(Vec::<String>::new);
    let mut selected_table = use_signal(|| None::<String>);
    let mut loading = use_signal(|| false);

    use_effect(move || {
        loading.set(true);
        spawn(async move {
            let cm = app_state.read().connection_manager.clone();
            if let Some(conn) = cm.get_active_connection().await {
                if let Some(keyspace) = conn.resolve_keyspace().await {
                    match conn.list_tables(&keyspace).await {
                        Ok(t) => tables.set(t),
                        Err(e) => {
                            tracing::error!(
                                "Failed to list tables for keyspace '{}': {}",
                                keyspace,
                                e
                            );
                            tables.set(Vec::new());
                        }
                    }
                } else {
                    tables.set(Vec::new());
                }
            } else {
                tables.set(Vec::new());
            }
            loading.set(false);
        });
    });

    rsx! {
        div {
            class: "tables-section",

            // Tables header
            div {
                class: "tables-header",
                h4 {
                    class: "tables-title",
                    "Tables"
                }

                if *loading.read() {
                    span {
                        class: "loading-indicator",
                        "⟳"
                    }
                }
            }

            // Tables list
            div {
                class: "tables-list",

                if tables.read().is_empty() && !*loading.read() {
                    div {
                        class: "empty-tables",
                        "No tables found"
                    }
                } else {
                    for (idx, table) in tables.read().iter().enumerate() {
                        div {
                            key: "{idx}",
                            class: if selected_table.read().as_ref() == Some(table) {
                                "table-item selected"
                            } else {
                                "table-item"
                            },
                            onclick: {
                                let table = table.clone();
                                move |_| {
                                    selected_table.set(Some(table.clone()));
                                    // Also update global state
                                    app_state.write().selected_table.set(Some(table.clone()));
                                    // Switch to Data tab to show the table data
                                    app_state.write().active_tab.set(crate::state::ActiveTab::Data);
                                }
                            },

                            span {
                                class: "table-icon",
                                "📋"
                            }

                            span {
                                class: "table-name",
                                "{table}"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SavedQueriesSection() -> Element {
    let mut app_state = use_context::<Signal<AppState>>();
    let mut saved_queries = app_state.read().saved_queries;

    let mut delete_query = move |id: Uuid| {
        saved_queries.write().retain(|q| q.id != id);
        save_saved_queries(&saved_queries.read());
    };

    rsx! {
        div {
            class: "tables-section",

            div {
                class: "tables-header",
                h4 {
                    class: "tables-title",
                    "Saved Queries"
                }
            }

            div {
                class: "tables-list",

                if saved_queries.read().is_empty() {
                    div {
                        class: "empty-tables",
                        "No saved queries"
                    }
                } else {
                    for query in saved_queries.read().iter() {
                        {
                            let query_text = query.query.clone();
                            let query_id = query.id;
                            rsx! {
                                div {
                                    key: "{query.id}",
                                    class: "saved-query-item",
                                    onclick: move |_| {
                                        app_state.write().pending_query.set(Some(query_text.clone()));
                                        app_state.read().active_tab.clone().set(ActiveTab::Query);
                                    },

                                    span {
                                        class: "saved-query-name",
                                        "{query.name}"
                                    }

                                    button {
                                        class: "btn-small btn-danger",
                                        onclick: move |e| {
                                            e.stop_propagation();
                                            delete_query(query_id);
                                        },
                                        "x"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
