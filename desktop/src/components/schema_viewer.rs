use dioxus::prelude::*;

use crate::cassandra::TableSchema;
use crate::state::AppState;

#[component]
pub fn SchemaViewer() -> Element {
    let app_state = use_context::<Signal<AppState>>();

    let mut keyspaces = use_signal(Vec::<String>::new);
    let mut selected_keyspace = use_signal(|| None::<String>);
    let mut tables = use_signal(Vec::<String>::new);
    let mut selected_table = use_signal(|| None::<String>);
    let mut schema = use_signal(|| None::<TableSchema>);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    // Load keyspaces on mount
    use_effect(move || {
        let cm = app_state.read().connection_manager.clone();
        spawn(async move {
            if let Some(conn) = cm.get_active_connection().await {
                // Pre-select the configured/first-user keyspace
                let resolved = conn.resolve_keyspace().await;
                match conn.list_keyspaces().await {
                    Ok(ks) => {
                        keyspaces.set(ks);
                        if let Some(default_ks) = resolved {
                            selected_keyspace.set(Some(default_ks));
                        }
                    }
                    Err(e) => error.set(Some(format!("Failed to load keyspaces: {}", e))),
                }
            }
        });
    });

    // Load tables when keyspace changes
    use_effect(move || {
        let ks = selected_keyspace.read().clone();
        let cm = app_state.read().connection_manager.clone();
        // Reset downstream state
        selected_table.set(None);
        schema.set(None);
        tables.set(Vec::new());

        if let Some(ks) = ks {
            spawn(async move {
                if let Some(conn) = cm.get_active_connection().await {
                    match conn.list_tables(&ks).await {
                        Ok(t) => tables.set(t),
                        Err(e) => error.set(Some(format!("Failed to load tables: {}", e))),
                    }
                }
            });
        }
    });

    // Load schema when table changes
    use_effect(move || {
        let ks = selected_keyspace.read().clone();
        let tbl = selected_table.read().clone();
        let cm = app_state.read().connection_manager.clone();
        schema.set(None);

        if let (Some(ks), Some(tbl)) = (ks, tbl) {
            loading.set(true);
            error.set(None);
            spawn(async move {
                if let Some(conn) = cm.get_active_connection().await {
                    match conn.describe_table(&ks, &tbl).await {
                        Ok(s) => schema.set(Some(s)),
                        Err(e) => error.set(Some(format!("Failed to describe table: {}", e))),
                    }
                }
                loading.set(false);
            });
        }
    });

    rsx! {
        div {
            class: "schema-viewer",

            // Header with keyspace + table selectors
            div {
                class: "schema-header",

                select {
                    class: "select-keyspace",
                    value: "{selected_keyspace.read().as_deref().unwrap_or(\"\")}",
                    onchange: move |e| {
                        let v = if e.value().is_empty() { None } else { Some(e.value()) };
                        selected_keyspace.set(v);
                    },
                    option { value: "", "Select keyspace..." }
                    for ks in keyspaces.read().iter() {
                        option { value: "{ks}", "{ks}" }
                    }
                }

                if selected_keyspace.read().is_some() {
                    select {
                        class: "select-keyspace",
                        value: "{selected_table.read().as_deref().unwrap_or(\"\")}",
                        onchange: move |e| {
                            let v = if e.value().is_empty() { None } else { Some(e.value()) };
                            selected_table.set(v);
                        },
                        option { value: "", "Select table..." }
                        for t in tables.read().iter() {
                            option { value: "{t}", "{t}" }
                        }
                    }
                }
            }

            // Error display
            if let Some(err) = error.read().as_ref() {
                div { class: "empty-state", "{err}" }
            }

            // Loading indicator
            if *loading.read() {
                div { class: "loading-indicator", "Loading schema..." }
            }

            // Column details table
            if let Some(table_schema) = schema.read().as_ref() {
                if table_schema.columns.is_empty() {
                    div { class: "empty-state", "No columns found" }
                } else {
                    table {
                        class: "results-table",
                        thead {
                            tr {
                                th { div { class: "column-header", span { class: "column-name", "Column" } } }
                                th { div { class: "column-header", span { class: "column-name", "Type" } } }
                                th { div { class: "column-header", span { class: "column-name", "Kind" } } }
                                th { div { class: "column-header", span { class: "column-name", "Order" } } }
                            }
                        }
                        tbody {
                            for col in table_schema.columns.iter() {
                                tr {
                                    key: "{col.name}",
                                    class: match col.kind.as_str() {
                                        "partition_key" => "schema-row-partition",
                                        "clustering" => "schema-row-clustering",
                                        _ => "",
                                    },
                                    td { span { class: "column-name", "{col.name}" } }
                                    td { span { class: "column-type", "{col.data_type}" } }
                                    td { "{col.kind}" }
                                    td {
                                        if col.kind == "clustering" {
                                            "{col.clustering_order}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else if !*loading.read() && selected_table.read().is_none() && error.read().is_none() {
                div { class: "empty-state", "Select a keyspace and table to view its schema" }
            }
        }
    }
}
