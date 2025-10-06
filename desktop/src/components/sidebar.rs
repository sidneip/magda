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
    
    // Load connections on mount and create default connection if none exists
    use_effect(move || {
        spawn(async move {
            let configs = app_state.read().connection_manager.get_configs().await;
            
            // If no connections exist, create a default one for localhost
            if configs.is_empty() {
                let default_config = crate::connection::ConnectionConfig::new("Local Cassandra", "localhost")
                    .with_keyspace("guruband".to_string()); // TODO: This should be configurable
                
                if let Ok(_id) = app_state.read().connection_manager.add_config(default_config).await {
                    tracing::info!("Created default connection to localhost:9042");
                    // Reload configs
                    let updated_configs = app_state.read().connection_manager.get_configs().await;
                    connections.set(updated_configs);
                } else {
                    connections.set(configs);
                }
            } else {
                connections.set(configs);
            }
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
            
            // Show connection dialog if needed (commented out for now)
            // if *show_connection_dialog.read() {
            //     div { "Connection dialog coming soon" }
            // }
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
                            tracing::info!("🔌 User clicked Connect button for: {}", conn_name);
                            spawn(async move {
                                tracing::debug!("📡 Attempting to connect to: {} (id: {})", conn_name, id);
                                match app_state.read()
                                    .connection_manager
                                    .connect(id)
                                    .await {
                                    Ok(_) => {
                                        tracing::info!("✅ Successfully connected to: {}", conn_name);
                                        is_connected.set(true);
                                    }
                                    Err(e) => {
                                        tracing::error!("❌ Failed to connect to {}: {}", conn_name, e);
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
    
    // Load real tables from Cassandra
    use_effect(move || {
        tracing::info!("🔄 TablesSection effect triggered for connection: {}", connection_name);
        loading.set(true);
        spawn(async move {
            tracing::debug!("📡 Attempting to get active connection...");
            
            // Get active connection from app state
            if let Some(connection) = app_state.read().connection_manager.get_active_connection().await {
                // Get the keyspace from connection config
                if let Some(keyspace) = &connection.config.keyspace {
                    tracing::info!("✅ Got active connection, listing tables from '{}' keyspace", keyspace);
                    
                    // List tables from the configured keyspace
                    match connection.list_tables(keyspace).await {
                        Ok(real_tables) => {
                            tracing::info!("✅ Successfully loaded {} tables from '{}' keyspace", real_tables.len(), keyspace);
                            for table in &real_tables {
                                tracing::debug!("  - Table: {}", table);
                            }
                            tables.set(real_tables);
                        }
                        Err(e) => {
                            tracing::error!("❌ Failed to load tables from keyspace '{}': {}", keyspace, e);
                            tables.set(Vec::new());
                        }
                    }
                } else {
                    tracing::warn!("⚠️ No keyspace configured for connection - cannot list tables");
                    tables.set(Vec::new());
                }
            } else {
                tracing::warn!("⚠️ No active connection found - cannot list tables");
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