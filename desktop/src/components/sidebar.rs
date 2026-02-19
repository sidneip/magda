use dioxus::prelude::*;
use uuid::Uuid;

use crate::connection::ConnectionConfig;
use crate::state::AppState;

#[component]
pub fn Sidebar() -> Element {
    let app_state = use_context::<Signal<AppState>>();
    let mut connections = use_signal(Vec::<ConnectionConfig>::new);
    let mut selected_connection = use_signal(|| None::<Uuid>);
    let mut show_connection_dialog = use_signal(|| false);
    
    // Load connections on mount
    use_effect(move || {
        spawn(async move {
            let configs = app_state.read().connection_manager.get_configs().await;
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
                    onclick: move |_| show_connection_dialog.set(true),
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
                                let _ = app_state.read()
                                    .connection_manager
                                    .set_active_connection(id)
                                    .await;
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

            // Connection dialog
            if *show_connection_dialog.read() {
                super::connection_dialog::ConnectionDialog {
                    on_close: move |_| show_connection_dialog.set(false),
                    on_save: move |config: ConnectionConfig| {
                        show_connection_dialog.set(false);
                        spawn(async move {
                            match app_state.read().connection_manager.add_config(config).await {
                                Ok(_) => {
                                    tracing::info!("Connection saved successfully");
                                    let updated = app_state.read().connection_manager.get_configs().await;
                                    connections.set(updated);
                                }
                                Err(e) => {
                                    tracing::error!("Failed to save connection: {}", e);
                                }
                            }
                        });
                    }
                }
            }
        }
    }
}

#[component]
fn ConnectionItem(
    connection: ConnectionConfig,
    is_selected: bool,
    on_select: EventHandler<Uuid>
) -> Element {
    let app_state = use_context::<Signal<AppState>>();
    let mut is_connected = use_signal(|| false);
    let mut is_connecting = use_signal(|| false);
    
    // Check connection status
    use_effect(move || {
        let id = connection.id;
        spawn(async move {
            let connected = app_state.read()
                .connection_manager
                .is_connected(id)
                .await;
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
                        onclick: move |e| {
                            e.stop_propagation();
                            is_connecting.set(true);
                            let id = connection.id;
                            let conn_name = connection.name.clone();
                            tracing::info!("Connecting to: {}", conn_name);
                            spawn(async move {
                                tracing::debug!("Attempting to connect to: {} (id: {})", conn_name, id);
                                match app_state.read()
                                    .connection_manager
                                    .connect(id)
                                    .await {
                                    Ok(_) => {
                                        tracing::info!("Connected to: {}", conn_name);
                                        is_connected.set(true);
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to connect to {}: {}", conn_name, e);
                                    }
                                }
                                is_connecting.set(false);
                            });
                        },
                        "Connect"
                    }
                }
                
                if *is_connected.read() {
                    button {
                        class: "btn-small btn-danger",
                        onclick: move |e| {
                            e.stop_propagation();
                            let id = connection.id;
                            spawn(async move {
                                let _ = app_state.read()
                                    .connection_manager
                                    .disconnect(id)
                                    .await;
                                is_connected.set(false);
                            });
                        },
                        "Disconnect"
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
            if let Some(conn) = app_state.read().connection_manager.get_active_connection().await {
                if let Some(keyspace) = conn.resolve_keyspace().await {
                    match conn.list_tables(&keyspace).await {
                        Ok(t) => tables.set(t),
                        Err(e) => {
                            tracing::error!("Failed to list tables for keyspace '{}': {}", keyspace, e);
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