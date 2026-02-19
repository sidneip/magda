use dioxus::prelude::*;

use crate::cql_tokenizer;
use crate::state::AppState;

const MAX_SUGGESTIONS: usize = 12;

/// Context keywords that trigger table-name suggestions instead of
/// generic keyword completions.
fn is_table_context(keyword: &str) -> bool {
    matches!(keyword, "FROM" | "INTO" | "TABLE" | "JOIN" | "UPDATE")
}

#[component]
pub fn CodeEditor(
    value: Signal<String>,
    on_execute: EventHandler<String>,
    is_executing: Signal<bool>,
) -> Element {
    let app_state = use_context::<Signal<AppState>>();

    // Autocomplete state
    let mut suggestions: Signal<Vec<String>> = use_signal(Vec::new);
    let mut selected_idx: Signal<usize> = use_signal(|| 0);
    let mut ac_visible: Signal<bool> = use_signal(|| false);
    let mut ac_word_start: Signal<usize> = use_signal(|| 0);

    // Highlighted HTML — recomputes reactively whenever `value` changes
    let highlighted = use_memo(move || {
        let src = value.read().clone();
        let tokens = cql_tokenizer::tokenize(&src);
        cql_tokenizer::to_highlighted_html(&tokens)
    });

    // Set up scroll sync once on mount
    use_effect(move || {
        spawn(async move {
            let _ = document::eval(
                r#"
                (function() {
                    const ta = document.getElementById('cql-textarea');
                    const pre = document.getElementById('cql-highlight');
                    if (ta && pre) {
                        ta.addEventListener('scroll', function() {
                            pre.scrollTop = ta.scrollTop;
                            pre.scrollLeft = ta.scrollLeft;
                        });
                    }
                })();
                "#,
            );
        });
    });

    // Fetch cursor position and compute suggestions
    let compute_suggestions = move |text: String| {
        let connection_manager = app_state.read().connection_manager.clone();
        spawn(async move {
            // Get cursor position from the textarea
            let cursor_eval = document::eval(
                r#"
                (function() {
                    const ta = document.getElementById('cql-textarea');
                    return ta ? ta.selectionStart : 0;
                })()
                "#,
            );
            let cursor: usize = match cursor_eval.await {
                Ok(val) => val.as_f64().unwrap_or(0.0) as usize,
                Err(_) => return,
            };

            let (partial, word_start) = cql_tokenizer::word_at_cursor(&text, cursor);
            if partial.len() < 2 {
                ac_visible.set(false);
                return;
            }

            ac_word_start.set(word_start);

            // Check if we're in a table-name context
            let prev_kw = cql_tokenizer::keyword_before_cursor(&text, cursor);
            let in_table_ctx = prev_kw.as_deref().map_or(false, is_table_context);

            let mut items: Vec<String> = Vec::new();

            if in_table_ctx {
                // Try to get table names from the active connection
                if let Some(conn) = connection_manager.get_active_connection().await {
                    if let Some(ks) = conn.resolve_keyspace().await {
                        if let Ok(tables) = conn.list_tables(&ks).await {
                            let upper_partial = partial.to_ascii_uppercase();
                            items = tables
                                .into_iter()
                                .filter(|t| t.to_ascii_uppercase().starts_with(&upper_partial))
                                .take(MAX_SUGGESTIONS)
                                .collect();
                        }
                    }
                }
            }

            // Fall back to (or supplement with) keyword/type/function completions
            if items.is_empty() {
                items = cql_tokenizer::suggest_completions(partial, MAX_SUGGESTIONS)
                    .into_iter()
                    .map(String::from)
                    .collect();
            }

            if items.is_empty() {
                ac_visible.set(false);
            } else {
                suggestions.set(items);
                selected_idx.set(0);
                ac_visible.set(true);
            }
        });
    };

    // Apply the selected completion
    let mut apply_completion = move |item: String| {
        let mut text = value.read().clone();
        let word_start = *ac_word_start.read();

        // Get current cursor to know where the partial word ends
        let partial_end = {
            let bytes = text.as_bytes();
            let mut end = word_start;
            while end < bytes.len()
                && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_')
            {
                end += 1;
            }
            end
        };

        text.replace_range(word_start..partial_end, &item);
        let new_cursor = word_start + item.len();
        value.set(text);
        ac_visible.set(false);

        // Restore cursor position after the inserted text
        let js = format!(
            r#"
            (function() {{
                const ta = document.getElementById('cql-textarea');
                if (ta) {{
                    ta.focus();
                    ta.setSelectionRange({pos}, {pos});
                }}
            }})();
            "#,
            pos = new_cursor
        );
        spawn(async move {
            let _ = document::eval(&js);
        });
    };

    // Event handlers
    let on_input = move |e: Event<FormData>| {
        let text = e.value();
        value.set(text.clone());
        compute_suggestions(text);
    };

    let on_keydown = move |e: Event<KeyboardData>| {
        let is_ac = *ac_visible.read();

        // Ctrl+Enter / Cmd+Enter → execute
        if (e.modifiers().contains(Modifiers::CONTROL)
            || e.modifiers().contains(Modifiers::META))
            && e.key() == Key::Enter
        {
            ac_visible.set(false);
            if !value.read().trim().is_empty() && !*is_executing.read() {
                on_execute.call(value.read().clone());
            }
            return;
        }

        if is_ac {
            match e.key() {
                Key::ArrowDown => {
                    e.prevent_default();
                    let len = suggestions.read().len();
                    let cur = *selected_idx.read();
                    if len > 0 {
                        selected_idx.set((cur + 1) % len);
                    }
                }
                Key::ArrowUp => {
                    e.prevent_default();
                    let len = suggestions.read().len();
                    let cur = *selected_idx.read();
                    if len > 0 {
                        selected_idx.set(if cur == 0 { len - 1 } else { cur - 1 });
                    }
                }
                Key::Tab | Key::Enter => {
                    e.prevent_default();
                    let idx = *selected_idx.read();
                    let item = suggestions.read().get(idx).cloned();
                    if let Some(item) = item {
                        apply_completion(item);
                    }
                }
                Key::Escape => {
                    e.prevent_default();
                    ac_visible.set(false);
                }
                _ => {}
            }
        }
    };

    rsx! {
        div {
            class: "code-editor-wrapper",

            // The textarea captures all input
            textarea {
                id: "cql-textarea",
                class: "cql-textarea",
                value: "{value.read()}",
                oninput: on_input,
                onkeydown: on_keydown,
                placeholder: "Enter your CQL query here...\n\nExamples:\n\u{2022} SELECT * FROM keyspace.table LIMIT 10;\n\u{2022} DESCRIBE KEYSPACES;\n\u{2022} CREATE TABLE ...;",
                spellcheck: "false",
                autocomplete: "off",
            }

            // The highlight overlay
            pre {
                id: "cql-highlight",
                class: "cql-highlight",
                "aria-hidden": "true",
                dangerous_inner_html: "{highlighted}"
            }

            // Autocomplete dropdown
            if *ac_visible.read() && !suggestions.read().is_empty() {
                div {
                    class: "cql-autocomplete",

                    for (i, item) in suggestions.read().iter().enumerate() {
                        div {
                            key: "{item}",
                            class: if i == *selected_idx.read() { "ac-item ac-item-selected" } else { "ac-item" },
                            // mousedown fires before blur, so the textarea keeps focus
                            onmousedown: {
                                let item = item.clone();
                                move |e: Event<MouseData>| {
                                    e.prevent_default();
                                    apply_completion(item.clone());
                                }
                            },
                            "{item}"
                        }
                    }
                }
            }
        }
    }
}
