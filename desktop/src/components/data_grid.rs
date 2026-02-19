use dioxus::prelude::*;
use serde_json::Value;

use crate::state::{AppState, DEFAULT_PAGE_SIZE};

#[derive(Clone, Debug)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
}

#[derive(Clone, Debug)]
pub struct QueryResult {
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<Vec<Value>>,
    pub execution_time_ms: u64,
    pub row_count: usize,
}

#[component]
pub fn DataGrid(
    #[props(optional)] table_name: Option<String>
) -> Element {
    let app_state = use_context::<Signal<AppState>>();
    let mut query_result = use_signal(|| None::<QueryResult>);
    let mut selected_row = use_signal(|| None::<usize>);
    let mut loading = use_signal(|| false);
    
    // Load table data when table_name changes
    use_effect(use_reactive!(|table_name| {
        // Clear previous results when table changes
        query_result.set(None);
        
        if let Some(ref table) = table_name {
            loading.set(true);
            let table_clone = table.clone();
            spawn(async move {
                if let Some(connection) = app_state.read().connection_manager.get_active_connection().await {
                    if let Some(keyspace) = connection.resolve_keyspace().await {
                        // Validate identifiers before interpolating into CQL
                        if let Err(e) = crate::cassandra::validate_cql_identifier(&keyspace) {
                            tracing::error!("Invalid keyspace name: {}", e);
                            query_result.set(None);
                            loading.set(false);
                            return;
                        }
                        if let Err(e) = crate::cassandra::validate_cql_identifier(&table_clone) {
                            tracing::error!("Invalid table name: {}", e);
                            query_result.set(None);
                            loading.set(false);
                            return;
                        }
                        let query = format!("SELECT * FROM {}.{} LIMIT {}", keyspace, table_clone, DEFAULT_PAGE_SIZE);
                        match connection.execute_query(&query).await {
                            Ok(result) => {
                                tracing::info!("Loaded {} rows from table {} in keyspace {}", result.row_count, table_clone, keyspace);
                                query_result.set(Some(result));
                            }
                            Err(e) => {
                                tracing::error!("Failed to load data from table {}: {}", table_clone, e);
                                query_result.set(None);
                            }
                        }
                    } else {
                        tracing::warn!("No keyspace available for data grid");
                        query_result.set(None);
                    }
                } else {
                    tracing::warn!("No active connection found");
                    query_result.set(None);
                }
                loading.set(false);
            });
        } else {
            loading.set(false);
        }
    }));
    
    rsx! {
        div {
            class: "data-grid",
            
            // Results header
            div {
                class: "results-header",
                
                div {
                    class: "results-info",
                    
                    if *loading.read() {
                        span { "Loading..." }
                    } else if let Some(result) = query_result.read().as_ref() {
                        if result.row_count > 0 {
                            span { "{result.row_count} rows" }
                            span { class: "separator", "•" }
                            span { "{result.execution_time_ms}ms" }
                        } else {
                            span { "No results to display" }
                        }
                    } else {
                        span { "Select a table to view data" }
                    }
                }
                
                div {
                    class: "results-actions",
                    
                    button {
                        class: "btn-icon",
                        title: "Export as CSV",
                        "📥"
                    }
                    
                    button {
                        class: "btn-icon",
                        title: "Copy results",
                        "📋"
                    }
                }
            }
            
            // Table container
            div {
                class: "table-container",
                
                table {
                    class: "data-table",
                    
                    if let Some(result) = query_result.read().as_ref() {
                        // Table header
                        thead {
                            tr {
                                for column in result.columns.iter() {
                                    th {
                                        div {
                                            class: "column-header",
                                            
                                            span {
                                                class: "column-name",
                                                "{column.name}"
                                            }
                                            
                                            span {
                                                class: "column-type",
                                                "{column.data_type}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Table body
                        tbody {
                            if result.rows.is_empty() {
                                tr {
                                    td {
                                        colspan: "{result.columns.len()}",
                                        class: "empty-state",
                                        "No data available"
                                    }
                                }
                            } else {
                                for (idx, row) in result.rows.iter().enumerate() {
                                    tr {
                                        key: "{idx}",
                                        class: if *selected_row.read() == Some(idx) { "selected" } else { "" },
                                        onclick: move |_| selected_row.set(Some(idx)),
                                        
                                        for (col_idx, value) in row.iter().enumerate() {
                                            td {
                                                key: "{col_idx}",
                                                "{format_value(value)}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // No data loaded yet
                        tbody {
                            tr {
                                td {
                                    colspan: "4",
                                    class: "empty-state",
                                    "Select a table or execute a query to see results"
                                }
                            }
                        }
                    }
                }
            }
            
            // Pagination
            div {
                class: "pagination",
                
                button {
                    class: "btn-icon",
                    disabled: true,
                    "◀"
                }
                
                span {
                    class: "page-info",
                    "Page 1 of 1"
                }
                
                button {
                    class: "btn-icon",
                    disabled: true,
                    "▶"
                }
            }
        }
    }
}

fn format_value(value: &Value) -> String {
    match value {
        Value::Null => "NULL".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(arr) => format!("[{} items]", arr.len()),
        Value::Object(obj) => format!("{{{}}} fields", obj.len()),
    }
}