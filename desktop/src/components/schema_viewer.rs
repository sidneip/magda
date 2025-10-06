use dioxus::prelude::*;

#[component]
pub fn SchemaViewer() -> Element {
    rsx! {
        div {
            class: "schema-viewer",
            
            div {
                class: "schema-header",
                h3 { "Schema Information" }
            }
            
            div {
                class: "schema-content",
                
                // Keyspace selector
                div {
                    class: "keyspace-selector",
                    
                    select {
                        class: "select-keyspace",
                        option { "Select a keyspace..." }
                    }
                }
                
                // Table list
                div {
                    class: "table-list",
                    
                    div {
                        class: "empty-state",
                        "Connect to a database and select a keyspace to view schema"
                    }
                }
            }
        }
    }
}