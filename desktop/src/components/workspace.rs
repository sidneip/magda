use dioxus::prelude::*;

use crate::state::{ActiveTab, AppState, QueryVariable, DEFAULT_PAGE_SIZE};
use super::query_editor::QueryEditor;
use super::data_grid::DataGrid;
use super::schema_viewer::SchemaViewer;
use super::variables_panel::VariablesPanel;

fn substitute_variables(query: &str, vars: &[QueryVariable]) -> String {
    let mut result = query.to_string();
    for var in vars {
        if !var.name.is_empty() {
            result = result.replace(&format!("{{{{{}}}}}", var.name), &var.value);
        }
    }
    result
}

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

                TabButton {
                    label: "Variables",
                    is_active: *active_tab.read() == ActiveTab::Variables,
                    onclick: move |_| active_tab.set(ActiveTab::Variables)
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
                        SchemaViewer {}
                    },
                    ActiveTab::History => rsx! {
                        HistoryWorkspace {}
                    },
                    ActiveTab::Variables => rsx! {
                        VariablesPanel {}
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
    let mut app_state = use_context::<Signal<AppState>>();

    // Full cached result from the last query execution
    let mut cached_result = use_signal(|| None::<crate::components::data_grid::QueryResult>);
    let mut query_error = use_signal(|| None::<String>);
    let mut is_executing = use_signal(|| false);
    let mut current_page = use_signal(|| 1u32);
    let mut original_query = use_signal(|| String::new());

    let page_size = DEFAULT_PAGE_SIZE as usize;

    // Derive the current page slice from cache
    let page_rows = {
        let cached = cached_result.read();
        if let Some(ref result) = *cached {
            let start = ((*current_page.read() as usize) - 1) * page_size;
            let end = (start + page_size).min(result.rows.len());
            if start < result.rows.len() {
                Some((result.columns.clone(), result.rows[start..end].to_vec(), result.execution_time_ms, result.row_count))
            } else {
                Some((result.columns.clone(), vec![], result.execution_time_ms, result.row_count))
            }
        } else {
            None
        }
    };

    let total_pages = {
        let cached = cached_result.read();
        cached.as_ref().map(|r| {
            let total = r.rows.len();
            ((total + page_size - 1) / page_size).max(1) as u32
        }).unwrap_or(1)
    };

    // Execute query: fetch all rows once and cache them
    let mut run_query = move |query: String| {
        original_query.set(query.clone());
        current_page.set(1);
        is_executing.set(true);
        query_error.set(None);
        cached_result.set(None);

        // Substitute variables before execution, keep original for history
        let vars = app_state.read().query_variables.read().clone();
        let substituted = substitute_variables(&query, &vars);

        spawn(async move {
            let cm = app_state.read().connection_manager.clone();
            if let Some(connection) = cm.get_active_connection().await {
                tracing::debug!("Executing query: {}", substituted);
                match connection.execute_query(&substituted).await {
                    Ok(result) => {
                        let execution_time = result.execution_time_ms;
                        tracing::info!("Query returned {} rows in {}ms", result.row_count, execution_time);
                        cached_result.set(Some(result));

                        // Store original query with placeholders in history
                        let history_item = crate::state::QueryHistoryItem {
                            id: uuid::Uuid::new_v4(),
                            query: query.clone(),
                            success: true,
                            execution_time_ms: execution_time,
                            executed_at: chrono::Utc::now(),
                        };
                        app_state.write().query_history.write().push(history_item);
                    }
                    Err(e) => {
                        let error_msg = format!("Query failed: {}", e);
                        tracing::error!("{}", error_msg);
                        query_error.set(Some(error_msg));

                        let history_item = crate::state::QueryHistoryItem {
                            id: uuid::Uuid::new_v4(),
                            query: query.clone(),
                            success: false,
                            execution_time_ms: 0,
                            executed_at: chrono::Utc::now(),
                        };
                        app_state.write().query_history.write().push(history_item);
                    }
                }
            } else {
                query_error.set(Some("No active connection available".to_string()));
            }
            is_executing.set(false);
        });
    };

    rsx! {
        div {
            class: "query-workspace",

            // Query editor at the top
            div {
                class: "query-editor-container",
                QueryEditor {
                    is_executing: is_executing,
                    on_execute: move |query: String| {
                        run_query(query);
                    }
                }
            }

            // Results grid at the bottom
            div {
                class: "query-results-container",

                if let Some(error) = query_error.read().as_ref() {
                    div {
                        class: "query-error",
                        div { class: "error-header", "Error" }
                        div { class: "error-message", "{error}" }
                    }
                } else if let Some((columns, rows, exec_time, total_rows)) = page_rows {
                    div {
                        class: "query-results",

                        div {
                            class: "results-header",
                            div {
                                class: "results-info",
                                span { "Results: {total_rows} rows in {exec_time}ms" }
                                if total_pages > 1 {
                                    span {
                                        class: "pagination-hint",
                                        " (page {current_page.read()} of {total_pages})"
                                    }
                                }
                            }

                            div {
                                class: "results-actions",
                                button {
                                    class: "btn-small",
                                    onclick: move |_| {
                                        if let Some(ref result) = *cached_result.read() {
                                            let csv = super::data_grid::export_to_csv(result);
                                            spawn(async move {
                                                if let Some(path) = rfd::AsyncFileDialog::new()
                                                    .set_file_name("export.csv")
                                                    .add_filter("CSV", &["csv"])
                                                    .save_file()
                                                    .await
                                                {
                                                    if let Err(e) = tokio::fs::write(path.path(), csv.as_bytes()).await {
                                                        tracing::error!("Failed to write CSV: {}", e);
                                                    }
                                                }
                                            });
                                        }
                                    },
                                    "Export CSV"
                                }
                            }

                            if total_pages > 1 {
                                div {
                                    class: "pagination-controls",

                                    button {
                                        class: "btn btn-secondary",
                                        disabled: *current_page.read() <= 1,
                                        onclick: move |_| {
                                            let p = *current_page.read();
                                            if p > 1 { current_page.set(p - 1); }
                                        },
                                        "Previous"
                                    }

                                    span {
                                        class: "page-info",
                                        "Page {current_page.read()} / {total_pages}"
                                    }

                                    button {
                                        class: "btn btn-secondary",
                                        disabled: *current_page.read() >= total_pages,
                                        onclick: move |_| {
                                            let p = *current_page.read();
                                            if p < total_pages { current_page.set(p + 1); }
                                        },
                                        "Next"
                                    }
                                }
                            }
                        }

                        if !rows.is_empty() {
                            div {
                                class: "results-table-container",
                                table {
                                    class: "results-table",
                                    thead {
                                        tr {
                                            for column in columns.iter() {
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
                                        for row in rows.iter() {
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
                } else if *is_executing.read() {
                    div {
                        class: "executing-indicator",
                        "Executing query..."
                    }
                } else {
                    div {
                        class: "no-results",
                        p { "Run a query to see results" }
                        p {
                            class: "empty-state-hint",
                            "Use Ctrl+Enter or click Execute"
                        }
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
    
    use_effect(move || {
        spawn(async move {
            if let Some(conn) = app_state.read().connection_manager.get_active_connection().await {
                if let Some(keyspace) = conn.resolve_keyspace().await {
                    match conn.list_tables(&keyspace).await {
                        Ok(t) => tables.set(t),
                        Err(e) => tracing::error!("Failed to load tables: {}", e),
                    }
                } else {
                    tables.set(Vec::new());
                }
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
fn HistoryWorkspace() -> Element {
    let mut app_state = use_context::<Signal<AppState>>();
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
                        {
                            let query = item.query.clone();
                            rsx! {
                                div {
                                    key: "{item.id}",
                                    class: if item.success { "history-item success" } else { "history-item error" },
                                    style: "cursor: pointer;",
                                    onclick: move |_| {
                                        app_state.write().pending_query.set(Some(query.clone()));
                                        app_state.read().active_tab.clone().set(ActiveTab::Query);
                                    },

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
    }
}