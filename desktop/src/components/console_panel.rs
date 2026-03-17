use dioxus::prelude::*;

use crate::state::{AppState, ConsoleCategory, StatusLevel};

#[component]
pub fn ConsolePanel() -> Element {
    let mut app_state = use_context::<Signal<AppState>>();
    let console_log = app_state.read().console_log;
    let mut filter = use_signal(|| None::<ConsoleCategory>);

    let entries: Vec<_> = console_log
        .read()
        .iter()
        .filter(|e| {
            filter
                .read()
                .as_ref()
                .is_none_or(|f| e.category == *f)
        })
        .cloned()
        .collect();

    rsx! {
        div {
            class: "console-panel",

            // Console toolbar
            div {
                class: "console-toolbar",

                span {
                    class: "console-title",
                    "Console"
                }

                div {
                    class: "console-filters",

                    button {
                        class: if filter.read().is_none() { "console-filter-btn active" } else { "console-filter-btn" },
                        onclick: move |_| filter.set(None),
                        "All"
                    }
                    button {
                        class: if *filter.read() == Some(ConsoleCategory::Connection) { "console-filter-btn active" } else { "console-filter-btn" },
                        onclick: move |_| filter.set(Some(ConsoleCategory::Connection)),
                        "Connection"
                    }
                    button {
                        class: if *filter.read() == Some(ConsoleCategory::Query) { "console-filter-btn active" } else { "console-filter-btn" },
                        onclick: move |_| filter.set(Some(ConsoleCategory::Query)),
                        "Query"
                    }
                    button {
                        class: if *filter.read() == Some(ConsoleCategory::System) { "console-filter-btn active" } else { "console-filter-btn" },
                        onclick: move |_| filter.set(Some(ConsoleCategory::System)),
                        "System"
                    }
                }

                div {
                    class: "console-actions",

                    button {
                        class: "btn-small",
                        onclick: move |_| {
                            app_state.write().console_log.write().clear();
                        },
                        "Clear"
                    }

                    button {
                        class: "btn-small",
                        onclick: move |_| {
                            app_state.write().console_visible.set(false);
                        },
                        "x"
                    }
                }
            }

            // Console entries
            div {
                class: "console-entries",

                if entries.is_empty() {
                    div {
                        class: "console-empty",
                        "No console output yet"
                    }
                } else {
                    for (idx, entry) in entries.iter().rev().enumerate() {
                        {
                            let ts = entry.timestamp.format("%H:%M:%S").to_string();
                            let entry_class = match entry.level {
                                StatusLevel::Info => "console-entry console-entry-info",
                                StatusLevel::Success => "console-entry console-entry-success",
                                StatusLevel::Error => "console-entry console-entry-error",
                            };
                            let cat_label = match entry.category {
                                ConsoleCategory::Connection => "CONN",
                                ConsoleCategory::Query => "QUERY",
                                ConsoleCategory::System => "SYS",
                            };
                            rsx! {
                                div {
                                    key: "{idx}",
                                    class: entry_class,

                                    span {
                                        class: "console-timestamp",
                                        "{ts}"
                                    }

                                    span {
                                        class: "console-category",
                                        "{cat_label}"
                                    }

                                    span {
                                        class: "console-message",
                                        "{entry.message}"
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
