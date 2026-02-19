use dioxus::prelude::*;

use crate::state::AppState;

#[component]
pub fn QueryEditor(on_execute: EventHandler<String>, is_executing: Signal<bool>) -> Element {
    let mut app_state = use_context::<Signal<AppState>>();
    let mut query_text = use_signal(String::new);

    // Consume pending_query from AppState (set by history click)
    use_effect(move || {
        let pending = app_state.read().pending_query.read().clone();
        if let Some(query) = pending {
            query_text.set(query);
            app_state.write().pending_query.set(None);
        }
    });

    rsx! {
        div {
            class: "query-editor",
            
            // Toolbar
            div {
                class: "query-toolbar",
                
                button {
                    class: "btn btn-primary",
                    disabled: *is_executing.read(),
                    onclick: move |_| {
                        if !query_text.read().trim().is_empty() {
                            on_execute.call(query_text.read().clone());
                        }
                    },
                    
                    if *is_executing.read() {
                        "Executing..."
                    } else {
                        "▶ Execute"
                    }
                }
                
                button {
                    class: "btn",
                    onclick: move |_| query_text.set(String::new()),
                    "Clear"
                }
                
                div {
                    class: "query-shortcuts",
                    span { 
                        class: "shortcut-hint",
                        "Ctrl+Enter to execute" 
                    }
                }
            }
            
            // Editor area
            div {
                class: "editor-container",
                
                textarea {
                    class: "query-textarea",
                    value: "{query_text.read()}",
                    oninput: move |e| query_text.set(e.value()),
                    onkeydown: move |e| {
                        // Handle Ctrl+Enter or Cmd+Enter
                        if (e.modifiers().contains(Modifiers::CONTROL) || e.modifiers().contains(Modifiers::META)) && e.key() == Key::Enter {
                            if !query_text.read().trim().is_empty() && !*is_executing.read() {
                                on_execute.call(query_text.read().clone());
                            }
                        }
                    },
                    placeholder: "Enter your CQL query here...\n\nExamples:\n• SELECT * FROM keyspace.table LIMIT 10;\n• DESCRIBE KEYSPACES;\n• CREATE TABLE ...;",
                    spellcheck: "false"
                }
            }
        }
    }
}