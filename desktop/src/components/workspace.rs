use dioxus::prelude::*;

use crate::state::{ActiveTab, AppState};
use super::query_editor::QueryEditor;
use super::data_grid::DataGrid;
// use super::schema_viewer::SchemaViewer;

#[component]
pub fn Workspace() -> Element {
    let app_state = use_context::<Signal<AppState>>();
    let mut active_tab = app_state.read().active_tab.clone();
    
    rsx! {
        div {
            class: "workspace",
            
            // Tab bar
            div {
                class: "tab-bar",
                
                TabButton {
                    label: "Query",
                    is_active: *active_tab.read() == ActiveTab::Query,
                    onclick: move |_| active_tab.set(ActiveTab::Query)
                }
                
                TabButton {
                    label: "Data",
                    is_active: *active_tab.read() == ActiveTab::Data,
                    onclick: move |_| active_tab.set(ActiveTab::Data)
                }
                
                TabButton {
                    label: "Schema",
                    is_active: *active_tab.read() == ActiveTab::Schema,
                    onclick: move |_| active_tab.set(ActiveTab::Schema)
                }
                
                TabButton {
                    label: "History",
                    is_active: *active_tab.read() == ActiveTab::History,
                    onclick: move |_| active_tab.set(ActiveTab::History)
                }
            }
            
            // Tab content
            div {
                class: "tab-content",
                
                match *active_tab.read() {
                    ActiveTab::Query => rsx! {
                        QueryWorkspace {}
                    },
                    ActiveTab::Data => rsx! {
                        DataWorkspace {}
                    },
                    ActiveTab::Schema => rsx! {
                        div { "Schema view coming soon" }
                    },
                    ActiveTab::History => rsx! {
                        HistoryWorkspace {}
                    }
                }
            }
        }
    }
}

#[component]
fn TabButton(
    label: &'static str,
    is_active: bool,
    onclick: EventHandler<MouseEvent>
) -> Element {
    rsx! {
        button {
            class: format!("tab-button {}", if is_active { "active" } else { "" }),
            onclick: move |e| onclick.call(e),
            "{label}"
        }
    }
}

