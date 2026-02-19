use dioxus::prelude::*;

use crate::config::save_saved_queries;
use crate::state::{AppState, SavedQuery};

#[component]
pub fn QueryEditor(on_execute: EventHandler<String>, is_executing: Signal<bool>) -> Element {
    let mut app_state = use_context::<Signal<AppState>>();
    let mut query_text = app_state.read().query_text;
    let mut show_save_input = use_signal(|| false);
    let mut save_name = use_signal(String::new);

    // Consume pending_query from AppState (set by history/saved query click)
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

                button {
                    class: "btn",
                    disabled: query_text.read().trim().is_empty(),
                    onclick: move |_| {
                        show_save_input.set(true);
                        save_name.set(String::new());
                    },
                    "Save"
                }

                if *show_save_input.read() {
                    input {
                        class: "save-query-input",
                        r#type: "text",
                        placeholder: "Query name...",
                        value: "{save_name.read()}",
                        oninput: move |e| save_name.set(e.value()),
                        onkeydown: move |e| {
                            if e.key() == Key::Enter && !save_name.read().trim().is_empty() {
                                let mut saved = app_state.read().saved_queries;
                                saved.write().push(SavedQuery {
                                    id: uuid::Uuid::new_v4(),
                                    name: save_name.read().trim().to_string(),
                                    query: query_text.read().clone(),
                                });
                                save_saved_queries(&saved.read());
                                show_save_input.set(false);
                            } else if e.key() == Key::Escape {
                                show_save_input.set(false);
                            }
                        },
                        autofocus: true
                    }

                    button {
                        class: "btn-small",
                        disabled: save_name.read().trim().is_empty(),
                        onclick: move |_| {
                            if !save_name.read().trim().is_empty() {
                                let mut saved = app_state.read().saved_queries;
                                saved.write().push(SavedQuery {
                                    id: uuid::Uuid::new_v4(),
                                    name: save_name.read().trim().to_string(),
                                    query: query_text.read().clone(),
                                });
                                save_saved_queries(&saved.read());
                                show_save_input.set(false);
                            }
                        },
                        "OK"
                    }

                    button {
                        class: "btn-small",
                        onclick: move |_| show_save_input.set(false),
                        "Cancel"
                    }
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

                super::code_editor::CodeEditor {
                    value: query_text,
                    on_execute: on_execute,
                    is_executing: is_executing,
                }
            }
        }
    }
}