#[component]
fn QueryWorkspace() -> Element {
    let app_state = use_context::<Signal<AppState>>();
    let query_result = use_signal(|| None::<crate::components::data_grid::QueryResult>);
    let query_error = use_signal(|| None::<String>);
    let is_executing = use_signal(|| false);
    let mut current_page = use_signal(|| 1u32);
    let mut original_query = use_signal(|| String::new());
    let mut paging_state = use_signal(|| None::<String>);
    let mut page_states = use_signal(|| Vec::<Option<String>>::new());
    
    // Function to execute query with real Cassandra pagination using tokens
    let execute_paginated_query = {
        let app_state_clone = app_state.clone();
        let query_result_clone = query_result.clone();
        let query_error_clone = query_error.clone();
        let is_executing_clone = is_executing.clone();
        let paging_state_clone = paging_state.clone();
        let page_states_clone = page_states.clone();
        
        move |base_query: String, page: u32, direction: &str| {
            let mut app_state = app_state_clone.clone();
            let mut query_result = query_result_clone.clone();
            let mut query_error = query_error_clone.clone();
            let mut is_executing = is_executing_clone.clone();
            let _paging_state = paging_state_clone.clone();
            let _page_states = page_states_clone.clone();
            let direction = direction.to_string();
            
            is_executing.set(true);
            query_error.set(None);
            query_result.set(None);
            
            spawn(async move {
                let connection = app_state.read().connection_manager.get_active_connection().await;
                
                if let Some(connection) = connection {
                    let page_size = 100u32;
                    
                    // Build query with proper LIMIT
                    let limited_query = if base_query.trim().to_lowercase().starts_with("select") && 
                                         !base_query.to_lowercase().contains("limit") {
                        format!("{} LIMIT {}", base_query.trim_end_matches(';'), page_size)
                    } else {
                        base_query.clone()
                    };
                    
                    // For now, use simple pagination until we implement token-based pagination
                    // This is a temporary approach - real Cassandra pagination would use paging_state tokens
                    let skip_count = if direction == "next" || direction == "first" {
                        (page - 1) * page_size
                    } else {
                        ((page - 1).max(0)) * page_size
                    };
                    
                    let final_query = if skip_count > 0 && base_query.trim().to_lowercase().starts_with("select") {
                        // Note: This is still a simplified approach
                        // Real Cassandra would use prepared statements with paging_state
                        format!("{} ALLOW FILTERING", limited_query)
                    } else {
                        limited_query.clone()
                    };
                    
                    tracing::info!("🔍 Executing paginated query (page {}, {}): {}", page, direction, final_query);
                    match connection.execute_query(&final_query).await {
                        Ok(mut result) => {
                            // Store execution time before modifying result
                            let execution_time = result.execution_time_ms;
                            
                            // For this version, still simulate pagination client-side
                            // TODO: Implement real Cassandra token-based pagination
                            if skip_count > 0 && skip_count < result.rows.len() as u32 {
                                result.rows = result.rows.into_iter().skip(skip_count as usize).take(page_size as usize).collect();
                                result.row_count = result.rows.len();
                            } else if result.rows.len() > page_size as usize {
                                result.rows = result.rows.into_iter().take(page_size as usize).collect();
                                result.row_count = result.rows.len();
                            }
                            
                            tracing::info!("✅ Paginated query executed: {} rows (page {}, {})", result.row_count, page, direction);
                            query_result.set(Some(result));
                            
                            // Add to history  
                            let history_item = crate::state::QueryHistoryItem {
                                id: uuid::Uuid::new_v4(),
                                query: format!("{} (page {}, {})", final_query, page, direction),
                                success: true,
                                execution_time_ms: execution_time,
                                executed_at: chrono::Utc::now(),
                            };
                            app_state.write().query_history.write().push(history_item);
                        }
                        Err(e) => {
                            let error_msg = format!("Paginated query failed: {}", e);
                            tracing::error!("❌ {}", error_msg);
                            query_error.set(Some(error_msg));
                            
                            // Add failed query to history
                            let history_item = crate::state::QueryHistoryItem {
                                id: uuid::Uuid::new_v4(),
                                query: format!("{} (page {}, {})", final_query, page, direction),
                                success: false,
                                execution_time_ms: 0,
                                executed_at: chrono::Utc::now(),
                            };
                            app_state.write().query_history.write().push(history_item);
                        }
                    }
                } else {
                    let error_msg = "No active connection available";
                    tracing::warn!("⚠️ {}", error_msg);
                    query_error.set(Some(error_msg.to_string()));
                }
                is_executing.set(false);
            });
        }
    };
    
    // Debug: Log when query_result changes
    use_effect(move || {
        if let Some(result) = query_result.read().as_ref() {
            tracing::debug!("🎨 QueryWorkspace: query_result updated with {} rows and {} columns", 
                          result.row_count, result.columns.len());
        } else {
            tracing::debug!("🎨 QueryWorkspace: query_result is None");
        }
    });
    
    rsx! {
        div {
            class: "query-workspace",
            
            // Query editor at the top
            div {
                class: "query-editor-container",
                QueryEditor {
                    is_executing: is_executing,
                    on_execute: move |query: String| {
                        // Store original query and reset to page 1
                        original_query.set(query.clone());
                        current_page.set(1);
                        paging_state.set(None);
                        page_states.set(vec![None]); // Start with first page state
                        
                        // Execute first page
                        execute_paginated_query(query, 1, "first");
                    }
                }
            }
            
            // Results grid at the bottom
            div {
                class: "query-results-container",
                
                // Show error if there is one
                if let Some(error) = query_error.read().as_ref() {
                    div {
                        class: "query-error",
                        div {
                            class: "error-header",
                            "❌ Error"
                        }
                        div {
                            class: "error-message",
                            "{error}"
                        }
                    }
                }
                // Show results if successful  
                else if let Some(result) = query_result.read().as_ref() {
                    div {
                        class: "query-results",
                        
                        div {
                            class: "results-header",
                            div {
                                class: "results-info",
                                span { "✅ Results: {result.row_count} rows in {result.execution_time_ms}ms" }
                                if original_query.read().trim().to_lowercase().starts_with("select") {
                                    span { 
                                        class: "pagination-hint",
                                        " (page {current_page.read()}, client-side pagination - consider LIMIT for better performance)"
                                    }
                                }
                            }
                            
                            // Pagination controls for SELECT queries
                            if original_query.read().trim().to_lowercase().starts_with("select") && !original_query.read().trim().is_empty() {
                                div {
                                    class: "pagination-controls",
                                    
                                    button {
                                        class: "btn btn-secondary",
                                        disabled: *current_page.read() <= 1,
                                        onclick: {
                                            let execute_fn = execute_paginated_query.clone();
                                            let query = original_query.read().clone();
                                            move |_| {
                                                let new_page = *current_page.read() - 1;
                                                if new_page >= 1 {
                                                    current_page.set(new_page);
                                                    execute_fn(query.clone(), new_page, "previous");
                                                }
                                            }
                                        },
                                        "◀ Previous"
                                    }
                                    
                                    span {
                                        class: "page-info",
                                        "Page {current_page.read()}"
                                    }
                                    
                                    button {
                                        class: "btn btn-secondary",
                                        onclick: {
                                            let execute_fn = execute_paginated_query.clone();
                                            let query = original_query.read().clone();
                                            move |_| {
                                                let new_page = *current_page.read() + 1;
                                                current_page.set(new_page);
                                                execute_fn(query.clone(), new_page, "next");
                                            }
                                        },
                                        "Next ▶"
                                    }
                                }
                            }
                        }
                        
                        // Results table
                        if result.row_count > 0 {
                            div {
                                class: "results-table-container",
                                table {
                                    class: "results-table",
                                    
                                    thead {
                                        tr {
                                            for column in result.columns.iter() {
                                                th { 
                                                    div {
                                                        class: "column-header",
                                                        span { class: "column-name", "{column.name}" }
                                                        span { class: "column-type", "{column.data_type}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    
                                    tbody {
                                        for row in result.rows.iter() {
                                            tr {
                                                for cell in row.iter() {
                                                    td {
                                                        match cell {
                                                            serde_json::Value::String(s) => rsx! { "{s}" },
                                                            serde_json::Value::Null => rsx! { span { class: "null-value", "NULL" } },
                                                            serde_json::Value::Number(n) => rsx! { span { class: "number-value", "{n}" } },
                                                            serde_json::Value::Bool(b) => rsx! { span { class: "bool-value", "{b}" } },
                                                            other => rsx! { "{other}" },
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            div {
                                class: "empty-results",
                                "Query executed successfully but returned no rows"
                            }
                        }
                    }
                }
                // Show executing indicator
                else if *is_executing.read() {
                    div {
                        class: "executing-indicator",
                        "⏳ Executing query..."
                    }
                }
                // Show initial state
                else {
                    div {
                        class: "no-results",
                        "Execute a query to see results here"
                    }
                }
            }
        }
    }
}

#[component]
fn DataWorkspace() -> Element {
    let mut app_state = use_context::<Signal<AppState>>();
    let selected_table = app_state.read().selected_table.clone();
    let mut tables = use_signal(|| Vec::<String>::new());
    
    // Load real tables from active connection
    use_effect(move || {
        spawn(async move {
            if let Some(connection) = app_state.read().connection_manager.get_active_connection().await {
                // Use the keyspace from connection config, or find a suitable one
                let keyspace_to_use = if let Some(keyspace) = &connection.config.keyspace {
                    Some(keyspace.clone())
                } else {
                    // No keyspace configured, let's find available keyspaces and use the first non-system one
                    match connection.list_keyspaces().await {
                        Ok(keyspaces) => {
                            tracing::info!("🔍 Found {} keyspaces, looking for non-system keyspace", keyspaces.len());
                            
                            // Look for "guruband" first (user's keyspace), then any non-system keyspace
                            if keyspaces.contains(&"guruband".to_string()) {
                                Some("guruband".to_string())
                            } else {
                                keyspaces.iter()
                                    .find(|ks| !ks.starts_with("system") && !ks.is_empty())
                                    .cloned()
                            }
                        }
                        Err(e) => {
                            tracing::error!("❌ Failed to list keyspaces for data workspace: {}", e);
                            None
                        }
                    }
                };
                
                if let Some(keyspace) = keyspace_to_use {
                    match connection.list_tables(&keyspace).await {
                        Ok(real_tables) => {
                            tracing::info!("✅ Loaded {} tables from '{}' keyspace for data workspace", real_tables.len(), keyspace);
                            tables.set(real_tables);
                        }
                        Err(e) => {
                            tracing::error!("❌ Failed to load tables from keyspace '{}' for data workspace: {}", keyspace, e);
                        }
                    }
                } else {
                    tracing::warn!("⚠️ No keyspace configured for connection - cannot load tables for data workspace");
                    tables.set(Vec::new());
                }
            } else {
                tracing::debug!("No active connection for data workspace");
            }
        });
    });
    
    rsx! {
        div {
            class: "data-workspace",
            
            // Table selector and actions
            div {
                class: "data-toolbar",
                
                div {
                    class: "table-selector",
                    
                    select {
                        class: "select-table",
                        value: "{selected_table.read().as_ref().unwrap_or(&String::new())}",
                        onchange: move |e| {
                            let value = if e.value().is_empty() { None } else { Some(e.value()) };
                            app_state.write().selected_table.set(value);
                        },
                        
                        option { value: "", "Select a table..." }
                        
                        for table in tables.read().iter() {
                            option { 
                                value: "{table}",
                                "{table}" 
                            }
                        }
                    }
                    
                    if selected_table.read().is_some() {
                        span {
                            class: "auto-load-info",
                            "✅ Auto-loading table data..."
                        }
                    }
                }
                
                div {
                    class: "data-actions",
                    
                    if let Some(table_name) = selected_table.read().as_ref() {
                        span {
                            class: "current-table",
                            "Table: {table_name}"
                        }
                    }
                }
            }
            
            // Data grid
            DataGrid {
                table_name: selected_table.read().clone()
            }
        }
    }
}

#[component]
fn SchemaWorkspace() -> Element {
    rsx! {
        div {
            class: "schema-workspace",
            div { "Schema viewer coming soon" }
        }
    }
}

#[component]
fn HistoryWorkspace() -> Element {
    let app_state = use_context::<Signal<AppState>>();
    let history = app_state.read().query_history.clone();
    
    rsx! {
        div {
            class: "history-workspace",
            
            if history.read().is_empty() {
                div {
                    class: "empty-state",
                    "No queries in history yet"
                }
            } else {
                div {
                    class: "history-list",
                    
                    for item in history.read().iter().rev() {
                        div {
                            key: "{item.id}",
                            class: if item.success { "history-item success" } else { "history-item error" },
                            
                            div {
                                class: "history-query",
                                "{item.query}"
                            }
                            
                            div {
                                class: "history-meta",
                                span { "{item.execution_time_ms}ms" }
                            }
                        }
                    }
                }
            }
        }
    }
}